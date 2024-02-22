use std::{
    cmp::Reverse,
    collections::{
        BTreeMap,
        HashMap,
    },
    io::Write,
    net::SocketAddr,
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
    sync::Arc,
    task::Poll,
};
use aargvark::{
    vark,
    Aargvark,
};
use chrono::Utc;
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
use futures::TryStreamExt;
use http_body::Frame;
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
    ErrContext,
    ResultContext,
};
use serde_json::Number;
use native::{
    cap_fn,
    ta_res,
    util::{
        Flag,
        Log,
    },
};
use shared::{
    model::{
        view::ViewPartList,
        C2SReq,
        CommitResp,
        FileHash,
        Node,
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
use tokio::{
    fs::{
        create_dir_all,
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
    task::spawn_blocking,
};
use rust_embed::RustEmbed;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::io::ReaderStream;

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub debug: bool,
    pub persistent_dir: PathBuf,
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

struct State {
    log: Log,
    db: Db<SqliteStorage>,
    files_dir: PathBuf,
    stage_dir: PathBuf,
}

async fn handle_req(state: Arc<State>, req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    let (head, body) = req.into_parts();
    match async {
        ta_res!(Response < BoxBody < Bytes, std:: io:: Error >>);
        let mut path_iter = head.uri.path().trim_matches('/').split('/');
        let mut path_first = path_iter.next().unwrap();
        if path_first == "" {
            path_first = "static";
        }
        match (head.method, path_first) {
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
                        let source = staged_file_path(&state.stage_dir, &hash)?;
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
                        let dest = file_path(&state.files_dir, &hash)?;
                        if let Some(p) = dest.parent() {
                            create_dir_all(&p)
                                .await
                                .context("Failed to create parent directories for uploaded file")?;
                        }
                        rename(&source, &dest).await.context("Failed to place uploaded file")?;
                        return Ok(response_200());
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
                        let Some(meta0) = spawn_blocking(
                            cap_fn!(()(state) {
                                state.db.run_script("{?[mimetype] := *meta{node:$node, mimetype:mimetype}}", {
                                    let mut m = BTreeMap::new();
                                    m.insert(
                                        "node".to_string(),
                                        DataValue::List(
                                            vec![
                                                DataValue::Str("file".into()),
                                                DataValue::Str(file.to_string().into())
                                            ],
                                        ),
                                    );
                                    m
                                }, cozo::ScriptMutability::Immutable)
                            })
                        ).await ?.await.map_err(
                            |e| loga::err(e.to_string()).context("Error looking up metadata")
                        ) ?.rows.into_iter().next() else {
                            return Ok(response_404());
                        };
                        let mut meta0 = meta0.into_iter();
                        let mimetype = unenum!(meta0.next().unwrap(), DataValue:: Str(s) => s).unwrap();
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
                        let Some(meta0) = spawn_blocking(
                            cap_fn!(()(state, file) {
                                state.db.run_script("{?[mimetype] := *meta{node:$node, mimetype:mimetype}}", {
                                    let mut m = BTreeMap::new();
                                    m.insert(
                                        "node".to_string(),
                                        DataValue::List(
                                            vec![
                                                DataValue::Str("file".into()),
                                                DataValue::Str(file.to_string().into())
                                            ],
                                        ),
                                    );
                                    m
                                }, cozo::ScriptMutability::Immutable)
                            })
                        ).await ?.await.map_err(
                            |e| loga::err(e.to_string()).context("Error looking up metadata")
                        ) ?.rows.into_iter().next() else {
                            return Ok(response_404());
                        };
                        let mut meta0 = meta0.into_iter();
                        let mimetype = unenum!(meta0.next().unwrap(), DataValue:: Str(s) => s).unwrap();
                        let file_path = file_path(&state.files_dir, &file)?;
                        let meta1 = file_path.metadata()?;
                        let mut file =
                            File::open(&file_path)
                                .await
                                .stack_context_with(
                                    &state.log,
                                    "Error opening stored file to read",
                                    ea!(path = file_path.to_string_lossy()),
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
                            |e| loga::err_with("Error running migration", ea!(err = e, version = 0, script = script)),
                        )?;
                }
            },
            1 => { },
            i => panic!("Unknown db schema version: {}", i),
        };
        let tm = taskmanager::TaskManager::new();

        // Client<->server
        tm.critical_stream(
            "Server",
            TcpListenerStream::new(
                TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
            ),
            {
                let state = Arc::new(State {
                    db: dbc.clone(),
                    log: log.clone(),
                    files_dir: files_dir,
                    stage_dir: stage_dir,
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
