use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env::current_dir,
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
    Aargvark,
    AargvarkFile,
    AargvarkFromStr,
    AargvarkJson,
};
use hyper::Uri;
use loga::{
    fatal,
    ResultContext,
    ea,
};
use mime_guess::MimeGuess;
use serde::{
    Deserialize,
    Serialize,
};
use shared::model::{
    C2SReq,
    Commit,
    CommitFile,
    CommitResp,
    FileHash,
    Node,
    Query,
    Triple,
    HEADER_OFFSET,
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
        File,
    },
    io::{
        AsyncReadExt,
        AsyncSeekExt,
    },
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliNode {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
    Upload(PathBuf),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CliTriple {
    pub subject: CliNode,
    pub predicate: String,
    pub object: CliNode,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CliCommit {
    #[serde(default)]
    pub remove: Vec<CliTriple>,
    //. TODO pub force_remove: Vec<CliTriple>,
    #[serde(default)]
    pub add: Vec<CliTriple>,
}

struct JsonKv {
    key: String,
    value: serde_json::Value,
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
struct ArgQuery {
    /// File containing cozo datalog query
    query: AargvarkFile,
    /// Parameters provided to datalog query (as in parameterized SQL queries)
    params: Vec<JsonKv>,
}

#[derive(Aargvark)]
enum Command {
    Query(ArgQuery),
    Commit(AargvarkJson<CliCommit>),
}

#[derive(Aargvark)]
struct Args {
    server: String,
    command: Command,
}

#[tokio::main]
async fn main() {
    match async {
        let log = Log::new().with_flags(&[Flag::Warn, Flag::Info]);
        let args = vark::<Args>();
        let server =
            Uri::from_str(
                &args.server,
            ).context_with("Couldn't parse specified server as URL", ea!(server = args.server))?;
        match args.command {
            Command::Query(q) => {
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let res =
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &HashMap::new(),
                        serde_json::to_vec(&C2SReq::Query(Query {
                            query: String::from_utf8(q.query.value).context("Query file contents isn't valid utf8")?,
                            parameters: q.params.into_iter().map(|kv| (kv.key, kv.value)).collect(),
                        })).unwrap(),
                        128 * 1024 * 1024,
                    ).await.stack_context(&log, "Failed to finish upload")?;
                let res =
                    serde_json::from_slice::<Vec<HashMap<String, serde_json::Value>>>(
                        &res,
                    ).stack_context(&log, "Error parsing response JSON")?;
                println!("{}", serde_json::to_string_pretty(&res).unwrap());
            },
            Command::Commit(c) => {
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
                let mut conn = new_conn(&server).await.stack_context(&log, "Error connecting to server")?;
                let commit_res =
                    htreq::post(
                        &log,
                        &mut conn,
                        &format!("{}/api", server),
                        &HashMap::new(),
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
                            let mut headers = HashMap::new();
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
                    htreq::post(
                        &log,
                        &mut conn,
                        format!("{}/api", server),
                        &HashMap::new(),
                        serde_json::to_vec(&C2SReq::UploadFinish(hash.clone())).unwrap(),
                        1024,
                    )
                        .await
                        .stack_context(&log, "Failed to finish upload")?;
                }
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
