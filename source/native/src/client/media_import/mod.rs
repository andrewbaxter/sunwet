pub mod gather_audio;
pub mod gather_comic;
pub mod gather_epub;
pub mod gather_video;
pub mod gather;

use {
    crate::client::req::{
        self,
        ENV_SUNWET,
    },
    aargvark::Aargvark,
    by_address::ByAddress,
    chrono::Utc,
    flowcontrol::shed,
    gather::GatherMedia,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    shared::{
        interface::{
            cli::{
                CliCommit,
                CliNode,
                CliTriple,
            },
            ont::{
                OBJ_IS_ALBUM,
                OBJ_IS_ARTIST,
                OBJ_IS_DOC,
                OBJ_IS_TRACK,
                OBJ_MEDIA_AUDIO,
                OBJ_MEDIA_BOOK,
                OBJ_MEDIA_COMIC,
                OBJ_MEDIA_IMAGE,
                OBJ_MEDIA_VIDEO,
                PREDICATE_ADD_TIMESTAMP,
                PREDICATE_ARTIST,
                PREDICATE_COVER,
                PREDICATE_DOC,
                PREDICATE_FILE,
                PREDICATE_INDEX,
                PREDICATE_IS,
                PREDICATE_LANG,
                PREDICATE_MEDIA,
                PREDICATE_NAME,
                PREDICATE_SUPERINDEX,
                PREDICATE_TRACK,
            },
            query::{
                ChainHead,
                ChainRoot,
                ChainTail,
                FilterExpr,
                FilterExprExistance,
                FilterExprExistsType,
                FilterExprJunction,
                FilterSuffix,
                FilterSuffixSimple,
                FilterSuffixSimpleOperator,
                JunctionType,
                MoveDirection,
                Query,
                QuerySuffix,
                Step,
                StepMove,
                StepSpecific,
                StrValue,
                Value,
            },
            triple::Node,
            wire::{
                ReqQuery,
                RespQueryRows,
                TreeNode,
            },
        },
        query_parser::compile_query,
    },
    std::{
        cell::RefCell,
        collections::{
            hash_map::Entry,
            BTreeMap,
            BTreeSet,
            HashMap,
            HashSet,
        },
        env,
        fs::{
            create_dir_all,
            write,
            File,
        },
        io,
        os::unix::ffi::OsStrExt,
        path::{
            Path,
            PathBuf,
        },
        rc::Rc,
    },
    uuid::Uuid,
    walkdir::WalkDir,
};

pub async fn node_id(
    log: &Log,
    query_cache: &mut HashMap<&'static str, Query>,
    query: &'static str,
    parameters: HashMap<String, Node>,
) -> Result<Node, loga::Error> {
    let query = match query_cache.entry(query) {
        Entry::Occupied(en) => en.get().clone(),
        Entry::Vacant(en) => en.insert(compile_query(query).map_err(loga::err)?).clone(),
    };
    return node_id_direct(log, query, parameters).await;
}

pub async fn node_id_artist(
    log: &Log,
    query_cache: &mut HashMap<&'static str, Query>,
    name: &str,
) -> Result<Node, loga::Error> {
    return node_id(
        log,
        query_cache,
        r#"$artist_name -< "sunwet/1/name" ?( -> "sunwet/1/is" == "sunwet/1/artist" ) { => id }"#,
        [(format!("artist_name"), Node::from_str(name))].into_iter().collect(),
    ).await;
}

pub async fn node_id_direct(
    log: &Log,
    query: Query,
    parameters: HashMap<String, Node>,
) -> Result<Node, loga::Error> {
    if env::var_os(ENV_SUNWET).is_none() {
        return Ok(Node::Value(serde_json::Value::String(Uuid::new_v4().hyphenated().to_string())));
    }
    let resp = req::req_simple(&log, ReqQuery {
        query: query.clone(),
        parameters: parameters.clone(),
        pagination: None,
    }).await?.rows;
    match resp {
        RespQueryRows::Scalar(rows) => {
            let mut rows_iter = rows.iter();
            let Some(first) = rows_iter.next() else {
                return Ok(Node::Value(serde_json::Value::String(Uuid::new_v4().hyphenated().to_string())));
            };
            if rows_iter.next().is_some() {
                return Err(
                    loga::err_with(
                        "Imported node id can't be matched, multiple potential existing nodes found",
                        ea!(query = query.dbg_str(), params = parameters.dbg_str(), res = rows.dbg_str()),
                    ),
                );
            }
            return Ok(first.clone());
        },
        RespQueryRows::Record(rows) => {
            let mut rows_iter = rows.iter();
            let Some(first) = rows_iter.next() else {
                return Ok(Node::Value(serde_json::Value::String(Uuid::new_v4().hyphenated().to_string())));
            };
            if rows_iter.next().is_some() {
                return Err(
                    loga::err_with(
                        "Imported node id can't be matched, multiple potential existing nodes found",
                        ea!(query = query.dbg_str(), params = parameters.dbg_str(), res = rows.dbg_str()),
                    ),
                );
            }
            let Some(id) = first.get("id") else {
                return Err(
                    loga::err_with(
                        "Assertion! Returned record missing [id] field",
                        ea!(query = query.dbg_str(), params = parameters.dbg_str(), res = rows.dbg_str()),
                    ),
                );
            };
            let TreeNode::Scalar(id) = id else {
                return Err(
                    loga::err_with(
                        "Assertion! Found id is not a scalar node (is array or record; bad query)",
                        ea!(query = query.dbg_str(), params = parameters.dbg_str(), res = rows.dbg_str()),
                    ),
                );
            };
            return Ok(id.clone());
        },
    }
}

pub fn node_upload(root: &Path, p: &Path) -> CliNode {
    return CliNode::Upload(p.strip_prefix(root).unwrap().to_path_buf());
}

pub fn node_value_str(v: &str) -> CliNode {
    return CliNode::Value(serde_json::Value::String(v.to_string()));
}

pub fn node_node(v: &Node) -> CliNode {
    match v {
        Node::File(v) => return CliNode::File(v.clone()),
        Node::Value(v) => return CliNode::Value(v.clone()),
    }
}

pub fn node_value_f64(v: f64) -> CliNode {
    return CliNode::Value(serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap()));
}

pub fn triple(sub: &CliNode, pred: &str, obj: &CliNode) -> CliTriple {
    return CliTriple {
        subject: sub.clone(),
        predicate: pred.to_string(),
        object: obj.clone(),
    };
}

pub fn obj_is_album() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_IS_ALBUM.to_string()));
}

pub fn obj_is_artist() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_IS_ARTIST.to_string()));
}

pub fn obj_media_audio() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_MEDIA_AUDIO.to_string()));
}

pub fn obj_media_video() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_MEDIA_VIDEO.to_string()));
}

pub fn obj_media_comic() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_MEDIA_COMIC.to_string()));
}

pub fn obj_media_book() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_MEDIA_BOOK.to_string()));
}

pub fn obj_media_image() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_MEDIA_IMAGE.to_string()));
}

pub fn obj_is_track() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_IS_TRACK.to_string()));
}

pub fn obj_is_document() -> CliNode {
    return CliNode::Value(serde_json::Value::String(OBJ_IS_DOC.to_string()));
}

fn is_image(p: &[u8]) -> bool {
    match p {
        b"png" | b"jpg" | b"jfif" | b"bmp" | b"tif" | b"gif" | b"webp" | b"webm" => true,
        _ => false,
    }
}

fn is_audio(p: &[u8]) -> bool {
    match p {
        b"mp3" | b"m4a" | b"aac" | b"ogg" | b"flac" | b"alac" => true,
        _ => false,
    }
}

fn is_video(p: &[u8]) -> bool {
    match p {
        b"mkv" => true,
        _ => false,
    }
}

fn is_comic(p: &[u8]) -> bool {
    match p {
        b"cbz" | b"cbr" | b"cb7" => true,
        _ => false,
    }
}

fn is_epub(p: &[u8]) -> bool {
    match p {
        b"epub" => true,
        _ => false,
    }
}

fn is_doc(p: &[u8]) -> bool {
    match p {
        b"txt" | b"md" | b"doc" | b"docx" | b"odt" | b"pdf" | b"rst" => true,
        _ => false,
    }
}

fn query_album_track(album: Node, superindex: Option<f64>, index: Option<f64>, name: &Option<String>) -> Query {
    let mut filter = vec![];
    {
        let subchain = ChainHead {
            root: None,
            steps: vec![Step {
                specific: StepSpecific::Move(StepMove {
                    dir: MoveDirection::Forward,
                    predicate: StrValue::Literal(PREDICATE_SUPERINDEX.to_string()),
                    filter: None,
                }),
                sort: None,
                first: false,
            }],
        };
        if let Some(superindex) = superindex {
            filter.push(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::Exists,
                subchain: subchain,
                suffix: Some(FilterSuffix::Simple(FilterSuffixSimple {
                    op: FilterSuffixSimpleOperator::Eq,
                    value: Value::Literal(
                        Node::Value(serde_json::Value::Number(serde_json::Number::from_f64(superindex).unwrap())),
                    ),
                })),
            }));
        } else {
            filter.push(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::DoesntExist,
                subchain: subchain,
                suffix: None,
            }));
        }
    }
    {
        let subchain = ChainHead {
            root: None,
            steps: vec![Step {
                specific: StepSpecific::Move(StepMove {
                    dir: MoveDirection::Forward,
                    predicate: StrValue::Literal(PREDICATE_INDEX.to_string()),
                    filter: None,
                }),
                sort: None,
                first: false,
            }],
        };
        if let Some(index) = index {
            filter.push(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::Exists,
                subchain: subchain,
                suffix: Some(FilterSuffix::Simple(FilterSuffixSimple {
                    op: FilterSuffixSimpleOperator::Eq,
                    value: Value::Literal(
                        Node::Value(serde_json::Value::Number(serde_json::Number::from_f64(index).unwrap())),
                    ),
                })),
            }));
        } else {
            filter.push(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::DoesntExist,
                subchain: subchain,
                suffix: None,
            }));
        }
    }
    if let Some(name) = name {
        filter.push(FilterExpr::Exists(FilterExprExistance {
            type_: FilterExprExistsType::Exists,
            subchain: ChainHead {
                root: None,
                steps: vec![Step {
                    specific: StepSpecific::Move(StepMove {
                        dir: MoveDirection::Forward,
                        predicate: StrValue::Literal(PREDICATE_NAME.to_string()),
                        filter: None,
                    }),
                    sort: None,
                    first: false,
                }],
            },
            suffix: Some(FilterSuffix::Simple(FilterSuffixSimple {
                op: FilterSuffixSimpleOperator::Eq,
                value: Value::Literal(Node::Value(serde_json::Value::String(name.to_string()))),
            })),
        }));
    }
    return Query {
        chain_head: ChainHead {
            root: Some(ChainRoot::Value(Value::Literal(album))),
            steps: vec![Step {
                specific: StepSpecific::Move(StepMove {
                    dir: MoveDirection::Forward,
                    predicate: StrValue::Literal(PREDICATE_TRACK.to_string()),
                    filter: Some(FilterExpr::Junction(FilterExprJunction {
                        type_: JunctionType::And,
                        subexprs: filter,
                    })),
                }),
                sort: None,
                first: false,
            }],
        },
        suffix: Some(QuerySuffix {
            chain_tail: ChainTail {
                bind: Some(format!("id")),
                subchains: Default::default(),
            },
            sort: None,
        }),
    };
}

async fn import_dir(log: &Log, root_dir: &PathBuf) -> Result<(), loga::Error> {
    let sunwet_out_meta_dir = root_dir.join("sunwet");
    let sunwet_out_meta = root_dir.join("sunwet.json");
    create_dir_all(&sunwet_out_meta_dir).context("Error making sunwet dir")?;
    let timestamp = node_value_str(&Utc::now().to_rfc3339());

    // Gather metadata from tracks, prepare dir-associated data
    struct GatherArtist {
        id: Node,
        name: String,
    }

    struct GatherTrack {
        file: PathBuf,
        type_: GatherMedia,
        index: Option<f64>,
        superindex: Option<f64>,
        artist: Vec<Rc<RefCell<GatherArtist>>>,
        name: Option<String>,
        lang: Option<String>,
        // Precedence -> hash -> prevalence in tracks
        covers: BTreeMap<usize, HashMap<PathBuf, usize>>,
    }

    struct GatherAlbum {
        name: String,
        artist: BTreeSet<ByAddress<Rc<RefCell<GatherArtist>>>>,
        tracks: Vec<Rc<RefCell<GatherTrack>>>,
        // Precedence -> hash -> prevalence in tracks
        covers: BTreeMap<usize, HashMap<PathBuf, usize>>,
        documents: Vec<PathBuf>,
    }

    #[derive(Default)]
    struct DirAssociations {
        album: HashSet<ByAddress<Rc<RefCell<GatherAlbum>>>>,
    }

    let mut dir_associations = HashMap::<PathBuf, DirAssociations>::new();

    #[derive(Hash, PartialEq, Eq, Clone)]
    struct AlbumKey {
        media: GatherMedia,
        album_artist: BTreeSet<ByAddress<Rc<RefCell<GatherArtist>>>>,
        name: String,
        lang: Option<String>,
    }

    let mut query_cache = HashMap::new();
    let mut albums = HashMap::<AlbumKey, Rc<RefCell<GatherAlbum>>>::new();
    let mut artists = HashMap::<String, Rc<RefCell<GatherArtist>>>::new();
    let mut leftover_files = vec![];
    for file in WalkDir::new(&root_dir) {
        let file = match file {
            Ok(f) => f,
            Err(e) => {
                log.log(loga::WARN, e.context("Failed to inspect file in dir, skipping"));
                continue;
            },
        };
        if file.path().starts_with(&sunwet_out_meta_dir) || file.path() == sunwet_out_meta {
            continue;
        }
        let meta = file.metadata()?;
        if meta.is_dir() {
            continue;
        }
        if file.path().file_name().is_none() || file.path().file_name().unwrap().as_bytes().starts_with(b".") {
            continue;
        }
        let e = file.path().extension().unwrap_or_default();
        let g;
        if is_audio(e.as_bytes()) {
            g = gather_audio::gather(&sunwet_out_meta_dir, file.path(), e);
        } else if is_video(e.as_bytes()) {
            g = gather_video::gather(file.path());
        } else if is_comic(e.as_bytes()) {
            g = gather_comic::gather(&sunwet_out_meta_dir, file.path());
        } else if is_epub(e.as_bytes()) {
            g = gather_epub::gather(&sunwet_out_meta_dir, file.path());
        } else {
            leftover_files.push(file);
            continue;
        }
        let g = g.context_with("Error gathering meta for file", ea!(path = file.path().dbg_str()))?;

        // Sanity check minimum meta
        if g.track_name.is_none() && g.track_index.is_none() {
            return Err(
                loga::err_with(
                    "File missing both index in album and track name, no identifier to look up remotely",
                    ea!(path = file.path().dbg_str()),
                ),
            );
        }
        if g.track_type != GatherMedia::Video && g.track_artist.is_empty() {
            return Err(loga::err_with("File missing track artist", ea!(path = file.path().dbg_str())));
        }

        // Build album artist
        let mut album_artist2 = BTreeSet::new();
        for artist_name in &g.album_artist {
            let artist = match artists.entry(artist_name.clone()) {
                Entry::Occupied(e) => {
                    e.get().clone()
                },
                Entry::Vacant(e) => {
                    e.insert(Rc::new(RefCell::new(GatherArtist {
                        id: node_id_artist(&log, &mut query_cache, artist_name).await?,
                        name: artist_name.clone(),
                    }))).clone()
                },
            };
            album_artist2.insert(ByAddress(artist));
        }
        let Some(album_name) = g.album_name.or_else(|| g.track_name.clone()) else {
            return Err(loga::err_with("File missing track and album name", ea!(ath = file.path().dbg_str())));
        };
        let album = match albums.entry(AlbumKey {
            media: g.track_type,
            album_artist: album_artist2.clone(),
            name: album_name.clone(),
            lang: g.track_language.clone(),
        }) {
            Entry::Occupied(e) => {
                e.get().clone()
            },
            Entry::Vacant(e) => {
                e.insert(Rc::new(RefCell::new(GatherAlbum {
                    name: album_name,
                    artist: album_artist2,
                    tracks: Default::default(),
                    covers: Default::default(),
                    documents: Default::default(),
                }))).clone()
            },
        };
        for (priority, cover) in &g.track_cover {
            *album.borrow_mut().covers.entry(*priority).or_default().entry(cover.clone()).or_default() += 1;
        }
        dir_associations
            .entry(file.path().parent().unwrap().to_path_buf())
            .or_insert(DirAssociations::default())
            .album
            .insert(ByAddress(album.clone()));

        // Assemble track
        let mut track_artist2 = vec![];
        for artist_name in &g.track_artist {
            let artist = match artists.entry(artist_name.clone()) {
                Entry::Occupied(e) => e.get().clone(),
                Entry::Vacant(e) => {
                    e.insert(Rc::new(RefCell::new(GatherArtist {
                        id: node_id_artist(&log, &mut query_cache, &artist_name).await?,
                        name: artist_name.clone(),
                    }))).clone()
                },
            };
            track_artist2.push(artist);
        }
        let track = Rc::new(RefCell::new(GatherTrack {
            type_: g.track_type,
            index: g.track_index,
            superindex: g.track_superindex,
            file: file.path().to_path_buf(),
            artist: track_artist2,
            name: g.track_name,
            lang: g.track_language,
            covers: Default::default(),
        }));
        for (priority, cover) in &g.track_cover {
            *track.borrow_mut().covers.entry(*priority).or_default().entry(cover.clone()).or_default() += 1;
        }
        album.borrow_mut().tracks.push(track);
    }

    // Gather non-track data (docs, covers) and associate with common dir albums
    for file in leftover_files {
        let Some(assoc) = dir_associations.get(file.path().parent().unwrap()) else {
            log.log_with(
                loga::WARN,
                "Skipping document in dir with no album association",
                ea!(path = file.path().dbg_str()),
            );
            continue;
        };
        if assoc.album.len() != 1 {
            log.log_with(
                loga::WARN,
                "Skipping document in dir with ambiguous album association",
                ea!(path = file.path().dbg_str()),
            );
            continue;
        }
        let album = assoc.album.iter().next().unwrap();
        let e = file.path().extension().unwrap_or_default();
        if is_image(e.as_bytes()) {
            let norm_filename =
                String::from_utf8_lossy(
                    file.path().with_extension("").file_name().unwrap_or_default().as_bytes(),
                ).to_ascii_lowercase();
            *album.borrow_mut().covers.entry(if norm_filename.as_str() == "cover" {
                5
            } else if norm_filename.contains("cover") {
                6
            } else {
                50
            }).or_default().entry(file.path().to_path_buf()).or_default() += 1;
        } else {
            album.borrow_mut().documents.push(file.path().to_path_buf());
        }
    }

    // Turn gathered data into triples
    let mut triples = vec![];
    for artist in artists.values() {
        let artist = artist.borrow();
        triples.push(triple(&node_node(&artist.id), PREDICATE_IS, &obj_is_artist()));
        triples.push(triple(&node_node(&artist.id), PREDICATE_NAME, &node_value_str(&artist.name)));
        triples.push(triple(&node_node(&artist.id), PREDICATE_ADD_TIMESTAMP, &timestamp));
    }
    for album in albums.values() {
        let album = album.borrow();
        let predicate_media = match album.tracks.iter().next().unwrap().borrow().type_ {
            GatherMedia::Audio => obj_media_audio(),
            GatherMedia::Video => obj_media_video(),
            GatherMedia::Comic => obj_media_comic(),
            GatherMedia::Book => obj_media_book(),
        };
        let album_id = match album.artist.iter().next() {
            Some(a) => node_id(
                &log,
                &mut query_cache,
                concat!(
                    r#"$artist_id -< "sunwet/1/artist" "#,
                    r#"  &( "#,
                    r#"    ?(-> "sunwet/1/is" == "sunwet/1/album") "#,
                    r#"    ?(-> "sunwet/1/name" == $album_name )"#,
                    r#"  )"#,
                    r#"  { => id } "#,
                ),
                [
                    (format!("artist_id"), a.0.borrow().id.clone()),
                    (format!("album_name"), Node::from_str(&album.name)),
                ]
                    .into_iter()
                    .collect(),
            ).await?,
            None => node_id(
                &log,
                &mut query_cache,
                concat!(
                    r#"$album_name -< "sunwet/1/name" "#,
                    r#"  &( "#,
                    r#"    ?(-> "sunwet/1/media" == $album_media) "#,
                    r#"    ?(-> "sunwet/1/is" == "sunwet/1/album") "#,
                    r#"  )"#,
                    r#"  { => id } "#,
                ),
                [
                    //. .
                    (format!("album_media"), match &predicate_media {
                        CliNode::Value(v) => Node::Value(v.clone()),
                        _ => panic!(),
                    }),
                    (format!("album_name"), Node::from_str(&album.name)),
                ].into_iter().collect(),
            ).await?,
        };
        triples.push(triple(&node_node(&album_id), PREDICATE_IS, &obj_is_album()));
        triples.push(triple(&node_node(&album_id), PREDICATE_MEDIA, &predicate_media));
        if let Some(lang) = &album.tracks.iter().next().unwrap().borrow().lang {
            triples.push(triple(&node_node(&album_id), PREDICATE_LANG, &node_value_str(&lang)));
        }
        triples.push(triple(&node_node(&album_id), PREDICATE_NAME, &node_value_str(&album.name)));
        for artist in &album.artist {
            triples.push(triple(&node_node(&album_id), PREDICATE_ARTIST, &node_node(&artist.borrow().id)));
        }
        triples.push(triple(&node_node(&album_id), PREDICATE_ADD_TIMESTAMP, &timestamp));
        shed!{
            'found _;
            for covers in album.covers.values() {
                let mut covers = covers.iter().collect::<Vec<_>>();
                covers.sort_by_cached_key(|c| *c.1);
                if let Some((cover, _)) = covers.into_iter().next() {
                    triples.push(triple(&node_node(&album_id), PREDICATE_COVER, &node_upload(root_dir, cover)));
                    break 'found;
                }
            }
        };
        for track in &album.tracks {
            let track = track.borrow();
            let track_id =
                node_id_direct(
                    &log,
                    query_album_track(album_id.clone(), track.superindex, track.index, &track.name),
                    Default::default(),
                ).await?;
            triples.push(triple(&node_node(&track_id), PREDICATE_IS, &obj_is_track()));
            if let Some(index) = track.index {
                triples.push(triple(&node_node(&track_id), PREDICATE_INDEX, &node_value_f64(index)));
            }
            if let Some(index) = track.superindex {
                triples.push(triple(&node_node(&track_id), PREDICATE_SUPERINDEX, &node_value_f64(index)));
            }
            if let Some(name) = track.name.as_ref() {
                triples.push(triple(&node_node(&track_id), PREDICATE_NAME, &node_value_str(name)));
            }
            for artist in &track.artist {
                triples.push(triple(&node_node(&track_id), PREDICATE_ARTIST, &node_node(&artist.borrow().id)));
            }
            triples.push(triple(&node_node(&track_id), PREDICATE_ADD_TIMESTAMP, &timestamp));
            triples.push(triple(&node_node(&track_id), PREDICATE_FILE, &node_upload(&root_dir, &track.file)));
            triples.push(triple(&node_node(&album_id), PREDICATE_TRACK, &node_node(&track_id)));
            shed!{
                'found _;
                for covers in track.covers.values() {
                    let mut covers = covers.iter().collect::<Vec<_>>();
                    covers.sort_by_cached_key(|c| *c.1);
                    if let Some((cover, _)) = covers.into_iter().next() {
                        triples.push(triple(&node_node(&track_id), PREDICATE_COVER, &node_upload(root_dir, cover)),);
                        break 'found;
                    }
                }
            };
        }
        for doc in &album.documents {
            let name = String::from_utf8_lossy(doc.file_name().unwrap_or_default().as_bytes());
            let doc_id =
                node_id(
                    &log,
                    &mut query_cache,
                    concat!(
                        r#"$document_name -< "sunwet/1/name" "#,
                        r#"  ?( -> "sunwet/1/is" == "sunwet/1/document" ) "#,
                        r#"  { => id } "#,
                    ),
                    [(format!("document_name"), Node::from_str(&name))].into_iter().collect(),
                ).await?;
            triples.push(triple(&node_node(&doc_id), PREDICATE_IS, &obj_is_document()));
            triples.push(triple(&node_node(&doc_id), PREDICATE_NAME, &node_value_str(&name)));
            triples.push(triple(&node_node(&doc_id), PREDICATE_ADD_TIMESTAMP, &timestamp));
            triples.push(triple(&node_node(&album_id), PREDICATE_DOC, &node_node(&doc_id)));
        }
    }
    write(sunwet_out_meta, serde_json::to_string_pretty(&CliCommit {
        add: triples,
        remove: vec![],
    }).unwrap()).context("Error writing sunwet.json")?;
    return Ok(());
}

/// Turn a directory/archive file into a sunwet commit directory ready for upload.
#[derive(Aargvark)]
pub struct PrepareImportCommitCommand {
    /// The archive or directory to import.
    source: PathBuf,
    /// The path to the directory to write the commit in. If not specified, uses the
    /// source directory or, if an archive, a directory with the name of the archive
    /// with the extension removed.
    dest: Option<PathBuf>,
}

pub async fn handle_prepare_media_import_commit(args: PrepareImportCommitCommand) -> Result<(), loga::Error> {
    let log = loga::Log::new_root(loga::INFO);
    let source_meta =
        args
            .source
            .metadata()
            .context_with("Can't read metadata of source path", ea!(path = args.source.dbg_str()))?;
    if source_meta.is_file() {
        let log = log.fork(ea!(source = args.source.to_string_lossy()));
        if let Some(e) = args.source.extension() {
            if is_image(e.as_bytes()) || is_audio(e.as_bytes()) || is_video(e.as_bytes()) ||
                is_doc(e.as_bytes()) ||
                is_comic(e.as_bytes()) | is_epub(e.as_bytes()) {
                let log = Log::new().fork(ea!(file = args.source.dbg_str()));
                let dest = match args.dest {
                    Some(d) => d,
                    None => args.source.with_extension(""),
                };
                create_dir_all(&dest)?;
                let mut out =
                    File::create(
                        dest.join(args.source.file_name().stack_context(&log, "File has invalid name")?),
                    ).stack_context(&log, "Error creating file in output directory")?;
                let mut source = File::open(&args.source).context("Error opening file")?;
                io::copy(&mut source, &mut out).stack_context(&log, "Error extracting contents")?;
                import_dir(&log, &dest).await?;
            } else if e.as_bytes() == b"zip" {
                let dest = match args.dest {
                    Some(d) => d,
                    None => args.source.with_extension(""),
                };
                create_dir_all(&dest)?;
                let mut zip =
                    zip::ZipArchive::new(
                        File::open(args.source).stack_context(&log, "Error opening file")?,
                    ).stack_context(&log, "Error opening file as zip archive")?;
                for i in 0 .. zip.len() {
                    match (|| {
                        let mut file = zip.by_index(i)?;
                        let log = Log::new().fork(ea!(archive_path = file.name()));
                        let mut out =
                            File::create(
                                dest.join(file.enclosed_name().stack_context(&log, "File has invalid name")?),
                            ).stack_context(&log, "Error creating file in output directory")?;
                        io::copy(&mut file, &mut out).stack_context(&log, "Error extracting contents")?;
                        return Ok(()) as Result<(), loga::Error>;
                    })() {
                        Ok(_) => (),
                        Err(e) => {
                            log.log_err(
                                loga::WARN,
                                e.context_with("Error extracting file from archive, skipping", ea!(index = i)),
                            );
                        },
                    }
                }
                import_dir(&log, &dest).await?;
            } else {
                return Err(loga::err("Unsupported source file type"));
            }
        } else {
            return Err(loga::err("File has no extension, unable to determine type"));
        }
    } else if source_meta.is_dir() {
        let dest = args.dest.as_ref().unwrap_or(&args.source);
        let log = log.fork(ea!(dest = dest.to_string_lossy()));
        import_dir(&log, dest).await?;
    } else {
        return Err(
            loga::err_with(
                format!("Unhandled source file type: {:?}", source_meta.file_type()),
                ea!(path = args.source.dbg_str()),
            ),
        );
    }
    return Ok(());
}
