use {
    super::{
        config::menu::MenuItem,
        query::Query,
    },
    crate::interface::{
        iam::IamTargetId,
        triple::{
            FileHash,
            Node,
        },
    },
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

pub mod link;

pub const HEADER_OFFSET: &'static str = "x-file-offset";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Triple {
    pub subject: Node,
    pub predicate: String,
    pub object: Node,
    pub iam_target: IamTargetId,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CommitFile {
    pub hash: FileHash,
    pub size: u64,
    pub mimetype: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CommitReq {
    pub add: Vec<Triple>,
    pub remove: Vec<Triple>,
    pub files: Vec<CommitFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CommitResp {
    pub incomplete: Vec<FileHash>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UploadFinishResp {
    pub done: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QueryReq {
    pub q: Query,
    pub parameters: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QueryResp {
    pub records: Vec<HashMap<String, Node>>,
}

pub type GetMenuResp = Vec<MenuItem>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum C2SReq {
    Commit(CommitReq),
    UploadFinish(FileHash),
    Query(QueryReq),
    GetMenu,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileGenerated {
    pub mime_type: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileUrlQuery {
    pub generated: Option<FileGenerated>,
}
