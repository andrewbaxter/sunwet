//! There are two types of generated files:
//!
//! * Replacements: like aac replacement for mp3, or transcoded media (lower res)
//!
//! * Auxiliary files: like extracted files (cover, vtt, book contents)
//!
//! Requests for replacement generated files will fall back to the original file if
//! the generated file doesn't exist, but missing aux files will respond with a 404.
//!
//! The difference is indicated with convention. Replacements are directly in the
//! generated files dir, whereas all auxiliary files must be placed in a directory
//! in the generated files dir. If the subpath is empty then the file server will
//! do the fallback.
use {
    crate::{
        interface::triple::DbNode,
        server::{
            db,
            dbutil::tx,
            filesutil::{
                file_path,
                genfile_path,
                get_hash_from_file_path,
                get_meta,
            },
            fsutil::{
                create_dirs,
                delete_tree,
                soft_read_dir,
            },
            state::{
                BackgroundJob,
                State,
            },
        },
    },
    async_walkdir::WalkDir,
    chrono::Utc,
    deadpool_sqlite::Pool,
    enclose::enclose,
    flowcontrol::{
        exenum,
        ta_return,
    },
    image::ImageReader,
    loga::{
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
        ea,
    },
    regex::Regex,
    serde::Deserialize,
    shared::{
        interface::{
            derived::{
                ComicManifest,
                ComicManifestPage,
            },
            triple::{
                FileHash,
                Node,
            },
            wire::{
                GEN_FILENAME_COMICMANIFEST,
                GENTYPE_CBZDIR,
                GENTYPE_EPUBHTML,
                GENTYPE_VTT,
                TRANSCODE_MIME_AAC,
                TRANSCODE_MIME_WEBM,
                gentype_transcode,
                gentype_vtt_subpath,
            },
        },
        steal,
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        path::{
            Path,
            PathBuf,
        },
        process::Stdio,
        sync::{
            Arc,
            Mutex,
        },
        time::SystemTime,
    },
    taskmanager::TaskManager,
    tempfile::tempdir_in,
    tokio::{
        fs::{
            read_to_string,
            remove_dir_all,
            remove_file,
            write,
        },
        process::Command,
        select,
        sync::mpsc::UnboundedReceiver,
    },
    tokio_stream::{
        StreamExt,
        wrappers::UnboundedReceiverStream,
    },
};

async fn generated_exists(state: &Arc<State>, file: &FileHash, gentype: &str) -> Result<bool, loga::Error> {
    let found = tx(&state.db, {
        let gentype = gentype.to_string();
        let file = file.clone();
        move |txn| {
            return Ok(db::gen_get(txn, &DbNode(Node::File(file)), &gentype)?);
        }
    }).await?;
    match found {
        Some(_) => {
            return Ok(genfile_path(state, file, gentype, "")?.exists());
        },
        None => {
            return Ok(false);
        },
    }
}

async fn commit_generated(
    state: &Arc<State>,
    file: FileHash,
    gentype: &str,
    mimetype: &str,
    temp_path: &Path,
    dest_path: &Path,
) -> Result<(), loga::Error> {
    delete_tree(dest_path).await?;
    if let Some(p) = dest_path.parent() {
        create_dirs(&p).await.context("Failed to create parent directories for generated file")?;
    }
    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .context_with(
            "Error moving generated file to final destination",
            ea!(source = temp_path.dbg_str(), dest = dest_path.dbg_str()),
        )?;
    let gentype = gentype.to_string();
    let mimetype = mimetype.to_string();
    tx(&state.db, move |txn| {
        return Ok(db::gen_ensure(txn, &DbNode(Node::File(file)), &gentype, &mimetype)?);
    }).await?;
    return Ok(());
}

#[derive(Deserialize)]
struct FfprobeStream {
    index: usize,
    codec_type: Option<String>,
    codec_name: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
}

async fn ffprobe(path: &Path) -> Result<FfprobeOutput, loga::Error> {
    let mut cmd = Command::new("ffprobe");
    cmd.kill_on_drop(true);
    cmd.stdin(Stdio::null());
    cmd.args(&["-v", "quiet"]);
    cmd.args(&["-print_format", "json"]);
    cmd.arg("-show_streams");
    cmd.arg(path);
    let streams_res = cmd.output().await?;
    if !streams_res.status.success() {
        return Err(
            loga::err_with("Ffprobe failed", ea!(path = path.dbg_str(), output = streams_res.pretty_dbg_str())),
        );
    }
    return Ok(
        serde_json::from_slice::<FfprobeOutput>(&streams_res.stdout).context("Error parsing ffprobe output json")?,
    );
}

fn is_text_sub(codec_name: &str) -> bool {
    match codec_name {
        "ass" | "srt" | "ssa" | "webvtt" | "subrip" | "stl" => {
            return true;
        },
        _ => {
            return false;
        },
    }
}

async fn generate_subs(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    for stream in ffprobe(source).await?.streams {
        let (Some(codec_type), Some(codec_name)) = (stream.codec_type, stream.codec_name) else {
            continue;
        };
        if codec_type != "subtitle" {
            continue
        }
        if !is_text_sub(&codec_name) {
            continue;
        }
        let Some(lang) = stream.tags.get("language") else {
            continue;
        };
        let gentype = GENTYPE_VTT;
        if generated_exists(state, file, gentype).await? {
            continue;
        }
        let tmp = tempdir_in(&state.temp_dir)?;
        let tempdest_path = tmp.path().join("out");
        let mut cmd = Command::new("ffmpeg");
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd.arg("-i").arg(&source);
        cmd.arg("-map").arg(format!("0:{}", stream.index));
        cmd.args(&["-codec:s", "webvtt"]);
        cmd.args(&["-f", "webvtt"]);
        cmd.arg(&tempdest_path);
        let extract_res = cmd.output().await?;
        if !extract_res.status.success() {
            return Err(
                loga::err_with(
                    "Extracting subtitle track failed",
                    ea!(track = stream.index, output = extract_res.pretty_dbg_str()),
                ),
            );
        }
        commit_generated(
            state,
            file.clone(),
            gentype,
            "text/vtt",
            &tempdest_path,
            &genfile_path(&state, file, &gentype, &gentype_vtt_subpath(&lang))?,
        ).await?;
    }
    return Ok(());
}

async fn generate_webm(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = TRANSCODE_MIME_WEBM;
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let mut include_streams = vec![];
    let mut first_video_stream = None;
    {
        let ffprobe = ffprobe(source).await?.streams;
        for (i, stream) in ffprobe.into_iter().enumerate() {
            let (Some(codec_type), Some(codec_name)) = (stream.codec_type, stream.codec_name) else {
                continue;
            };
            match codec_type.as_str() {
                "subtitle" => {
                    if is_text_sub(&codec_name) {
                        include_streams.push(i);
                    }
                },
                "video" => {
                    if first_video_stream.is_none() {
                        first_video_stream = Some(i);
                    }
                },
                "audio" => {
                    include_streams.push(i);
                },
                _ => { },
            }
        }
    }
    let Some(first_video_stream) = first_video_stream else {
        return Err(loga::err_with("Video file has no video stream", ea!(path = source.dbg_str())));
    };

    // Ffmpeg pass abstraction is leaky, need to ensure video stream index matches for
    // both passes
    include_streams.insert(0, first_video_stream);
    let tmp = tempdir_in(&state.temp_dir)?;
    let passlog_path = tmp.path().join("passlog");
    let tempdest_path = tmp.path().join("out");
    {
        let mut cmd = Command::new("ffmpeg");
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd.arg("-i").arg(source);

        // Video
        cmd.arg("-map").arg(&format!("0:{}", first_video_stream));
        cmd.args(&["-b:v", "0"]);
        cmd.args(&["-crf", "30"]);

        // Output
        cmd.args(&["-pass", "1"]);
        cmd.arg("-passlogfile").arg(&passlog_path);
        cmd.args(&["-f", "webm"]);
        cmd.args(&["-y", "/dev/null"]);
        let pass1_res =
            cmd.output().await.context_with("Error starting webm conversion pass 1", ea!(command = cmd.dbg_str()))?;
        if !pass1_res.status.success() {
            return Err(
                loga::err_with(
                    "Generating webm, pass 1 failed",
                    ea!(output = pass1_res.pretty_dbg_str(), command = cmd.dbg_str()),
                ),
            );
        }
    }
    {
        let mut cmd = Command::new("ffmpeg");
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd.arg("-i").arg(source);
        for stream_i in include_streams {
            cmd.arg("-map").arg(&format!("0:{}", stream_i));
        }

        // Video
        cmd.args(&["-b:v", "0"]);
        cmd.args(&["-crf", "30"]);
        cmd.args(&["-pass", "2"]);
        cmd.arg("-passlogfile").arg(&passlog_path);

        // Audio
        cmd
        // ffmpeg bug 5718 re: opus 5.1(side)
        .args(&["-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"]);

        // Output
        cmd.args(&["-f", "webm"]);
        cmd.arg(&tempdest_path);
        let pass2_res =
            cmd.output().await.context_with("Error starting webm conversion pass 2", ea!(command = cmd.dbg_str()))?;
        if !pass2_res.status.success() {
            return Err(
                loga::err_with(
                    "Generating webm, pass 2 failed",
                    ea!(output = pass2_res.pretty_dbg_str(), command = cmd.dbg_str()),
                ),
            );
        }
    }
    commit_generated(
        state,
        file.clone(),
        &gentype,
        mimetype,
        &tempdest_path,
        &genfile_path(&state, file, &gentype, "")?,
    ).await?;
    return Ok(());
}

async fn generate_aac(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = TRANSCODE_MIME_AAC;
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let tmp = tempdir_in(&state.temp_dir)?;
    let tempdest_path = tmp.path().join("out");
    let mut cmd = Command::new("ffmpeg");
    cmd.kill_on_drop(true);
    cmd.arg("-i");
    cmd.arg(source);
    cmd.arg("-codec:a");
    cmd.arg("aac");
    cmd.arg("-f");
    cmd.arg("adts");
    cmd.arg(&tempdest_path);
    let res = cmd.output().await.context_with("Error converting audio to aac", ea!(command = cmd.dbg_str()))?;
    if !res.status.success() {
        return Err(
            loga::err_with("Error converting audio to aac", ea!(res = res.dbg_str(), command = cmd.dbg_str())),
        );
    }
    commit_generated(
        state,
        file.clone(),
        &gentype,
        mimetype,
        &tempdest_path,
        &genfile_path(&state, file, &gentype, "")?,
    ).await?;
    return Ok(());
}

async fn generate_book_html_dir(
    state: &Arc<State>,
    file: &FileHash,
    source: &Path,
    mime: &str,
) -> Result<(), loga::Error> {
    let gentype = GENTYPE_EPUBHTML;
    if generated_exists(state, file, gentype).await? {
        return Ok(());
    }
    let tmp_dest = tempdir_in(&state.temp_dir)?;
    let out = tmp_dest.path().join("index.html");
    let mut cmd = Command::new("pandoc");
    cmd.kill_on_drop(true);
    cmd.arg("--from");
    cmd.arg(match mime {
        "application/epub+zip" => "epub",
        _ => return Ok(()),
    });
    cmd.arg(source);
    cmd.arg("--standalone");
    cmd.arg("--self-contained");
    cmd.arg("--output");
    cmd.arg("index.html");
    cmd.arg("--extract-media");
    cmd.arg(".");
    let res =
        cmd
            .current_dir(tmp_dest.path())
            .output()
            .await
            .context_with("Error converting ebook to html", ea!(command = cmd.dbg_str()))?;
    if !res.status.success() {
        return Err(
            loga::err_with("Error converting ebook to html", ea!(res = res.dbg_str(), command = cmd.dbg_str())),
        );
    }
    commit_generated(
        state,
        file.clone(),
        gentype,
        "text/html",
        &out,
        &genfile_path(&state, file, gentype, "")?,
    ).await?;
    return Ok(());
}

async fn generate_comic_dir(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let gentype = GENTYPE_CBZDIR;
    if generated_exists(state, file, gentype).await? {
        return Ok(());
    }
    let tmp_dest = tempdir_in(&state.temp_dir)?;
    let mut cmd = Command::new("7zz");
    cmd.kill_on_drop(true);
    cmd.arg("x");
    cmd.arg(source);
    let res =
        cmd
            .current_dir(&tmp_dest)
            .output()
            .await
            .context_with("Error extracting comic", ea!(command = cmd.dbg_str()))?;
    if !res.status.success() {
        return Err(loga::err_with("Error extracting comic", ea!(res = res.dbg_str(), command = cmd.dbg_str())));
    }
    let index_matcher = Regex::new("(\\d+)").unwrap();
    let manga_matcher = Regex::new("<\\s*Manga\\s*>\\s*Yes").unwrap();
    let mut manifest = BTreeMap::new();
    let mut rtl = false;
    let mut dest_walk = WalkDir::new(&tmp_dest);
    while let Some(entry) = dest_walk.next().await {
        let entry = entry.context_with("Error reading entry in generated dir", ea!(dir = tmp_dest.dbg_str()))?;
        let path = entry.path();
        match async {
            ta_return!((), loga::Error);
            let meta = tokio::fs::metadata(&path).await.context("Error reading fs metadata")?;
            if !meta.is_file() {
                return Ok(());
            }
            let str_path = path.to_string_lossy();
            if str_path.to_ascii_lowercase().ends_with("comicinfo.xml") {
                rtl =
                    manga_matcher.is_match(
                        &read_to_string(entry.path()).await.context("Error reading comic info")?,
                    );
            } else if mime_guess::from_path(entry.path()).first_or_octet_stream().type_().as_str() == "image" {
                let mut sort = vec![];
                for seg in index_matcher.captures_iter(&str_path) {
                    sort.push(usize::from_str_radix(seg.get(1).unwrap().as_str(), 10).unwrap_or(usize::MAX));
                }
                let (width, height) =
                    ImageReader::open(entry.path())?.into_dimensions().context("Error reading image dimensions")?;
                manifest.insert(sort, ComicManifestPage {
                    width: width,
                    height: height,
                    path: entry.path().strip_prefix(&tmp_dest).unwrap().to_path_buf().to_string_lossy().to_string(),
                });
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                return Err(e.context_with("Error processing extacted comic file", ea!(path = path.dbg_str())));
            },
        }
    }
    let manifest_path = tmp_dest.path().join(GEN_FILENAME_COMICMANIFEST);
    write(&manifest_path, serde_json::to_string_pretty(&ComicManifest {
        rtl: rtl,
        pages: manifest.into_values().collect::<Vec<_>>(),
    }).unwrap()).await.context_with("Error creating sunwet manifest", ea!(path = manifest_path.dbg_str()))?;
    commit_generated(
        state,
        file.clone(),
        gentype,
        "",
        &tmp_dest.path(),
        &genfile_path(&state, file, gentype, "")?,
    ).await?;
    return Ok(());
}

async fn generate_files(
    state: &Arc<State>,
    log: &Log,
    file: &FileHash,
    include_slow: bool,
) -> Result<(), loga::Error> {
    let Some(meta) = get_meta(&state, &file).await? else {
        return Ok(());
    };
    let source = file_path(&state, &file)?;
    let mime = meta.mimetype.as_ref().map(|x| x.as_str()).unwrap_or("");
    let mime_slice = mime.split_once("/").unwrap_or((mime, ""));
    match (mime_slice.0, mime_slice.1) {
        ("video", _) if include_slow => {
            generate_subs(&state, &file, &source).await.log(&log, loga::WARN, "Error doing sub file generation");
            if mime_slice.1 != "webm" {
                generate_webm(&state, &file, &source)
                    .await
                    .log(&log, loga::WARN, "Error doing webm transcode file generation");
            }
        },
        ("audio", _) => {
            match mime_slice.1 {
                "aac" | "mp3" => { },
                _ => {
                    generate_aac(&state, &file, &source)
                        .await
                        .log(&log, loga::WARN, "Error doing webm transcode file generation");
                },
            }
        },
        ("application", "epub+zip") => {
            generate_book_html_dir(&state, &file, &source, mime)
                .await
                .log(&log, loga::WARN, "Error doing epub html generation");
        },
        ("application", "x-cbr") | ("application", "x-cbz") | ("application", "x-cb7") => {
            generate_comic_dir(&state, &file, &source)
                .await
                .log(&log, loga::WARN, "Error doing comic extraction/meta generation");
        },
        _ => { },
    }
    return Ok(());
}

pub fn start_background_job(state: &Arc<State>, tm: &TaskManager, rx: UnboundedReceiver<BackgroundJob>) {
    tm.stream("Background", UnboundedReceiverStream::new(rx), {
        let state = state.clone();
        let log = state.log.fork(ea!(subsys = "filegen"));
        let tm = tm.clone();
        move |file| {
            let log = log.clone();
            let state = state.clone();
            let tm = tm.clone();
            let work = async move {
                let log = &log;
                let state = &state;
                match file {
                    BackgroundJob::GenerateOne(file) => {
                        let log = log.fork(ea!(file = file.to_string()));
                        log.log_with(loga::DEBUG, "Generating one file", ea!(file = file));
                        generate_files(&state, &log, &file, true)
                            .await
                            .log(&log, loga::WARN, "Error generating derived files");
                    },
                    BackgroundJob::All => {
                        match async {
                            ta_return!((), loga::Error);

                            // # Generate/derive files
                            log.log(loga::DEBUG, "Doing file generation");
                            for slow in [false, true] {
                                async fn generate_batch(
                                    state: &Arc<State>,
                                    dbc: &Pool,
                                    slow: bool,
                                    batch: Vec<DbNode>,
                                ) -> Result<(), loga::Error> {
                                    let (found_sub, found_obj) = tx(&dbc, {
                                        let batch = batch.clone();
                                        move |txn| {
                                            return Ok(
                                                (
                                                    db::node_include_current_existing_subj(
                                                        txn,
                                                        batch.iter().collect(),
                                                    )?,
                                                    db::node_include_current_existing_obj(
                                                        txn,
                                                        batch.iter().collect(),
                                                    )?,
                                                ),
                                            );
                                        }
                                    }).await?;
                                    let found_keys =
                                        found_sub.into_iter().chain(found_obj.into_iter()).collect::<HashSet<_>>();
                                    for key in batch {
                                        if !found_keys.contains(&key) {
                                            continue;
                                        }
                                        let file = exenum!(key.0, Node:: File(x) => x).unwrap();
                                        let log = state.log.fork(ea!(subsys = "filegen", file = file.to_string()));
                                        generate_files(&state, &log, &file, slow).await?;
                                    }
                                    return Ok(());
                                }

                                let batch = Arc::new(Mutex::new(vec![]));
                                soft_read_dir(&log, &state.files_dir, async |hash_type| {
                                    soft_read_dir(&log, &hash_type.path(), async |hash_part1| {
                                        soft_read_dir(&log, &hash_part1.path(), async |hash_part2| {
                                            soft_read_dir(&log, &hash_part2.path(), async |entry| {
                                                if !entry
                                                    .metadata()
                                                    .await
                                                    .stack_context(&log, "Error reading metadata")?
                                                    .is_file() {
                                                    return Ok(());
                                                }
                                                let path = entry.path();
                                                let log = log.fork(ea!(path = path.to_string_lossy()));
                                                let Some(file) =
                                                    get_hash_from_file_path(&log, &state.files_dir, &path) else {
                                                        return Ok(());
                                                    };
                                                let consume_batch = {
                                                    let mut batch = batch.lock().unwrap();
                                                    batch.push(DbNode(Node::File(file)));
                                                    if batch.len() >= 1000 {
                                                        Some(steal(&mut *batch))
                                                    } else {
                                                        None
                                                    }
                                                };
                                                if let Some(batch) = consume_batch {
                                                    generate_batch(&state, &state.db, slow, batch).await?;
                                                }
                                                return Ok(());
                                            }).await;
                                            return Ok(());
                                        }).await;
                                        return Ok(());
                                    }).await;
                                    return Ok(());
                                }).await;
                                let batch = steal(&mut *batch.lock().unwrap());
                                if !batch.is_empty() {
                                    generate_batch(&state, &state.db, slow, batch).await?;
                                }
                            }

                            // # Garbage collect
                            //
                            // Clean graph
                            log.log(loga::DEBUG, "Doing database garbage collection");
                            tx(&state.db, |txn| {
                                let epoch = Utc::now() - chrono::Duration::days(365);
                                db::triple_gc_deleted(txn, epoch)?;
                                db::meta_gc(txn)?;
                                db::commit_gc(txn)?;
                                db::gen_gc(txn)?;
                                return Ok(());
                            }).await?;

                            // Clean up unreferenced files
                            {
                                log.log(loga::DEBUG, "Doing unreferenced file garbage collection");

                                async fn clean_batch(
                                    log: &Log,
                                    dbc: &Pool,
                                    mut batch: HashMap<FileHash, PathBuf>,
                                ) -> Result<(), loga::Error> {
                                    let unfiltered_keys =
                                        batch.keys().map(|k| DbNode(Node::File(k.clone()))).collect::<Vec<_>>();
                                    let found_keys = tx(&dbc, move |txn| {
                                        return Ok(
                                            db::meta_include_existing(txn, unfiltered_keys.iter().collect())?,
                                        );
                                    }).await?;
                                    for key in found_keys {
                                        batch.remove(&exenum!(key.0, Node:: File(x) => x).unwrap());
                                    }
                                    for path in batch.values() {
                                        log.log_with(
                                            loga::DEBUG,
                                            "Garbage collecting file",
                                            ea!(file = path.dbg_str()),
                                        );
                                        remove_file(path)
                                            .await
                                            .log_with(
                                                &log,
                                                loga::WARN,
                                                "Failed to delete unreferenced file",
                                                ea!(path = path.display().to_string()),
                                            );
                                    }
                                    return Ok(());
                                }

                                let batch = Arc::new(Mutex::new(HashMap::new()));
                                soft_read_dir(&log, &state.files_dir, enclose!((batch) | entry_hashtype | async move {
                                    soft_read_dir(
                                        &log,
                                        &entry_hashtype.path(),
                                        enclose!((batch) | entry_hash1 | async move {
                                            soft_read_dir(
                                                &log,
                                                &entry_hash1.path(),
                                                enclose!((batch) | entry_hash2 | async move {
                                                    soft_read_dir(
                                                        &log,
                                                        &entry_hash2.path(),
                                                        enclose!((batch) | entry | async move {
                                                            let path = entry.path();
                                                            let Some(hash) =
                                                                get_hash_from_file_path(
                                                                    &log,
                                                                    &state.files_dir,
                                                                    &path
                                                                ) else {
                                                                    return Ok(());
                                                                };
                                                            let consume_batch = {
                                                                let mut batch = batch.lock().unwrap();
                                                                batch.insert(hash.clone(), path);
                                                                if batch.len() >= 1000 {
                                                                    Some(steal(&mut *batch))
                                                                } else {
                                                                    None
                                                                }
                                                            };
                                                            if let Some(batch) = consume_batch {
                                                                clean_batch(&log, &state.db, batch).await?;
                                                            }
                                                            return Ok(());
                                                        })
                                                    ).await;
                                                    return Ok(());
                                                })
                                            ).await;
                                            return Ok(());
                                        })
                                    ).await;
                                    return Ok(());
                                })).await;
                                let batch = steal(&mut *batch.lock().unwrap());
                                if !batch.is_empty() {
                                    clean_batch(&log, &state.db, batch).await?;
                                }
                            }

                            // Clean up unreferenced generated files
                            {
                                log.log(loga::DEBUG, "Doing unreferenced generated file garbage collection");

                                async fn clean_batch(
                                    log: &Log,
                                    dbc: &Pool,
                                    batch: Vec<(FileHash, PathBuf)>,
                                ) -> Result<(), loga::Error> {
                                    let unfiltered_keys =
                                        batch
                                            .iter()
                                            .map(|(k, _)| DbNode(Node::File(k.clone())))
                                            .collect::<HashSet<_>>();
                                    let found_keys =
                                        tx(&dbc, move |txn| {
                                            return Ok(
                                                db::gen_include_existing(txn, unfiltered_keys.iter().collect())?,
                                            );
                                        })
                                            .await?
                                            .into_iter()
                                            .map(|x| exenum!(x.0, Node:: File(x) => x).unwrap())
                                            .collect::<HashSet<_>>();
                                    for (hash, path) in batch {
                                        if found_keys.contains(&hash) {
                                            continue;
                                        }
                                        log.log_with(
                                            loga::DEBUG,
                                            "Garbage collecting generated file",
                                            ea!(path = path.dbg_str()),
                                        );
                                        match tokio::fs::metadata(&path).await {
                                            Ok(meta) => {
                                                if meta.is_dir() {
                                                    remove_dir_all(&path).await
                                                } else {
                                                    remove_file(&path).await
                                                }.log_with(
                                                    &log,
                                                    loga::WARN,
                                                    "Failed to delete unreferenced file",
                                                    ea!(path = path.display().to_string()),
                                                )
                                            },
                                            Err(e) => {
                                                log.log_err(
                                                    loga::WARN,
                                                    e.context_with(
                                                        "Unable to get file to clean metadata",
                                                        ea!(path = path.dbg_str()),
                                                    ),
                                                );
                                            },
                                        }
                                    }
                                    return Ok(());
                                }

                                let batch = Arc::new(Mutex::new(vec![]));
                                soft_read_dir(
                                    &log,
                                    &state.genfiles_dir,
                                    enclose!((batch) | entry_hashtype | async move {
                                        soft_read_dir(
                                            &log,
                                            &entry_hashtype.path(),
                                            enclose!((batch) | entry_hash1 | async move {
                                                soft_read_dir(
                                                    &log,
                                                    &entry_hash1.path(),
                                                    enclose!((batch) | entry_hash2 | async move {
                                                        soft_read_dir(
                                                            &log,
                                                            &entry_hash2.path(),
                                                            enclose!((batch) | entry | async move {
                                                                let path = entry.path();
                                                                let Some(hash) =
                                                                    get_hash_from_file_path(
                                                                        &log,
                                                                        &state.genfiles_dir,
                                                                        &path
                                                                    ) else {
                                                                        return Ok(());
                                                                    };
                                                                let consume_batch = {
                                                                    let mut batch = batch.lock().unwrap();
                                                                    batch.push((hash.clone(), path));
                                                                    if batch.len() >= 1000 {
                                                                        Some(steal(&mut *batch))
                                                                    } else {
                                                                        None
                                                                    }
                                                                };
                                                                if let Some(batch) = consume_batch {
                                                                    clean_batch(&log, &state.db, batch).await?;
                                                                }
                                                                return Ok(());
                                                            })
                                                        ).await;
                                                        return Ok(());
                                                    })
                                                ).await;
                                                return Ok(());
                                            })
                                        ).await;
                                        return Ok(());
                                    }),
                                ).await;
                                let batch = steal(&mut *batch.lock().unwrap());
                                if !batch.is_empty() {
                                    clean_batch(&log, &state.db, batch).await?;
                                }
                            }

                            // Clean up stale partially-uploaded files
                            log.log(loga::DEBUG, "Cleaning up stale partial uploads");
                            soft_read_dir(&log, &state.stage_dir, |entry| async move {
                                let day = std::time::Duration::from_secs(60 * 60 * 24);
                                let path = entry.path();
                                let log = log.fork(ea!(path = path.to_string_lossy()));
                                let meta =
                                    entry
                                        .metadata()
                                        .await
                                        .context_with("Error reading metadata", ea!(path = path.dbg_str()))?;
                                if !meta.is_file() {
                                    return Ok(());
                                }
                                let modified_time = match meta.modified() {
                                    Ok(t) => t,
                                    Err(e) => {
                                        log.log_err(
                                            loga::WARN,
                                            e.context_with(
                                                "Error reading file modified time, assuming old/corrupt and removing",
                                                ea!(path = path.dbg_str()),
                                            ),
                                        );
                                        SystemTime::UNIX_EPOCH
                                    },
                                };
                                if SystemTime::now().duration_since(modified_time).unwrap_or(day * 100) > day * 3 {
                                    log.log_with(
                                        loga::DEBUG,
                                        "Garbage collecting stale partial upload",
                                        ea!(file = path.dbg_str()),
                                    );
                                    remove_file(&path)
                                        .await
                                        .log_with(
                                            &log,
                                            loga::WARN,
                                            "Failed to delete stale partial upload file",
                                            ea!(path = path.display().to_string()),
                                        );
                                }
                                return Ok(());
                            }).await;
                            log.log(loga::DEBUG, "Background work done");
                            return Ok(());
                        }.await {
                            Ok(_) => { },
                            Err(e) => {
                                log.log_err(
                                    loga::WARN,
                                    e.context("Error walking existing files to confirm file generation"),
                                );
                            },
                        }
                    },
                }
            };
            async move {
                select!{
                    _ = work => {
                    },
                    _ = tm.until_terminate() => {
                    }
                }
            }
        }
    });
}
