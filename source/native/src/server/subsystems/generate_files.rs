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
            state::State,
        },
    },
    async_walkdir::WalkDir,
    flowcontrol::ta_return,
    image::ImageReader,
    loga::{
        ea,
        DebugDisplay,
        Log,
        ResultContext,
    },
    regex::Regex,
    serde::Deserialize,
    shared::interface::{
        derived::{
            ComicManifest,
            ComicManifestPage,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            gentype_transcode,
            gentype_vtt_subpath,
            GENTYPE_DIR,
            GENTYPE_VTT,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
        },
        path::Path,
        process::Stdio,
        sync::Arc,
    },
    taskmanager::TaskManager,
    tempfile::tempdir_in,
    tokio::{
        fs::{
            read_to_string,
            write,
        },
        process::Command,
        select,
        sync::mpsc::UnboundedReceiver,
    },
    tokio_stream::{
        wrappers::UnboundedReceiverStream,
        StreamExt,
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
    gentype: String,
    mimetype: String,
    temp_path: &Path,
    dest_path: &Path,
) -> Result<(), loga::Error> {
    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .context_with(
            "Error moving generated file to final destination",
            ea!(source = temp_path.dbg_str(), dest = dest_path.dbg_str()),
        )?;
    tx(&state.db, move |txn| {
        return Ok(db::gen_insert(txn, &DbNode(Node::File(file)), &gentype, &mimetype)?);
    }).await?;
    return Ok(());
}

async fn generate_subs(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let streams_res =
        Command::new("ffprobe")
            .stdin(Stdio::null())
            .args(&["-v", "quiet"])
            .args(&["-print_format", "json"])
            .arg("-show_streams")
            .arg(&source)
            .output()
            .await?;
    if !streams_res.status.success() {
        return Err(loga::err_with("Getting video streams failed", ea!(output = streams_res.pretty_dbg_str())));
    }

    #[derive(Deserialize)]
    struct Stream {
        index: usize,
        codec_type: Option<String>,
        codec_name: Option<String>,
        #[serde(default)]
        tags: HashMap<String, String>,
    }

    #[derive(Deserialize)]
    struct Streams {
        streams: Vec<Stream>,
    }

    let streams =
        serde_json::from_slice::<Streams>(&streams_res.stdout).context("Error parsing video streams json")?;
    for stream in streams.streams {
        let (Some(codec_type), Some(codec_name)) = (stream.codec_type, stream.codec_name) else {
            continue;
        };
        if codec_type != "subtitle" {
            continue
        }
        match codec_name.as_str() {
            "ass" | "srt" | "ssa" | "webvtt" | "subrip" | "stl" => { },
            _ => {
                continue
            },
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
        let extract_res =
            Command::new("ffmpeg")
                .stdin(Stdio::null())
                .arg("-i")
                .arg(&source)
                .args(&["-map", "0:s:0"])
                .args(&["-codec:s", "webvtt"])
                .args(&["-f", "webvtt"])
                .arg(&tempdest_path)
                .output()
                .await?;
        if !extract_res.status.success() {
            return Err(
                loga::err_with(
                    "Extracting subtitle track failed",
                    ea!(track = stream.index, output = extract_res.pretty_dbg_str()),
                ),
            );
        }
        let dest = genfile_path(&state, file, &gentype, &gentype_vtt_subpath(&lang))?;
        if let Some(p) = dest.parent() {
            create_dirs(&p).await.context("Failed to create parent directories for generated subtitle file")?;
        }
        commit_generated(
            state,
            file.clone(),
            gentype.to_string(),
            "text/vtt".to_string(),
            &tempdest_path,
            &dest,
        ).await?;
    }
    return Ok(());
}

async fn generate_webm(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = "video/webm";
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let tmp = tempdir_in(&state.temp_dir)?;
    let passlog_path = tmp.path().join("passlog");
    let tempdest_path = tmp.path().join("out");
    {
        let mut cmd_pass1 = Command::new("ffmpeg");
        cmd_pass1
            .stdin(Stdio::null())
            .arg("-i")
            .arg(source)
            .args(&["-b:v", "0"])
            .args(&["-crf", "30"])
            .args(&["-pass", "1"])
            .arg("-passlogfile")
            .arg(&passlog_path)
            .arg("-an")
            .args(&["-f", "webm"])
            .args(&["-y", "/dev/null"]);
        let pass1_res =
            cmd_pass1
                .output()
                .await
                .context_with("Error starting webm conversion pass 1", ea!(command = cmd_pass1.dbg_str()))?;
        if !pass1_res.status.success() {
            return Err(
                loga::err_with(
                    "Generating webm, pass 1 failed",
                    ea!(output = pass1_res.pretty_dbg_str(), command = cmd_pass1.dbg_str()),
                ),
            );
        }
    }
    {
        let mut cmd_pass2 = Command::new("ffmpeg");
        cmd_pass2
            .stdin(Stdio::null())
            .arg("-i")
            .arg(source)
            // ffmpeg bug 5718 re: opus 5.1(side)
            .args(&["-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"])
            .args(&["-b:v", "0"])
            .args(&["-crf", "30"])
            .args(&["-pass", "2"])
            .arg("-passlogfile")
            .arg(&passlog_path)
            .args(&["-f", "webm"])
            .arg(&tempdest_path);
        let pass2_res =
            cmd_pass2
                .output()
                .await
                .context_with("Error starting webm conversion pass 2", ea!(command = cmd_pass2.dbg_str()))?;
        if !pass2_res.status.success() {
            return Err(
                loga::err_with(
                    "Generating webm, pass 2 failed",
                    ea!(output = pass2_res.pretty_dbg_str(), command = cmd_pass2.dbg_str()),
                ),
            );
        }
    }
    let dest = genfile_path(&state, file, &gentype, "")?;
    delete_tree(&dest).await?;
    if let Some(p) = dest.parent() {
        create_dirs(&p).await.context("Failed to create parent directories for generated webm file")?;
    }
    commit_generated(state, file.clone(), gentype, mimetype.to_string(), &tempdest_path, &dest).await?;
    return Ok(());
}

async fn generate_aac(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = "audio/aac";
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let tmp = tempdir_in(&state.temp_dir)?;
    let tempdest_path = tmp.path().join("out");
    let mut cmd = Command::new("ffmpeg");
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
    let dest = genfile_path(&state, file, &gentype, "")?;
    delete_tree(&dest).await?;
    if let Some(p) = dest.parent() {
        create_dirs(&p).await.context("Failed to create parent directories for generated audio file")?;
    }
    commit_generated(state, file.clone(), gentype, mimetype.to_string(), &tempdest_path, &dest).await?;
    return Ok(());
}

async fn generate_book_html_dir(
    state: &Arc<State>,
    file: &FileHash,
    source: &Path,
    mime: &str,
) -> Result<(), loga::Error> {
    let gentype = GENTYPE_DIR;
    if generated_exists(state, file, gentype).await? {
        return Ok(());
    }
    let tmp_dest = tempdir_in(&state.temp_dir)?;
    let mut cmd = Command::new("pandoc");
    cmd.arg("--from");
    cmd.arg(match mime {
        "application/epub+zip" => "epub",
        _ => return Ok(()),
    });
    cmd.arg(source);
    cmd.arg("--standalone");
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
    let dest = genfile_path(&state, file, gentype, "")?;
    delete_tree(&dest).await?;
    if let Some(p) = dest.parent() {
        create_dirs(&p).await?;
    }
    commit_generated(state, file.clone(), gentype.to_string(), "".to_string(), tmp_dest.path(), &dest).await?;
    return Ok(());
}

async fn generate_comic_dir(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let gentype = GENTYPE_DIR;
    if generated_exists(state, file, gentype).await? {
        return Ok(());
    }
    let tmp_dest = tempdir_in(&state.temp_dir)?;
    let mut cmd = Command::new("7zz");
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
    let manifest_path = tmp_dest.path().join("sunwet.json");
    write(&manifest_path, serde_json::to_string_pretty(&ComicManifest {
        rtl: rtl,
        pages: manifest.into_values().collect::<Vec<_>>(),
    }).unwrap()).await.context_with("Error creating sunwet manifest", ea!(path = manifest_path.dbg_str()))?;
    let dest = genfile_path(&state, file, gentype, "")?;
    delete_tree(&dest).await?;
    if let Some(p) = dest.parent() {
        create_dirs(&p).await?;
    }
    commit_generated(state, file.clone(), gentype.to_string(), "".to_string(), &tmp_dest.path(), &dest).await?;
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

pub fn start_generate_files(state: &Arc<State>, tm: &TaskManager, rx: UnboundedReceiver<Option<FileHash>>) {
    tm.stream("Generate files", UnboundedReceiverStream::new(rx), {
        let state = state.clone();
        let log = state.log.fork(ea!(subsys = "filegen"));
        let tm = tm.clone();
        move |file| {
            let log = log.clone();
            let state = state.clone();
            let tm = tm.clone();
            let work = async move {
                match file {
                    Some(file) => {
                        let log = log.fork(ea!(file = file.to_string()));
                        generate_files(&state, &log, &file, true)
                            .await
                            .log(&log, loga::WARN, "Error generating derived files");
                    },
                    None => {
                        match async {
                            ta_return!((), loga::Error);
                            for slow in [false, true] {
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
                                                let log =
                                                    state.log.fork(ea!(subsys = "filegen", file = file.to_string()));
                                                generate_files(&state, &log, &file, slow).await?;
                                                return Ok(());
                                            }).await;
                                            return Ok(());
                                        }).await;
                                        return Ok(());
                                    }).await;
                                    return Ok(());
                                }).await;
                            }
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
