use {
    crate::{
        cap_fn,
        interface::{
            config::{
                Config,
                IamGrants,
                MaybeFdap,
            },
            triple::{
                DbFileHash,
                DbNode,
            },
        },
    },
    aargvark::{
        traits_impls::AargvarkJson,
        Aargvark,
    },
    access::{
        identify_requester,
        is_admin,
    },
    chrono::DateTime,
    dbutil::tx,
    flowcontrol::{
        shed,
        ta_return,
    },
    fsutil::create_dirs,
    handlers::{
        handle_files::{
            handle_commit,
            handle_file_get,
            handle_file_head,
            handle_file_post,
            handle_finish_upload,
            handle_form_commit,
        },
        handle_gc::handle_gc,
        handle_link::{
            handle_ws,
            handle_ws_link,
            handle_ws_main,
        },
        handle_menu::handle_get_filtered_client_config,
        handle_oidc,
        handle_static,
    },
    http::{
        status,
        Uri,
    },
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::htserve::{
        responses::{
            response_401,
            response_404,
        },
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    hyper::{
        body::{
            Bytes,
            Incoming,
        },
        server::conn::http1,
        service::service_fn,
        Method,
        Request,
        Response,
    },
    hyper_util::rt::TokioIo,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            C2SReq,
            RespGetTriplesAround,
            RespHistoryCommit,
            RespQuery,
            RespWhoAmI,
            TreeNode,
            Triple,
        },
    },
    state::{
        build_global_config,
        get_global_config,
        get_iam_grants,
        FdapGlobalState,
        FdapState,
        FdapUsersState,
        GlobalState,
        LocalUsersState,
        MenuItem,
        State,
        UsersState,
    },
    std::{
        collections::{
            BTreeMap,
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
            atomic::AtomicU8,
            Arc,
            Mutex,
        },
        time::Duration,
    },
    tokio::net::TcpListener,
    tokio_stream::wrappers::TcpListenerStream,
};

pub mod db;
pub mod filesutil;
pub mod defaultviews;
pub mod state;
pub mod dbutil;
pub mod query;
pub mod query_test;
pub mod access;
pub mod fsutil;
pub mod handlers;

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
                let session = path_iter.next().unwrap();
                match link_type {
                    "link" => {
                        {
                            let Some(want_session) = &*state.link_session.lock().unwrap() else {
                                return Ok(response_401());
                            };
                            if want_session.as_str() != session {
                                return Ok(response_401());
                            }
                        }
                        return Ok(handle_ws(state, head, upgrade, handle_ws_link));
                    },
                    "main" => {
                        *state.link_session.lock().unwrap() = Some(session.to_string());
                        return Ok(handle_ws(state, head, upgrade, handle_ws_main));
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
                            return Ok(handle_oidc::handle_oidc(oidc_state, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "logout" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            return Ok(handle_oidc::handle_logout(oidc_state, &state.log, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "api" => {
                        let Some(identity) = identify_requester(&state, &head.headers).await? else {
                            return Ok(response_401());
                        };
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

                            impl ReqResp for shared::interface::wire::ReqFormCommit { }

                            impl ReqResp for shared::interface::wire::ReqGetClientConfig { }

                            impl ReqResp for shared::interface::wire::ReqQuery { }

                            impl ReqResp for shared::interface::wire::ReqViewQuery { }

                            impl ReqResp for shared::interface::wire::ReqHistory { }

                            impl ReqResp for shared::interface::wire::ReqGetTriplesAround { }

                            impl ReqResp for shared::interface::wire::ReqUploadFinish { }

                            impl ReqResp for shared::interface::wire::ReqWhoAmI { }
                        }

                        use resp::ReqResp;

                        let resp: (resp::RespToken, resp::Resp);
                        match req {
                            C2SReq::Commit(req) => {
                                if !is_admin(&state, &identity).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                resp = req.respond()(handle_commit(state, req).await.err_internal()?);
                            },
                            C2SReq::FormCommit(req) => {
                                let responder = req.respond();
                                match get_iam_grants(&state, &identity).await.err_internal()? {
                                    IamGrants::Admin => { },
                                    IamGrants::Limited(grants) => {
                                        if !grants.contains(&req.menu_item_id) {
                                            return Ok(response_401());
                                        }
                                    },
                                }
                                resp = responder(handle_form_commit(state, req).await?);
                            },
                            C2SReq::UploadFinish(req) => {
                                let responder = req.respond();
                                let Some(res) =
                                    handle_finish_upload(state, &identity, req.0).await.err_internal()? else {
                                        return Ok(response_401());
                                    };
                                resp = responder(res);
                            },
                            C2SReq::Query(req) => {
                                if !is_admin(&state, &identity).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                resp =
                                    req.respond()(
                                        RespQuery {
                                            records: query::execute_query(&state.db, req.query, req.parameters)
                                                .await
                                                .err_internal()?,
                                        },
                                    );
                            },
                            C2SReq::ViewQuery(req) => {
                                let responder = req.respond();
                                match get_iam_grants(&state, &identity).await.err_internal()? {
                                    IamGrants::Admin => { },
                                    IamGrants::Limited(grants) => {
                                        if !grants.contains(&req.menu_item_id) {
                                            return Ok(response_401());
                                        }
                                    },
                                }
                                let global_config = get_global_config(&state).await.err_internal()?;
                                let Some(MenuItem::View(menu_item)) =
                                    global_config.menu_items.get(&req.menu_item_id) else {
                                        return Err(
                                            loga::err_with(
                                                "No known view menu_item with id",
                                                ea!(menu_item = req.menu_item_id),
                                            ),
                                        ).err_external();
                                    };
                                shed!{
                                    'granted _;
                                    match get_iam_grants(&state, &identity).await.err_internal()? {
                                        IamGrants::Admin => break 'granted,
                                        IamGrants::Limited(grants) => {
                                            for id in &menu_item.self_and_ancestors {
                                                if grants.contains(id) {
                                                    break 'granted;
                                                }
                                            }
                                        },
                                    }
                                    return Ok(response_401());
                                }
                                let view = global_config.views.get(&req.menu_item_id).unwrap();
                                let Some(query) = view.queries.get(&req.query) else {
                                    return Err(
                                        loga::err_with(
                                            "No known query with id in view",
                                            ea!(view = menu_item.item.view_id, query = req.query),
                                        ),
                                    ).err_external();
                                };
                                let mut view_hash = DefaultHasher::new();
                                view.hash(&mut view_hash);
                                let view_hash = view_hash.finish();
                                let records =
                                    query::execute_query(&state.db, query.clone(), req.parameters)
                                        .await
                                        .err_internal()?;

                                fn gather_files(files: &mut Vec<FileHash>, r: &TreeNode) {
                                    match r {
                                        TreeNode::Scalar(s) => {
                                            if let Node::File(s) = s {
                                                files.push(s.clone());
                                            }
                                        },
                                        TreeNode::Array(a) => {
                                            for v in a {
                                                gather_files(files, v);
                                            }
                                        },
                                        TreeNode::Record(r) => {
                                            for v in r.values() {
                                                gather_files(files, v);
                                            }
                                        },
                                    }
                                }

                                let mut files = vec![];
                                for record in &records {
                                    for v in record.values() {
                                        gather_files(&mut files, v);
                                    }
                                }
                                tx(&state.db, {
                                    move |txn| {
                                        db::file_access_clear_nonversion(txn, &req.menu_item_id, view_hash as i64)?;
                                        for file in files {
                                            db::file_access_insert(
                                                txn,
                                                &DbFileHash(file.clone()),
                                                &req.menu_item_id,
                                                view_hash as i64,
                                            )?;
                                        }
                                        return Ok(());
                                    }
                                }).await.err_internal()?;
                                resp = responder(RespQuery { records: records });
                            },
                            C2SReq::History(req) => {
                                if !is_admin(&state, &identity).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                let (commits, triples) = tx(&state.db, move |txn| {
                                    let start = DateTime::from_str(&req.start_incl.to_string()).unwrap();
                                    let end = DateTime::from_str(&req.end_excl.to_string()).unwrap();
                                    return Ok(
                                        (
                                            db::commit_list_between(txn, start, end)?,
                                            db::triple_list_between(txn, start, end)?,
                                        ),
                                    );
                                }).await.err_internal()?;
                                let mut out = BTreeMap::new();
                                for c in commits {
                                    out.insert(c.timestamp, RespHistoryCommit {
                                        timestamp: c.timestamp.to_rfc3339().parse().unwrap(),
                                        desc: c.description,
                                        add: vec![],
                                        remove: vec![],
                                    });
                                }
                                for t in triples {
                                    let Some(commit) = out.get_mut(&t.timestamp) else {
                                        state
                                            .log
                                            .log_with(
                                                loga::WARN,
                                                "Triple detached from commit - this is probably a bug",
                                                ea!(
                                                    stamp = t.timestamp,
                                                    subject = t.subject.0.dbg_str(),
                                                    predicate = t.predicate,
                                                    object = t.object.0.dbg_str()
                                                ),
                                            );
                                        continue;
                                    };
                                    let t1 = Triple {
                                        subject: t.subject.0,
                                        predicate: t.predicate,
                                        object: t.object.0,
                                    };
                                    if t.exists {
                                        commit.add.push(t1);
                                    } else {
                                        commit.remove.push(t1)
                                    }
                                }
                                resp = req.respond()(out.into_values().collect());
                            },
                            C2SReq::GetTriplesAround(req) => {
                                if !is_admin(&state, &identity).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                let responder = req.respond();
                                let (incoming, outgoing) = tx(&state.db, move |txn| {
                                    return Ok(
                                        (
                                            db::triple_list_to(txn, &DbNode(req.node.clone()))?,
                                            db::triple_list_from(txn, &DbNode(req.node.clone()))?,
                                        ),
                                    );
                                }).await.err_internal()?;
                                resp = responder(RespGetTriplesAround {
                                    incoming: incoming.into_iter().filter_map(|x| if !x.exists {
                                        None
                                    } else {
                                        Some(Triple {
                                            subject: x.subject.0,
                                            predicate: x.predicate,
                                            object: x.object.0,
                                        })
                                    }).collect(),
                                    outgoing: outgoing.into_iter().filter_map(|x| if !x.exists {
                                        None
                                    } else {
                                        Some(Triple {
                                            subject: x.subject.0,
                                            predicate: x.predicate,
                                            object: x.object.0,
                                        })
                                    }).collect(),
                                });
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
                                    access::Identity::Public => RespWhoAmI::Public,
                                });
                            },
                        }
                        return Ok(resp.1);
                    },
                    "file" => {
                        let Some(identity) = identify_requester(&state, &head.headers).await? else {
                            return Ok(response_401());
                        };
                        let hash = path_iter.next().context("Missing file hash in path").err_external()?;
                        let file =
                            FileHash::from_str(hash)
                                .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))
                                .err_external()?;
                        match head.method {
                            Method::HEAD => {
                                return handle_file_head(state, &identity, file).await;
                            },
                            Method::GET => {
                                return handle_file_get(state, &identity, head, file).await;
                            },
                            Method::POST => {
                                return handle_file_post(state, &identity, head, file, body).await.err_internal();
                            },
                            _ => return Ok(response_404()),
                        }
                    },
                    _ => {
                        return handle_static::handle_static(head.uri.path().trim_matches('/')).await;
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
    let cache_dir = config.cache_dir;
    create_dirs(&cache_dir).await?;
    create_dirs(&config.graph_dir).await?;
    let stage_dir = config.files_dir.join("stage");
    create_dirs(&stage_dir).await?;
    let files_dir = config.files_dir.join("ready");
    create_dirs(&files_dir).await?;
    let db_path = config.graph_dir.join("db.sqlite3");
    let db = deadpool_sqlite::Config::new(&db_path).create_pool(deadpool_sqlite::Runtime::Tokio1).unwrap();
    db.get().await?.interact(|db| {
        db::migrate(db)?;
        db.execute(include_str!("setup_fts.sql"), ())?;
        return Ok(()) as Result<_, loga::Error>;
    }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;
    let tm = taskmanager::TaskManager::new();

    // GC
    tm.periodic("Garbage collection", Duration::from_secs(24 * 60 * 60), cap_fn!(()(log, db, files_dir, cache_dir) {
        let log = log.fork(ea!(sys = "gc"));
        match handle_gc(&log, &db, &files_dir, &cache_dir).await {
            Ok(_) => { },
            Err(e) => {
                log.log_err(loga::WARN, e.context("Error performing database garbage collection"));
            },
        }
    }));

    // Client<->server
    tm.critical_stream(
        "Server",
        TcpListenerStream::new(
            TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
        ),
        {
            let oidc_state = match &config.oidc {
                Some(oidc_config) => {
                    Some(handle_oidc::new_state(&log, oidc_config.clone()).await?)
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
                                .build()?,
                        },
                    )
                },
                None => None,
            };
            let global_state = match &config.global {
                MaybeFdap::Fdap(subpath) => {
                    let Some(fdap) = &fdap_state else {
                        return Err(
                            loga::err("Global config set to use FDAP but no FDAP configured at config root"),
                        );
                    };
                    GlobalState::Fdap(FdapGlobalState {
                        fdap: fdap.clone(),
                        subpath: subpath.clone(),
                        cache: Mutex::new(None),
                    })
                },
                MaybeFdap::Local(global_config) => {
                    GlobalState::Local(build_global_config(global_config)?)
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
            let state = Arc::new(State {
                oidc_state: oidc_state,
                fdap_state: fdap_state,
                global_state: global_state,
                users_state: users_state,
                tm: tm.clone(),
                db: db.clone(),
                log: log.clone(),
                files_dir: files_dir,
                stage_dir: stage_dir,
                cache_dir: cache_dir,
                finishing_uploads: Mutex::new(HashSet::new()),
                link_bg: Mutex::new(None),
                link_ids: AtomicU8::new(0),
                link_main: Mutex::new(None),
                link_links: Mutex::new(HashMap::new()),
                link_public_files: Mutex::new(HashSet::new()),
                link_session: Mutex::new(None),
            });
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

    // Wait for shutdown, cleanup
    tm.join(&log).await?;
    return Ok(());
}
