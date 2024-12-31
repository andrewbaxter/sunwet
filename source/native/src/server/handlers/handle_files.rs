use {
    crate::{
        interface::triple::{
            DbIamTargetId,
            DbIamTargetIds,
            DbNode,
        },
        server::{
            access::{
                can_read,
                CanRead,
                Identity,
            },
            db::{
                self,
                Metadata,
            },
            dbutil::tx,
            filesutil::{
                file_path,
                generated_path,
                hash_file_sha256,
                staged_file_path,
            },
            state::State,
        },
    },
    chrono::Utc,
    flowcontrol::shed,
    http::Response,
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::htserve::{
        self,
        responses::{
            body_empty,
            response_200_json,
            response_401,
            response_404,
        },
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    hyper::body::{
        Bytes,
        Incoming,
    },
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    serde::Deserialize,
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            FileUrlQuery,
            ReqCommit,
            RespCommit,
            RespUploadFinish,
            HEADER_OFFSET,
        },
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        io::{
            self,
        },
        process::Stdio,
        sync::Arc,
    },
    tempfile::tempdir,
    tokio::{
        fs::{
            create_dir_all,
            rename,
            File,
        },
        io::{
            AsyncSeekExt,
            AsyncWriteExt,
        },
        process::Command,
    },
};

async fn get_meta(state: &Arc<State>, hash: &FileHash) -> Result<Option<db::Metadata>, loga::Error> {
    let state = state.clone();
    let hash = hash.clone();
    let Some(meta) = tx(&state.db, move |txn| {
        return Ok(db::meta_get(txn, &DbNode(Node::File(hash)))?);
    }).await? else {
        return Ok(None);
    };
    return Ok(Some(meta));
}

pub async fn handle_commit(state: Arc<State>, c: ReqCommit) -> Result<RespCommit, loga::Error> {
    for info in &c.files {
        if file_path(&state.files_dir, &info.hash)?.exists() {
            continue;
        }
        let path = staged_file_path(&state.stage_dir, &info.hash)?;
        if let Some(parent) = path.parent() {
            create_dir_all(&parent).await.stack_context(&state.log, "Failed to create upload staging dirs")?;
        }
        let f = File::create(&path).await.stack_context(&state.log, "Failed to create upload staged file")?;
        f.set_len(info.size).await.stack_context(&state.log, "Error preallocating disk space for upload")?;
    }
    let incomplete = tx(&state.db, move |txn| {
        let stamp = Utc::now();
        let mut incomplete = vec![];
        for info in c.files {
            incomplete.push(info.hash.clone());
            db::meta_insert(txn, &DbNode(Node::File(info.hash)), &info.mimetype, "")?;
        }
        for t in c.remove {
            if let Some(t) =
                db::triple_get(txn, &DbNode(t.subject.clone()), &t.predicate, &DbNode(t.object.clone()))? {
                if !t.exists {
                    continue;
                }
            } else {
                continue;
            }
            db::triple_insert(txn, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, false);
        }
        for t in c.add {
            if let Some(t) =
                db::triple_get(txn, &DbNode(t.subject.clone()), &t.predicate, &DbNode(t.object.clone()))? {
                if t.exists {
                    continue;
                }
            }
            db::triple_insert(txn, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, true);
        }
        return Ok(incomplete);
    }).await?;
    return Ok(RespCommit { incomplete: incomplete });
}

pub async fn handle_finish_upload(state: Arc<State>, hash: FileHash) -> Result<RespUploadFinish, loga::Error> {
    let done;
    if file_path(&state.files_dir, &hash)?.exists() {
        done = true;
    } else {
        done = false;
        if state.finishing_uploads.lock().unwrap().insert(hash.clone()) {
            state.tm.task(format!("Finish upload ({})", hash.to_string()), {
                let state = state.clone();
                async move {
                    match async {
                        let source = staged_file_path(&state.stage_dir, &hash)?;

                        // Validate hash
                        let got_hash = hash_file_sha256(&state.log, &source).await.context("Failed to hash staged uploaded file")?;
                        if got_hash != hash {
                            return Err(
                                loga::err_with(
                                    "Uploaded file hash mismatch",
                                    ea!(want_hash = hash, got_hash = got_hash),
                                ),
                            );
                        }

                        // Pre-generate web files for video
                        shed!{
                            let Some(meta) = get_meta(&state, &hash).await? else {
                                break;
                            };
                            match meta.mimetype.as_str() {
                                "video/x-matroska" | "video/mp4" | "video/webm" => { },
                                _ => {
                                    break;
                                },
                            }

                            // Extract subs
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
                                return Err(
                                    loga::err_with(
                                        "Getting video streams failed",
                                        ea!(output = streams_res.pretty_dbg_str()),
                                    ),
                                );
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
                                serde_json::from_slice::<Streams>(
                                    &streams_res.stdout,
                                ).context("Error parsing video streams json")?;
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
                                let subtitle_dest = generated_path(&state.cache_dir, &hash, "text/vtt", &lang)?;
                                if let Some(p) = subtitle_dest.parent() {
                                    create_dir_all(&p)
                                        .await
                                        .context_with(
                                            "Failed to create parent directories for generated subtitle file",
                                            ea!(path = subtitle_dest.display()),
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
                                        .arg(&subtitle_dest)
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
                            }

                            // Webm
                            if meta.mimetype.as_str() != "video/webm" {
                                let webm_tmp = tempdir()?;
                                let webm_dest = generated_path(&state.cache_dir, &hash, "video/webm", "")?;
                                if let Some(p) = webm_dest.parent() {
                                    create_dir_all(&p)
                                        .await
                                        .context_with(
                                            "Failed to create parent directories for generated webm file",
                                            ea!(path = webm_dest.display()),
                                        )?;
                                }
                                let pass1_res =
                                    Command::new("ffmpeg")
                                        .stdin(Stdio::null())
                                        .arg("-i")
                                        .arg(&source)
                                        .args(&["-b:v", "0"])
                                        .args(&["-crf", "30"])
                                        .args(&["-pass", "1"])
                                        .arg("-passlogfile")
                                        .arg(&webm_tmp.path().join("passlog"))
                                        .arg("-an")
                                        .args(&["-f", "webm"])
                                        .args(&["-y", "/dev/null"])
                                        .output()
                                        .await
                                        .context("Error starting webm conversion pass 1")?;
                                if !pass1_res.status.success() {
                                    return Err(
                                        loga::err_with(
                                            "Generating webm, pass 1 failed",
                                            ea!(output = pass1_res.pretty_dbg_str()),
                                        ),
                                    );
                                }
                                let pass2_res =
                                    Command::new("ffmpeg")
                                        .stdin(Stdio::null())
                                        .arg("-i")
                                        .arg(&source)
                                        .args(&["-b:v", "0"])
                                        .args(&["-crf", "30"])
                                        .args(&["-pass", "2"])
                                        .arg("-passlogfile")
                                        .arg(&webm_tmp.path().join("passlog"))
                                        .args(&["-f", "webm"])
                                        .arg(webm_dest)
                                        .output()
                                        .await
                                        .context("Error starting webm conversion pass 1")?;
                                if !pass2_res.status.success() {
                                    return Err(
                                        loga::err_with(
                                            "Generating webm, pass 2 failed",
                                            ea!(output = pass2_res.pretty_dbg_str()),
                                        ),
                                    );
                                }
                            }
                        }

                        // Place file
                        let dest = file_path(&state.files_dir, &hash)?;
                        if let Some(p) = dest.parent() {
                            create_dir_all(&p)
                                .await
                                .context_with(
                                    "Failed to create parent directories for uploaded file",
                                    ea!(path = dest.display()),
                                )?;
                        }
                        rename(&source, &dest).await.context("Failed to place uploaded file")?;
                        return Ok(());
                    }.await {
                        Ok(_) => { },
                        Err(e) => {
                            state
                                .log
                                .log_err(
                                    loga::WARN,
                                    e.context_with("Error finishing upload", ea!(hash = hash.to_string())),
                                );
                        },
                    }
                    state.finishing_uploads.lock().unwrap().remove(&hash);
                }
            });
        }
    }
    return Ok(RespUploadFinish { done: done });
}

pub async fn handle_file_head(
    state: Arc<State>,
    identity: &Identity,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    let Some(meta) = get_meta(&state, &file).await.err_internal()? else {
        return Ok(response_404());
    };
    if let Some(r) = can_read_file(&state, identity, &file, &meta).await? {
        return Ok(r);
    }
    return Ok(
        Response::builder()
            .status(200)
            .header("Content-Type", meta.mimetype.as_str())
            .header("Accept-Ranges", "bytes")
            .body(body_empty())
            .unwrap(),
    );
}

pub async fn handle_file_get(
    state: Arc<State>,
    identity: &Identity,
    head: http::request::Parts,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    let Some(meta) = get_meta(&state, &file).await.err_internal()? else {
        return Ok(response_404());
    };
    if let Some(r) = can_read_file(&state, identity, &file, &meta).await? {
        return Ok(r);
    }
    let query;
    if let Some(q) = head.uri.query() {
        query =
            serde_json::from_str::<FileUrlQuery>(
                &urlencoding::decode(&q).context("Error url-decoding query").err_external()?,
            )
                .context("Error parsing query string")
                .err_external()?;
    } else {
        query = FileUrlQuery { generated: None };
    }
    let mimetype;
    let local_path;
    if let Some(generated) = query.generated {
        if generated.mime_type == meta.mimetype && generated.name == "" {
            mimetype = meta.mimetype;
            local_path = file_path(&state.files_dir, &file).err_internal()?;
        } else {
            local_path =
                generated_path(&state.cache_dir, &file, &generated.mime_type, &generated.name).err_internal()?;
            mimetype = generated.mime_type;
        }
    } else {
        mimetype = meta.mimetype;
        local_path = file_path(&state.files_dir, &file).err_internal()?;
    }
    return Ok(htserve::responses::response_file(&head.headers, &mimetype, &local_path).await.err_internal()?);
}

pub async fn handle_file_post(
    state: Arc<State>,
    head: http::request::Parts,
    file: FileHash,
    body: Incoming,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    let offset = async {
        Ok(
            head
                .headers
                .get(HEADER_OFFSET)
                .context("Missing header")?
                .to_str()
                .context("Not valid utf-8")?
                .parse::<u64>()
                .context("Couldn't parse as integer")?,
        ) as
            Result<u64, loga::Error>
    }.await.stack_context_with(&state.log, "Error reading header", ea!(header = HEADER_OFFSET))?;
    let file_path = staged_file_path(&state.stage_dir, &file)?;
    let mut file =
        File::options()
            .write(true)
            .open(&file_path)
            .await
            .stack_context_with(
                &state.log,
                "Error opening staged file to write",
                ea!(path = file_path.to_string_lossy()),
            )?;
    file.seek(io::SeekFrom::Start(offset)).await.stack_context(&state.log, "Error seeking to upload part start")?;

    // TODO bg process to write chunks, wait on finish until all written
    let chunk = body.collect().await.stack_context(&state.log, "Error reading chunk")?.to_bytes();
    file.write_all(&chunk).await.stack_context(&state.log, "Error writing chunk")?;
    file.flush().await?;
    return Ok(response_200_json(()));
}
