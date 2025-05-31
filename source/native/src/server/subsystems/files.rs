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
                genfile_path,
                get_meta,
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
    flowcontrol::superif,
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
        ResultContext,
    },
    shared::interface::{
        config::form::{
            InputOrInline,
            InputOrInlineText,
        },
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
            TreeNode,
            Triple,
            HEADER_OFFSET,
        },
    },
    std::{
        collections::HashSet,
        hash::{
            DefaultHasher,
            Hash,
            Hasher,
        },
        io::{
            self,
        },
        sync::Arc,
    },
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
    },
};

async fn commit(
    state: Arc<State>,
    c: ReqCommit,
    update_access_reqs: Option<(MenuItemId, u64)>,
) -> Result<RespCommit, loga::Error> {
    // Preallocate files for upload, confirm already present files
    let mut incomplete = vec![];
    for info in &c.files {
        if file_path(&state, &info.hash)?.exists() {
            continue;
        }
        incomplete.push(info.hash.clone());
        let path = staged_file_path(&state, &info.hash)?;
        if let Some(parent) = path.parent() {
            create_dir_all(&parent).await.stack_context(&state.log, "Failed to create upload staging dirs")?;
        }
        let f = File::create(&path).await.stack_context(&state.log, "Failed to create upload staged file")?;
        f.set_len(info.size).await.stack_context(&state.log, "Error preallocating disk space for upload")?;
    }

    // Write new triples, commit (no-op if all triples already committed)
    tx(&state.db, move |txn| {
        // Update access if writing as non-admin
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

        // Update file meta
        for info in c.files {
            db::meta_upsert_file(txn, &DbNode(Node::File(info.hash)), Some(&info.mimetype))?;
        }

        // Insert triples
        let mut modified = false;
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
            modified = true;
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
                db::meta_upsert_fulltext(txn, &DbNode(node.clone()), &fulltext)?;
                return Ok(());
            }

            update_fulltext(txn, &t.subject)?;
            update_fulltext(txn, &t.object)?;
            db::triple_insert(txn, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, true)?;
            modified = true;
        }

        // Write commit if changed
        if modified {
            db::commit_insert(txn, stamp, &c.comment)?;
        }
        return Ok(());
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
    let mut form_hash = DefaultHasher::new();
    menu_item_form.item.hash(&mut form_hash);

    // Build form data
    let mut add = vec![];
    let get_data = |field| {
        let v = c.parameters.get(field).unwrap();
        match v {
            TreeNode::Scalar(v) => {
                return Ok(vec![v.clone()]);
            },
            TreeNode::Array(ns) => {
                let mut s1 = vec![];
                for v in ns {
                    let TreeNode::Scalar(v) = v else {
                        return Err(loga::err("Nested QueryResValue field in form data (likely bug)"));
                    };
                    s1.push(v.clone());
                }
                return Ok(s1);
            },
            TreeNode::Record(_) => {
                return Err(loga::err("Record QueryResValue field in form data (likely bug)"));
            },
        }
    };
    for triple in &menu_item_form.item.outputs {
        let subjects;
        match &triple.subject {
            InputOrInline::Input(field) => {
                subjects = get_data(field).err_external()?;
            },
            InputOrInline::Inline(v) => {
                subjects = vec![v.clone()];
            },
        }
        let predicate;
        match &triple.predicate {
            InputOrInlineText::Input(field) => {
                let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                    c.parameters.get(field) else {
                        return Err(
                            loga::err(
                                format!("Field {} must be a string to be used as a predicate, but it is not", field),
                            ),
                        ).err_external();
                    };
                predicate = v.clone();
            },
            InputOrInlineText::Inline(t) => {
                predicate = t.clone();
            },
        }
        let objects;
        match &triple.object {
            InputOrInline::Input(field) => {
                objects = get_data(field).err_external()?;
            },
            InputOrInline::Inline(v) => {
                objects = vec![v.clone()];
            },
        }
        for subj in subjects {
            for obj in &objects {
                add.push(Triple {
                    subject: subj.clone(),
                    predicate: predicate.clone(),
                    object: obj.clone(),
                });
            }
        }
    }
    return Ok(commit(state, ReqCommit {
        comment: format!("Form [{}]", c.menu_item_id),
        add: add,
        remove: vec![],
        files: vec![],
    }, Some((c.menu_item_id, form_hash.finish()))).await.err_internal()?);
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
    if file_path(&state, &file)?.exists() {
        done = true;
    } else {
        done = false;
        if state.finishing_uploads.lock().unwrap().insert(file.clone()) {
            state.tm.task(format!("Finish upload ({})", file.to_string()), {
                let state = state.clone();
                async move {
                    match async {
                        let source = staged_file_path(&state, &file)?;

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

                        // Place file
                        let dest = file_path(&state, &file)?;
                        if let Some(p) = dest.parent() {
                            create_dir_all(&p)
                                .await
                                .context_with(
                                    "Failed to create parent directories for uploaded file",
                                    ea!(path = dest.display()),
                                )?;
                        }
                        rename(&source, &dest).await.context("Failed to place uploaded file")?;

                        // Trigger generation
                        state.generate_files.send(Some(file.clone())).ignore();
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
    if let Identity::Link(l) = identity {
        if let Some(session) = state.link_sessions.get(l).await {
            if session.public_files.lock().unwrap().contains(&file) {
                return Ok(true);
            }
        }
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
            .header(
                "Content-Type",
                meta.mimetype.as_ref().map(|x| x.as_str()).unwrap_or("application/octet-stream"),
            )
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
        let Some(gen_mimetype) = tx(&state.db, {
            let gentype = gentype.clone();
            move |txn| Ok(db::gen_get(txn, &search_node, &gentype)?)
        }).await.err_internal()? else {
            if required {
                return Ok(response_404());
            }
            break 'nogen;
        };
        mimetype = gen_mimetype;
        local_path = genfile_path(&state, &file, &gentype).err_internal()?;
    } 'nogen {
        mimetype = meta.mimetype.unwrap_or_else(|| format!("application/octet-stream"));
        local_path = file_path(&state, &file).err_internal()?;
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
    let file_path = staged_file_path(&state, &file)?;
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
