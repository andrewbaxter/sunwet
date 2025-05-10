use {
    crate::{
        interface::{
            config::{
                IamGrants,
                MenuItemId,
            },
            triple::{
                DbFileHash,
                DbNode,
            },
        },
        server::{
            access::Identity,
            db::{
                self,
            },
            dbutil::tx,
            filesutil::{
                file_path,
                generated_path,
                hash_file_sha256,
                staged_file_path,
            },
            state::{
                get_global_config,
                get_iam_grants,
                MenuItem,
                State,
            },
        },
    },
    chrono::Utc,
    flowcontrol::{
        shed,
        superif,
    },
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
    shared::{
        form::build_form_commit,
        interface::{
            triple::{
                FileHash,
                Node,
            },
            wire::{
                FileUrlQuery,
                ReqCommit,
                ReqFormCommit,
                RespCommit,
                RespUploadFinish,
                HEADER_OFFSET,
            },
        },
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        hash::{
            DefaultHasher,
            Hash,
            Hasher,
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

async fn commit(
    state: Arc<State>,
    c: ReqCommit,
    update_access_reqs: Option<(MenuItemId, u64)>,
) -> Result<RespCommit, loga::Error> {
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
        if let Some((page_access, form_version_hash)) = update_access_reqs {
            db::file_access_clear_nonversion(txn, &page_access, form_version_hash as i64)?;
            for file in &c.files {
                db::file_access_insert(
                    txn,
                    &DbFileHash(file.hash.clone()),
                    &page_access,
                    form_version_hash as i64,
                )?;
            }
        }
        let mut incomplete = vec![];
        for info in c.files {
            incomplete.push(info.hash.clone());
            db::meta_insert(txn, &DbNode(Node::File(info.hash)), &info.mimetype, "")?;
        }
        let stamp = Utc::now();
        for t in c.remove {
            if let Some(t) =
                db::triple_get(txn, &DbNode(t.subject.clone()), &t.predicate, &DbNode(t.object.clone()))? {
                if !t.exists {
                    continue;
                }
            } else {
                continue;
            }
            db::triple_insert(txn, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, false)?;
        }
        for t in c.add {
            if let Some(t) =
                db::triple_get(txn, &DbNode(t.subject.clone()), &t.predicate, &DbNode(t.object.clone()))? {
                if t.exists {
                    continue;
                }
            }

            fn update_fulltext(txn: &rusqlite::Transaction, node: &Node) -> Result<(), loga::Error> {
                let mut fulltext = String::new();

                fn gather_value_text(fulltext: &mut String, value: &serde_json::Value) {
                    match value {
                        serde_json::Value::Null => {
                            // nop
                        },
                        serde_json::Value::Bool(_) => {
                            // nop
                        },
                        serde_json::Value::Number(_) => {
                            // nop
                        },
                        serde_json::Value::String(v) => {
                            fulltext.push_str(v);
                            fulltext.push_str(" ");
                        },
                        serde_json::Value::Array(v) => {
                            for v in v {
                                gather_value_text(fulltext, v);
                            }
                        },
                        serde_json::Value::Object(v) => {
                            for (k, v) in v {
                                fulltext.push_str(k);
                                fulltext.push_str(" ");
                                gather_value_text(fulltext, v);
                            }
                        },
                    }
                }

                match node {
                    Node::File(_) => {
                        // nop
                    },
                    Node::Value(v) => gather_value_text(&mut fulltext, v),
                }
                db::meta_insert(txn, &DbNode(node.clone()), "", &fulltext)?;
                return Ok(());
            }

            update_fulltext(txn, &t.subject)?;
            update_fulltext(txn, &t.object)?;
            db::triple_insert(txn, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, true)?;
        }
        return Ok(incomplete);
    }).await?;
    return Ok(RespCommit { incomplete: incomplete });
}

pub async fn handle_commit(state: Arc<State>, c: ReqCommit) -> Result<RespCommit, loga::Error> {
    return Ok(commit(state, c, None).await?);
}

pub async fn handle_form_commit(state: Arc<State>, c: ReqFormCommit) -> Result<RespCommit, VisErr<loga::Error>> {
    let global_config = get_global_config(&state).await.err_internal()?;
    let Some(MenuItem::Form(menu_item_form)) = global_config.menu_items.get(&c.menu_item_id) else {
        return Err(loga::err_with("No known form menu item with id", ea!(id = c.menu_item_id))).err_external();
    };
    let form = global_config.forms.get(&menu_item_form.item.form_id).unwrap();
    let mut form_hash = DefaultHasher::new();
    form.hash(&mut form_hash);
    return Ok(
        commit(
            state,
            build_form_commit(form, &c.parameters).map_err(loga::err).err_external()?,
            Some((c.menu_item_id, form_hash.finish())),
        )
            .await
            .err_internal()?,
    );
}

pub async fn handle_finish_upload(
    state: Arc<State>,
    identity: &Identity,
    file: FileHash,
) -> Result<Option<RespUploadFinish>, loga::Error> {
    if !can_access_file(&state, identity, &file).await? {
        return Ok(None);
    }
    let done;
    if file_path(&state.files_dir, &file)?.exists() {
        done = true;
    } else {
        done = false;
        if state.finishing_uploads.lock().unwrap().insert(file.clone()) {
            state.tm.task(format!("Finish upload ({})", file.to_string()), {
                let state = state.clone();
                async move {
                    match async {
                        let source = staged_file_path(&state.stage_dir, &file)?;

                        // Validate hash
                        let got_hash = hash_file_sha256(&state.log, &source).await.context("Failed to hash staged uploaded file")?;
                        if got_hash != file {
                            return Err(
                                loga::err_with(
                                    "Uploaded file hash mismatch",
                                    ea!(want_hash = file, got_hash = got_hash),
                                ),
                            );
                        }

                        // Pre-generate web files for video
                        shed!{
                            let Some(meta) = get_meta(&state, &file).await? else {
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
                                let subtitle_dest = generated_path(&state.genfiles_dir, &file, "text/vtt", &lang)?;
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
                                let webm_dest = generated_path(&state.genfiles_dir, &file, "video/webm", "")?;
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
                        let dest = file_path(&state.files_dir, &file)?;
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
                                    e.context_with("Error finishing upload", ea!(hash = file.to_string())),
                                );
                        },
                    }
                    state.finishing_uploads.lock().unwrap().remove(&file);
                }
            });
        }
    }
    return Ok(Some(RespUploadFinish { done: done }));
}

async fn get_file_page_reqs(state: &State, file: &FileHash) -> Result<HashSet<MenuItemId>, loga::Error> {
    let file = DbFileHash(file.clone());
    let access = tx(&state.db, move |txn| Ok(db::file_access_get(txn, &file)?)).await?;
    return Ok(access.into_iter().collect());
}

async fn can_access_file(state: &State, identity: &Identity, file: &FileHash) -> Result<bool, loga::Error> {
    match get_iam_grants(state, identity).await? {
        IamGrants::Admin => return Ok(true),
        IamGrants::Limited(grants) => {
            let page_reqs = get_file_page_reqs(state, file).await?;
            for grant in grants {
                if page_reqs.contains(&grant) {
                    return Ok(true);
                }
            }
        },
    }
    if state.link_public_files.lock().unwrap().contains(&file) {
        return Ok(true);
    }
    return Ok(false);
}

pub async fn handle_file_head(
    state: Arc<State>,
    identity: &Identity,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    let Some(meta) = get_meta(&state, &file).await.err_internal()? else {
        return Ok(response_404());
    };
    if !can_access_file(&state, identity, &file).await.err_internal()? {
        return Ok(response_401());
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
    if !can_access_file(&state, identity, &file).await.err_internal()? {
        return Ok(response_401());
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
        query = FileUrlQuery::default();
    }
    let mimetype;
    let local_path;
    superif!({
        let Some((gentype, required)) = query.derivation else {
            break 'nogen;
        };
        let search_node = DbNode(Node::File(file.clone()));
        let Some(gen_meta) =
            tx(&state.db, move |txn| Ok(db::gen_get(txn, &search_node, &gentype)?)).await.err_internal()? else {
                if required {
                    return Ok(response_404());
                }
                break 'nogen;
            };
        mimetype = gen_meta.mimetype;
        local_path = state.genfiles_dir.join(gen_meta.filename);
    } 'nogen {
        mimetype = meta.mimetype;
        local_path = file_path(&state.files_dir, &file).err_internal()?;
    });
    return Ok(htserve::responses::response_file(&head.headers, &mimetype, &local_path).await.err_internal()?);
}

pub async fn handle_file_post(
    state: Arc<State>,
    identity: &Identity,
    head: http::request::Parts,
    file: FileHash,
    body: Incoming,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    if !can_access_file(&state, identity, &file).await? {
        return Ok(response_401());
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
    return Ok(response_200_json(()));
}
