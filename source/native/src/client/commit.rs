use {
    crate::{
        client::req::{
            req,
            server_headers,
            server_url,
        },
        server::filesutil::hash_file_sha256,
    },
    aargvark::{
        traits_impls::AargvarkJson,
        Aargvark,
    },
    http::Uri,
    htwrap::{
        htreq::{
            self,
            Conn,
        },
        url::UriJoin,
    },
    loga::{
        ea,
        Log,
        ResultContext,
    },
    mime_guess::MimeGuess,
    shared::interface::{
        cli::{
            CliCommit,
            CliNode,
        },
        triple::Node,
        wire::{
            CommitFile,
            ReqCommit,
            ReqUploadFinish,
            Triple,
            HEADER_OFFSET,
        },
    },
    std::{
        collections::{
            hash_map::Entry,
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
        sync::Arc,
        time::Duration,
    },
    tokio::{
        fs::File,
        io::{
            AsyncReadExt,
            AsyncSeekExt,
        },
        spawn,
        sync::Semaphore,
        task::JoinHandle,
        time::sleep,
    },
};

#[derive(Aargvark)]
pub struct CommitCommand {
    debug: Option<()>,
    commit: AargvarkJson<CliCommit>,
    /// Message to attach to the commit; if missing, uses generic message
    comment: Option<String>,
}

pub async fn handle_commit(c: CommitCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let base_dir = match c.commit.source {
        aargvark::traits_impls::Source::Stdin => current_dir().stack_context(
            &log,
            "Error determining current dir for relative path normalization",
        )?,
        aargvark::traits_impls::Source::File(p) => p
            .canonicalize()
            .stack_context(&log, "Error getting normalized commit path")?
            .parent()
            .unwrap()
            .to_path_buf(),
    };

    // Hash files
    async fn process_file(
        base_dir: &PathBuf,
        limit: &Arc<Semaphore>,
        log: &Log,
        files: &mut HashMap<PathBuf, JoinHandle<Result<CommitFile, loga::Error>>>,
        node: &CliNode,
    ) {
        let CliNode::Upload(v) = node else {
            return;
        };
        let path = base_dir.join(v);
        match files.entry(path.clone()) {
            Entry::Occupied(_) => { },
            Entry::Vacant(entry) => {
                let sem_token = limit.clone().acquire_owned().await.unwrap();
                entry.insert(spawn({
                    let log = log.clone();
                    async move {
                        let m =
                            path
                                .metadata()
                                .context_with("Unable to read file metadata", ea!(path = path.to_string_lossy()))?;
                        log.log_with(loga::INFO, "Hashing file before upload", ea!(file = path.to_string_lossy()));
                        let out = CommitFile {
                            hash: hash_file_sha256(&log, &path).await?,
                            size: m.size(),
                            mimetype: MimeGuess::from_path(&path).first_or_octet_stream().to_string(),
                        };
                        drop(sem_token);
                        return Ok(out);
                    }
                }));
            },
        }
    }

    let mut files = HashMap::new();
    let limit = Arc::new(Semaphore::new(16));
    for t in &c.commit.value.add {
        process_file(&base_dir, &limit, &log, &mut files, &t.subject).await;
        process_file(&base_dir, &limit, &log, &mut files, &t.object).await;
    }
    for t in &c.commit.value.remove {
        process_file(&base_dir, &limit, &log, &mut files, &t.subject).await;
        process_file(&base_dir, &limit, &log, &mut files, &t.object).await;
    }
    let mut errors = vec![];
    let mut files1 = HashMap::new();
    for (k, v) in files {
        match v.await.map_err(|e| e.into()).and_then(|e| e) {
            Ok(v) => {
                files1.insert(k, v);
            },
            Err(e) => {
                errors.push(e);
            },
        }
    }
    if !errors.is_empty() {
        return Err(loga::agg_err("Error(s) while processing commit files", errors));
    }
    let files = files1;

    // # Build commit info
    let mut commit = ReqCommit {
        comment: c.comment.unwrap_or_else(|| format!("Commit via CLI")),
        add: vec![],
        remove: vec![],
        files: vec![],
    };

    async fn process_node(
        commit: &mut ReqCommit,
        files: &HashMap<PathBuf, CommitFile>,
        base_dir: &Path,
        n: CliNode,
    ) -> Result<Node, loga::Error> {
        match n {
            CliNode::File(v) => return Ok(Node::File(v)),
            CliNode::Value(v) => return Ok(Node::Value(v)),
            CliNode::Upload(v) => {
                let path = base_dir.join(v);
                let info = files.get(&path).unwrap();
                commit.files.push(info.clone());
                return Ok(Node::File(info.hash.clone()));
            },
        }
    }

    log.log(loga::INFO, "Processing commit");
    for (i, t) in c.commit.value.add.into_iter().enumerate() {
        let s =
            process_node(&mut commit, &files, &base_dir, t.subject)
                .await
                .stack_context(&log, format!("Failed to process subject in add triple {}", i))?;
        let o =
            process_node(&mut commit, &files, &base_dir, t.object)
                .await
                .stack_context(&log, format!("Failed to process object in add triple {}", i))?;
        commit.add.push(Triple {
            subject: s,
            predicate: t.predicate,
            object: o,
        });
    }
    for (i, t) in c.commit.value.remove.into_iter().enumerate() {
        let s =
            process_node(&mut commit, &files, &base_dir, t.subject)
                .await
                .stack_context(&log, format!("Failed to process subject in remove triple {}", i))?;
        let o =
            process_node(&mut commit, &files, &base_dir, t.object)
                .await
                .stack_context(&log, format!("Failed to process object in remove triple {}", i))?;
        commit.remove.push(Triple {
            subject: s,
            predicate: t.predicate,
            object: o,
        });
    }

    // # Send commit
    async fn reconnect(log: &Log, url: &Uri) -> Conn {
        loop {
            match htreq::connect(&url).await {
                Ok(c) => return c,
                Err(e) => {
                    log.log_err(loga::WARN, e.stack_context(&log, "Error connecting to server"));
                    sleep(Duration::from_secs(1)).await;
                },
            }
        }
    }

    log.log(loga::INFO, "Sending triples");
    let url = server_url()?;
    let headers = server_headers()?;
    let mut conn = reconnect(&log, &url).await;
    let commit_res = loop {
        match req(&log, &mut conn, &headers, &url, commit.clone()).await {
            Ok(r) => break r,
            Err(e) => {
                log.log_err(loga::WARN, e.stack_context(&log, "Error posting commit"));
                sleep(Duration::from_secs(1)).await;
                conn = reconnect(&log, &url).await;
            },
        }
    };

    // # Upload new files
    let incomplete = commit_res.incomplete.into_iter().collect::<HashSet<_>>();
    for (p, info) in files {
        if !incomplete.contains(&info.hash) {
            continue;
        }
        let log = log.fork(ea!(state = "upload", file = p.to_string_lossy()));
        log.log(loga::INFO, "Uploading file");
        const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
        let chunks = info.size.div_ceil(CHUNK_SIZE);
        let mut f = File::open(&p).await.stack_context(&log, "Failed to open file for upload")?;
        for i in 0 .. chunks {
            f.seek(SeekFrom::Start(i * CHUNK_SIZE)).await.stack_context(&log, "Failed to seek to next chunk")?;
            let mut chunk = vec![];
            let chunk_start = i * CHUNK_SIZE;
            let chunk_size = (info.size - chunk_start).min(CHUNK_SIZE);
            chunk.resize(chunk_size as usize, 0);
            f.read_exact(&mut chunk).await.stack_context(&log, "Error reading chunk from source file")?;
            let url = url.join(format!("file/{}", info.hash.to_string()));
            let headers = {
                let mut headers = headers.clone();
                headers.insert(HEADER_OFFSET.to_string(), chunk_start.to_string());
                headers
            };
            loop {
                match htreq::post(&log, &mut conn, &url, &headers, chunk.clone(), 1024).await {
                    Ok(_) => {
                        break;
                    },
                    Err(e) => {
                        log.log_err(
                            loga::WARN,
                            e.stack_context_with(
                                &log,
                                "Error uploading chunk",
                                ea!(
                                    chunk = format!("{}/{}", i + 1, chunks),
                                    range = format!("{}..{}B", i * CHUNK_SIZE, chunk_start + chunk_size)
                                ),
                            ),
                        );
                        sleep(Duration::from_secs(1)).await;
                        conn = reconnect(&log, &url).await;
                    },
                }
            }
        }
        log.log(loga::INFO, "Verifying upload");
        loop {
            match req(&log, &mut conn, &headers, &url, ReqUploadFinish(info.hash.clone())).await {
                Ok(resp) => {
                    if resp.done {
                        break;
                    }
                    sleep(Duration::from_secs(1)).await;
                },
                Err(e) => {
                    log.log_err(loga::WARN, e.stack_context(&log, "Failed to verify upload"));
                    sleep(Duration::from_secs(1)).await;
                    conn = reconnect(&log, &url).await;
                },
            }
        }
    }
    return Ok(());
}
