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
    htwrap::{
        htreq,
        url::UriJoin,
    },
    loga::{
        ea,
        Log,
        ResultContext,
    },
    mime_guess::MimeGuess,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        iam::IamTargetId,
        triple::{
            FileHash,
            Node,
        },
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
    },
    tokio::{
        fs::File,
        io::{
            AsyncReadExt,
            AsyncSeekExt,
        },
    },
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliNode {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
    Upload(PathBuf),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CliTriple {
    pub subject: CliNode,
    pub predicate: String,
    pub object: CliNode,
    pub iam_target: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CliCommit {
    #[serde(default)]
    pub remove: Vec<CliTriple>,
    #[serde(default)]
    pub add: Vec<CliTriple>,
}

#[derive(Aargvark)]
pub struct ChangeCommand(AargvarkJson<CliCommit>);

pub async fn handle_change(c: ChangeCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(loga::INFO);

    // # Build commit info
    let mut commit = ReqCommit {
        add: vec![],
        remove: vec![],
        files: vec![],
    };
    let mut files = HashMap::new();

    async fn process_node(
        log: &Log,
        commit: &mut ReqCommit,
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
                    std::collections::hash_map::Entry::Occupied(h) => return Ok(Node::File(h.get().0.clone())),
                    std::collections::hash_map::Entry::Vacant(e) => {
                        let m =
                            path
                                .metadata()
                                .context_with("Unable to read file metadata", ea!(path = path.to_string_lossy()))?;
                        log.log_with(loga::INFO, "Hashing file before upload", ea!(file = path.to_string_lossy()));
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

    log.log(loga::INFO, "Processing commit");
    let base_dir = match c.0.source {
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
    for (i, t) in c.0.value.add.into_iter().enumerate() {
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
            iam_target: IamTargetId(t.iam_target),
        });
    }
    for (i, t) in c.0.value.remove.into_iter().enumerate() {
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
            iam_target: IamTargetId(t.iam_target),
        });
    }

    // # Send commit
    log.log(loga::INFO, "Sending triples");
    let url = server_url()?;
    let headers = server_headers()?;
    let mut conn = htreq::connect(&url).await.stack_context(&log, "Error connecting to server")?;
    let commit_res =
        req(&log, &mut conn, &headers, &url, commit.clone()).await.stack_context(&log, "Error posting commit")?;

    // # Upload new files
    let incomplete = commit_res.incomplete.into_iter().collect::<HashSet<_>>();
    for (p, (hash, size)) in files {
        if !incomplete.contains(&hash) {
            continue;
        }
        let log = log.fork(ea!(state = "upload", file = p.to_string_lossy()));
        log.log(loga::INFO, "Uploading file");
        const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
        let chunks = size.div_ceil(CHUNK_SIZE);
        let mut f = File::open(&p).await.stack_context(&log, "Failed to open file for upload")?;
        for i in 0 .. chunks {
            f.seek(SeekFrom::Start(i * CHUNK_SIZE)).await.stack_context(&log, "Failed to seek to next chunk")?;
            let mut chunk = vec![];
            let chunk_start = i * CHUNK_SIZE;
            let chunk_size = (size - chunk_start).min(CHUNK_SIZE);
            chunk.resize(chunk_size as usize, 0);
            f.read_exact(&mut chunk).await.stack_context(&log, "Error reading chunk from source file")?;
            htreq::post(&log, &mut conn, &url.join(format!("file/{}", hash.to_string())), &{
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
        log.log(loga::INFO, "Verifying upload");
        loop {
            let resp =
                req(&log, &mut conn, &headers, &url, ReqUploadFinish(hash.clone()))
                    .await
                    .stack_context(&log, "Failed to verify upload")?;
            if resp.done {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    return Ok(());
}
