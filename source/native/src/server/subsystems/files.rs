use {
    crate::{
        interface::triple::{
            DbFileHash,
            DbNode,
        },
        server::{
            access::{
                AccessSourceId,
                DbAccessSourceId,
            },
            dbwrite,
            dbutil,
            dbutil::tx,
            filesutil::{
                file_path,
                genfile_path,
                get_meta,
                hash_file_sha256,
                staged_file_path,
            },
            state::{
                BackgroundJob,
                State,
                get_global_config,
            },
        },
    },
    chrono::Utc,
    flowcontrol::superif,
    http::Response,
    http_body_util::{
        BodyExt,
        combinators::BoxBody,
    },
    htwrap::htserve::{
        self,
        responses::{
            body_empty,
            response_200_json,
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
        ResultContext,
        conversion::ResultIgnore,
        ea,
    },
    shared::interface::{
        config::form::{
            FormId,
            InputOrInline,
            InputOrInlineText,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            HEADER_OFFSET,
            ReqCommitForm,
            ReqCommitFree,
            RespCommit,
            RespUploadFinish,
            TreeNode,
            Triple,
        },
    },
    std::{
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
            File,
            create_dir_all,
            rename,
        },
        io::{
            AsyncSeekExt,
            AsyncWriteExt,
        },
    },
};

async fn commit(
    state: Arc<State>,
    c: ReqCommitFree,
    update_access_reqs: Option<(FormId, u64)>,
) -> Result<RespCommit, loga::Error> {
    // Preallocate files for upload, confirm already present files
    let mut incomplete = vec![];
    for info in &c.files {
        if file_path(&state, &info.hash)?.exists() {
            continue;
        }
        incomplete.push(info.hash.clone());
        let path = staged_file_path(&state, &info.hash)?;
        create_dir_all(&path.parent().unwrap())
            .await
            .stack_context(&state.log, "Failed to create upload staging dirs")?;
        let f = File::create(&path).await.stack_context(&state.log, "Failed to create upload staged file")?;
        f.set_len(info.size).await.stack_context(&state.log, "Error preallocating disk space for upload")?;
    }

    // Write new triples, commit (no-op if all triples already committed)
    tx(&state.db, move |txn| {
        let mut db = dbutil::db3(txn);

        // Update access if writing as non-admin - this is because multi-part uploads get
        // re-accessed checked so need to establish chain of trust for writing from commit
        if let Some((form_id, form_version_hash)) = update_access_reqs {
            let page_access = DbAccessSourceId(AccessSourceId::FormId(form_id));
            let form_version_hash_i64 = form_version_hash as i64;
            dbutil::file_access_gc(&mut db, &page_access, &form_version_hash_i64).context("Error clearing file access")?;
            for file in &c.files {
                let filehash = DbFileHash(file.hash.clone());
                dbutil::file_access_insert(&mut db, &filehash, &page_access, &form_version_hash_i64)
                    .context("Error inserting file access")?;
            }
        }

        // Update file meta
        for info in c.files {
            let node = DbNode(Node::File(info.hash));
            let mimetype = Some(info.mimetype);
            dbutil::meta_upsert_mimetype(&mut db, &node, &mimetype).context("Error upserting file meta")?;
        }

        // Insert triples
        let mut modified = false;
        let stamp = Utc::now();

        fn update_fulltext<
            C: good_ormning::runtime::sqlite::SqliteConnection,
        >(db: &mut crate::server::db::Db3<C>, node: &Node) -> Result<(), loga::Error> {
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
            let node_db = DbNode(node.clone());
            dbutil::meta_upsert_fulltext(db, &node_db, &fulltext).context("Error updating fulltext")?;
            return Ok(());
        }

        for t in c.remove {
            let subject = DbNode(t.subject.clone());
            let object = DbNode(t.object.clone());
            if !dbutil::triple_snapshot_exists(&mut db, &subject, &t.predicate, &object)? {
                continue;
            }
            dbwrite::write_triple(&mut db, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, false)?;
            modified = true;
        }
        for t in c.add {
            let subject = DbNode(t.subject.clone());
            let object = DbNode(t.object.clone());
            if dbutil::triple_snapshot_exists(&mut db, &subject, &t.predicate, &object)? {
                continue;
            }
            update_fulltext(&mut db, &t.subject)?;
            update_fulltext(&mut db, &t.object)?;
            dbwrite::write_triple(&mut db, &DbNode(t.subject), &t.predicate, &DbNode(t.object), stamp, true)?;
            modified = true;
        }

        // Write commit if changed
        if modified {
            dbutil::commit_insert(&mut db, &stamp, &c.comment).context("Error inserting commit")?;
        }
        return Ok(());
    }).await?;
    return Ok(RespCommit { incomplete: incomplete });
}

pub async fn handle_commit(state: Arc<State>, c: ReqCommitFree) -> Result<RespCommit, loga::Error> {
    return Ok(commit(state, c, None).await?);
}

pub async fn handle_form_commit(state: Arc<State>, c: ReqCommitForm) -> Result<RespCommit, VisErr<loga::Error>> {
    let global_config = get_global_config(&state).await.err_internal()?;
    let Some(form) = global_config.forms.get(&c.form_id) else {
        return Err(loga::err_with("No known form with id", ea!(id = c.form_id))).err_external();
    };
    let mut form_hash = DefaultHasher::new();
    form.item.hash(&mut form_hash);

    // Build form data
    let mut add = vec![];
    let get_data = |field| {
        let v = c.parameters.get(field).context(format!("Missing field [{}]", field))?;
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
    for triple in &form.item.outputs {
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
    return Ok(commit(state, ReqCommitFree {
        comment: format!("Form [{}]", c.form_id),
        add: add,
        remove: vec![],
        files: vec![],
    }, Some((c.form_id.clone(), form_hash.finish()))).await.err_internal()?);
}

pub async fn handle_finish_upload(
    state: Arc<State>,
    file: FileHash,
) -> Result<Option<RespUploadFinish>, loga::Error> {
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
                        state.background.send(BackgroundJob::GenerateOne(file.clone())).ignore();
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

pub async fn handle_file_head(
    state: Arc<State>,
    file: FileHash,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    let Some(meta) = get_meta(&state, &file).await.err_internal()? else {
        return Ok(response_404());
    };
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
    head: http::request::Parts,
    file: FileHash,
    gentype: String,
    subpath: String,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    let Some(meta) = get_meta(&state, &file).await.err_internal()? else {
        return Ok(response_404());
    };
    let mimetype;
    let local_path;
    superif!({
        if gentype.is_empty() {
            break 'nogen;
        }
        let search_node = DbNode(Node::File(file.clone()));
        let Some(gen_mimetype) = tx(&state.db, {
            let gentype = gentype.to_string();
            move |txn| {
                let mut db = dbutil::db3(txn);
                Ok(dbutil::generated_get_mimetype(&mut db, &search_node, &gentype)?)
            }
        }).await.err_internal()? else {
            break 'nogen;
        };
        let gen_path = genfile_path(&state, &file, &gentype, &subpath).err_internal()?;
        if !gen_path.exists() {
            break 'nogen;
        }
        mimetype = gen_mimetype;
        local_path = gen_path;
    } 'nogen {
        if !subpath.is_empty() {
            return Ok(response_404());
        }
        mimetype = meta.mimetype.unwrap_or_else(|| format!("application/octet-stream"));
        local_path = file_path(&state, &file).err_internal()?;
    });
    return Ok(
        htserve::responses::response_file(&head.headers, &mimetype, &local_path, true, &state.http_resp_headers)
            .await
            .err_internal()?,
    );
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
    let file_path = staged_file_path(&state, &file)?;
    let mut file =
        File::options()
            .write(true)
            .create(true)
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
