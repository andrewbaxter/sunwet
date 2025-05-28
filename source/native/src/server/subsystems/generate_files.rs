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
            state::State,
        },
    },
    async_walkdir::WalkDir,
    flowcontrol::ta_return,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    serde::Deserialize,
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            gentype_transcode,
            gentype_vtt,
        },
    },
    std::{
        collections::HashMap,
        path::Path,
        process::Stdio,
        sync::Arc,
    },
    taskmanager::TaskManager,
    tempfile::tempdir_in,
    tokio::{
        fs::create_dir_all,
        process::Command,
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
            return Ok(genfile_path(state, file, gentype)?.exists());
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
) -> Result<(), loga::Error> {
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
        codec_type: String,
        codec_name: String,
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
        if stream.codec_type != "subtitle" {
            continue
        }
        match stream.codec_name.as_str() {
            "ass" | "srt" | "ssa" | "webvtt" | "subrip" | "stl" => { },
            _ => {
                continue
            },
        }
        let Some(lang) = stream.tags.get("language") else {
            continue;
        };
        let gentype = gentype_vtt(&lang);
        if generated_exists(state, file, &gentype).await? {
            continue;
        }
        let dest = genfile_path(&state, file, &gentype)?;
        if let Some(p) = dest.parent() {
            create_dir_all(&p)
                .await
                .context_with(
                    "Failed to create parent directories for generated subtitle file",
                    ea!(path = dest.display()),
                )?;
        }
        let extract_res =
            Command::new("ffmpeg")
                .stdin(Stdio::null())
                .arg("-i")
                .arg(&source)
                .args(&["-map", "0:s:0"])
                .args(&["-codec:s", "webvtt"])
                .args(&["-f", "webvtt"])
                .arg(&dest)
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
        commit_generated(state, file.clone(), gentype, "text/vtt".to_string()).await?;
    }
    return Ok(());
}

async fn generate_webm(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = "video/webm";
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let dest = genfile_path(&state, file, &gentype)?;
    if let Some(p) = dest.parent() {
        create_dir_all(&p)
            .await
            .context_with("Failed to create parent directories for generated webm file", ea!(path = dest.display()))?;
    }
    let tmp = tempdir_in(&state.temp_dir)?;
    let passlog_path = tmp.path().join("passlog");
    let pass1_res =
        Command::new("ffmpeg")
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
            .args(&["-y", "/dev/null"])
            .output()
            .await
            .context("Error starting webm conversion pass 1")?;
    if !pass1_res.status.success() {
        return Err(loga::err_with("Generating webm, pass 1 failed", ea!(output = pass1_res.pretty_dbg_str())));
    }
    let pass2_res =
        Command::new("ffmpeg")
            .stdin(Stdio::null())
            .arg("-i")
            .arg(source)
            .args(&["-b:v", "0"])
            .args(&["-crf", "30"])
            .args(&["-pass", "2"])
            .arg("-passlogfile")
            .arg(&passlog_path)
            .args(&["-f", "webm"])
            .arg(&dest)
            .output()
            .await
            .context("Error starting webm conversion pass 2")?;
    if !pass2_res.status.success() {
        return Err(loga::err_with("Generating webm, pass 2 failed", ea!(output = pass2_res.pretty_dbg_str())));
    }
    commit_generated(state, file.clone(), gentype, mimetype.to_string()).await?;
    return Ok(());
}

async fn generate_aac(state: &Arc<State>, file: &FileHash, source: &Path) -> Result<(), loga::Error> {
    let mimetype = "audio/aac";
    let gentype = gentype_transcode(mimetype);
    if generated_exists(state, file, &gentype).await? {
        return Ok(());
    }
    let dest = genfile_path(&state, file, &gentype)?;
    if let Some(p) = dest.parent() {
        create_dir_all(&p)
            .await
            .context_with(
                "Failed to create parent directories for generated audio file",
                ea!(path = dest.display()),
            )?;
    }
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i");
    cmd.arg(source);
    cmd.arg("-codec:a");
    cmd.arg("aac");
    cmd.arg("-f");
    cmd.arg("adts");
    cmd.arg(&dest);
    let res = cmd.output().await.context_with("Error converting audio to aac", ea!(command = cmd.dbg_str()))?;
    if !res.status.success() {
        return Err(
            loga::err_with("Error converting audio to aac", ea!(res = res.dbg_str(), command = cmd.dbg_str())),
        );
    }
    commit_generated(state, file.clone(), gentype, mimetype.to_string()).await?;
    return Ok(());
}

async fn generate_files(state: &Arc<State>, log: &Log, file: &FileHash) -> Result<(), loga::Error> {
    let Some(meta) = get_meta(&state, &file).await? else {
        return Ok(());
    };
    let source = file_path(&state, &file)?;
    let mime = meta.mimetype.as_str().split_once("/").unwrap_or((meta.mimetype.as_str(), ""));
    match mime.0 {
        "video" => {
            generate_subs(&state, &file, &source).await.log(&log, loga::WARN, "Error doing sub file generation");
            if mime.1 != "webm" {
                generate_webm(&state, &file, &source)
                    .await
                    .log(&log, loga::WARN, "Error doing webm transcode file generation");
            }
        },
        "audio" => {
            match mime.1 {
                "aac" | "mp3" => { },
                _ => {
                    generate_aac(&state, &file, &source)
                        .await
                        .log(&log, loga::WARN, "Error doing webm transcode file generation");
                },
            }
        },
        _ => { },
    }
    return Ok(());
}

pub fn start_generate_files(state: &Arc<State>, tm: &TaskManager, rx: UnboundedReceiver<Option<FileHash>>) {
    tm.stream("Generate files", UnboundedReceiverStream::new(rx), {
        let state = state.clone();
        let log = state.log.fork(ea!(subsys = "filegen"));
        move |file| {
            let log = log.clone();
            let state = state.clone();
            async move {
                match file {
                    Some(file) => {
                        let log = log.fork(ea!(file = file.to_string()));
                        generate_files(&state, &log, &file)
                            .await
                            .log(&log, loga::WARN, "Error generating derived files");
                    },
                    None => {
                        match async {
                            ta_return!((), loga::Error);
                            let mut walk = WalkDir::new(&state.files_dir);
                            while let Some(entry) = walk.next().await {
                                let entry = match entry {
                                    Ok(entry) => entry,
                                    Err(e) => {
                                        log.log_err(
                                            loga::WARN,
                                            e.stack_context(&log, "Unable to scan file in files_dir"),
                                        );
                                        continue;
                                    },
                                };
                                let path = entry.path();
                                if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
                                    continue;
                                }
                                let log = log.fork(ea!(path = path.to_string_lossy()));
                                let Some(file) = get_hash_from_file_path(&log, &state.files_dir, &path) else {
                                    continue;
                                };
                                let log = state.log.fork(ea!(subsys = "filegen", file = file.to_string()));
                                generate_files(&state, &log, &file)
                                    .await
                                    .log(&log, loga::WARN, "Error generating derived files");
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
            }
        }
    });
}
