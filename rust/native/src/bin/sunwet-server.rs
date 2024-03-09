use std::{
    collections::{
        BTreeMap,
        HashMap,
        HashSet,
    },
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::AtomicU8,
        Arc,
        Mutex,
    },
};
use aargvark::{
    vark,
    Aargvark,
};
use chrono::{
    Duration,
};
use cozo::{
    DataValue,
    DbInstance,
    NamedRows,
};
use http_body_util::{
    combinators::BoxBody,
    BodyExt,
};
use hyper::{
    body::{
        Bytes,
        Incoming,
    },
    server::conn::http1,
    service::service_fn,
    Method,
    Request,
    Response,
};
use hyper_util::rt::TokioIo;
use loga::{
    ea,
    fatal,
    DebugDisplay,
    ErrContext,
    ResultContext,
};
use native::{
    cap_fn,
    ta_res,
    util::{
        Flag,
        Log,
    },
};
use serverlib::{
    handle_link::{
        handle_ws,
        handle_ws_link,
        handle_ws_main,
    },
    httpresp::response_401,
    state::{
        State,
    },
};
use shared::model::{
    C2SReq,
    FileHash,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::{
    fs::create_dir_all,
    net::TcpListener,
    task::spawn_blocking,
};
use tokio_stream::wrappers::TcpListenerStream;
use crate::serverlib::{
    auth::check_auth,
    dbutil::{
        cozo_to_json,
        json_to_cozo,
    },
    defaultviews::default_view_albums,
    handle_files::{
        handle_commit,
        handle_file_get,
        handle_file_head,
        handle_file_post,
        handle_finish_upload,
    },
    handle_static::handle_static,
    handle_views::{
        handle_view_delete,
        handle_view_ensure,
        handle_view_list,
    },
    httpresp::{
        body_full,
        response_200_json,
        response_404,
    },
    task_gc::handle_gc,
};

pub mod serverlib;

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub debug: bool,
    pub persistent_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub bind_addr: SocketAddr,
}

#[derive(Aargvark)]
pub struct Args {
    pub config: aargvark::AargvarkJson<Config>,
}

async fn handle_req(state: Arc<State>, mut req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        let upgrade = hyper_tungstenite::upgrade(&mut req, None);
        let (head, _) = req.into_parts();
        let mut path_iter = head.uri.path().trim_matches('/').split('/');
        let link_type = path_iter.next().unwrap();
        let session = path_iter.next().unwrap();
        match link_type {
            "link" => {
                {
                    let Some(want_session) =&* state.link_session.lock().unwrap() else {
                        return response_401();
                    };
                    if want_session.as_str() != session {
                        return response_401();
                    }
                }
                return handle_ws(state, head, upgrade, handle_ws_link);
            },
            "main" => {
                if let Some(resp) = check_auth(&state, &head) {
                    return resp;
                }
                *state.link_session.lock().unwrap() = Some(session.to_string());
                return handle_ws(state, head, upgrade, handle_ws_main);
            },
            _ => {
                state.log.log_with(Flag::Debug, "Websocket connection on unknown path", ea!(path = link_type));
                return response_404();
            },
        }
    }
    let (head, body) = req.into_parts();
    let url = head.uri.clone();
    match async {
        let state = state.clone();
        ta_res!(Response < BoxBody < Bytes, std:: io:: Error >>);
        let mut path_iter = head.uri.path().trim_matches('/').split('/');
        let mut path_first = path_iter.next().unwrap();
        if path_first == "" {
            path_first = "static";
        }
        match (head.method.clone(), path_first) {
            (Method::GET, "static") => {
                return handle_static(path_iter).await;
            },
            (Method::POST, "api") => {
                if let Some(resp) = check_auth(&state, &head) {
                    return Ok(resp);
                }
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
                        let mut parameters = BTreeMap::new();
                        for (k, v) in q.parameters {
                            parameters.insert(k, json_to_cozo(v)?);
                        }
                        let res = match spawn_blocking(cap_fn!(()(state) {
                            state.db.run_script_read_only(&q.query, parameters)
                        })).await?.await {
                            Ok(r) => r,
                            Err(e) => {
                                return Ok(
                                    Response::builder()
                                        .status(400)
                                        .body(body_full(e.to_string().as_bytes().to_vec()))
                                        .unwrap(),
                                );
                            },
                        };
                        let mut out = vec![];
                        for row in res.rows {
                            let mut row_out = HashMap::<String, serde_json::Value>::new();
                            for (header, col) in res.headers.iter().zip(row) {
                                row_out.insert(header.clone(), cozo_to_json(col)?);
                            }
                            out.push(row_out);
                        }
                        return Ok(response_200_json(out));
                    },
                    C2SReq::ViewsList => {
                        return handle_view_list(state).await;
                    },
                    C2SReq::ViewEnsure(args) => {
                        return handle_view_ensure(state, args).await;
                    },
                    C2SReq::ViewDelete(id) => {
                        return handle_view_delete(state, id).await;
                    },
                }
            },
            (m, "file") => {
                let hash = path_iter.next().context("Missing file hash in path")?;
                let file =
                    FileHash::from_str(
                        hash,
                    ).map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))?;
                match m {
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
    }.await {
        Ok(r) => r,
        Err(e) => {
            state.log.log_err(Flag::Warn, e.context_with("Error serving response", ea!(url = url)));
            return Response::builder()
                .status(503)
                .body(http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed())
                .unwrap();
        },
    }
}

#[tokio::main]
async fn main() {
    async fn inner() -> Result<(), loga::Error> {
        let config = vark::<Args>().config.value;
        let mut flags = vec![Flag::Warn, Flag::Info];
        if config.debug {
            flags.push(Flag::Debug);
        }
        let log = &Log::new().with_flags(&flags);
        create_dir_all(&config.persistent_dir).await.context("Failed to ensure persistent dir")?;
        let files_dir = config.persistent_dir.join("files");
        create_dir_all(&files_dir).await.context("Failed to ensure files dir")?;
        let stage_dir = config.persistent_dir.join("stage");
        create_dir_all(&stage_dir).await.context("Failed to ensure stage dir")?;
        let generated_dir = config.cache_dir.join("generated");
        create_dir_all(&generated_dir).await.context("Failed to ensure generated dir")?;
        let dbc =
            match DbInstance::new(
                "sqlite",
                &config.persistent_dir.join("db.cozo"),
                "",
            ).map_err(|e| loga::err(e.to_string()))? {
                DbInstance::Mem(_) => unreachable!(),
                DbInstance::Sqlite(dbc) => dbc,
            };
        match dbc.run_script(
            "?[u, v] <- [[0, 0]] :create schema_ver { unique: Int = u, => version: Int = v }",
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        ) {
            Ok(_) => { },
            Err(e) => if e.code().map(|x| x.to_string()).as_ref().map(|e| e.as_str()) !=
                Some("eval::stored_relation_conflict") {
                return Err(loga::err(e));
            },
        };
        match dbc
            .run_script("?[v] := *schema_ver{ version: v }", BTreeMap::new(), cozo::ScriptMutability::Immutable)
            .map_err(|e| loga::err(e))?
            .rows
            .into_iter()
            .next()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .get_int()
            .unwrap() {
            0 => {
                for script in [include_str!("migrate_00_00.cozo"), include_str!("migrate_00_01.cozo")] {
                    dbc
                        .run_script(script, BTreeMap::new(), cozo::ScriptMutability::Mutable)
                        .map_err(
                            |e| loga::err_with(
                                "Error running migration",
                                ea!(err = e.dbg_str(), version = 0, script = script),
                            ),
                        )?;
                }

                // Create default views
                let mut rows = vec![];
                for (id, def) in [("albums", default_view_albums())] {
                    rows.push(
                        vec![
                            DataValue::Str(id.into()),
                            DataValue::Str(serde_json::to_string(&def).unwrap().as_str().into())
                        ],
                    );
                }
                let mut params = BTreeMap::new();
                params.insert("view".to_string(), NamedRows {
                    headers: vec!["id".to_string(), "def".to_string()],
                    rows: rows,
                    next: None,
                });
                dbc
                    .import_relations(params)
                    .map_err(|e| loga::err(e.to_string()).context("Error running query"))?;
            },
            1 => { },
            i => panic!("Unknown db schema version: {}", i),
        };
        let tm = taskmanager::TaskManager::new();

        // GC
        //. tm.periodic(
        //.     "Garbage collection",
        //.     Duration::hours(24).to_std().unwrap(),
        //.     cap_fn!(()(log, dbc, files_dir, generated_dir) {
        //.         let log = log.fork(ea!(sys = "gc"));
        //.         match handle_gc(&log, &dbc, &files_dir, &generated_dir).await {
        //.             Ok(_) => { },
        //.             Err(e) => {
        //.                 log.log_err(Flag::Warn, e.context("Error performing garbage collection"));
        //.             },
        //.         }
        //.     }),
        //. );
        // Client<->server
        tm.critical_stream(
            "Server",
            TcpListenerStream::new(
                TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
            ),
            {
                let state = Arc::new(State {
                    tm: tm.clone(),
                    db: dbc.clone(),
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
                            log.log_err(Flag::Debug, e.context("Error opening peer stream"));
                            return Ok(());
                        },
                    };
                    let io = TokioIo::new(stream);
                    tokio::task::spawn(async move {
                        match async {
                            ta_res!(());
                            http1::Builder::new().serve_connection(io, service_fn(cap_fn!((req)(state) {
                                return Ok(handle_req(state, req).await) as Result<_, std::io::Error>;
                            }))).await?;
                            return Ok(());
                        }.await {
                            Ok(_) => (),
                            Err(e) => {
                                log.log_err(Flag::Debug, e.context("Error serving connection"));
                            },
                        }
                    });
                    return Ok(());
                })
            },
        );

        // Wait for shutdown, cleanup
        tm.join(log, Flag::Info).await?;
        return Ok(());
    }

    match inner().await {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
