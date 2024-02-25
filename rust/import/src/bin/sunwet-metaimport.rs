use std::{
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
};
use aargvark::Aargvark;
use loga::{
    ea,
    fatal,
    ErrContext,
    ResultContext,
    StandardFlag,
    StandardLog,
};
use shared::model::cli::{
    CliCommit,
    CliNode,
    CliTriple,
};
use symphonia::core::{
    io::MediaSourceStream,
    probe::Hint,
};
use walkdir::WalkDir;
use import::data::{
    node_id,
    node_upload,
    node_value_str,
    node_value_usize,
    pred_artist,
    pred_document,
    pred_image,
    pred_element,
    pred_file,
    pred_index,
    pred_is,
    pred_media,
    pred_name,
    pred_name_sort,
    root_album_id,
    root_albumset_id,
    root_artist_id,
    root_audio_id,
    root_track_id,
    root_video_id,
    triple,
};

/// Prepare a sunwet commit file for a directory/archive of media files based on
/// the metadata tags on those files (ex: ID3 tags).
#[derive(Aargvark)]
enum Args {
    File {
        file: PathBuf,
        out_dir: PathBuf,
    },
    Dir(PathBuf),
}

fn import_dir(log: &StandardLog, root_dir: PathBuf) -> Result<(), loga::Error> {
    let vlc_inst = vlc::Instance::new().context("Error initializing lib vlc")?;

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
                log.log(StandardFlag::Warning, e.context("Failed to inspect file in dir, skipping"));
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
        match e.as_bytes() {
            b"png" | b"jpg" | b"bmp" | b"tif" | b"gif" | b"webp" | b"webm" => {
                images.push(file.path().to_path_buf());
            },
            b"mp3" | b"m4a" | b"aac" | b"ogg" | b"flac" | b"alac" => {
                let mut info =
                    match symphonia
                    ::default
                    ::get_probe().format(
                        &Hint::new().with_extension(&e.to_str().unwrap()),
                        MediaSourceStream::new(Box::new(File::open(file.path())?), Default::default()),
                        &Default::default(),
                        &Default::default(),
                    ) {
                        Ok(i) => i,
                        Err(e) => {
                            log.log_err(StandardFlag::Warning, e.context("Unable to read audio file metadata"));
                            continue;
                        },
                    };
                let mut album_artist = vec![];
                let mut album_artist_sort = vec![];
                let mut track_artist = vec![];
                let mut track_artist_sort = vec![];
                let mut track_name = vec![];
                let mut track_name_sort = vec![];
                let mut track_number = None;
                let mut disk_number = None;
                let Some(metadata) = info.metadata.get() else {
                    log.log(StandardFlag::Warning, "File has no metadata, skipping");
                    continue;
                };
                let Some(metadata) = metadata.current() else {
                    log.log(StandardFlag::Warning, "File has no metadata revision, skipping");
                    continue;
                };
                for tag in metadata.tags() {
                    match tag.std_key {
                        Some(k) => match k {
                            symphonia::core::meta::StandardTagKey::Album => {
                                albumset.name.push(tag.value.to_string());
                            },
                            symphonia::core::meta::StandardTagKey::AlbumArtist => {
                                albumset.artist.push(tag.value.to_string());
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
                                albumset.artist_sort.push(tag.value.to_string());
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
                let album = match albumset.albums.iter().find(|a| a.borrow().index == disk_number) {
                    Some(a) => a.clone(),
                    None => {
                        let a = Rc::new(RefCell::new(GatherAlbum {
                            id: node_id(),
                            index: disk_number,
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
            },
            b"mp4" | b"mkv" => {
                let info = match vlc::Media::new_path(&vlc_inst, file.path()) {
                    Some(i) => i,
                    None => {
                        log.log(StandardFlag::Warning, "Unable to read metadata in video file");
                        continue;
                    },
                };
                for tag_k in [vlc::Meta::Album, vlc::Meta::ShowName] {
                    if let Some(tag_value) = info.get_meta(tag_k) {
                        albumset.name.push(tag_value.clone());
                    }
                }
                let mut disk_number = None;
                for tag_k in [vlc::Meta::Season] {
                    if let Some(tag_value) = info.get_meta(tag_k) {
                        disk_number = Some(usize::from_str_radix(&tag_value, 10)?);
                    }
                }
                let album = match albumset.albums.iter().find(|a| a.borrow().index == disk_number) {
                    Some(a) => a.clone(),
                    None => {
                        let a = Rc::new(RefCell::new(GatherAlbum {
                            id: node_id(),
                            index: disk_number,
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
                let mut track_artist = vec![];
                for tag_k in [vlc::Meta::Artist] {
                    if let Some(tag_value) = info.get_meta(tag_k) {
                        track_artist.push(tag_value);
                    }
                }
                album.artist.extend(track_artist.clone());
                album.artist_sort.extend(track_artist.clone());
                albumset.artist.extend(track_artist.clone());
                albumset.artist_sort.extend(track_artist.clone());
                album.tracks.push(Rc::new(RefCell::new(GatherTrack {
                    type_: GatherTrackType::Video,
                    id: node_id(),
                    index: {
                        let mut track_number = None;
                        for tag_k in [vlc::Meta::TrackNumber, vlc::Meta::Episode] {
                            if let Some(tag_value) = info.get_meta(tag_k) {
                                track_number = Some(usize::from_str_radix(&tag_value, 10)?);
                            }
                        }
                        track_number
                    },
                    file: file.path().to_path_buf(),
                    artist: track_artist,
                    artist_sort: vec![],
                    name: {
                        let mut values = vec![];
                        for tag_k in [vlc::Meta::Title] {
                            if let Some(tag_value) = info.get_meta(tag_k) {
                                values.push(tag_value);
                            }
                        }
                        values
                    },
                    name_sort: vec![],
                })));
            },
            b"txt" | b"md" | b"doc" | b"docx" | b"odt" | b"pdf" | b"rst" => {
                documents.push(file.path().to_path_buf());
            },
            _ => {
                continue;
            },
        }
    }

    // Turn gathered data into triples
    let mut artists = HashMap::<String, CliNode>::new();
    let mut triples = vec![];
    let mut build_artist = |triples: &mut Vec<CliTriple>, name: &str, name_sort: &str| -> CliNode {
        let artist_id = artists.entry(name.to_string()).or_insert_with(|| CliNode::Id(node_id())).clone();
        triples.push(triple(&artist_id, &pred_is(), &root_artist_id()));
        triples.push(triple(&artist_id, &pred_name(), &node_value_str(name)));
        triples.push(triple(&artist_id, &pred_name_sort(), &node_value_str(name_sort)));
        return artist_id;
    };
    let albumset_id = CliNode::Id(node_id());
    triples.push(triple(&albumset_id, &pred_is(), &root_albumset_id()));
    triples.push(triple(&albumset_id, &pred_is(), &root_albumset_id()));
    triples.push(triple(&albumset_id, &pred_media(), &root_audio_id()));
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
        let album_id = CliNode::Id(album.id.clone());
        triples.push(triple(&albumset_id, &pred_element(), &album_id));
        if let Some(index) = album.index {
            triples.push(triple(&album_id, &pred_index(), &node_value_usize(index)));
        }
        triples.push(triple(&album_id, &pred_is(), &root_album_id()));
        triples.push(triple(&album_id, &pred_media(), &root_audio_id()));
        if albumset.albums.len() > 1 && album.index.is_some() {
            let index = album.index.unwrap();
            for name in &albumset.name {
                triples.push(
                    triple(&album_id, &pred_name(), &node_value_str(&format!("{} (Disk {})", name, index))),
                );
            }
            for album_name_sort in &albumset.name_sort {
                triples.push(
                    triple(
                        &album_id,
                        &pred_name_sort(),
                        &node_value_str(&format!("{} (Disk {})", album_name_sort, index)),
                    ),
                );
                break;
            }
        }
        for (v, v_sort) in pair_artists(&album.artist, &album.artist_sort) {
            let artist = build_artist(&mut triples, v, v_sort);
            triples.push(triple(&album_id, &pred_artist(), &artist));
        }

        // Tracks
        for track in &album.tracks {
            let track = track.borrow();
            let track_id = CliNode::Id(track.id.clone());
            triples.push(triple(&album_id, &pred_element(), &track_id));
            if let Some(index) = track.index {
                triples.push(triple(&track_id, &pred_index(), &node_value_usize(index)));
            }
            triples.push(triple(&track_id, &pred_is(), &root_track_id()));
            match track.type_ {
                GatherTrackType::Audio => {
                    triples.push(triple(&track_id, &pred_media(), &root_audio_id()));
                },
                GatherTrackType::Video => {
                    triples.push(triple(&track_id, &pred_media(), &root_video_id()));
                },
            }
            triples.push(triple(&track_id, &pred_file(), &node_upload(&track.file)));
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
                            subj = Some(CliNode::Id(assoc.album.iter().next().unwrap().clone()));
                        } else {
                            subj = Some(albumset_id.clone());
                        }
                        break;
                    },
                    None => { },
                }
            }
            let subj = subj.unwrap_or_else(|| albumset_id.clone());
            triples.push(triple(&subj, &predicate, &node_upload(&v)));
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

fn main1() -> Result<(), loga::Error> {
    let args = aargvark::vark::<Args>();
    let log = loga::StandardLog::new().with_flags(&[StandardFlag::Error, StandardFlag::Warning, StandardFlag::Info]);
    match args {
        Args::File { file, out_dir } => {
            let log = log.fork(ea!(source = file.to_string_lossy(), dest = out_dir.to_string_lossy()));
            match file.extension() {
                Some(e) => match e.as_bytes() {
                    b"zip" => {
                        create_dir_all(&out_dir)?;
                        let mut zip =
                            zip::ZipArchive::new(
                                File::open(file).stack_context(&log, "Error opening file")?,
                            ).stack_context(&log, "Error opening file as zip archive")?;
                        for i in 0 .. zip.len() {
                            match (|| {
                                let mut file = zip.by_index(i)?;
                                let log = StandardLog::new().fork(ea!(archive_path = file.name()));
                                let mut out =
                                    File::create(
                                        out_dir.join(
                                            file.enclosed_name().stack_context(&log, "File has invalid name")?,
                                        ),
                                    ).stack_context(&log, "Error creating file in output directory")?;
                                io::copy(&mut file, &mut out).stack_context(&log, "Error extracting contents")?;
                                return Ok(()) as Result<(), loga::Error>;
                            })() {
                                Ok(_) => (),
                                Err(e) => {
                                    log.log_err(
                                        StandardFlag::Warning,
                                        e.context_with(
                                            "Error extracting file {} from archive, skipping",
                                            ea!(index = i),
                                        ),
                                    );
                                },
                            }
                        }
                        import_dir(&log, out_dir)?;
                    },
                    _ => {
                        return Err(loga::err("Unsupported source file type"));
                    },
                },
                None => {
                    return Err(loga::err("File has no extension, unable to determine type"));
                },
            }
        },
        Args::Dir(d) => {
            let log = log.fork(ea!(dest = d.to_string_lossy()));
            import_dir(&log, d)?;
        },
    }
    return Ok(());
}

fn main() {
    match main1() {
        Ok(_) => { },
        Err(e) => fatal(e),
    }
}
