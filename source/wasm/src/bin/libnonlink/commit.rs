use {
    super::{
        api::req_post_json,
        state::state,
    },
    crate::libnonlink::api::file_post_json,
    chrono::{
        Utc,
    },
    gloo::{
        file::Blob,
        timers::future::TimeoutFuture,
    },
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            CommitFile,
            ReqUploadFinish,
        },
    },
    std::collections::HashMap,
    web_sys::File,
};

#[derive(Clone)]
pub enum CommitNode {
    Node(Node),
    File(usize, File),
    DatetimeNow,
}

impl PartialEq<CommitNode> for CommitNode {
    fn eq(&self, other: &CommitNode) -> bool {
        match (self, other) {
            (Self::Node(l0), Self::Node(r0)) => l0 == r0,
            _ => false,
        }
    }
}

pub struct CommitTriple {
    pub subject: CommitNode,
    pub predicate: String,
    pub object: CommitNode,
}

pub struct UploadFile {
    data: Vec<u8>,
    hash: FileHash,
    size: u64,
}

pub async fn prep_node(
    return_files: &mut HashMap<usize, FileHash>,
    commit_files: &mut Vec<CommitFile>,
    upload_files: &mut Vec<UploadFile>,
    n: CommitNode,
) -> Option<Node> {
    match n {
        CommitNode::Node(n) => return Some(n),
        CommitNode::File(unique, file) => {
            let b = match gloo::file::futures::read_as_bytes(&Blob::from(file.clone())).await {
                Ok(b) => b,
                Err(e) => {
                    state().log.log(&format!("Error reading file for commit: {}", e));
                    return None;
                },
            };
            let hash = FileHash::from_sha256(Sha256::digest(&b));
            let size = file.size() as u64;
            return_files.insert(unique, hash.clone());
            upload_files.push(UploadFile {
                data: b,
                hash: hash.clone(),
                size: size,
            });
            commit_files.push(CommitFile {
                hash: hash.clone(),
                size: size,
                mimetype: file.type_(),
            });
            return Some(Node::File(hash));
        },
        CommitNode::DatetimeNow => {
            return Some(Node::Value(serde_json::Value::String(Utc::now().to_rfc3339())));
        },
    }
}

pub async fn upload_files(files: Vec<UploadFile>) -> Result<(), String> {
    for file in files {
        const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
        let chunks = file.size.div_ceil(CHUNK_SIZE);
        for i in 0 .. chunks {
            let chunk_start = i * CHUNK_SIZE;
            let chunk_size = (file.size - chunk_start).min(CHUNK_SIZE);
            file_post_json(
                &file.hash,
                chunk_start,
                &file.data[chunk_start as usize .. (chunk_start + chunk_size) as usize],
            ).await;
        }
        loop {
            let resp = req_post_json(ReqUploadFinish(file.hash.clone())).await;
            if resp.done {
                break;
            }
            TimeoutFuture::new(1000).await;
        }
    }
    return Ok(());
}
