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
    sha2::{
        Digest,
        Sha256,
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
            OBJ_MEDIA_IMAGE,
            OBJ_MEDIA_VIDEO,
            PREDICATE_ADD_TIMESTAMP,
            PREDICATE_ARTIST,
            PREDICATE_COVER,
            PREDICATE_DOC,
            PREDICATE_FILE,
            PREDICATE_INDEX,
            PREDICATE_IS,
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

fn is_doc(p: &[u8]) -> bool {
    match p {
        b"txt" | b"md" | b"doc" | b"docx" | b"odt" | b"pdf" | b"rst" => true,
        _ => false,
    }
}

fn import_dir(log: &Log, root_dir: &PathBuf) -> Result<(), loga::Error> {
    let sunwet_dir = root_dir.join("sunwet");
    create_dir_all(&sunwet_dir).context("Error making sunwet dir")?;
    let timestamp = node_value_str(&Utc::now().to_rfc3339());

    // Gather metadata from tracks, prepare dir-associated data
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum GatherTrackType {
        Audio,
        Video,
    }

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
        let log = log.fork(ea!(file = file.path().to_string_lossy()));
        let meta = file.metadata()?;
        if meta.is_dir() {
            continue;
        }
        if file.path().file_name().is_none() || file.path().file_name().unwrap().as_bytes().starts_with(b".") {
            continue;
        }
        let e = file.path().extension().unwrap_or_default();
        let mut album_name = None;
        let mut album_artist = BTreeSet::new();
        let mut track_artist = vec![];
        let mut track_name = None;
        let mut track_index = None;
        let track_type;
        let mut track_superindex = None;
        let mut album_cover = HashMap::new();
        if is_audio(e.as_bytes()) {
            track_type = GatherTrackType::Audio;
            let mut info =
                match symphonia
                ::default
                ::get_probe().format(
                    &symphonia::core::probe::Hint::new().with_extension(&e.to_str().unwrap()),
                    symphonia::core::io::MediaSourceStream::new(
                        Box::new(File::open(file.path())?),
                        Default::default(),
                    ),
                    &Default::default(),
                    &Default::default(),
                ) {
                    Ok(i) => i,
                    Err(e) => {
                        log.log_err(loga::WARN, e.context("Unable to read audio file"));
                        continue;
                    },
                };
            let mut parse_metadata = |metadata: &symphonia::core::meta::MetadataRevision| {
                for tag in metadata.tags() {
                    match tag.std_key {
                        Some(k) => match k {
                            symphonia::core::meta::StandardTagKey::Album => {
                                album_name = Some(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::AlbumArtist => {
                                album_artist.insert(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::Artist => {
                                track_artist.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::DiscNumber => {
                                track_superindex = Some(usize::from_str_radix(&tag.value.to_string(), 10)?);
                            },
                            symphonia::core::meta::StandardTagKey::TrackNumber => {
                                track_index =
                                    Some(
                                        usize::from_str_radix(&tag.value.to_string().split("/").next().unwrap(), 10)?,
                                    );
                            },
                            symphonia::core::meta::StandardTagKey::TrackTitle => {
                                track_name = Some(tag.value.to_string());
                            },
                            _ => { },
                        },
                        None => { },
                    }
                }
                for v in metadata.visuals() {
                    let priority = match v.usage {
                        Some(u) => match u {
                            symphonia::core::meta::StandardVisualKey::FrontCover => 0,
                            symphonia::core::meta::StandardVisualKey::Media => 10,
                            symphonia::core::meta::StandardVisualKey::Illustration => 20,
                            symphonia::core::meta::StandardVisualKey::BandArtistLogo => 30,
                            symphonia::core::meta::StandardVisualKey::Leaflet => 40,
                            symphonia::core::meta::StandardVisualKey::FileIcon => 500,
                            symphonia::core::meta::StandardVisualKey::OtherIcon => 500,
                            symphonia::core::meta::StandardVisualKey::BackCover => 500,
                            symphonia::core::meta::StandardVisualKey::LeadArtistPerformerSoloist => 500,
                            symphonia::core::meta::StandardVisualKey::ArtistPerformer => 500,
                            symphonia::core::meta::StandardVisualKey::Conductor => 500,
                            symphonia::core::meta::StandardVisualKey::BandOrchestra => 500,
                            symphonia::core::meta::StandardVisualKey::Composer => 500,
                            symphonia::core::meta::StandardVisualKey::Lyricist => 500,
                            symphonia::core::meta::StandardVisualKey::RecordingLocation => 500,
                            symphonia::core::meta::StandardVisualKey::RecordingSession => 500,
                            symphonia::core::meta::StandardVisualKey::Performance => 500,
                            symphonia::core::meta::StandardVisualKey::ScreenCapture => 500,
                            symphonia::core::meta::StandardVisualKey::PublisherStudioLogo => 500,
                        },
                        None => 1000,
                    };
                    let suffix = match v.media_type.as_str() {
                        "image/jpeg" => "jpg",
                        "image/png" => "png",
                        "image/webp" => "webp",
                        "image/avif" => "avif",
                        "image/gif" => "gif",
                        "image/tiff" => "tif",
                        _ => {
                            continue;
                        },
                    };
                    let digest = hex::encode(Sha256::digest(&v.data));
                    let path = sunwet_dir.join(format!("{}.{}", digest, suffix));
                    if !path.exists() {
                        std::fs::write(&path, &v.data).context("Error writing cover from file")?;
                    }
                    album_cover.insert(priority, path);
                }
                return Ok(()) as Result<(), loga::Error>;
            };
            shed!{
                let Some(metadata) = info.metadata.get() else {
                    break;
                };
                let Some(metadata) = metadata.current() else {
                    break;
                };
                parse_metadata(metadata)?;
            }
            if let Some(metadata) = info.format.metadata().current() {
                parse_metadata(metadata)?;
            }
        } else if is_video(e.as_bytes()) {
            track_type = GatherTrackType::Video;
            let elements = match mkvdump::parse_elements_from_file(file.path(), false) {
                Ok(e) => e,
                Err(e) => {
                    log.log_err(
                        loga::WARN,
                        loga::err(e.to_string()).context("Unable to read metadata in video file"),
                    );
                    continue;
                },
            };

            // Must access untyped json values due to
            // (https://github.com/cadubentzen/mkvdump/issues/138)
            let serde_json::Value::Array(tree) =
                serde_json::to_value(&mkvparser::tree::build_element_trees(&elements)).unwrap() else {
                    continue
                };

            fn parse_value<
                'a,
            >(
                x: &'a serde_json::Value,
            ) -> Option<
                (&'a String, Option<&'a Vec<serde_json::Value>>, &'a serde_json::Map<String, serde_json::Value>),
            > {
                let serde_json::Value::Object(child) = x else {
                    return None;
                };
                let Some(serde_json::Value::String(id)) = child.get("id") else {
                    return None;
                };
                let children_;
                if let Some(serde_json::Value::Array(children)) = x.get("children") {
                    children_ = Some(children);
                } else {
                    children_ = None;
                }
                return Some((id, children_, child));
            }

            fn find_element_with_id<
                'a,
            >(
                arr: &'a Vec<serde_json::Value>,
                key: &str,
            ) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
                for child in arr {
                    let serde_json::Value::Object(child) = child else {
                        continue;
                    };
                    let Some(serde_json::Value::String(id)) = child.get("id") else {
                        continue;
                    };
                    if id.as_str() == key {
                        return Some(child);
                    }
                }
                return None;
            }

            fn get_children<
                'a,
            >(x: &'a serde_json::Map<String, serde_json::Value>) -> Option<&'a Vec<serde_json::Value>> {
                let Some(serde_json::Value::Array(children)) = x.get("children") else {
                    return None;
                };
                return Some(children);
            }

            fn find_child_with_id<
                'a,
            >(
                x: &'a serde_json::Map<String, serde_json::Value>,
                key: &str,
            ) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
                let Some(children) = get_children(x) else {
                    return None;
                };
                for child in children {
                    let serde_json::Value::Object(child) = child else {
                        continue;
                    };
                    let Some(serde_json::Value::String(id)) = child.get("id") else {
                        continue;
                    };
                    if id.as_str() == key {
                        return Some(child);
                    }
                }
                return None;
            }

            let Some(segment) = find_element_with_id(&tree, "Segment") else {
                continue;
            };
            let Some(tags) = find_child_with_id(segment, "Tags") else {
                continue;
            };
            let Some(tags_children) = get_children(tags) else {
                continue;
            };
            for tag in tags_children {
                let Some((_, Some(tag_children), _)) = parse_value(tag) else {
                    continue;
                };
                let mut levels = vec![];
                let mut tags = vec![];
                for child in tag_children {
                    let Some((child_id, Some(child_children), _)) = parse_value(child) else {
                        continue;
                    };
                    match child_id.as_str() {
                        "Targets" => {
                            for value_obj in child_children {
                                let serde_json::Value::Object(value_obj) = value_obj else {
                                    continue;
                                };
                                let Some(serde_json::Value::String(value)) = value_obj.get("value") else {
                                    continue;
                                };
                                levels.push(value.clone());
                            }
                        },
                        "SimpleTag" => {
                            let parent_tag;
                            match (
                                find_element_with_id(child_children, "TagName").and_then(|v| v.get("value")),
                                find_element_with_id(child_children, "TagString").and_then(|v| v.get("value")),
                            ) {
                                (Some(serde_json::Value::String(k)), Some(serde_json::Value::String(v))) => {
                                    parent_tag = k.clone();
                                    tags.push((k.clone(), v.clone()));
                                },
                                _ => {
                                    continue;
                                },
                            }
                            for maybe_nested in child_children {
                                let Some((maybe_nested_id, Some(nested_children), _)) =
                                    parse_value(maybe_nested) else {
                                        continue;
                                    };
                                if maybe_nested_id != "SimpleTag" {
                                    continue;
                                }
                                match (
                                    find_element_with_id(nested_children, "TagName").and_then(|v| v.get("value")),
                                    find_element_with_id(
                                        nested_children,
                                        "TagString",
                                    ).and_then(|v| v.get("value")),
                                ) {
                                    (Some(serde_json::Value::String(k)), Some(serde_json::Value::String(v))) => {
                                        tags.push((format!("{}__{}", parent_tag, k), v.clone()));
                                    },
                                    _ => {
                                        continue;
                                    },
                                }
                            }
                        },
                        _ => {
                            continue;
                        },
                    }
                }
                for level in &levels {
                    match level.as_str() {
                        "EDITION / ISSUE / VOLUME / OPUS / SEASON / SEQUEL" | "fake_ALBUM" => {
                            for (k, v) in &tags {
                                match k.as_str() {
                                    "TITLE" => {
                                        album_name = Some(v.clone());
                                    },
                                    "ARTIST" => {
                                        album_artist.insert(v.clone());
                                    },
                                    _ => { },
                                }
                            }
                        },
                        "TRACK / SONG / CHAPTER" => {
                            for (k, v) in &tags {
                                match k.as_str() {
                                    "TITLE" => {
                                        track_name = Some(v.clone());
                                    },
                                    "ARTIST" => {
                                        track_artist.push(v.clone());
                                    },
                                    "PART_NUMBER" => {
                                        track_index = Some(usize::from_str_radix(&v, 10)?);
                                    },
                                    _ => { },
                                }
                            }
                        },
                        _ => { },
                    }
                }
            }
        } else {
            leftover_files.push(file);
            continue;
        }

        // Sanity check minimum meta
        let Some(track_name) = track_name else {
            return Err(loga::err_with("File missing track name", ea!(path = file.path().dbg_str())));
        };
        if track_artist.is_empty() {
            return Err(loga::err_with("File missing track artist", ea!(path = file.path().dbg_str())));
        }

        // Build album artist
        let mut album_artist2 = BTreeSet::new();
        for artist in &album_artist {
            album_artist2.insert(
                ByAddress(artists.entry(artist.clone()).or_insert_with(|| Rc::new(RefCell::new(GatherArtist {
                    id: node_id(),
                    name: artist.clone(),
                }))).clone()),
            );
        }
        let album_name = album_name.unwrap_or_else(|| track_name.clone());
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
        for (priority, cover) in album_cover {
            *album.borrow_mut().covers.entry(priority).or_default().entry(cover).or_default() += 1;
        }
        dir_associations
            .entry(file.path().parent().unwrap().to_path_buf())
            .or_insert(DirAssociations::default())
            .album
            .insert(ByAddress(album.clone()));

        // Assemble track
        let mut track_artist2 = vec![];
        for artist in &track_artist {
            track_artist2.push(artists.entry(artist.clone()).or_insert_with(|| Rc::new(RefCell::new(GatherArtist {
                id: node_id(),
                name: artist.clone(),
            }))).clone());
        }
        album.borrow_mut().tracks.push(Rc::new(RefCell::new(GatherTrack {
            id: node_id(),
            type_: track_type,
            index: track_index,
            superindex: track_superindex,
            file: file.path().to_path_buf(),
            artist: track_artist2,
            name: track_name,
        })));
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
                },
            ),
        );
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
    write(root_dir.join("sunwet.json"), serde_json::to_string_pretty(&CliCommit {
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
            if is_image(e.as_bytes()) || is_audio(e.as_bytes()) || is_video(e.as_bytes()) || is_doc(e.as_bytes()) {
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
