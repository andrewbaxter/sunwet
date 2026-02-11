pub mod db;
pub mod filesutil;
pub mod defaultviews;
pub mod state;
pub mod dbutil;
pub mod query;
pub mod query_test;
pub mod access;
pub mod fsutil;
pub mod subsystems;

use {
    crate::{
        cap_fn,
        interface::{
            config::{
                Config,
                MaybeFdap,
            },
            triple::{
                DbFileHash,
                DbNode,
            },
        },
        server::{
            access::{
                AccessRes,
                AccessSourceId,
                DbAccessSourceId,
                Identity,
                can_access_file,
            },
            state::{
                BackgroundJob,
                IamGrants,
            },
        },
    },
    aargvark::{
        Aargvark,
        traits_impls::AargvarkJson,
    },
    access::{
        check_is_admin,
        identify_requester,
    },
    dbutil::tx,
    deadpool_sqlite::{
        Hook,
        HookError,
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    fsutil::create_dirs,
    good_ormning_runtime::GoodError,
    http::{
        HeaderMap,
        HeaderName,
        HeaderValue,
        Uri,
        status,
    },
    http_body_util::{
        BodyExt,
        combinators::BoxBody,
    },
    htwrap::htserve::{
        responses::{
            response_401,
            response_403,
            response_404,
        },
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    hyper::{
        Method,
        Request,
        Response,
        body::{
            Bytes,
            Incoming,
        },
        server::conn::http1,
        service::service_fn,
    },
    hyper_util::rt::TokioIo,
    loga::{
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
        ea,
    },
    moka::future::Cache,
    shared::interface::{
        config::view::ViewId,
        query::Query,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            C2SReq,
            NodeMeta,
            Pagination,
            ReqCommit,
            ReqHistoryFilterPredicate,
            RespHistory,
            RespHistoryEvent,
            RespQuery,
            RespQueryRows,
            RespWhoAmI,
            TreeNode,
            Triple,
        },
    },
    state::{
        FdapGlobalState,
        FdapState,
        FdapUsersState,
        GlobalState,
        LocalUsersState,
        State,
        UsersState,
        build_global_config,
        get_global_config,
        get_iam_grants,
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
        str::FromStr,
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
    subsystems::{
        background::start_background_job,
        files::{
            handle_commit,
            handle_file_get,
            handle_file_head,
            handle_file_post,
            handle_finish_upload,
            handle_form_commit,
        },
        link::{
            handle_link_ws,
            handle_ws_link,
            handle_ws_main,
        },
        menu::handle_get_filtered_client_config,
        oidc,
        static_,
    },
    tokio::{
        net::TcpListener,
        sync::mpsc,
    },
    tokio_stream::wrappers::TcpListenerStream,
};

fn gather_record_files(files: &mut Vec<FileHash>, r: &TreeNode) {
    match r {
        TreeNode::Scalar(s) => {
            if let Node::File(s) = s {
                files.push(s.clone());
            }
        },
        TreeNode::Array(a) => {
            for v in a {
                gather_record_files(files, v);
            }
        },
        TreeNode::Record(r) => {
            for v in r.values() {
                gather_record_files(files, v);
            }
        },
    }
}

async fn handle_query_req(
    state: Arc<State>,
    query: Query,
    parameters: HashMap<String, Node>,
    pagination: Option<Pagination>,
    view_access: Option<(ViewId, u64)>,
) -> Result<RespQuery, VisErr<loga::Error>> {
    let expect_count = pagination.as_ref().map(|x| x.count);
    let results = query::execute_query(&state.db, query, parameters, pagination).await?;
    let page_end = expect_count.and_then(|x| {
        match &results {
            query::QueryResults::Scalar(rows) => {
                if rows.len() < x {
                    return None;
                } else {
                    return rows.last().cloned();
                }
            },
            query::QueryResults::Record(rows) => {
                if rows.len() < x {
                    return None;
                } else {
                    return rows.last().map(|x| x.head_data.clone());
                }
            },
        }
    });
    let mut files = vec![];
    let out_rows;
    match results {
        query::QueryResults::Scalar(rows) => {
            for scalar in &rows {
                if let Node::File(s) = scalar {
                    files.push(s.clone());
                }
            }
            out_rows = RespQueryRows::Scalar(rows);
        },
        query::QueryResults::Record(rows) => {
            let mut out_rows1 = vec![];
            for record in rows {
                for v in record.tail_data.values() {
                    gather_record_files(&mut files, v);
                }
                out_rows1.push(record.tail_data);
            }
            out_rows = RespQueryRows::Record(out_rows1);
        },
    }
    let meta = tx(&state.db, {
        move |txn| {
            if let Some((view_id, view_hash)) = view_access {
                let access_source_id = DbAccessSourceId(AccessSourceId::ViewId(view_id.clone()));
                db::file_access_clear_nonversion(txn, &access_source_id, view_hash as i64)?;
                for file in &files {
                    db::file_access_insert(txn, &DbFileHash(file.clone()), &access_source_id, view_hash as i64)?;
                }
            }
            let mut meta = HashMap::new();
            for file in files {
                let node = Node::File(file);
                if let Some(node_meta) = db::meta_get(txn, &DbNode(node.clone()))? {
                    meta.insert(node, NodeMeta { mime: node_meta.mimetype });
                }
            }
            return Ok(meta);
        }
    }).await.err_internal()?;
    return Ok(RespQuery {
        rows: out_rows,
        meta: meta.into_iter().collect(),
        next_page_key: page_end,
    });
}

async fn handle_req(state: Arc<State>, mut req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    let url = req.uri().clone();
    match {
        let state = state.clone();
        async move {
            if (|| false)() {
                return Err(loga::err("")).err_internal() as Result<_, VisErr<loga::Error>>;
            }
            if hyper_tungstenite::is_upgrade_request(&req) {
                // Websocket req
                let upgrade = hyper_tungstenite::upgrade(&mut req, None);
                let (head, _) = req.into_parts();
                let mut path_iter = head.uri.path().trim_matches('/').split('/');
                let link_type = path_iter.next().unwrap();
                let link_session = path_iter.next().unwrap();
                match link_type {
                    "link" => {
                        return Ok(
                            handle_link_ws(state, link_session.to_string(), head, upgrade, handle_ws_link).await,
                        );
                    },
                    "main" => {
                        return Ok(
                            handle_link_ws(state, link_session.to_string(), head, upgrade, handle_ws_main).await,
                        );
                    },
                    _ => {
                        state
                            .log
                            .log_with(loga::DEBUG, "Websocket connection on unknown path", ea!(path = link_type));
                        return Ok(response_404());
                    },
                }
            } else {
                // Normal HTTP req
                let (head, body) = req.into_parts();
                let mut path_iter = head.uri.path().trim_matches('/').split('/');
                let path_first = path_iter.next().unwrap();
                match path_first {
                    "oidc" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            return Ok(oidc::handle_oidc(oidc_state, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "logout" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            return Ok(oidc::handle_logout(oidc_state, &state.log, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "api" => {
                        let identity = identify_requester(&state, &head.headers).await?;
                        let req =
                            serde_json::from_slice::<C2SReq>(
                                &body
                                    .collect()
                                    .await
                                    .context("Error reading request bytes")
                                    .err_external()?
                                    .to_bytes(),
                            )
                                .context("Failed to parse json request body")
                                .err_external()?;

                        pub mod resp {
                            use {
                                http::Response,
                                http_body_util::combinators::BoxBody,
                                htwrap::htserve::responses::response_200_json,
                                hyper::body::Bytes,
                                serde::Serialize,
                                shared::interface::{
                                    wire::C2SReqTrait,
                                },
                            };

                            // Private constructor
                            pub struct RespToken(());

                            pub type Resp = Response<BoxBody<Bytes, std::io::Error>>;

                            pub trait ReqResp: C2SReqTrait {
                                fn respond(&self) -> fn(Self::Resp) -> (RespToken, Resp) {
                                    fn acceptor<T: Serialize>(r: T) -> (RespToken, Resp) {
                                        return (RespToken(()), response_200_json(r));
                                    }

                                    return acceptor::<Self::Resp>;
                                }
                            }

                            impl ReqResp for shared::interface::wire::ReqCommit { }

                            impl ReqResp for shared::interface::wire::ReqGetClientConfig { }

                            impl ReqResp for shared::interface::wire::ReqQuery { }

                            impl ReqResp for shared::interface::wire::ReqViewQuery { }

                            impl ReqResp for shared::interface::wire::ReqGetNodeMeta { }

                            impl ReqResp for shared::interface::wire::ReqHistory { }

                            impl ReqResp for shared::interface::wire::ReqGetTriplesAround { }

                            impl ReqResp for shared::interface::wire::ReqUploadFinish { }

                            impl ReqResp for shared::interface::wire::ReqWhoAmI { }
                        }

                        use resp::ReqResp;

                        let resp: (resp::RespToken, resp::Resp);
                        match req {
                            C2SReq::Commit(req) => {
                                let responder = req.respond();
                                match req {
                                    ReqCommit::Free(req) => {
                                        // Check access
                                        match check_is_admin(&state, &identity, "Commit").await.err_internal()? {
                                            AccessRes::Yes => { },
                                            AccessRes::NoAccess => {
                                                return Ok(response_403());
                                            },
                                            AccessRes::NoIdent => {
                                                return Ok(response_401());
                                            },
                                        }
                                        resp = responder(handle_commit(state, req).await.err_internal()?);
                                    },
                                    ReqCommit::Form(req) => {
                                        {
                                            // Check access
                                            let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                            let res = shed!{
                                                'ok _;
                                                match &grants {
                                                    IamGrants::Admin => {
                                                        break 'ok AccessRes::Yes;
                                                    },
                                                    IamGrants::Limited(grants) => {
                                                        if grants.forms.contains(&req.form_id) {
                                                            break 'ok AccessRes::Yes;
                                                        }
                                                    },
                                                }
                                                if matches!(identity, Identity::Public) {
                                                    break 'ok AccessRes::NoIdent;
                                                }
                                                else {
                                                    break 'ok AccessRes::NoAccess;
                                                    return Ok(response_403());
                                                }
                                            };
                                            state
                                                .log
                                                .log_with(
                                                    loga::DEBUG,
                                                    "Form commit access result",
                                                    ea!(
                                                        identity = identity.dbg_str(),
                                                        grants = grants.dbg_str(),
                                                        form_id = req.form_id,
                                                        result = res.dbg_str()
                                                    ),
                                                );
                                            match res {
                                                AccessRes::Yes => { },
                                                AccessRes::NoIdent => {
                                                    return Ok(response_401());
                                                },
                                                AccessRes::NoAccess => {
                                                    return Ok(response_403());
                                                },
                                            }
                                        }
                                        resp = responder(handle_form_commit(state, req).await?);
                                    },
                                }
                            },
                            C2SReq::UploadFinish(req) => {
                                let responder = req.respond();
                                match can_access_file(&state, &identity, &req.0).await.err_internal()? {
                                    AccessRes::Yes => (),
                                    AccessRes::NoIdent => return Ok(response_401()),
                                    AccessRes::NoAccess => return Ok(response_403()),
                                }
                                let Some(res) = handle_finish_upload(state, req.0).await.err_internal()? else {
                                    return Ok(response_401());
                                };
                                resp = responder(res);
                            },
                            C2SReq::Query(req) => {
                                match check_is_admin(&state, &identity, "Query").await.err_internal()? {
                                    AccessRes::Yes => { },
                                    AccessRes::NoAccess => {
                                        return Ok(response_403());
                                    },
                                    AccessRes::NoIdent => {
                                        return Ok(response_401());
                                    },
                                }
                                let responder = req.respond();
                                resp =
                                    responder(
                                        handle_query_req(
                                            state,
                                            req.query,
                                            req.parameters,
                                            req.pagination,
                                            None,
                                        ).await?,
                                    );
                            },
                            C2SReq::ViewQuery(req) => {
                                let responder = req.respond();
                                let global_config = get_global_config(&state).await.err_internal()?;
                                let Some(view) = global_config.views.get(&req.view_id) else {
                                    return Err(
                                        loga::err_with("No known view menu_item with id", ea!(view = req.view_id)),
                                    ).err_external();
                                };
                                {
                                    // Access check
                                    let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                    let res = shed!{
                                        'ok _;
                                        match &grants {
                                            IamGrants::Admin => {
                                                break 'ok AccessRes::Yes;
                                            },
                                            IamGrants::Limited(grants) => {
                                                if grants.views.contains(&req.view_id) {
                                                    break 'ok AccessRes::Yes;
                                                }
                                            },
                                        }
                                        if matches!(identity, Identity::Public) {
                                            break 'ok AccessRes::NoIdent;
                                        }
                                        else {
                                            break 'ok AccessRes::NoAccess;
                                            return Ok(response_403());
                                        }
                                    };
                                    state
                                        .log
                                        .log_with(
                                            loga::DEBUG,
                                            "View query access result",
                                            ea!(
                                                identity = identity.dbg_str(),
                                                grants = grants.dbg_str(),
                                                view_id = req.view_id,
                                                result = res.dbg_str()
                                            ),
                                        );
                                    match res {
                                        AccessRes::Yes => { },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                    }
                                }
                                let Some(query) = view.item.queries.get(&req.query) else {
                                    return Err(
                                        loga::err_with(
                                            "No known query with id in view",
                                            ea!(view = req.view_id, query = req.query),
                                        ),
                                    ).err_external();
                                };
                                let mut view_hash = DefaultHasher::new();
                                view.item.hash(&mut view_hash);
                                let view_hash = view_hash.finish();
                                resp =
                                    responder(
                                        handle_query_req(
                                            state,
                                            query.clone(),
                                            req.parameters,
                                            req.pagination,
                                            Some((req.view_id.clone(), view_hash)),
                                        ).await?,
                                    );
                            },
                            C2SReq::GetNodeMeta(req) => {
                                let responder = req.respond();
                                let meta = tx(&state.db, move |txn| {
                                    return Ok(db::meta_get(txn, &DbNode(req.node))?);
                                }).await.err_internal()?;
                                resp = responder(match meta {
                                    Some(m) => Some(NodeMeta { mime: m.mimetype }),
                                    None => None,
                                });
                            },
                            C2SReq::History(req) => {
                                let responder = req.respond();
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "History").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let (events, commit_descriptions) = tx(&state.db, move |txn| {
                                    let events;
                                    if let Some(f) = req.filter {
                                        if let Some(p) = f.predicate {
                                            match p {
                                                ReqHistoryFilterPredicate::Incoming(p) => {
                                                    events = if let Some(after) = req.page_key {
                                                        db::hist_list_by_predicate_object_after(
                                                            txn,
                                                            after.0,
                                                            &DbNode(after.1.subject),
                                                            &after.1.predicate,
                                                            &DbNode(after.1.object),
                                                            &p,
                                                            &DbNode(f.node),
                                                        )?
                                                    } else {
                                                        db::hist_list_by_predicate_object(txn, &p, &DbNode(f.node))?
                                                    };
                                                },
                                                ReqHistoryFilterPredicate::Outgoing(p) => {
                                                    events = if let Some(after) = req.page_key {
                                                        db::hist_list_by_subject_predicate_after(
                                                            txn,
                                                            after.0,
                                                            &DbNode(after.1.subject),
                                                            &after.1.predicate,
                                                            &DbNode(after.1.object),
                                                            &DbNode(f.node),
                                                            &p,
                                                        )?
                                                    } else {
                                                        db::hist_list_by_subject_predicate(
                                                            txn,
                                                            &DbNode(f.node),
                                                            &p,
                                                        )?
                                                    };
                                                },
                                            }
                                        } else {
                                            events = if let Some(after) = req.page_key {
                                                db::hist_list_by_node_after(
                                                    txn,
                                                    after.0,
                                                    &DbNode(after.1.subject),
                                                    &after.1.predicate,
                                                    &DbNode(after.1.object),
                                                    &DbNode(f.node),
                                                )?
                                            } else {
                                                db::hist_list_by_node(txn, &DbNode(f.node))?
                                            };
                                        }
                                    } else {
                                        events = if let Some(after) = req.page_key {
                                            db::hist_list_all_after(
                                                txn,
                                                after.0,
                                                &DbNode(after.1.subject),
                                                &after.1.predicate,
                                                &DbNode(after.1.object),
                                            )?
                                        } else {
                                            db::hist_list_all(txn)?
                                        };
                                    }
                                    let mut commit_descriptions = HashMap::new();
                                    for ev in &events {
                                        match commit_descriptions.entry(ev.commit_) {
                                            std::collections::hash_map::Entry::Occupied(_) => (),
                                            std::collections::hash_map::Entry::Vacant(entry) => {
                                                entry.insert(
                                                    db::commit_get(txn, ev.commit_)?
                                                        .ok_or_else(
                                                            || GoodError(
                                                                format!(
                                                                    "Triple references nonexistent commit [{}]",
                                                                    ev.commit_.to_rfc3339()
                                                                ),
                                                            ),
                                                        )?
                                                        .description,
                                                );
                                            },
                                        }
                                    }
                                    return Ok((events, commit_descriptions));
                                }).await.err_internal()?;
                                resp = responder(RespHistory {
                                    events: events.into_iter().map(|x| RespHistoryEvent {
                                        delete: !x.exists,
                                        commit: x.commit_,
                                        triple: Triple {
                                            subject: x.subject.0,
                                            predicate: x.predicate,
                                            object: x.object.0,
                                        },
                                    }).collect(),
                                    commit_descriptions: commit_descriptions,
                                });
                            },
                            C2SReq::GetTriplesAround(req) => {
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "Get triples around").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let responder = req.respond();
                                let triples = tx(&state.db, {
                                    let nodes = req.nodes.clone();
                                    move |txn| {
                                        let nodes = nodes.into_iter().map(DbNode).collect::<Vec<_>>();
                                        return Ok(db::triple_list_around(txn, nodes.iter().collect())?);
                                    }
                                }).await.err_internal()?;
                                resp = responder(triples.into_iter().map(|t| Triple {
                                    subject: t.subject.0,
                                    predicate: t.predicate,
                                    object: t.object.0,
                                }).collect());
                            },
                            C2SReq::GetClientConfig(req) => {
                                resp =
                                    req.respond()(
                                        handle_get_filtered_client_config(state, &identity).await.err_internal()?,
                                    );
                            },
                            C2SReq::WhoAmI(req) => {
                                resp = req.respond()(match identity {
                                    access::Identity::Token(_) => RespWhoAmI::Token,
                                    access::Identity::User(ident) => RespWhoAmI::User(ident.0),
                                    access::Identity::Link(_) => RespWhoAmI::Public,
                                    access::Identity::Public => RespWhoAmI::Public,
                                });
                            },
                        }
                        return Ok(resp.1);
                    },
                    "file" => {
                        let identity = identify_requester(&state, &head.headers).await?;
                        let hash_gentype = path_iter.next().context("Missing file hash in path").err_external()?;
                        let (hash, gentype) = hash_gentype.split_once(".").unwrap_or((hash_gentype, ""));
                        let file =
                            FileHash::from_str(hash)
                                .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))
                                .err_external()?;
                        match can_access_file(&state, &identity, &file).await.err_internal()? {
                            AccessRes::Yes => (),
                            AccessRes::NoIdent => return Ok(response_401()),
                            AccessRes::NoAccess => return Ok(response_403()),
                        }
                        let gentype = gentype.to_string();
                        let subpath =
                            path_iter
                                .map(|x| urlencoding::decode(x).unwrap_or(x.into()))
                                .collect::<Vec<_>>()
                                .join("/");
                        match head.method {
                            Method::HEAD => {
                                // Inaccurate for non-file derivations, but HEAD is mostly intended for maybe
                                // media range request
                                return handle_file_head(state, file).await;
                            },
                            Method::GET => {
                                return handle_file_get(state, head, file, gentype, subpath).await;
                            },
                            Method::POST => {
                                return handle_file_post(state, head, file, body).await.err_internal();
                            },
                            _ => return Ok(response_404()),
                        }
                    },
                    _ => {
                        return static_::handle_static(
                            state,
                            &head.headers,
                            head.uri.path().trim_matches('/'),
                        ).await;
                    },
                }
            }
        }
    }.await {
        Ok(r) => {
            return r;
        },
        Err(e) => {
            match e {
                VisErr::External(e) => {
                    return Response::builder()
                        .status(status::StatusCode::BAD_REQUEST)
                        .body(
                            http_body_util::Full::new(Bytes::from(e.into_bytes()))
                                .map_err(|_| std::io::Error::other(""))
                                .boxed(),
                        )
                        .unwrap();
                },
                VisErr::Internal(e) => {
                    state.log.log_err(loga::WARN, e.context_with("Error serving response", ea!(url = url)));
                    return Response::builder()
                        .status(503)
                        .body(
                            http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed(),
                        )
                        .unwrap();
                },
            }
        },
    }
}

#[derive(Aargvark)]
pub struct Args {
    config: AargvarkJson<Config>,
    validate: Option<()>,
}

pub async fn main(args: Args) -> Result<(), loga::Error> {
    let config = args.config.value;
    if args.validate.is_some() {
        return Ok(());
    }
    let log = Log::new_root(match config.debug {
        true => loga::DEBUG,
        false => loga::INFO,
    });
    let tm = taskmanager::TaskManager::new();
    {
        let genfiles_dir = config.cache_dir.join("genfiles");
        create_dirs(&genfiles_dir).await?;
        create_dirs(&config.graph_dir).await?;
        let stage_dir = config.files_dir.join("stage");
        create_dirs(&stage_dir).await?;
        let files_dir = config.files_dir.join("ready");
        create_dirs(&files_dir).await?;
        create_dirs(&config.temp_dir).await?;
        let db_path = config.graph_dir.join("db.sqlite3");
        let db =
            deadpool_sqlite::Config::new(&db_path)
                .builder(deadpool_sqlite::Runtime::Tokio1)
                .context("Error creating sqlite pool builder")?
                .post_create(Hook::async_fn(|db, _| Box::pin(async {
                    db
                        .interact(|db| {
                            db.busy_timeout(Duration::from_secs(60 * 10))?;
                            rusqlite::vtab::array::load_module(db)?;
                            return Ok(());
                        })
                        .await
                        .map_err(|e| HookError::Message(e.to_string().into()))?
                        .map_err(|e| HookError::Backend(e))?;
                    return Ok(());
                })))
                .build()
                .context("Error creating sqlite pool")?;
        db.get().await?.interact({
            let log = log.clone();
            move |db| {
                db::migrate(db)?;
                if db
                    .prepare("select 1 from sqlite_master where type='table' and name='meta_fts'")
                    .context("Error preparing statement to check for meta_fts")?
                    .query([])
                    .context("Error running query to check for meta_fts")?
                    .next()
                    .context("Error reading query to check for meta_fts results")?
                    .is_none() {
                    log.log(loga::DEBUG, "Initializing fts table");
                    db.execute_batch(include_str!("setup_fts.sql")).context("Error setting up meta_fts")?;
                    log.log(loga::DEBUG, "Done initializing fts table");
                }
                return Ok(()) as Result<_, loga::Error>;
            }
        }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;

        // Setup state
        let oidc_state = match &config.oidc {
            Some(oidc_config) => {
                Some(oidc::new_state(&log, oidc_config.clone()).await.context("Error creating oidc state")?)
            },
            None => None,
        };
        let fdap_state = match &config.fdap {
            Some(fdap_config) => {
                Some(
                    FdapState {
                        fdap_client: fdap::Client::builder()
                            .with_base_url(Uri::from_str(&fdap_config.url).context("Invalid fdap url")?)
                            .with_token(fdap_config.token.clone())
                            .build()
                            .context("Error setting up fdap client")?,
                    },
                )
            },
            None => None,
        };
        let global_state = match &config.global {
            MaybeFdap::Fdap(subpath) => {
                let Some(fdap) = &fdap_state else {
                    return Err(loga::err("Global config set to use FDAP but no FDAP configured at config root"));
                };
                GlobalState::Fdap(FdapGlobalState {
                    fdap: fdap.clone(),
                    subpath: subpath.clone(),
                    cache: Mutex::new(None),
                })
            },
            MaybeFdap::Local(global_config) => {
                GlobalState::Local(build_global_config(global_config).context("Error assembling local config")?)
            },
        };
        let users_state = match &config.user {
            MaybeFdap::Fdap(subpath) => {
                let Some(fdap) = &fdap_state else {
                    return Err(loga::err("User config set to use FDAP but no FDAP configured at config root"));
                };
                UsersState::Fdap(FdapUsersState {
                    fdap: fdap.clone(),
                    user_subpath: subpath.clone(),
                    cache: Cache::builder().time_to_live(Duration::from_secs(10)).build(),
                })
            },
            MaybeFdap::Local(users_config) => UsersState::Local(
                LocalUsersState {
                    users: users_config.users.iter().map(|(k, v)| (k.clone(), Arc::new(v.clone()))).collect(),
                },
            ),
        };
        let (background_tx, background_rx) = mpsc::unbounded_channel();
        let state = Arc::new(State {
            temp_dir: config.temp_dir,
            oidc_state: oidc_state,
            fdap_state: fdap_state,
            global_state: global_state,
            users_state: users_state,
            tm: tm.clone(),
            db: db.clone(),
            log: log.clone(),
            files_dir: files_dir.clone(),
            stage_dir: stage_dir,
            genfiles_dir: genfiles_dir.clone(),
            finishing_uploads: Mutex::new(HashSet::new()),
            background: background_tx,
            http_resp_headers: HeaderMap::from_iter([
                //. .
                ("cross-origin-embedder-policy", "require-corp"),
                ("cross-origin-opener-policy", "same-origin"),
            ].into_iter().map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))),
            link_bg: Mutex::new(None),
            link_sessions: Cache::builder().time_to_idle(Duration::from_secs(24 * 60 * 60)).build(),
        });

        // Background tasks
        state.background.send(BackgroundJob::All).log(&log, loga::WARN, "Error triggering initial generate files scan");
        start_background_job(&state, &tm, background_rx);

        // Client<->server
        tm.critical_stream(
            "Server",
            TcpListenerStream::new(
                TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
            ),
            {
                cap_fn!((stream)(log, state) {
                    let stream = match stream {
                        Ok(s) => s,
                        Err(e) => {
                            log.log_err(loga::DEBUG, e.context("Error opening peer stream"));
                            return Ok(());
                        },
                    };
                    let io = TokioIo::new(stream);
                    tokio::task::spawn(async move {
                        match async {
                            ta_return!((), loga::Error);
                            http1::Builder::new().serve_connection(io, service_fn(cap_fn!((req)(state) {
                                return Ok(handle_req(state, req).await) as Result<_, std::io::Error>;
                            }))).with_upgrades().await?;
                            return Ok(());
                        }.await {
                            Ok(_) => (),
                            Err(e) => {
                                log.log_err(loga::DEBUG, e.context("Error serving connection"));
                            },
                        }
                    });
                    return Ok(());
                })
            },
        );
    }

    // Wait for shutdown, cleanup
    tm.join(&log).await?;
    return Ok(());
}
