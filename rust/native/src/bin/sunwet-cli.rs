use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env::{
        self,
        current_dir,
    },
    io::SeekFrom,
    os::unix::fs::MetadataExt,
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
};
use aargvark::{
    vark,
};
use chrono::Duration;
use http::header::AUTHORIZATION;
use http_body_util::Full;
use hyper::{
    body::Bytes,
    Method,
    Request,
    Uri,
};
use loga::{
    ea,
    fatal,
    DebugDisplay,
    ResultContext,
};
use mime_guess::{
    Mime,
    MimeGuess,
};
use serde::Deserialize;
use shared::{
    bb,
    model::{
        self,
        cli::{
            CliCommit,
            CliNode,
            CliTriple,
        },
        view::ViewPartList,
        C2SReq,
        Commit,
        CommitFile,
        CommitResp,
        FileHash,
        Node,
        Query,
        Triple,
        UploadFinishResp,
        HEADER_OFFSET,
    },
};
use native::{
    htreq::{
        self,
        new_conn,
    },
    util::{
        hash_file_sha256,
        Flag,
        Log,
    },
};
use tokio::{
    fs::{
        write,
        File,
    },
    io::{
        AsyncReadExt,
        AsyncSeekExt,
        AsyncWriteExt,
    },
};

pub mod args {
    use std::path::PathBuf;
    use aargvark::{
        Aargvark,
        AargvarkFile,
        AargvarkFromStr,
        AargvarkJson,
        AargvarkYaml,
    };
    use serde::{
        de::DeserializeOwned,
    };
    use shared::model::{
        cli::CliCommit,
        View as ViewDef,
    };

    pub struct JsonKv {
        pub key: String,
        pub value: serde_json::Value,
    }

    impl AargvarkFromStr for JsonKv {
        fn from_str(s: &str) -> Result<Self, String> {
            let Some((k, v)) = s.split_once("=") else {
                return Err(
                    "Parameters must be in the form K=V where K is an unquoted string corresponding to a query variable and V is a JSON value (i.e. if a string, quoted - you may need double quotes due to shell parsing)".to_string(),
                );
            };
            let v = match serde_json::from_str(v) {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!("Error parsing value as JSON: {}", e));
                },
            };
            return Ok(JsonKv {
                key: k.to_string(),
                value: v,
            });
        }

        fn build_help_pattern(_state: &mut aargvark::HelpState) -> aargvark::HelpPattern {
            return aargvark::HelpPattern(vec![aargvark::HelpPatternElement::Type("KEY=JSON-VALUE".to_string())]);
        }
    }

    #[derive(Aargvark)]
    pub struct Query {
        /// File containing cozo datalog query
        pub query: AargvarkFile,
        /// Parameters provided to datalog query (as in parameterized SQL queries)
        pub params: Vec<JsonKv>,
    }

    #[derive(Aargvark)]
    pub enum JsonOrYaml<T: 'static + DeserializeOwned> {
        Json(AargvarkJson<T>),
        Yaml(AargvarkYaml<T>),
    }

    #[derive(Aargvark)]
    pub struct Export {
        /// Where to write exported triples and files.
        pub out_dir: PathBuf,
        /// The query to run to generate triples to export. The query result should have
        /// three field named `subject`, `predicate`, and `object` corresponding to the
        /// triple fields.
        pub query: Query,
    }

    #[derive(Aargvark)]
    pub struct ViewEnsure {
        pub id: String,
        pub definition: JsonOrYaml<ViewDef>,
    }

    #[derive(Aargvark)]
    pub struct ViewDelete {
        pub id: String,
    }

    #[derive(Aargvark)]
    pub enum View {
        List,
        Ensure(ViewEnsure),
        Delete(ViewDelete),
    }

    #[derive(Aargvark)]
    pub enum Command {
        /// Run a query and return the response as json.
        Query(Query),
        /// Upload triples and files to the database.
        Commit(AargvarkJson<CliCommit>),
        /// Download triples and files from the database in a format suitable for
        /// committing again.
        Export(Export),
        /// Download a file by its hash.
        Download(String),
        /// Commands for configuring UI views.
        View(View),
    }

    #[derive(Aargvark)]
    pub struct Args {
        pub server: String,
        pub command: Command,
    }
}

async fn download(
    log: &Log,
    conn: &mut htreq::Conn,
    headers: &HashMap<String, String>,
    server: &Uri,
    out_dir: &Path,
    hash: &FileHash,
) -> Result<PathBuf, loga::Error> {
    let mut req = Request::builder().uri(format!("{}/file/{}", server, hash.to_string())).method(Method::GET);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let (_, headers, continue_recv) =
        htreq::send_recv_head(&log, conn, Duration::seconds(15), req.body(Full::new(Bytes::new())).unwrap())
            .await
            .stack_context(&log, "Error getting file")?;
    let out_path = bb!{
        'named _;
        bb!{
            let Some(content_type) = headers.get("Content-Type") else {
                break;
            };
            let Ok(content_type) = content_type.to_str() else {
                break;
            };
            let Ok(mime) = Mime:: from_str(content_type) else {
                break;
            };
            let Some(suffix) = mime.suffix() else {
                break;
            };
            break 'named out_dir.join(format!("{}.{}", hash.to_string(), suffix));
        }
        break 'named out_dir.join(hash.to_string());
    };
    let mut out =
        File::create(&out_path).await.context_with("Error creating file", ea!(path = out_path.to_string_lossy()))?;
    htreq::recv_body_write(continue_recv, &mut out).await?;
    out.flush().await.context_with("Error flushing data to file", ea!(path = out_path.to_string_lossy()))?;
    return Ok(out_path);
}

#[tokio::main]
async fn main() {
    match async {
        let log = Log::new().with_flags(&[Flag::Warn, Flag::Info]);
        let args = vark::<args::Args>();
        let server =
            Uri::from_str(
                &args.server,
            ).context_with("Couldn't parse specified server as URL", ea!(server = args.server))?;
        let mut headers = HashMap::new();
        if let Some(token) = env::var("SUNWET_TOKEN").ok() {
            headers.insert(AUTHORIZATION.to_string(), format!("Bearer {}", token));
        }
        match args.command {
            args::Command::Query(q) => {
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let res =
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &headers,
                        serde_json::to_vec(&C2SReq::Query(Query {
                            query: String::from_utf8(q.query.value).context("Query file contents isn't valid utf8")?,
                            parameters: q.params.into_iter().map(|kv| (kv.key, kv.value)).collect(),
                        })).unwrap(),
                        128 * 1024 * 1024,
                    ).await.stack_context(&log, "Failed to make request")?;
                let res =
                    serde_json::from_slice::<Vec<HashMap<String, serde_json::Value>>>(
                        &res,
                    ).stack_context(&log, "Error parsing response JSON")?;
                println!("{}", serde_json::to_string_pretty(&res).unwrap());
            },
            args::Command::Commit(c) => {
                let log = log.fork(ea!(command = "commit"));
                let mut commit = Commit::default();
                let mut files = HashMap::new();

                async fn process_node(
                    log: &Log,
                    commit: &mut Commit,
                    files: &mut HashMap<PathBuf, (FileHash, u64)>,
                    base_dir: &Path,
                    n: CliNode,
                ) -> Result<Node, loga::Error> {
                    match n {
                        CliNode::Id(v) => return Ok(Node::Id(v)),
                        CliNode::File(v) => return Ok(Node::File(v)),
                        CliNode::Value(v) => return Ok(Node::Value(v)),
                        CliNode::Upload(v) => {
                            let path = base_dir.join(v);
                            match files.entry(path.clone()) {
                                std::collections::hash_map::Entry::Occupied(h) => return Ok(
                                    Node::File(h.get().0.clone()),
                                ),
                                std::collections::hash_map::Entry::Vacant(e) => {
                                    let m =
                                        path
                                            .metadata()
                                            .context_with(
                                                "Unable to read file metadata",
                                                ea!(path = path.to_string_lossy()),
                                            )?;
                                    log.log_with(
                                        Flag::Info,
                                        "Hashing file before upload",
                                        ea!(file = path.to_string_lossy()),
                                    );
                                    let hash = hash_file_sha256(&log, &path).await?;
                                    e.insert((hash.clone(), m.size()));
                                    commit.files.push(CommitFile {
                                        hash: hash.clone(),
                                        size: m.size(),
                                        mimetype: MimeGuess::from_path(&path).first_or_octet_stream().to_string(),
                                    });
                                    return Ok(Node::File(hash));
                                },
                            }
                        },
                    }
                }

                log.log(Flag::Info, "Processing commit");
                let base_dir = match c.source {
                    aargvark::Source::Stdin => current_dir().stack_context(
                        &log,
                        "Error determining current dir for relative path normalization",
                    )?,
                    aargvark::Source::File(p) => p
                        .canonicalize()
                        .stack_context(&log, "Error getting normalized commit path")?
                        .parent()
                        .unwrap()
                        .to_path_buf(),
                };
                for (i, t) in c.value.add.into_iter().enumerate() {
                    let s =
                        process_node(&log, &mut commit, &mut files, &base_dir, t.subject)
                            .await
                            .stack_context(&log, format!("Failed to process subject in add triple {}", i))?;
                    let o =
                        process_node(&log, &mut commit, &mut files, &base_dir, t.object)
                            .await
                            .stack_context(&log, format!("Failed to process object in add triple {}", i))?;
                    commit.add.push(Triple {
                        subject: s,
                        predicate: t.predicate,
                        object: o,
                    });
                }
                for (i, t) in c.value.remove.into_iter().enumerate() {
                    let s =
                        process_node(&log, &mut commit, &mut files, &base_dir, t.subject)
                            .await
                            .stack_context(&log, format!("Failed to process subject in remove triple {}", i))?;
                    let o =
                        process_node(&log, &mut commit, &mut files, &base_dir, t.object)
                            .await
                            .stack_context(&log, format!("Failed to process object in remove triple {}", i))?;
                    commit.remove.push(Triple {
                        subject: s,
                        predicate: t.predicate,
                        object: o,
                    });
                }
                log.log(Flag::Info, "Sending triples");
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let commit_res =
                    htreq::post(
                        &log,
                        &mut conn,
                        &format!("{}/api", server),
                        &headers,
                        serde_json::to_vec(&C2SReq::Commit(commit.clone())).unwrap(),
                        1024,
                    )
                        .await
                        .stack_context(&log, "Error posting commit")?;
                let commit_res =
                    serde_json::from_slice::<CommitResp>(
                        &commit_res,
                    ).stack_context(&log, "Unable to parse commit response from server")?;
                let incomplete = commit_res.incomplete.into_iter().collect::<HashSet<_>>();
                for (p, (hash, size)) in files {
                    if !incomplete.contains(&hash) {
                        continue;
                    }
                    let log = log.fork(ea!(state = "upload", file = p.to_string_lossy()));
                    log.log(Flag::Info, "Uploading file");
                    const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
                    let chunks = size.div_ceil(CHUNK_SIZE);
                    let mut f = File::open(&p).await.stack_context(&log, "Failed to open file for upload")?;
                    for i in 0 .. chunks {
                        f
                            .seek(SeekFrom::Start(i * CHUNK_SIZE))
                            .await
                            .stack_context(&log, "Failed to seek to next chunk")?;
                        let mut chunk = vec![];
                        let chunk_start = i * CHUNK_SIZE;
                        let chunk_size = (size - chunk_start).min(CHUNK_SIZE);
                        chunk.resize(chunk_size as usize, 0);
                        f.read_exact(&mut chunk).await.stack_context(&log, "Error reading chunk from source file")?;
                        htreq::post(&log, &mut conn, format!("{}/file/{}", &server, hash.to_string()), &{
                            let mut headers = headers.clone();
                            headers.insert(HEADER_OFFSET.to_string(), chunk_start.to_string());
                            headers
                        }, chunk, 1024)
                            .await
                            .stack_context_with(
                                &log,
                                "Error uploading chunk",
                                ea!(
                                    chunk = format!("{}/{}", i + 1, chunks),
                                    range = format!("{}..{}B", i * CHUNK_SIZE, chunk_start + chunk_size)
                                ),
                            )?;
                    }
                    log.log(Flag::Info, "Verifying upload");
                    loop {
                        let resp_bytes =
                            htreq::post(
                                &log,
                                &mut conn,
                                format!("{}/api", server),
                                &headers,
                                serde_json::to_vec(&C2SReq::UploadFinish(hash.clone())).unwrap(),
                                1024,
                            )
                                .await
                                .stack_context(&log, "Failed to verify upload")?;
                        let resp =
                            serde_json::from_slice::<UploadFinishResp>(
                                &resp_bytes,
                            ).context("Error parsing response from server")?;
                        if resp.done {
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            },
            args::Command::Download(hash) => {
                let hash = FileHash::from_str(&hash).map_err(|e| loga::err(e))?;
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let path =
                    download(
                        &log,
                        &mut conn,
                        &headers,
                        &server,
                        &current_dir().context("Couldn't determine current directory")?,
                        &hash,
                    ).await?;
                println!("{}", path.to_str().unwrap());
            },
            args::Command::Export(export) => {
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let res =
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &headers,
                        serde_json::to_vec(&C2SReq::Query(Query {
                            query: String::from_utf8(
                                export.query.query.value,
                            ).context("Query file contents isn't valid utf8")?,
                            parameters: export.query.params.into_iter().map(|kv| (kv.key, kv.value)).collect(),
                        })).unwrap(),
                        128 * 1024 * 1024,
                    ).await.stack_context(&log, "Failed to make request")?;

                #[derive(Deserialize)]
                struct RespTriple {
                    subject: (String, serde_json::Value),
                    predicate: String,
                    object: (String, serde_json::Value),
                }

                let res =
                    serde_json::from_slice::<Vec<RespTriple>>(
                        &res,
                    ).stack_context(&log, "Error parsing response JSON")?;
                let mut triples = vec![];
                for row in res {
                    async fn process_node(
                        log: &Log,
                        conn: &mut htreq::Conn,
                        headers: &HashMap<String, String>,
                        server: &Uri,
                        out_dir: &Path,
                        node_in: (String, serde_json::Value),
                    ) -> Result<CliNode, loga::Error> {
                        match node_in.0.as_str() {
                            "id" => {
                                let serde_json:: Value:: String(v) = node_in.1 else {
                                    return Err(
                                        loga::err_with(
                                            "Id nodes must have a string value, but got another json type",
                                            ea!(value = node_in.1.dbg_str()),
                                        ),
                                    );
                                };
                                return Ok(CliNode::Id(v));
                            },
                            "value" => {
                                return Ok(CliNode::Value(node_in.1));
                            },
                            "file" => {
                                let serde_json:: Value:: String(hash_raw) = node_in.1 else {
                                    return Err(
                                        loga::err_with(
                                            "File nodes must have a hash string value, but got another json type",
                                            ea!(value = node_in.1.dbg_str()),
                                        ),
                                    );
                                };
                                let hash =
                                    FileHash::from_str(
                                        &hash_raw,
                                    ).map_err(
                                        |e| loga::err(
                                            e.to_string(),
                                        ).context_with("Failed to parse hash for node", ea!(hash = hash_raw)),
                                    )?;
                                let path = download(&log, conn, headers, &server, out_dir, &hash).await?;
                                return Ok(CliNode::Upload(path));
                            },
                            _ => {
                                return Err(loga::err_with("Unexpected node type", ea!(type_ = node_in.0)));
                            },
                        }
                    }

                    triples.push(CliTriple {
                        subject: process_node(&log, &mut conn, &headers, &server, &export.out_dir, row.subject).await?,
                        predicate: row.predicate,
                        object: process_node(&log, &mut conn, &headers, &server, &export.out_dir, row.object).await?,
                    });
                }
                let commit_path = export.out_dir.join("sunwet.json");
                write(&commit_path, &serde_json::to_vec_pretty(&CliCommit {
                    add: triples,
                    remove: vec![],
                }).unwrap())
                    .await
                    .context_with("Error writing commit file", ea!(path = commit_path.to_string_lossy()))?;
            },
            args::Command::View(c) => match c {
                args::View::List => {
                    let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                    let res =
                        htreq::post(
                            &log,
                            &mut conn,
                            format!("{}/api", server),
                            &headers,
                            serde_json::to_vec(&C2SReq::ViewsList).unwrap(),
                            128 * 1024 * 1024,
                        )
                            .await
                            .stack_context(&log, "Failed to make request")?;
                    let res =
                        serde_json::from_slice::<HashMap<String, ViewPartList>>(
                            &res,
                        ).stack_context(&log, "Error parsing response JSON")?;
                    println!("{}", serde_json::to_string_pretty(&res).unwrap());
                },
                args::View::Ensure(args) => {
                    let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &headers,
                        serde_json::to_vec(&C2SReq::ViewEnsure(model::ViewEnsure {
                            id: args.id,
                            def: match args.definition {
                                args::JsonOrYaml::Json(v) => v.value,
                                args::JsonOrYaml::Yaml(v) => v.value,
                            },
                        })).unwrap(),
                        1024,
                    ).await.stack_context(&log, "Failed to make request")?;
                },
                args::View::Delete(args) => {
                    let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &headers,
                        serde_json::to_vec(&C2SReq::ViewDelete(args.id)).unwrap(),
                        1024,
                    )
                        .await
                        .stack_context(&log, "Failed to make request")?;
                },
            },
        }
        return Ok(());
    }.await {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
