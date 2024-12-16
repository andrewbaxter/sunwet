use {
    crate::serverlib::{
        defaultviews::default_view_albums,
        handle_files::{
            handle_commit,
            handle_file_get,
            handle_file_head,
            handle_file_post,
            handle_finish_upload,
        },
        handle_menu::handle_get_menu,
        handle_static::handle_static,
        task_gc::handle_gc,
    },
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
        Aargvark,
    },
    chrono::Duration,
    flowcontrol::{
        shed,
        ta_return,
    },
    http::{
        status,
        Uri,
    },
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::{
        htreq,
        htserve::{
            self,
            responses::{
                response_200_json,
                response_400,
                response_401,
                response_404,
            },
            viserr::{
                ResultVisErr,
                VisErr,
            },
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
        fatal,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    native::{
        cap_fn,
        interface::config::{
            access::UserAccess,
            Config,
        },
    },
    platform_info::{
        PlatformInfo,
        PlatformInfoAPI,
        UNameAPI,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serverlib::{
        access::Identity,
        db,
        dbutil::tx,
        handle_link::{
            handle_ws,
            handle_ws_link,
            handle_ws_main,
        },
        handle_oidc::{
            self,
            handle_oidc,
        },
        query::{
            self,
        },
        state::{
            IdentityStatePublic,
            State,
        },
    },
    shared::interface::{
        iam::{
            IamUserGroupId,
            IdentityId,
            IAM_TARGET_ADMIN,
        },
        triple::FileHash,
        wire::{
            C2SReq,
            QueryResp,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        net::SocketAddr,
        os::unix::ffi::OsStrExt,
        path::PathBuf,
        str::FromStr,
        sync::{
            atomic::AtomicU8,
            Arc,
            Mutex,
        },
    },
    tokio::{
        fs::create_dir_all,
        net::TcpListener,
        task::spawn_blocking,
    },
    tokio_stream::wrappers::TcpListenerStream,
};

pub mod serverlib;

#[derive(Aargvark)]
pub struct Args {
    pub config: AargvarkJson<Config>,
}

async fn check_identity(state: &State, req: &Request<Incoming>) -> Result<Option<Identity>, VisErr<loga::Error>> {
    match &state.identity_mode {
        serverlib::state::IdentityState::Admin => {
            return Ok(Some(Identity::Admin));
        },
        serverlib::state::IdentityState::Public(ident_state) => {
            if let Some(want_token) = &ident_state.admin_token {
                if let Ok(got_token) = htserve::auth::get_auth_token(req.headers()) {
                    if !htserve::auth::check_auth_token_hash(&want_token, &got_token) {
                        return Ok(None);
                    }
                    return Ok(Some(Identity::Admin));
                }
            }
            if let Some(user) = handle_oidc::get_req_identity(&state.log, &state.oidc_state, req.headers()).await {
                if let Some(groups) = ident_state.user_group_cache.try_get_with(user.clone(), async move {
                    let Some(json) =
                        ident_state
                            .fdap_client
                            .user_get(&user.0, [ident_state.fdap_user_subpath.as_str(), "access_groups"], 1000000)
                            .await
                            .context("Error looking up user groups")? else {
                            return Ok(None);
                        };
                    let Ok(groups) = serde_json::from_value::<Vec<IamUserGroupId>>(json) else {
                        return Err(loga::err_with("User has invalid access group json", ea!(user = user.0)));
                    };
                    return Ok(Some(groups));
                }).await.map_err(|e| e.as_ref().clone()).err_internal()? {
                    return Ok(Some(Identity::NonAdmin(groups)));
                }
            }
            return Ok(Some(Identity::NonAdmin(state.config.access.world_group_membership.clone())));
        },
    }
}

async fn handle_req(state: Arc<State>, mut req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    match async {
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
                    state.log.log_with(loga::DEBUG, "Websocket connection on unknown path", ea!(path = link_type));
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
                    return Ok(handle_oidc(&state.oidc_state, head, req.uri().path()).await?);
                },
                "static" => {
                    return handle_static(path_iter).await;
                },
                "api" => {
                    let req =
                        serde_json::from_slice::<C2SReq>(
                            &body.collect().await.context("Error reading request bytes")?.to_bytes(),
                        ).context("Failed to parse json request body")?;
                    match req {
                        C2SReq::Commit(c) => {
                            return handle_commit(state, c).await;
                        },
                        C2SReq::UploadFinish(hash) => {
                            return handle_finish_upload(state, hash).await;
                        },
                        C2SReq::Query(q) => {
                            return Ok(
                                response_200_json(
                                    QueryResp {
                                        records: query::execute_query(&state.db, access, q.q, q.parameters).await?,
                                    },
                                ),
                            );
                        },
                        C2SReq::GetMenu => {
                            return handle_get_menu(state, &access).await;
                        },
                    }
                },
                "file" => {
                    let hash = path_iter.next().context("Missing file hash in path")?;
                    let file =
                        FileHash::from_str(
                            hash,
                        ).map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))?;
                    match *req.method() {
                        Method::HEAD => {
                            return handle_file_head(state, head, file).await;
                        },
                        Method::GET => {
                            return handle_file_get(state, head, file).await;
                        },
                        Method::POST => {
                            return handle_file_post(state, head, file, body).await;
                        },
                        _ => return Ok(response_404()),
                    }
                },
                _ => return Ok(response_404()),
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
                    state.log.log_err(loga::WARN, e.context_with("Error serving response", ea!(url = req.uri())));
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

#[tokio::main]
async fn main() {
    async fn inner() -> Result<(), loga::Error> {
        let config = vark::<Args>().config.value;
        let log = Log::new_root(match config.debug {
            true => loga::DEBUG,
            false => loga::INFO,
        });
        create_dir_all(&config.persistent_dir).await.context("Failed to ensure persistent dir")?;
        let files_dir = config.persistent_dir.join("files");
        create_dir_all(&files_dir).await.context("Failed to ensure files dir")?;
        let stage_dir = config.persistent_dir.join("stage");
        create_dir_all(&stage_dir).await.context("Failed to ensure stage dir")?;
        let generated_dir = config.cache_dir.join("generated");
        create_dir_all(&generated_dir).await.context("Failed to ensure generated dir")?;
        let db_path = config.persistent_dir.join("server.sqlite3");
        let db = deadpool_sqlite::Config::new(&db_path).create_pool(deadpool_sqlite::Runtime::Tokio1).unwrap();
        db.get().await?.interact(|db| {
            return db::migrate(db);
        }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;
        let tm = taskmanager::TaskManager::new();

        // GC
        tm.periodic(
            "Garbage collection",
            Duration::hours(24).to_std().unwrap(),
            cap_fn!(()(log, db, files_dir, generated_dir) {
                let log = log.fork(ea!(sys = "gc"));
                match handle_gc(&log, &db, &files_dir, &generated_dir).await {
                    Ok(_) => { },
                    Err(e) => {
                        log.log_err(loga::WARN, e.context("Error performing database garbage collection"));
                    },
                }
            }),
        );

        // Client<->server
        tm.critical_stream(
            "Server",
            TcpListenerStream::new(
                TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
            ),
            {
                let state = Arc::new(State {
                    identity_mode: config.identity,
                    tm: tm.clone(),
                    db: db.clone(),
                    log: log.clone(),
                    files_dir: files_dir,
                    stage_dir: stage_dir,
                    generated_dir: generated_dir,
                    finishing_uploads: Mutex::new(HashSet::new()),
                    link_bg: Mutex::new(None),
                    link_ids: AtomicU8::new(0),
                    link_main: Mutex::new(None),
                    link_links: Mutex::new(HashMap::new()),
                    link_public_files: HashSet::new(),
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

    match inner().await {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
