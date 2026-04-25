use {
    crate::log::Log,
    chrono::Utc,
    gloo::file::Blob,
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::CommitFile,
    },
    std::rc::Rc,
};

#[derive(Clone)]
pub enum CommitNode {
    Node(Node),
    File(usize, Blob),
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
    pub data: Vec<u8>,
    pub hash: FileHash,
}

pub struct PrepNodeResult {
    pub node: Node,
    pub return_file: Option<(usize, FileHash)>,
    pub commit_file: Option<CommitFile>,
    pub upload_file: Option<UploadFile>,
}

pub async fn prep_node(log: &Rc<dyn Log>, n: CommitNode) -> Option<PrepNodeResult> {
    match n {
        CommitNode::Node(n) => Some(PrepNodeResult {
            node: n,
            return_file: None,
            commit_file: None,
            upload_file: None,
        }),
        CommitNode::File(unique, file) => {
            let b = match gloo::file::futures::read_as_bytes(&file).await {
                Ok(b) => b,
                Err(e) => {
                    log.log(&format!("Error reading file for commit: {}", e));
                    return None;
                },
            };
            let hash = FileHash::from_sha256(Sha256::digest(&b));
            let size = file.size();
            let mimetype = file.raw_mime_type();
            Some(PrepNodeResult {
                node: Node::File(hash.clone()),
                return_file: Some((unique, hash.clone())),
                commit_file: Some(CommitFile {
                    hash: hash.clone(),
                    size,
                    mimetype,
                }),
                upload_file: Some(UploadFile {
                    data: b,
                    hash: hash.clone(),
                }),
            })
        },
        CommitNode::DatetimeNow => Some(PrepNodeResult {
            node: Node::Value(serde_json::Value::String(Utc::now().to_rfc3339())),
            return_file: None,
            commit_file: None,
            upload_file: None,
        }),
    }
}
