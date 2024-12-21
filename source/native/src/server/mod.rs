use {
    crate::{
        cap_fn,
        interface::config::{
            Config,
            MaybeFdap,
        },
    },
    access::{
        can_read,
        can_write,
        identify_requester,
        CanRead,
        ReadRestriction,
    },
    chrono::DateTime,
    dbutil::tx,
    flowcontrol::ta_return,
    fsutil::create_dirs,
    handlers::{
        handle_files::{
            handle_commit,
            handle_file_get,
            handle_file_head,
            handle_file_post,
            handle_finish_upload,
        },
        handle_gc::handle_gc,
        handle_link::{
            handle_ws,
            handle_ws_link,
            handle_ws_main,
        },
        handle_menu::handle_get_menu,
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
        self,
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
        triple::FileHash,
        wire::{
            C2SReq,
            RespHistoryCommit,
            RespQuery,
            Triple,
        },
    },
    state::{
        FdapGlobalState,
        FdapState,
        FdapUsersState,
        GlobalConfig,
        GlobalState,
        LocalUsersState,
        State,
        UsersState,
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
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
                let mut path_first = path_iter.next().unwrap();
                if path_first == "" {
                    path_first = "static";
                }
                match path_first {
                    "oidc" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            let mut subpath = path_iter.collect::<Vec<_>>();
                            if subpath.is_empty() {
                                subpath.push("");
                            }
                            let subpath = subpath.join("/");
                            return Ok(handle_oidc::handle_oidc(oidc_state, head, &subpath).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "static" => {
                        return handle_static::handle_static(path_iter).await;
                    },
                    "api" => {
                        let Some(ident) = identify_requester(&state, &head.headers).await? else {
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
                                shared::interface::wire::C2SReqTrait,
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

                            impl ReqResp for shared::interface::wire::ReqGetMenu { }

                            impl ReqResp for shared::interface::wire::ReqQuery { }

                            impl ReqResp for shared::interface::wire::ReqHistory { }

                            impl ReqResp for shared::interface::wire::ReqUploadFinish { }
                        }

                        use resp::ReqResp;

                        let resp: (resp::RespToken, resp::Resp);
                        match req {
                            C2SReq::Commit(req) => {
                                if !can_write(&state, &ident).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                resp = req.respond()(handle_commit(state, req).await.err_internal()?);
                            },
                            C2SReq::UploadFinish(req) => {
                                if !can_write(&state, &ident).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                resp = req.respond()(handle_finish_upload(state, req.0).await.err_internal()?);
                            },
                            C2SReq::Query(req) => {
                                let read_restriction;
                                match can_read(&state, &ident).await.err_internal()? {
                                    CanRead::All => {
                                        read_restriction = ReadRestriction::None;
                                    },
                                    CanRead::Restricted(targets) => {
                                        read_restriction = ReadRestriction::Some(targets);
                                    },
                                    CanRead::No => {
                                        return Ok(response_401());
                                    },
                                }
                                resp =
                                    req.respond()(
                                        RespQuery {
                                            records: query::execute_query(
                                                &state.db,
                                                read_restriction,
                                                req.query,
                                                req.parameters,
                                            )
                                                .await
                                                .err_internal()?,
                                        },
                                    );
                            },
                            C2SReq::History(req) => {
                                if !can_write(&state, &ident).await.err_internal()? {
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
                                        iam_target: t.iam_target.0,
                                    };
                                    if t.exists {
                                        commit.add.push(t1);
                                    } else {
                                        commit.remove.push(t1)
                                    }
                                }
                                resp = req.respond()(out.into_values().collect());
                            },
                            C2SReq::GetMenu(req) => {
                                let access_restriction;
                                match can_read(&state, &ident).await.err_internal()? {
                                    access::CanRead::All => {
                                        access_restriction = None;
                                    },
                                    access::CanRead::Restricted(targets) => {
                                        access_restriction = Some(targets);
                                    },
                                    access::CanRead::No => {
                                        return Ok(response_401());
                                    },
                                }
                                resp =
                                    req.respond()(handle_get_menu(state, access_restriction).await.err_internal()?);
                            },
                        }
                        return Ok(resp.1);
                    },
                    "file" => {
                        let Some(ident) = identify_requester(&state, &head.headers).await? else {
                            return Ok(response_401());
                        };
                        let hash = path_iter.next().context("Missing file hash in path").err_external()?;
                        let file =
                            FileHash::from_str(hash)
                                .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))
                                .err_external()?;
                        match head.method {
                            Method::HEAD => {
                                return handle_file_head(state, &ident, file).await;
                            },
                            Method::GET => {
                                return handle_file_get(state, &ident, head, file).await;
                            },
                            Method::POST => {
                                if !can_write(&state, &ident).await.err_internal()? {
                                    return Ok(response_401());
                                }
                                return handle_file_post(state, head, file, body).await.err_internal();
                            },
                            _ => return Ok(response_404()),
                        }
                    },
                    _ => return Ok(response_404()),
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

async fn main(config: Config) -> Result<(), loga::Error> {
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
        return db::migrate(db);
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
                    Some(
                        handle_oidc::new_state(
                            &log,
                            &Uri::from_str(&oidc_config.provider_url).context("Invalid oidc provider url")?,
                            oidc_config.clone(),
                        ).await?,
                    )
                },
                None => None,
            };
            let fdap_state = match &config.fdap {
                Some(fdap_config) => {
                    Some(
                        FdapState {
                            fdap_client: fdap::Client::builder()
                                .with_base_url(Uri::from_str(&fdap_config.url).context("Invalid fdap url")?)
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
                MaybeFdap::Local(global_config) => GlobalState::Local(Arc::new(GlobalConfig {
                    config: global_config.clone(),
                    admin_token: global_config
                        .admin_token
                        .as_ref()
                        .map(|x| x.as_str())
                        .map(htserve::auth::hash_auth_token),
                })),
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
                        }))).await?;
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
