use {
    aargvark::Aargvark,
    flowcontrol::shed,
    loga::{
        ea,
        fatal,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    native::client::commit::{
        CliCommit,
        CliNode,
        CliTriple,
    },
    std::{
        cell::RefCell,
        collections::{
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

const PREFIX_SUNWET1: &str = "sunwet/1";

// Link to file node from metadata node representing file
pub fn pred_file() -> String {
    return format!("{PREFIX_SUNWET1}/file");
}

// Human-known name for something
pub fn pred_name() -> String {
    return format!("{PREFIX_SUNWET1}/name");
}

// A mangling of the human-known name that can be unambiguously sorted by a
// computer (ex: hiragana/katagana instead of kanji)
pub fn pred_name_sort() -> String {
    return format!("{PREFIX_SUNWET1}/name_sort");
}

// Link to artist
pub fn pred_artist() -> String {
    return format!("{PREFIX_SUNWET1}/artist");
}

// Link to cover (file node)
pub fn pred_image() -> String {
    return format!("{PREFIX_SUNWET1}/cover");
}

// Link to booklet (file node)
pub fn pred_document() -> String {
    return format!("{PREFIX_SUNWET1}/booklet");
}

pub fn pred_media() -> String {
    return format!("{PREFIX_SUNWET1}/media");
}

pub fn pred_index() -> String {
    return format!("{PREFIX_SUNWET1}/index");
}

pub fn pred_element() -> String {
    return format!("{PREFIX_SUNWET1}/element");
}

/// Typing, can be chained to form hierarchy
pub fn pred_is() -> String {
    return format!("{PREFIX_SUNWET1}/is");
}

pub fn root_albumset_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/albumset")));
}

pub fn root_album_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/album")));
}

pub fn root_track_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/track")));
}

pub fn root_artist_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/artist")));
}

pub fn root_audio_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/audio")));
}

pub fn root_video_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/video")));
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
    fn parents(root_dir: &Path, start: &Path) -> Vec<PathBuf> {
        let mut out = vec![];
        let mut at = start;
        loop {
            out.push(at.to_path_buf());
            if at == root_dir {
                break;
            }
            at = at.parent().unwrap();
        }
        return out;
    }

    // Gather metadata
    enum GatherTrackType {
        Audio,
        Video,
    }

    struct GatherTrack {
        type_: GatherTrackType,
        id: String,
        file: PathBuf,
        index: Option<usize>,
        artist: Vec<String>,
        artist_sort: Vec<String>,
        name: Vec<String>,
        name_sort: Vec<String>,
    }

    struct GatherAlbum {
        id: String,
        index: Option<usize>,
        name: Vec<String>,
        name_sort: Vec<String>,
        artist: Vec<String>,
        artist_sort: Vec<String>,
        tracks: Vec<Rc<RefCell<GatherTrack>>>,
    }

    #[derive(Default)]
    struct GatherAlbumset {
        name: Vec<String>,
        name_sort: Vec<String>,
        artist: Vec<String>,
        artist_sort: Vec<String>,
        albums: Vec<Rc<RefCell<GatherAlbum>>>,
    }

    #[derive(Default)]
    struct DirAssociations {
        album: HashSet<String>,
    }

    let mut albumset = GatherAlbumset::default();
    let mut dir_associations = HashMap::<PathBuf, DirAssociations>::new();
    let mut images = vec![];
    let mut documents = vec![];
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
        let Some(e) = file.path().extension() else {
            continue;
        };
        if is_image(e.as_bytes()) {
            images.push(file.path().to_path_buf());
        } else if is_audio(e.as_bytes()) {
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
            let mut found_metadata = false;
            let mut album_artist = vec![];
            let mut album_artist_sort = vec![];
            let mut track_artist = vec![];
            let mut track_artist_sort = vec![];
            let mut track_name = vec![];
            let mut track_name_sort = vec![];
            let mut track_number = None;
            let mut disk_number = None;
            let mut parse_metadata = |metadata: &symphonia::core::meta::MetadataRevision| {
                found_metadata = true;
                for tag in metadata.tags() {
                    match tag.std_key {
                        Some(k) => match k {
                            symphonia::core::meta::StandardTagKey::Album => {
                                albumset.name.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::AlbumArtist => {
                                album_artist.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::Artist => {
                                track_artist.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::DiscNumber => {
                                disk_number = Some(usize::from_str_radix(&tag.value.to_string(), 10)?);
                            },
                            symphonia::core::meta::StandardTagKey::SortAlbum => {
                                albumset.name_sort.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::SortAlbumArtist => {
                                album_artist_sort.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::SortArtist => {
                                track_artist_sort.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::SortTrackTitle => {
                                track_name_sort.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::TrackNumber => {
                                track_number =
                                    Some(
                                        usize::from_str_radix(&tag.value.to_string().split("/").next().unwrap(), 10)?,
                                    );
                            },
                            symphonia::core::meta::StandardTagKey::TrackTitle => {
                                track_name.push(tag.value.to_string());
                            },
                            _ => { },
                        },
                        None => { },
                    }
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
            if !found_metadata {
                log.log(loga::WARN, "File has no metadata, skipping");
                continue;
            }
            let album = match albumset.albums.iter().find(|a| a.borrow().index == disk_number) {
                Some(a) => a.clone(),
                None => {
                    let a = Rc::new(RefCell::new(GatherAlbum {
                        id: node_id(),
                        index: disk_number,
                        name: vec![],
                        name_sort: vec![],
                        artist: vec![],
                        artist_sort: vec![],
                        tracks: vec![],
                    }));
                    albumset.albums.push(a.clone());
                    a
                },
            };
            let mut album = album.borrow_mut();
            for parent in parents(&root_dir, file.path().parent().unwrap()) {
                dir_associations
                    .entry(parent)
                    .or_insert(DirAssociations::default())
                    .album
                    .insert(album.id.clone());
            }
            album.artist.extend(album_artist.clone());
            album.artist_sort.extend(album_artist_sort.clone());
            album.tracks.push(Rc::new(RefCell::new(GatherTrack {
                type_: GatherTrackType::Audio,
                id: node_id(),
                index: track_number,
                file: file.path().to_path_buf(),
                artist: track_artist,
                artist_sort: track_artist_sort,
                name: track_name,
                name_sort: track_name_sort,
            })));
        } else if is_video(e.as_bytes()) {
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
            let mut album_name = vec![];
            let mut album_name_sort = vec![];
            let mut album_artist = vec![];
            let mut album_artist_sort = vec![];
            let mut track_name = vec![];
            let mut track_name_sort = vec![];
            let mut track_artist = vec![];
            let mut track_artist_sort = vec![];
            let mut disk_number = None;
            let mut track_number = None;
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
                        "COLLECTION" => {
                            for (k, v) in &tags {
                                match k.as_str() {
                                    "TITLE" => {
                                        albumset.name.push(v.to_string());
                                    },
                                    "TITLE__SORT_WITH" => {
                                        albumset.name_sort.push(v.to_string());
                                    },
                                    "ARTIST" => {
                                        albumset.artist.push(v.to_string());
                                    },
                                    "ARTIST__SORT_WITH" => {
                                        albumset.artist_sort.push(v.to_string());
                                    },
                                    _ => { },
                                }
                            }
                        },
                        "EDITION / ISSUE / VOLUME / OPUS / SEASON / SEQUEL" | "fake_ALBUM" => {
                            for (k, v) in &tags {
                                match k.as_str() {
                                    "TITLE" => {
                                        album_name.push(v.clone());
                                    },
                                    "TITLE__SORT_WITH" => {
                                        album_name_sort.push(v.clone());
                                    },
                                    "ARTIST" => {
                                        album_artist.push(v.clone());
                                    },
                                    "ARTIST__SORT_WITH" => {
                                        album_artist_sort.push(v.clone());
                                    },
                                    "PART_NUMBER" => {
                                        disk_number = Some(usize::from_str_radix(&v, 10)?);
                                    },
                                    _ => { },
                                }
                            }
                        },
                        "TRACK / SONG / CHAPTER" => {
                            for (k, v) in &tags {
                                match k.as_str() {
                                    "TITLE" => {
                                        track_name.push(v.clone());
                                    },
                                    "TITLE__SORT_WITH" => {
                                        track_name_sort.push(v.clone());
                                    },
                                    "ARTIST" => {
                                        track_artist.push(v.clone());
                                    },
                                    "ARTIST__SORT_WITH" => {
                                        track_artist_sort.push(v.clone());
                                    },
                                    "PART_NUMBER" => {
                                        track_number = Some(usize::from_str_radix(&v, 10)?);
                                    },
                                    _ => { },
                                }
                            }
                        },
                        _ => { },
                    }
                }
            }
            let album = match albumset.albums.iter().find(|a| a.borrow().index == disk_number) {
                Some(a) => a.clone(),
                None => {
                    let a = Rc::new(RefCell::new(GatherAlbum {
                        id: node_id(),
                        index: disk_number,
                        name: vec![],
                        name_sort: vec![],
                        artist: vec![],
                        artist_sort: vec![],
                        tracks: vec![],
                    }));
                    albumset.albums.push(a.clone());
                    a
                },
            };
            let mut album = album.borrow_mut();
            for parent in parents(&root_dir, file.path().parent().unwrap()) {
                dir_associations
                    .entry(parent)
                    .or_insert(DirAssociations::default())
                    .album
                    .insert(album.id.clone());
            }
            album.name.extend(album_name.clone());
            album.name_sort.extend(album_name_sort.clone());
            album.artist.extend(album_artist.clone());
            album.artist_sort.extend(album_artist_sort.clone());
            album.tracks.push(Rc::new(RefCell::new(GatherTrack {
                type_: GatherTrackType::Video,
                id: node_id(),
                index: track_number,
                file: file.path().to_path_buf(),
                artist: track_artist,
                artist_sort: track_artist_sort,
                name: track_name,
                name_sort: track_name_sort,
            })));
        } else if is_doc(e.as_bytes()) {
            documents.push(file.path().to_path_buf());
        } else {
            continue;
        }
    }

    // Turn gathered data into triples
    let mut artists = HashMap::<String, CliNode>::new();
    let mut triples = vec![];
    let mut build_artist = |triples: &mut Vec<CliTriple>, name: &str, name_sort: &str| -> CliNode {
        let artist_id =
            artists.entry(name.to_string()).or_insert_with(|| CliNode::Value(node_id().into())).clone();
        triples.push(triple(&artist_id, &pred_is(), &root_artist_value()));
        triples.push(triple(&artist_id, &pred_name(), &node_value_str(name)));
        triples.push(triple(&artist_id, &pred_name_sort(), &node_value_str(name_sort)));
        return artist_id;
    };
    let albumset_id = CliNode::Value(node_id().into());
    triples.push(triple(&albumset_id, &pred_is(), &root_albumset_value()));
    triples.push(triple(&albumset_id, &pred_is(), &root_albumset_value()));
    triples.push(triple(&albumset_id, &pred_media(), &root_audio_value()));
    for v in albumset.name.iter().collect::<HashSet<_>>() {
        triples.push(triple(&albumset_id, &pred_name(), &node_value_str(&v)));
    }
    for v in albumset.name_sort.iter().collect::<HashSet<_>>() {
        triples.push(triple(&albumset_id, &pred_name_sort(), &node_value_str(v)));
    }

    fn pair_artists<'a>(artists: &'a [String], artists_sort: &'a [String]) -> HashSet<(&'a str, &'a str)> {
        let artists_sort_iter;
        if artists_sort.len() == 0 {
            artists_sort_iter = artists.iter();
        } else {
            artists_sort_iter = artists_sort.iter();
        }
        let out =
            artists
                .iter()
                .map(|x| x.as_str())
                .zip(artists_sort_iter.map(|x| x.as_str()))
                .collect::<HashSet<_>>();
        return out;
    }

    for (v, v_sort) in pair_artists(&albumset.artist, &albumset.artist_sort) {
        let artist = build_artist(&mut triples, v, v_sort);
        triples.push(triple(&albumset_id, &pred_artist(), &artist));
    }

    // Albums
    albumset.albums.sort_by_cached_key(|a| a.borrow().index.unwrap_or(usize::MAX));
    for album in &albumset.albums {
        let album = album.borrow();
        let album_id = CliNode::Value(album.id.clone().into());
        triples.push(triple(&albumset_id, &pred_element(), &album_id));
        if let Some(index) = album.index {
            triples.push(triple(&album_id, &pred_index(), &node_value_usize(index)));
        }
        triples.push(triple(&album_id, &pred_is(), &root_album_value()));
        triples.push(triple(&album_id, &pred_media(), &root_audio_value()));
        if album.name.len() >= 1 {
            for name in &album.name {
                triples.push(triple(&album_id, &pred_name(), &node_value_str(&name)));
            }
        } else if albumset.albums.len() > 1 && album.index.is_some() {
            let index = album.index.unwrap();
            for name in &albumset.name {
                triples.push(
                    triple(&album_id, &pred_name(), &node_value_str(&format!("{} (Disk {})", name, index))),
                );
            }
        }
        if album.name_sort.len() >= 1 {
            for name in &album.name_sort {
                triples.push(triple(&album_id, &pred_name_sort(), &node_value_str(&name)));
            }
        } else if albumset.albums.len() > 1 && album.index.is_some() {
            let index = album.index.unwrap();
            for album_name_sort in &albumset.name_sort {
                triples.push(
                    triple(
                        &album_id,
                        &pred_name_sort(),
                        &node_value_str(&format!("{} (Disk {})", album_name_sort, index)),
                    ),
                );
            }
        }
        for (v, v_sort) in pair_artists(&album.artist, &album.artist_sort) {
            let artist = build_artist(&mut triples, v, v_sort);
            triples.push(triple(&album_id, &pred_artist(), &artist));
        }

        // Tracks
        for track in &album.tracks {
            let track = track.borrow();
            let track_id = CliNode::Value(track.id.clone().into());
            triples.push(triple(&album_id, &pred_element(), &track_id));
            if let Some(index) = track.index {
                triples.push(triple(&track_id, &pred_index(), &node_value_usize(index)));
            }
            triples.push(triple(&track_id, &pred_is(), &root_track_value()));
            match track.type_ {
                GatherTrackType::Audio => {
                    triples.push(triple(&track_id, &pred_media(), &root_audio_value()));
                },
                GatherTrackType::Video => {
                    triples.push(triple(&track_id, &pred_media(), &root_video_value()));
                },
            }
            triples.push(triple(&track_id, &pred_file(), &node_upload(&root_dir, &track.file)));
            for v in track.name.iter().collect::<HashSet<_>>() {
                triples.push(triple(&track_id, &pred_name(), &node_value_str(v)));
            }
            for v in track.name_sort.iter().collect::<HashSet<_>>() {
                triples.push(triple(&track_id, &pred_name_sort(), &node_value_str(v)));
            }
            for (v, v_sort) in pair_artists(&track.artist, &track.artist_sort) {
                let artist = build_artist(&mut triples, v, v_sort);
                triples.push(triple(&track_id, &pred_artist(), &artist));
            }
        }
    }
    let mut assoc_nontrack = |files: Vec<PathBuf>, predicate: &str| {
        for v in files {
            let mut subj = None;
            for parent in parents(&root_dir, v.parent().unwrap()) {
                match dir_associations.get(&parent) {
                    Some(assoc) => {
                        if assoc.album.len() == 1 {
                            subj = Some(CliNode::Value(assoc.album.iter().next().unwrap().clone().into()));
                        } else {
                            subj = Some(albumset_id.clone());
                        }
                        break;
                    },
                    None => { },
                }
            }
            let subj = subj.unwrap_or_else(|| albumset_id.clone());
            triples.push(triple(&subj, &predicate, &node_upload(&root_dir, &v)));
        }
    };
    assoc_nontrack(images, &pred_image());
    assoc_nontrack(documents, &pred_document());
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
