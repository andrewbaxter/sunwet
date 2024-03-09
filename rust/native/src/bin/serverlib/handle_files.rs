use std::{
    cmp::Reverse,
    collections::{
        BTreeMap,
        HashMap,
    },
    io::{
        self,
        Write,
    },
    process::Stdio,
    sync::Arc,
    task::Poll,
};
use chrono::Utc;
use cozo::{
    DataValue,
    NamedRows,
    Validity,
    ValidityTs,
};
use futures::TryStreamExt;
use http::Response;
use http_body::Frame;
use http_body_util::{
    combinators::BoxBody,
    BodyExt,
};
use hyper::body::{
    Bytes,
    Incoming,
};
use loga::{
    ea,
    DebugDisplay,
    ErrContext,
    ResultContext,
};
use native::{
    cap_fn,
    util::{
        Flag,
        Log,
    },
};
use serde::Deserialize;
use sha2::{
    Digest,
    Sha256,
};
use shared::{
    bb,
    model::{
        Commit,
        CommitResp,
        FileHash,
        FileUrlQuery,
        Node,
        UploadFinishResp,
        HEADER_OFFSET,
    },
    unenum,
};
use tempfile::tempdir;
use tokio::{
    fs::{
        create_dir_all,
        rename,
        File,
    },
    io::{
        copy,
        AsyncReadExt,
        AsyncSeekExt,
        AsyncWrite,
        AsyncWriteExt,
    },
    process::Command,
    task::spawn_blocking,
};
use tokio_util::io::ReaderStream;
use super::{
    auth::{
        check_auth,
        check_file_auth,
    },
    dbutil::{
        node_to_meta_row,
        node_to_row,
    },
    filesutil::{
        file_path,
        generated_path,
        staged_file_path,
    },
    httpresp::{
        body_empty,
        response_200,
        response_200_json,
        response_400,
        response_404,
    },
    state::State,
};

async fn get_mimetype(state: &Arc<State>, hash: &FileHash) -> Result<Option<String>, loga::Error> {
    let state = state.clone();
    let hash = hash.clone();
    let Some(meta0) = spawn_blocking(
        cap_fn!(()(state) {
            state.db.run_script("{?[mimetype] := *meta{node:$node, mimetype:mimetype}}", {
                let mut m = BTreeMap::new();
                m.insert(
                    "node".to_string(),
                    DataValue::List(vec![DataValue::Str("file".into()), DataValue::Str(hash.to_string().into())]),
                );
                m
            }, cozo::ScriptMutability::Immutable)
        })
    ).await ?.await.map_err(
        |e| loga::err(e.dbg_str()).context("Error looking up metadata")
    ) ?.rows.into_iter().next() else {
        return Ok(None);
    };
    let mut meta0 = meta0.into_iter();
    let mimetype = unenum!(meta0.next().unwrap(), DataValue:: Str(s) => s).unwrap();
    return Ok(Some(mimetype.to_string()));
}

pub async fn handle_commit(
    state: Arc<State>,
    c: Commit,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    let mut incomplete = vec![];
    let mut meta_rows = vec![];
    for info in c.files {
        let mut meta = HashMap::new();
        meta.insert(
            "node".to_string(),
            node_to_row(
                &Node::File(info.hash.clone()),
            ).context_with("Unable to convert file hash for db insert", ea!(hash = info.hash))?,
        );
        meta.insert("mimetype".to_string(), DataValue::Str(info.mimetype.clone().into()));
        meta.insert("text".to_string(), DataValue::Str("".into()));
        meta_rows.push(meta);
        if file_path(&state.files_dir, &info.hash)?.exists() {
            continue;
        }
        incomplete.push(info.hash.clone());
        let path = staged_file_path(&state.stage_dir, &info.hash)?;
        if let Some(parent) = path.parent() {
            create_dir_all(&parent).await.stack_context(&state.log, "Failed to create upload staging dirs")?;
        }
        let f = File::create(&path).await.stack_context(&state.log, "Failed to create upload staged file")?;
        f.set_len(info.size).await.stack_context(&state.log, "Error preallocating disk space for upload")?;
    }
    let ver_now = ValidityTs(Reverse(Utc::now().timestamp_micros()));
    let mut triple_rows = vec![];
    for (i, t) in c.remove.iter().enumerate() {
        let log = Log::new().fork(ea!(section = "remove", triple = i));
        let subj_log = log.fork(ea!(subject = serde_json::to_string_pretty(&t.subject).unwrap()));
        let obj_log = log.fork(ea!(object = serde_json::to_string_pretty(&t.object).unwrap()));
        node_to_meta_row(&mut meta_rows, &t.subject).stack_context(&subj_log, "Error extracting metadata")?;
        node_to_meta_row(&mut meta_rows, &t.object).stack_context(&obj_log, "Error extracting metadata")?;
        triple_rows.push(
            vec![
                node_to_row(&t.subject).stack_context(&subj_log, "Unable to convert for db insert")?,
                DataValue::Str(t.predicate.as_str().into()),
                node_to_row(&t.object).stack_context(&obj_log, "Unable to convert for db insert")?,
                DataValue::Validity(Validity {
                    timestamp: ver_now,
                    is_assert: Reverse(false),
                })
            ],
        );
    }
    for (i, t) in c.add.iter().enumerate() {
        let log = Log::new().fork(ea!(section = "add", triple = i));
        let subj_log = log.fork(ea!(subject = serde_json::to_string_pretty(&t.subject).unwrap()));
        let obj_log = log.fork(ea!(object = serde_json::to_string_pretty(&t.object).unwrap()));
        node_to_meta_row(&mut meta_rows, &t.subject).stack_context(&subj_log, "Error extracting metadata")?;
        node_to_meta_row(&mut meta_rows, &t.object).stack_context(&obj_log, "Error extracting metadata")?;
        triple_rows.push(
            vec![
                node_to_row(&t.subject).stack_context(&subj_log, "Unable to convert for db insert")?,
                DataValue::Str(t.predicate.as_str().into()),
                node_to_row(&t.object).stack_context(&obj_log, "Unable to convert for db insert")?,
                DataValue::Validity(Validity {
                    timestamp: ver_now,
                    is_assert: Reverse(true),
                })
            ],
        );
    }
    let mut params = BTreeMap::new();
    params.insert("triple".to_string(), NamedRows {
        headers: vec!["subject".to_string(), "predicate".to_string(), "object".to_string(), "ver".to_string()],
        rows: triple_rows,
        next: None,
    });
    if !meta_rows.is_empty() {
        let headers = meta_rows.iter().next().unwrap().keys().cloned().collect::<Vec<_>>();
        let rows = meta_rows.into_iter().map(|mut r| {
            let mut out = vec![];
            for h in &headers {
                out.push(r.remove(h).unwrap());
            }
            out
        }).collect::<Vec<_>>();
        params.insert("meta".to_string(), NamedRows {
            headers: headers,
            rows: rows,
            next: None,
        });
    }
    spawn_blocking(cap_fn!(()(state) {
        state.db.import_relations(params)
    })).await?.await.map_err(|e| loga::err(e.to_string()).context("Error running query"))?;
    return Ok(response_200_json(CommitResp { incomplete: incomplete }));
}

pub async fn handle_finish_upload(
    state: Arc<State>,
    hash: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
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
                        let mut got_file = File::open(&source).await.context("Failed to open staged uploaded file")?;
                        match &hash {
                            FileHash::Sha256(hash) => {
                                struct HashAsyncWriter {
                                    hash: Sha256,
                                }

                                impl AsyncWrite for HashAsyncWriter {
                                    fn poll_write(
                                        mut self: std::pin::Pin<&mut Self>,
                                        _cx: &mut std::task::Context<'_>,
                                        buf: &[u8],
                                    ) -> Poll<Result<usize, std::io::Error>> {
                                        return Poll::Ready(self.as_mut().hash.write_all(buf).map(|_| buf.len()));
                                    }

                                    fn poll_flush(
                                        self: std::pin::Pin<&mut Self>,
                                        _cx: &mut std::task::Context<'_>,
                                    ) -> Poll<Result<(), std::io::Error>> {
                                        return Poll::Ready(Ok(()));
                                    }

                                    fn poll_shutdown(
                                        self: std::pin::Pin<&mut Self>,
                                        _cx: &mut std::task::Context<'_>,
                                    ) -> Poll<Result<(), std::io::Error>> {
                                        return Poll::Ready(Ok(()));
                                    }
                                }

                                let mut got_hash = HashAsyncWriter { hash: Sha256::new() };
                                copy(&mut got_file, &mut got_hash)
                                    .await
                                    .context("Failed to read staged uploaded file")?;
                                let got_hash = hex::encode(&got_hash.hash.finalize());
                                if &got_hash != hash {
                                    drop(got_file);
                                    return Err(
                                        loga::err_with(
                                            "Uploaded file hash mismatch",
                                            ea!(want_hash = hash, got_hash = got_hash),
                                        ),
                                    );
                                }
                            },
                        }

                        // Pre-generate web files for video
                        bb!{
                            let Some(mimetype) = get_mimetype(&state, &hash).await ? else {
                                break;
                            };
                            match mimetype.as_str() {
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
                                let subtitle_dest = generated_path(&state.generated_dir, &hash, "text/vtt", &lang)?;
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
                            if mimetype.as_str() != "video/webm" {
                                let webm_tmp = tempdir()?;
                                let webm_dest = generated_path(&state.generated_dir, &hash, "video/webm", "")?;
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
                                    Flag::Warn,
                                    e.context_with("Error finishing upload", ea!(hash = hash.to_string())),
                                );
                        },
                    }
                    state.finishing_uploads.lock().unwrap().remove(&hash);
                }
            });
        }
    }
    return Ok(response_200_json(UploadFinishResp { done: done }));
}

pub async fn handle_file_head(
    state: Arc<State>,
    head: http::request::Parts,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    if let Some(resp) = check_file_auth(&state, &head, &file) {
        return Ok(resp);
    }
    let Some(mimetype) = get_mimetype(&state, &file).await ? else {
        return Ok(response_404());
    };
    return Ok(
        Response::builder()
            .status(200)
            .header("Content-Type", mimetype.as_str())
            .header("Accept-Ranges", "bytes")
            .body(body_empty())
            .unwrap(),
    );
}

pub async fn handle_file_get(
    state: Arc<State>,
    head: http::request::Parts,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    if let Some(resp) = check_file_auth(&state, &head, &file) {
        return Ok(resp);
    }
    let query;
    if let Some(q) = head.uri.query() {
        query =
            serde_json::from_str::<FileUrlQuery>(
                &urlencoding::decode(&q).context("Error url-decoding query")?,
            ).context("Error parsing query string")?;
    } else {
        query = FileUrlQuery { generated: None };
    }
    let Some(main_mimetype) = get_mimetype(&state, &file).await ? else {
        return Ok(response_404());
    };
    let mimetype;
    let local_path;
    if let Some(generated) = query.generated {
        if generated.mime_type == main_mimetype && generated.name == "" {
            mimetype = main_mimetype;
            local_path = file_path(&state.files_dir, &file)?;
        } else {
            local_path = generated_path(&state.generated_dir, &file, &generated.mime_type, &generated.name)?;
            mimetype = generated.mime_type;
        }
    } else {
        mimetype = main_mimetype;
        local_path = file_path(&state.files_dir, &file)?;
    }
    let meta1 = match local_path.metadata() {
        Ok(m) => m,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => {
                    return Ok(response_404());
                },
                _ => {
                    return Err(
                        e.stack_context_with(
                            &state.log,
                            "Error opening stored file to read",
                            ea!(path = local_path.to_string_lossy()),
                        ),
                    );
                },
            }
        },
    };
    let mut file = File::open(&local_path).await?;
    if let Some(ranges) = head.headers.get("Accept-Ranges") {
        let Some(ranges_text) = ranges.to_str() ?.strip_prefix("bytes=") else {
            return Ok(response_400("Ranges missing bytes= prefix"));
        };
        let mut ranges = vec![];
        for range in ranges_text.split(",") {
            let Some((start, end)) = range.trim().split_once("-") else {
                return Ok(response_400("Ranges missing -"));
            };
            let start = if start == "" {
                None
            } else {
                Some(usize::from_str_radix(start, 10)?)
            };
            let end = if end == "" {
                None
            } else {
                let v = usize::from_str_radix(end, 10)?;
                if v == 0 {
                    return Ok(response_400("Zero end range"));
                }
                Some(v + 1)
            };
            let actual_start;
            let actual_end;
            match (start, end) {
                (Some(start), Some(end)) => {
                    actual_start = start;
                    actual_end = end;
                },
                (Some(start), None) => {
                    actual_start = start;
                    actual_end = meta1.len() as usize;
                },
                (None, Some(rev_start)) => {
                    actual_end = meta1.len() as usize;
                    actual_start = actual_end.saturating_sub(rev_start);
                },
                (None, None) => {
                    return Ok(response_400("Invalid range unbounded on both sides"));
                },
            }
            ranges.push((actual_start, actual_end));
        }
        if ranges.len() == 1 {
            let (start, end) = ranges.pop().unwrap();
            file.seek(io::SeekFrom::Start(start as u64)).await?;
            return Ok(
                Response::builder()
                    .status(206)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Type", mimetype.as_str())
                    .header("Cache-Control", format!("max-age=2147483648,immutable"))
                    .header("Content-Range", format!("bytes {}-{}/{}", start, end - 1, meta1.len()))
                    .header("Content-Length", end - start)
                    .body(
                        http_body_util::StreamBody::new(
                            ReaderStream::new(file.take((end - start) as u64)).map_ok(Frame::data),
                        ).boxed(),
                    )
                    .unwrap(),
            );
        } else {
            let boundary = "3d6b6a416f9b5";
            let mut content_len = 0;
            let mut ranges2 = vec![];
            for (i, (start, end)) in ranges.into_iter().enumerate() {
                let subheader = format!("{}--{}\nContent-Type: {}\nContent-Range: bytes {}-{}/{}\n\n", if i == 0 {
                    ""
                } else {
                    "\r\n"
                }, boundary, mimetype, start, end - 1, meta1.len()).into_bytes();
                content_len += subheader.len() + (end - start);
                ranges2.push((start, end, subheader));
            }
            let ranges = ranges2;
            let footer = format!("\r\n--{}--", boundary).into_bytes();
            content_len += footer.len();
            return Ok(
                Response::builder()
                    .status(206)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Type", format!("multipart/byteranges; boundary={boundary}"))
                    .header("Content-Length", content_len)
                    .body(BoxBody::new(http_body_util::StreamBody::new(async_stream::try_stream!{
                        for (start, end, subheader) in ranges {
                            yield Frame::data(Bytes::from(subheader));
                            file.seek(io::SeekFrom::Start(start as u64)).await?;
                            let mut remaining = end - start;
                            while remaining > 0 {
                                let mut buf = vec![];
                                let subchunk_len = (8 * 1024 * 1024).min(remaining);
                                buf.resize(subchunk_len, 0);
                                file.read(&mut buf).await?;
                                remaining -= subchunk_len;
                                yield Frame::data(Bytes::from(buf));
                            }
                        }
                        yield Frame::data(Bytes::from(footer));
                    })))
                    .unwrap(),
            );
        }
    } else {
        return Ok(
            Response::builder()
                .status(200)
                .header("Accept-Ranges", "bytes")
                .header("Content-Type", mimetype.as_str())
                .header("Cache-Control", format!("max-age=2147483648,immutable"))
                .header("Content-Length", meta1.len().to_string())
                .body(http_body_util::StreamBody::new(ReaderStream::new(file).map_ok(Frame::data)).boxed())
                .unwrap(),
        );
    }
}

pub async fn handle_file_post(
    state: Arc<State>,
    head: http::request::Parts,
    file: FileHash,
    body: Incoming,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    if let Some(resp) = check_auth(&state, &head) {
        return Ok(resp);
    }
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
    return Ok(response_200());
}
