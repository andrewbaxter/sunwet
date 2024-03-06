use std::{
    cmp::Reverse,
    collections::{
        BTreeMap,
        HashMap,
        HashSet,
    },
    io::Write,
    net::SocketAddr,
    path::{
        Component,
        Path,
        PathBuf,
    },
    process::Stdio,
    str::FromStr,
    sync::{
        atomic::{
            AtomicU8,
            Ordering,
        },
        Arc,
        Mutex,
    },
    task::Poll,
};
use aargvark::{
    vark,
    Aargvark,
};
use async_walkdir::WalkDir;
use chrono::{
    Duration,
    Utc,
};
use cozo::{
    DataValue,
    Db,
    DbInstance,
    NamedRows,
    Num,
    SqliteStorage,
    Validity,
    ValidityTs,
};
use futures::{
    SinkExt,
    TryStreamExt,
};
use http::{
    header::AUTHORIZATION,
    request::Parts,
};
use http_body::Frame;
use http_body_util::{
    combinators::BoxBody,
    BodyExt,
    Full,
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
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use loga::{
    ea,
    fatal,
    DebugDisplay,
    ErrContext,
    ResultContext,
};
use serde_json::Number;
use native::{
    cap_fn,
    ta_res,
    util::{
        spawn_scoped,
        Flag,
        Log,
        ScopeValue,
    },
};
use shared::{
    bb,
    model::{
        link::{
            WsC2S,
            WsL2S,
            WsS2C,
            WsS2L,
        },
        view::ViewPartList,
        C2SReq,
        CommitResp,
        FileHash,
        FileUrlQuery,
        Node,
        UploadFinishResp,
        HEADER_OFFSET,
    },
    unenum,
};
use serde::{
    Deserialize,
    Serialize,
};
use sha2::{
    Sha256,
    Digest,
};
use taskmanager::TaskManager;
use tokio::{
    process::Command,
    fs::{
        create_dir_all,
        remove_file,
        rename,
        File,
    },
    io::{
        self,
        copy,
        AsyncReadExt,
        AsyncSeekExt,
        AsyncWrite,
        AsyncWriteExt,
    },
    net::TcpListener,
    select,
    sync::{
        mpsc,
        oneshot,
    },
    task::spawn_blocking,
};
use rust_embed::RustEmbed;
use tokio_stream::{
    wrappers::TcpListenerStream,
    StreamExt,
};
use tokio_util::io::ReaderStream;
use tempfile::tempdir;

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

fn file_path(root_path: &Path, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(root_path.join("sha256").join(&hash[0 .. 2]).join(&hash[2 .. 4]).join(hash));
        },
    }
}

fn generated_path(root_path: &Path, hash: &FileHash, generation: &str) -> Result<PathBuf, loga::Error> {
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(
                root_path
                    .join("sha256")
                    .join(&hash[0 .. 2])
                    .join(&hash[2 .. 4])
                    .join(format!("{}.{}", hash, generation)),
            );
        },
    }
}

fn staged_file_path(root_path: &Path, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(root_path.join(&format!("sha256_{}", hash)));
        },
    }
}

pub fn node_to_meta_row(rows: &mut Vec<HashMap<String, DataValue>>, n: &Node) -> Result<(), loga::Error> {
    let Node:: Value(serde_json::Value::String(v)) = n else {
        return Ok(());
    };
    let mut out = HashMap::new();
    out.insert("node".to_string(), node_to_row(n)?);
    out.insert("mimetype".to_string(), DataValue::Str("text/plain".into()));
    out.insert("text".to_string(), DataValue::Str(v.into()));
    rows.push(out);
    return Ok(());
}

pub fn node_to_row(n: &Node) -> Result<DataValue, loga::Error> {
    return Ok(match n {
        Node::Id(id) => DataValue::List(vec![DataValue::Str("id".into()), DataValue::Str(id.into())]),
        Node::File(hash) => DataValue::List(
            vec![DataValue::Str("file".into()), DataValue::Str(hash.to_string().into())],
        ),
        Node::Value(v) => DataValue::List(vec![DataValue::Str("value".into()), match v {
            serde_json::Value::Null => return Err(loga::err("Got null value; value nodes must be non-null")),
            serde_json::Value::Bool(v) => DataValue::Bool(*v),
            serde_json::Value::Number(v) => DataValue::Num(if v.is_f64() {
                Num::Float(v.as_f64().context("Json float out of range")?)
            } else {
                Num::Int(v.as_i64().context("Json float out of range")?)
            }),
            serde_json::Value::String(v) => DataValue::Str(v.into()),
            serde_json::Value::Array(_) => return Err(loga::err("Got array value; value nodes must be primitive")),
            serde_json::Value::Object(_) => return Err(loga::err("Got obj value; value nodes must be primitive")),
        }]),
    });
}

pub fn json_to_cozo(d: serde_json::Value) -> Result<DataValue, loga::Error> {
    match d {
        serde_json::Value::Null => return Ok(DataValue::Null),
        serde_json::Value::Bool(v) => return Ok(DataValue::Bool(v)),
        serde_json::Value::Number(v) => return Ok(DataValue::Num(if v.is_f64() {
            Num::Float(v.as_f64().context("Json float out of range")?)
        } else {
            Num::Int(v.as_i64().context("Json float out of range")?)
        })),
        serde_json::Value::String(v) => return Ok(DataValue::Str(v.into())),
        serde_json::Value::Array(v) => {
            let mut out = vec![];
            for v in v {
                out.push(json_to_cozo(v)?);
            }
            return Ok(DataValue::List(out));
        },
        serde_json::Value::Object(_) => return Err(loga::err("Objects aren't valid parameters")),
    }
}

pub fn cozo_to_json(d: DataValue) -> Result<serde_json::Value, loga::Error> {
    return Ok(match d {
        DataValue::Null => serde_json::Value::Null,
        DataValue::Bool(v) => serde_json::Value::Bool(v),
        DataValue::Num(v) => match v {
            Num::Int(v) => serde_json::Value::Number(Number::from(v)),
            Num::Float(v) => serde_json::Value::Number(Number::from_f64(v).unwrap()),
        },
        DataValue::Str(v) => serde_json::Value::String(v.to_string()),
        DataValue::List(v) => {
            let mut out = vec![];
            for v in v {
                out.push(cozo_to_json(v)?);
            }
            serde_json::Value::Array(out)
        },
        DataValue::Json(v) => v.0,
        DataValue::Validity(v) => {
            let mut o = serde_json::Map::new();
            o.insert("is_assert".to_string(), serde_json::Value::Bool(v.is_assert.0));
            o.insert("timestamp".to_string(), serde_json::Value::Number(Number::from(v.timestamp.0.0)));
            serde_json::Value::Object(o)
        },
        DataValue::Bot => panic!(),
        DataValue::Bytes(v) => serde_json::Value::String(hex::encode(&v)),
        DataValue::Uuid(v) => serde_json::Value::String(v.0.to_string()),
        DataValue::Regex(_) => panic!(),
        DataValue::Set(_) => panic!(),
        DataValue::Vec(v) => {
            let mut out = vec![];
            match v {
                cozo::Vector::F32(v) => {
                    for x in v {
                        out.push(
                            serde_json::Value::Number(
                                Number::from_f64(
                                    x as f64,
                                ).context("Received non-finite number which isn't supported in json")?,
                            ),
                        );
                    }
                },
                cozo::Vector::F64(v) => {
                    for x in v {
                        out.push(
                            serde_json::Value::Number(
                                Number::from_f64(
                                    x,
                                ).context("Received non-finite number which isn't supported in json")?,
                            ),
                        );
                    }
                },
            }
            serde_json::Value::Array(out)
        },
    });
}

pub fn body_empty() -> BoxBody<Bytes, std::io::Error> {
    return http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed();
}

pub fn body_full(data: Vec<u8>) -> BoxBody<Bytes, std::io::Error> {
    return http_body_util::Full::new(Bytes::from(data)).map_err(|_| std::io::Error::other("")).boxed();
}

pub fn body_json(data: impl Serialize) -> BoxBody<Bytes, std::io::Error> {
    return body_full(serde_json::to_vec(&data).unwrap());
}

pub fn response_400(message: impl ToString) -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(400).body(body_full(message.to_string().as_bytes().to_vec())).unwrap();
}

pub fn response_200() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(200).body(body_empty()).unwrap();
}

pub fn response_200_json(v: impl Serialize) -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(200).body(body_json(v)).unwrap();
}

pub fn response_404() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(404).body(body_empty()).unwrap();
}

pub fn response_401() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(401).body(body_empty()).unwrap();
}

fn check_auth(_state: &Arc<State>, parts: &Parts) -> Option<Response<BoxBody<Bytes, std::io::Error>>> {
    if parts.headers.get(AUTHORIZATION).is_none() {
        return Some(response_401());
    }
    return None;
}

fn check_file_auth(
    state: &Arc<State>,
    parts: &Parts,
    file: &FileHash,
) -> Option<Response<BoxBody<Bytes, std::io::Error>>> {
    if state.link_public_files.contains(file) {
        return None;
    }
    return check_auth(state, parts);
}

struct WsState<M> {
    send: mpsc::Sender<M>,
    ready: Mutex<Option<oneshot::Sender<Duration>>>,
}

struct State {
    tm: TaskManager,
    log: Log,
    db: Db<SqliteStorage>,
    files_dir: PathBuf,
    generated_dir: PathBuf,
    stage_dir: PathBuf,
    finishing_uploads: Mutex<HashSet<FileHash>>,
    // Websockets
    link_ids: AtomicU8,
    link_main: Mutex<Option<Arc<WsState<WsS2C>>>>,
    link_links: Mutex<HashMap<u8, Arc<WsState<WsS2L>>>>,
    link_bg: Mutex<Option<ScopeValue>>,
    link_public_files: HashSet<FileHash>,
    link_session: Mutex<Option<String>>,
}

fn handle_ws_main(state: Arc<State>, websocket: HyperWebsocket) {
    let (tx, mut rx) = mpsc::channel(10);
    *state.link_main.lock().unwrap() = Some(Arc::new(WsState {
        send: tx,
        ready: Mutex::new(None),
    }));
    tokio::spawn(async move {
        match async {
            ta_res!(());
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_res!(bool);

                    select!{
                        m = rx.recv() => {
                            let Some(to_remote) = m else {
                                return Ok(false);
                            };
                            websocket
                                .send(
                                    hyper_tungstenite::tungstenite::Message::text(
                                        serde_json::to_string(&to_remote).unwrap(),
                                    ),
                                )
                                .await?;
                        },
                        m = websocket.next() => {
                            let Some(from_remote) = m else {
                                return Ok(false);
                            };
                            let from_remote = from_remote?;

                            bb!{
                                match &from_remote {
                                    hyper_tungstenite::tungstenite::Message::Text(m) => {
                                        match serde_json::from_str::<WsC2S>(
                                            &m,
                                        ).context("Error parsing message json")? {
                                            WsC2S::Prepare(prepare) => {
                                                let (main_ready_tx, main_ready_rx) = oneshot::channel();
                                                let Some(main) = state.link_main.lock().unwrap().clone() else {
                                                    continue;
                                                };
                                                *main.ready.lock().unwrap() = Some(main_ready_tx);
                                                let mut link_readies = vec![];
                                                let links =
                                                    state
                                                        .link_links
                                                        .lock()
                                                        .unwrap()
                                                        .values()
                                                        .cloned()
                                                        .collect::<Vec<_>>();
                                                for link in &links {
                                                    let link = link.clone();
                                                    let (ready_tx, ready_rx) = oneshot::channel();
                                                    *link.ready.lock().unwrap() = Some(ready_tx);
                                                    _ = link.send.send(WsS2L::Prepare(prepare.clone())).await;
                                                    link_readies.push(ready_rx);
                                                }
                                                *state.link_bg.lock().unwrap() = Some(spawn_scoped(async move {
                                                    let mut delays = vec![];
                                                    if let Ok(delay) = main_ready_rx.await {
                                                        delays.push(delay);
                                                    }
                                                    for ready in link_readies {
                                                        if let Ok(delay) = ready.await {
                                                            delays.push(delay);
                                                        }
                                                    }
                                                    delays.sort();
                                                    let delay = delays.last().unwrap();
                                                    let start_at = Utc::now() + *delay * 5;
                                                    _ = main.send.send(WsS2C::Play(start_at)).await;
                                                    for link in links {
                                                        _ = link.send.send(WsS2L::Play(start_at)).await;
                                                    }
                                                }));
                                            },
                                            WsC2S::Ready(sent_at) => {
                                                bb!{
                                                    let Some(main) = state.link_main.lock().unwrap().clone() else {
                                                        break;
                                                    };
                                                    let Some(ready) = main.ready.lock().unwrap().take() else {
                                                        break;
                                                    };
                                                    _ = ready.send(Utc::now() - sent_at);
                                                }
                                            },
                                            WsC2S::Pause => {
                                                let links =
                                                    state
                                                        .link_links
                                                        .lock()
                                                        .unwrap()
                                                        .values()
                                                        .cloned()
                                                        .collect::<Vec<_>>();
                                                for link in links {
                                                    _ = link.send.send(WsS2L::Pause);
                                                }
                                            },
                                        }
                                    },
                                    _ => {
                                        state
                                            .log
                                            .log_with(
                                                Flag::Debug,
                                                "Received unhandled websocket message type",
                                                ea!(message = from_remote.dbg_str()),
                                            );
                                    },
                                }
                            }
                        }
                    }

                    return Ok(true);
                }.await {
                    Ok(live) => {
                        if !live {
                            break;
                        }
                    },
                    Err(e) => {
                        state.log.log_err(Flag::Debug, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(Flag::Debug, e.context("Error in websocket connection"));
            },
        }
        *state.link_main.lock().unwrap() = None;
        *state.link_session.lock().unwrap() = None;
    });
}

fn handle_ws_link(state: Arc<State>, websocket: HyperWebsocket) {
    let id = state.link_ids.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = mpsc::channel(10);
    state.link_links.lock().unwrap().insert(id, Arc::new(WsState {
        send: tx,
        ready: Mutex::new(None),
    }));
    tokio::spawn(async move {
        match async {
            ta_res!(());
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_res!(bool);

                    select!{
                        m = rx.recv() => {
                            let Some(to_remote) = m else {
                                return Ok(false);
                            };
                            websocket
                                .send(
                                    hyper_tungstenite::tungstenite::Message::text(
                                        serde_json::to_string(&to_remote).unwrap(),
                                    ),
                                )
                                .await?;
                        },
                        m = websocket.next() => {
                            let Some(from_remote) = m else {
                                return Ok(false);
                            };
                            let from_remote = from_remote?;
                            match &from_remote {
                                hyper_tungstenite::tungstenite::Message::Text(m) => {
                                    match serde_json::from_str::<WsL2S>(
                                        &m,
                                    ).context("Error parsing message json")? {
                                        WsL2S::Ready(sent_at) => {
                                            bb!{
                                                let links = state.link_links.lock().unwrap();
                                                let Some(link) = links.get(&id) else {
                                                    break;
                                                };
                                                let Some(ready) = link.ready.lock().unwrap().take() else {
                                                    break;
                                                };
                                                _ = ready.send(Utc::now() - sent_at);
                                            }
                                        },
                                    }
                                },
                                _ => {
                                    state
                                        .log
                                        .log_with(
                                            Flag::Debug,
                                            "Received unhandled websocket message type",
                                            ea!(message = from_remote.dbg_str()),
                                        );
                                },
                            }
                        }
                    }

                    return Ok(true);
                }.await {
                    Ok(live) => {
                        if !live {
                            break;
                        }
                    },
                    Err(e) => {
                        state.log.log_err(Flag::Debug, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(Flag::Debug, e.context("Error in websocket connection"));
            },
        }
        state.link_links.lock().unwrap().remove(&id);
    });
}

fn handle_ws(
    state: Arc<State>,
    head: http::request::Parts,
    upgrade:
        Result<
            (hyper::Response<Full<Bytes>>, HyperWebsocket),
            hyper_tungstenite::tungstenite::error::ProtocolError,
        >,
    handle: fn(Arc<State>, HyperWebsocket) -> (),
) -> Response<BoxBody<Bytes, std::io::Error>> {
    let (response, websocket) = match upgrade {
        Ok(x) => x,
        Err(e) => {
            state.log.log_err(Flag::Warn, e.context_with("Error serving response", ea!(url = head.uri)));
            return Response::builder()
                .status(503)
                .body(http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed())
                .unwrap();
        },
    };
    handle(state, websocket);
    let (resp_header, resp_body) = response.into_parts();
    return Response::from_parts(resp_header, resp_body.map_err(|_| std::io::Error::other("")).boxed());
}

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
    match async {
        ta_res!(Response < BoxBody < Bytes, std:: io:: Error >>);
        let mut path_iter = head.uri.path().trim_matches('/').split('/');
        let mut path_first = path_iter.next().unwrap();
        if path_first == "" {
            path_first = "static";
        }
        match (head.method.clone(), path_first) {
            (Method::GET, "static") => {
                #[derive(RustEmbed)]
                #[folder= "$CARGO_MANIFEST_DIR/../../stage/static"]
                struct Static;

                let mut path = path_iter.collect::<Vec<&str>>();
                let mut f = Static::get(&path.join("/"));
                if f.is_none() {
                    path.push("index.html");
                    f = Static::get(&path.join("/"));
                }
                match f {
                    Some(f) => {
                        return Ok(
                            Response::builder()
                                .status(200)
                                .header("Content-type", f.metadata.mimetype())
                                .header("Cross-Origin-Embedder-Policy", "require-corp")
                                .header("Cross-Origin-Opener-Policy", "same-origin")
                                .body(body_full(f.data.to_vec()))
                                .unwrap(),
                        );
                    },
                    None => {
                        return Ok(response_404());
                    },
                }
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
                                create_dir_all(&parent)
                                    .await
                                    .stack_context(&state.log, "Failed to create upload staging dirs")?;
                            }
                            let f =
                                File::create(&path)
                                    .await
                                    .stack_context(&state.log, "Failed to create upload staged file")?;
                            f
                                .set_len(info.size)
                                .await
                                .stack_context(&state.log, "Error preallocating disk space for upload")?;
                        }
                        let ver_now = ValidityTs(Reverse(Utc::now().timestamp_micros()));
                        let mut triple_rows = vec![];
                        for (i, t) in c.remove.iter().enumerate() {
                            let log = Log::new().fork(ea!(section = "remove", triple = i));
                            let subj_log =
                                log.fork(ea!(subject = serde_json::to_string_pretty(&t.subject).unwrap()));
                            let obj_log = log.fork(ea!(object = serde_json::to_string_pretty(&t.object).unwrap()));
                            node_to_meta_row(
                                &mut meta_rows,
                                &t.subject,
                            ).stack_context(&subj_log, "Error extracting metadata")?;
                            node_to_meta_row(
                                &mut meta_rows,
                                &t.object,
                            ).stack_context(&obj_log, "Error extracting metadata")?;
                            triple_rows.push(
                                vec![
                                    node_to_row(
                                        &t.subject,
                                    ).stack_context(&subj_log, "Unable to convert for db insert")?,
                                    DataValue::Str(t.predicate.as_str().into()),
                                    node_to_row(
                                        &t.object,
                                    ).stack_context(&obj_log, "Unable to convert for db insert")?,
                                    DataValue::Validity(Validity {
                                        timestamp: ver_now,
                                        is_assert: Reverse(false),
                                    })
                                ],
                            );
                        }
                        for (i, t) in c.add.iter().enumerate() {
                            let log = Log::new().fork(ea!(section = "add", triple = i));
                            let subj_log =
                                log.fork(ea!(subject = serde_json::to_string_pretty(&t.subject).unwrap()));
                            let obj_log = log.fork(ea!(object = serde_json::to_string_pretty(&t.object).unwrap()));
                            node_to_meta_row(
                                &mut meta_rows,
                                &t.subject,
                            ).stack_context(&subj_log, "Error extracting metadata")?;
                            node_to_meta_row(
                                &mut meta_rows,
                                &t.object,
                            ).stack_context(&obj_log, "Error extracting metadata")?;
                            triple_rows.push(
                                vec![
                                    node_to_row(
                                        &t.subject,
                                    ).stack_context(&subj_log, "Unable to convert for db insert")?,
                                    DataValue::Str(t.predicate.as_str().into()),
                                    node_to_row(
                                        &t.object,
                                    ).stack_context(&obj_log, "Unable to convert for db insert")?,
                                    DataValue::Validity(Validity {
                                        timestamp: ver_now,
                                        is_assert: Reverse(true),
                                    })
                                ],
                            );
                        }
                        let mut params = BTreeMap::new();
                        params.insert("triple".to_string(), NamedRows {
                            headers: vec![
                                "subject".to_string(),
                                "predicate".to_string(),
                                "object".to_string(),
                                "ver".to_string()
                            ],
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
                    },
                    C2SReq::UploadFinish(hash) => {
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
                                                            return Poll::Ready(
                                                                self.as_mut().hash.write_all(buf).map(|_| buf.len()),
                                                            );
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
                                                    "video/x-matroska" | "video/mp4" => { },
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
                                                    let subtitle_dest =
                                                        generated_path(
                                                            &state.generated_dir,
                                                            &hash,
                                                            &format!("webvtt_{}", lang),
                                                        )?;
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
                                                                ea!(
                                                                    track = stream.index,
                                                                    output = extract_res.pretty_dbg_str()
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                }

                                                // Webm
                                                let webm_tmp = tempdir()?;
                                                let webm_dest = generated_path(&state.generated_dir, &hash, "webm")?;
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
                                                        e.context_with(
                                                            "Error finishing upload",
                                                            ea!(hash = hash.to_string()),
                                                        ),
                                                    );
                                            },
                                        }
                                        state.finishing_uploads.lock().unwrap().remove(&hash);
                                    }
                                });
                            }
                        }
                        return Ok(response_200_json(UploadFinishResp { done: done }));
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
                        let res = spawn_blocking(cap_fn!(()(state) {
                            state.db.run_script_read_only(&"{?[id, def] := *view{id: id, def: def}}", BTreeMap::new())
                        })).await?.await.map_err(|e| loga::err(e.to_string()))?;
                        let mut out = HashMap::new();
                        for row in res.rows {
                            out.insert(
                                row.get(0).unwrap().get_str().unwrap().to_string(),
                                serde_json::from_str::<ViewPartList>(
                                    row.get(1).unwrap().get_str().unwrap(),
                                ).unwrap(),
                            );
                        }
                        return Ok(response_200_json(out));
                    },
                    C2SReq::ViewEnsure(args) => {
                        let mut params = BTreeMap::new();
                        params.insert("view".to_string(), NamedRows {
                            headers: vec!["id".to_string(), "def".to_string()],
                            rows: vec![
                                vec![
                                    DataValue::Str(args.id.as_str().into()),
                                    DataValue::Str(serde_json::to_string(&args.def).unwrap().as_str().into())
                                ]
                            ],
                            next: None,
                        });
                        spawn_blocking(cap_fn!(()(state) {
                            state.db.import_relations(params)
                        })).await?.await.map_err(|e| loga::err(e.to_string()).context("Error running query"))?;
                        return Ok(response_200());
                    },
                    C2SReq::ViewDelete(id) => {
                        spawn_blocking(cap_fn!(()(state) {
                            state.db.run_script_read_only(&"{?[id] <- [[$id]] :rm view {id}}", {
                                let mut m = BTreeMap::new();
                                m.insert("id".to_string(), DataValue::Str(id.as_str().into()));
                                m
                            })
                        })).await?.await.map_err(|e| loga::err(e.to_string()))?;
                        return Ok(response_200());
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
                    },
                    Method::GET => {
                        if let Some(resp) = check_file_auth(&state, &head, &file) {
                            return Ok(resp);
                        }
                        let query =
                            serde_urlencoded::from_str::<FileUrlQuery>(
                                head.uri.query().unwrap_or_default(),
                            ).context("Error parsing query string")?;
                        let mimetype;
                        let local_path;
                        if let Some(generated) = query.generated {
                            mimetype = generated.mime;
                            local_path = generated_path(&state.generated_dir, &file, &generated.name)?;
                        } else {
                            let Some(mimetype1) = get_mimetype(&state, &file).await ? else {
                                return Ok(response_404());
                            };
                            mimetype = mimetype1;
                            local_path = file_path(&state.files_dir, &file)?;
                        }
                        let meta1 = local_path.metadata()?;
                        let mut file =
                            File::open(&local_path)
                                .await
                                .stack_context_with(
                                    &state.log,
                                    "Error opening stored file to read",
                                    ea!(path = local_path.to_string_lossy()),
                                )?;
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
                                        .header(
                                            "Content-Range",
                                            format!("bytes {}-{}/{}", start, end - 1, meta1.len()),
                                        )
                                        .header("Content-Length", end - start)
                                        .body(
                                            http_body_util::StreamBody::new(
                                                ReaderStream::new(
                                                    file.take((end - start) as u64),
                                                ).map_ok(Frame::data),
                                            ).boxed(),
                                        )
                                        .unwrap(),
                                );
                            } else {
                                let boundary = "3d6b6a416f9b5";
                                let mut content_len = 0;
                                let mut ranges2 = vec![];
                                for (i, (start, end)) in ranges.into_iter().enumerate() {
                                    let subheader =
                                        format!(
                                            "{}--{}\nContent-Type: {}\nContent-Range: bytes {}-{}/{}\n\n",
                                            if i == 0 {
                                                ""
                                            } else {
                                                "\r\n"
                                            },
                                            boundary,
                                            mimetype,
                                            start,
                                            end - 1,
                                            meta1.len()
                                        ).into_bytes();
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
                                    .body(
                                        http_body_util::StreamBody::new(
                                            ReaderStream::new(file).map_ok(Frame::data),
                                        ).boxed(),
                                    )
                                    .unwrap(),
                            );
                        }
                    },
                    Method::POST => {
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
                        file
                            .seek(io::SeekFrom::Start(offset))
                            .await
                            .stack_context(&state.log, "Error seeking to upload part start")?;

                        // TODO bg process to write chunks, wait on finish until all written
                        let chunk = body.collect().await.stack_context(&state.log, "Error reading chunk")?.to_bytes();
                        file.write_all(&chunk).await.stack_context(&state.log, "Error writing chunk")?;
                        file.flush().await?;
                        return Ok(response_200());
                    },
                    _ => return Ok(response_404()),
                }
            },
            _ => return Ok(response_404()),
        }
    }.await {
        Ok(r) => r,
        Err(e) => {
            state.log.log_err(Flag::Warn, e.context_with("Error serving response", ea!(url = head.uri)));
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
            },
            1 => { },
            i => panic!("Unknown db schema version: {}", i),
        };
        let tm = taskmanager::TaskManager::new();

        // GC
        tm.periodic(
            "Garbage collection",
            Duration::hours(24).to_std().unwrap(),
            cap_fn!(()(log, dbc, files_dir, generated_dir) {
                let log = log.fork(ea!(sys = "gc"));
                match async {
                    ta_res!(());

                    // Clean up old triples
                    spawn_blocking({
                        let dbc = dbc.clone();
                        move || {
                            ta_res!(());
                            dbc
                                .run_script(include_str!("gc.cozo"), {
                                    let mut m = BTreeMap::new();
                                    m.insert(
                                        "cutoff".to_string(),
                                        DataValue::Num(Num::Int(Utc::now().timestamp_micros())),
                                    );
                                    m
                                }, cozo::ScriptMutability::Mutable)
                                .map_err(|e| loga::err(e.dbg_str()).context("Error running gc query"))?;
                            return Ok(());
                        }
                    }).await??;

                    // Clean up unreferenced files
                    async fn flush(
                        log: &Log,
                        dbc: &Db<SqliteStorage>,
                        batch: &mut HashMap<FileHash, PathBuf>,
                    ) -> Result<(), loga::Error> {
                        let db_files =
                            DataValue::List(
                                batch
                                    .keys()
                                    .map(|k| DataValue::List(vec![DataValue::Str(k.to_string().into())]))
                                    .collect::<Vec<_>>(),
                            );
                        let found = spawn_blocking({
                            let dbc = dbc.clone();
                            move || {
                                ta_res!(Vec < FileHash >);
                                let res =
                                    dbc
                                        .run_script(include_str!("gc_file_referenced.cozo"), {
                                            let mut m = BTreeMap::new();
                                            m.insert("files".to_string(), db_files);
                                            m
                                        }, cozo::ScriptMutability::Immutable)
                                        .map_err(
                                            |e| loga::err(e.dbg_str()).context("Error running file gc query"),
                                        )?;
                                let mut out = vec![];
                                for r in res.rows {
                                    let Some(DataValue::Str(hash)) = r.get(0) else {
                                        panic!("{:?}", r);
                                    };
                                    out.push(FileHash::from_str(hash.as_str()).unwrap());
                                }
                                return Ok(out);
                            }
                        }).await??;
                        for hash in found {
                            batch.remove(&hash);
                        }
                        for path in batch.values() {
                            match remove_file(path).await {
                                Ok(_) => { },
                                Err(e) => {
                                    log.log_err(
                                        Flag::Warn,
                                        e.context_with(
                                            "Failed to delete unreferenced file",
                                            ea!(path = path.display().to_string()),
                                        ),
                                    );
                                },
                            };
                        }
                        batch.clear();
                        return Ok(());
                    }

                    fn get_file_hash(log: &Log, root: &Path, path: &Path) -> Option<FileHash> {
                        let components = path.strip_prefix(root).unwrap().components().filter_map(|c| match c {
                            Component::Normal(c) => Some(c),
                            _ => None,
                        }).collect::<Vec<_>>();
                        let Some(hash_type) = components.first().and_then(|c| c.to_str()) else {
                            log.log(Flag::Warn, "File in files dir not in hash type directory");
                            return None;
                        };
                        let Some(hash_hash) = components.last().and_then(|c| c.to_str()) else {
                            log.log(Flag::Warn, "File in files dir has non-utf8 last path segment");
                            return None;
                        };
                        let hash = match FileHash::from_str(&format!("{}:{}", hash_type, hash_hash)) {
                            Ok(h) => h,
                            Err(e) => {
                                log.log_err(Flag::Warn, loga::err(e).context("Failed to determine hash for file"));
                                return None;
                            },
                        };
                        return Some(hash);
                    }

                    let mut walk = WalkDir::new(&files_dir);
                    let mut batch = HashMap::new();
                    while let Some(entry) = walk.next().await {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(e) => {
                                log.log_err(Flag::Warn, e.context("Unable to scan file in files_dir"));
                                continue;
                            },
                        };
                        let path = entry.path();
                        let log = log.fork(ea!(path = path.to_string_lossy()));
                        if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
                            continue;
                        }
                        let Some(hash) = get_file_hash(&log, &files_dir, &path) else {
                            continue;
                        };
                        batch.insert(hash.clone(), path);
                        if batch.len() >= 1000 {
                            flush(&log, &dbc, &mut batch).await?;
                        }
                    }
                    if !batch.is_empty() {
                        flush(&log, &dbc, &mut batch).await?;
                    }

                    // Clean up unreferenced generated files
                    let mut walk = WalkDir::new(&generated_dir);
                    while let Some(entry) = walk.next().await {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(e) => {
                                log.log_err(Flag::Warn, e.context("Unable to scan file in files_dir"));
                                continue;
                            },
                        };
                        let path = entry.path();
                        let log = log.fork(ea!(path = path.to_string_lossy()));
                        if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
                            continue;
                        }
                        let Some(hash) = get_file_hash(&log, &generated_dir, &path) else {
                            continue;
                        };
                        if !file_path(&files_dir, &hash).unwrap().exists() {
                            match remove_file(&path).await {
                                Ok(_) => { },
                                Err(e) => {
                                    log.log_err(
                                        Flag::Warn,
                                        e.context_with(
                                            "Failed to delete unreferenced generated file",
                                            ea!(path = path.display().to_string()),
                                        ),
                                    );
                                },
                            };
                        }
                    }

                    // Don
                    return Ok(());
                }.await {
                    Ok(_) => { },
                    Err(e) => {
                        log.log_err(Flag::Warn, e.context("Error performing garbage collection"));
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
