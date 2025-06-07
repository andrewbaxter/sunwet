use {
    aargvark::Aargvark,
    by_address::ByAddress,
    chrono::Utc,
    flowcontrol::shed,
    loga::{
        ea,
        fatal,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    native_import::{
        gather::GatherTrackType,
        gather_audio,
        gather_comic,
        gather_epub,
        gather_video,
    },
    shared::interface::{
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
    },
    std::{
        cell::RefCell,
        collections::{
            BTreeMap,
            BTreeSet,
            HashMap,
            HashSet,
        },
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

pub fn node_id() -> String {
    return Uuid::new_v4().hyphenated().to_string();
}

pub fn node_upload(root: &Path, p: &Path) -> CliNode {
    return CliNode::Upload(p.strip_prefix(root).unwrap().to_path_buf());
}

pub fn node_value_str(v: &str) -> CliNode {
    return CliNode::Value(serde_json::Value::String(v.to_string()));
}

pub fn node_value_usize(v: usize) -> CliNode {
    return CliNode::Value(serde_json::Value::Number(serde_json::Number::from(v as i64)));
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
        b"png" | b"jpg" | b"bmp" | b"tif" | b"gif" | b"webp" | b"webm" => true,
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
        b"cbz" | b"cbr" => true,
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

fn import_dir(log: &Log, root_dir: &PathBuf) -> Result<(), loga::Error> {
    let sunwet_out_meta_dir = root_dir.join("sunwet");
    let sunwet_out_meta = root_dir.join("sunwet.json");
    create_dir_all(&sunwet_out_meta_dir).context("Error making sunwet dir")?;
    let timestamp = node_value_str(&Utc::now().to_rfc3339());

    // Gather metadata from tracks, prepare dir-associated data
    struct GatherArtist {
        id: String,
        name: String,
    }

    struct GatherTrack {
        id: String,
        file: PathBuf,
        type_: GatherTrackType,
        index: Option<usize>,
        superindex: Option<usize>,
        artist: Vec<Rc<RefCell<GatherArtist>>>,
        name: String,
        lang: Option<String>,
        // Precedence -> hash -> prevalence in tracks
        covers: BTreeMap<usize, HashMap<PathBuf, usize>>,
    }

    struct GatherAlbum {
        id: String,
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
        album_artist: BTreeSet<ByAddress<Rc<RefCell<GatherArtist>>>>,
        name: String,
    }

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
        let Some(track_name) = g.track_name else {
            return Err(loga::err_with("File missing track name", ea!(path = file.path().dbg_str())));
        };
        if g.track_artist.is_empty() {
            return Err(loga::err_with("File missing track artist", ea!(path = file.path().dbg_str())));
        }

        // Build album artist
        let mut album_artist2 = BTreeSet::new();
        for artist in &g.album_artist {
            album_artist2.insert(
                ByAddress(artists.entry(artist.clone()).or_insert_with(|| Rc::new(RefCell::new(GatherArtist {
                    id: node_id(),
                    name: artist.clone(),
                }))).clone()),
            );
        }
        let album_name = g.album_name.unwrap_or_else(|| track_name.clone());
        let album = albums.entry(AlbumKey {
            album_artist: album_artist2.clone(),
            name: album_name.clone(),
        }).or_insert_with(|| Rc::new(RefCell::new(GatherAlbum {
            id: node_id(),
            name: album_name,
            artist: album_artist2,
            tracks: Default::default(),
            covers: Default::default(),
            documents: Default::default(),
        })));
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
        for artist in &g.track_artist {
            track_artist2.push(artists.entry(artist.clone()).or_insert_with(|| Rc::new(RefCell::new(GatherArtist {
                id: node_id(),
                name: artist.clone(),
            }))).clone());
        }
        let track = Rc::new(RefCell::new(GatherTrack {
            id: node_id(),
            type_: g.track_type,
            index: g.track_index,
            superindex: g.track_superindex,
            file: file.path().to_path_buf(),
            artist: track_artist2,
            name: track_name,
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
        triples.push(triple(&node_value_str(&artist.id), PREDICATE_IS, &obj_is_artist()));
        triples.push(triple(&node_value_str(&artist.id), PREDICATE_NAME, &node_value_str(&artist.name)));
        triples.push(triple(&node_value_str(&artist.id), PREDICATE_ADD_TIMESTAMP, &timestamp));
    }
    for album in albums.values() {
        let album = album.borrow();
        triples.push(triple(&node_value_str(&album.id), PREDICATE_IS, &obj_is_album()));
        triples.push(
            triple(
                &node_value_str(&album.id),
                PREDICATE_MEDIA,
                &match album.tracks.iter().next().unwrap().borrow().type_ {
                    GatherTrackType::Audio => obj_media_audio(),
                    GatherTrackType::Video => obj_media_video(),
                    GatherTrackType::Comic => obj_media_comic(),
                    GatherTrackType::Book => obj_media_book(),
                },
            ),
        );
        if let Some(lang) = &album.tracks.iter().next().unwrap().borrow().lang {
            triples.push(triple(&node_value_str(&album.id), PREDICATE_LANG, &node_value_str(&lang)));
        }
        triples.push(triple(&node_value_str(&album.id), PREDICATE_NAME, &node_value_str(&album.name)));
        for artist in &album.artist {
            triples.push(
                triple(&node_value_str(&album.id), PREDICATE_ARTIST, &node_value_str(&artist.borrow().id)),
            );
        }
        triples.push(triple(&node_value_str(&album.id), PREDICATE_ADD_TIMESTAMP, &timestamp));
        shed!{
            'found _;
            for covers in album.covers.values() {
                let mut covers = covers.iter().collect::<Vec<_>>();
                covers.sort_by_cached_key(|c| *c.1);
                if let Some((cover, _)) = covers.into_iter().next() {
                    triples.push(
                        triple(&node_value_str(&album.id), PREDICATE_COVER, &node_upload(root_dir, cover)),
                    );
                    break 'found;
                }
            }
        };
        for track in &album.tracks {
            let track = track.borrow();
            triples.push(triple(&node_value_str(&track.id), PREDICATE_IS, &obj_is_track()));
            if let Some(index) = track.index {
                triples.push(triple(&node_value_str(&track.id), PREDICATE_INDEX, &node_value_usize(index)));
            }
            if let Some(index) = track.superindex {
                triples.push(triple(&node_value_str(&track.id), PREDICATE_SUPERINDEX, &node_value_usize(index)));
            }
            triples.push(triple(&node_value_str(&track.id), PREDICATE_NAME, &node_value_str(&track.name)));
            for artist in &track.artist {
                triples.push(
                    triple(&node_value_str(&track.id), PREDICATE_ARTIST, &node_value_str(&artist.borrow().id)),
                );
            }
            triples.push(triple(&node_value_str(&track.id), PREDICATE_ADD_TIMESTAMP, &timestamp));
            triples.push(triple(&node_value_str(&track.id), PREDICATE_FILE, &node_upload(&root_dir, &track.file)));
            triples.push(triple(&node_value_str(&album.id), PREDICATE_TRACK, &node_value_str(&track.id)));
            shed!{
                'found _;
                for covers in track.covers.values() {
                    let mut covers = covers.iter().collect::<Vec<_>>();
                    covers.sort_by_cached_key(|c| *c.1);
                    if let Some((cover, _)) = covers.into_iter().next() {
                        triples.push(
                            triple(&node_value_str(&track.id), PREDICATE_COVER, &node_upload(root_dir, cover)),
                        );
                        break 'found;
                    }
                }
            };
        }
        for doc in &album.documents {
            let doc_id = node_id();
            triples.push(triple(&node_value_str(&doc_id), PREDICATE_IS, &obj_is_document()));
            triples.push(
                triple(
                    &node_value_str(&doc_id),
                    PREDICATE_NAME,
                    &node_value_str(&String::from_utf8_lossy(doc.file_name().unwrap_or_default().as_bytes())),
                ),
            );
            triples.push(triple(&node_value_str(&doc_id), PREDICATE_ADD_TIMESTAMP, &timestamp));
            triples.push(triple(&node_value_str(&album.id), PREDICATE_DOC, &node_value_str(&doc_id)));
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
struct Args {
    /// The archive or directory to import.
    source: PathBuf,
    /// The path to the directory to write the commit in. If not specified, uses the
    /// source directory or, if an archive, a directory with the name of the archive
    /// with the extension removed.
    dest: Option<PathBuf>,
}

fn main1() -> Result<(), loga::Error> {
    let args = aargvark::vark::<Args>();
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
                import_dir(&log, &dest)?;
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
                import_dir(&log, &dest)?;
            } else {
                return Err(loga::err("Unsupported source file type"));
            }
        } else {
            return Err(loga::err("File has no extension, unable to determine type"));
        }
    } else if source_meta.is_dir() {
        let dest = args.dest.as_ref().unwrap_or(&args.source);
        let log = log.fork(ea!(dest = dest.to_string_lossy()));
        import_dir(&log, dest)?;
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

fn main() {
    match main1() {
        Ok(_) => { },
        Err(e) => fatal(e),
    }
}
