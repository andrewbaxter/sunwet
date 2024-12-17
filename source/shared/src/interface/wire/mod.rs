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
    chrono::{
        DateTime,
        Utc,
    },
    serde::{
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

pub mod link;

pub const HEADER_OFFSET: &'static str = "x-file-offset";

pub trait C2SReqTrait: Serialize + DeserializeOwned + Into<C2SReq> {
    type Resp: Serialize + DeserializeOwned;
}

// # Commit
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
pub struct ReqCommit {
    pub add: Vec<Triple>,
    pub remove: Vec<Triple>,
    pub files: Vec<CommitFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespCommit {
    pub incomplete: Vec<FileHash>,
}

impl Into<C2SReq> for ReqCommit {
    fn into(self) -> C2SReq {
        return C2SReq::Commit(self);
    }
}

impl C2SReqTrait for ReqCommit {
    type Resp = RespCommit;
}

// # Upload finish
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqUploadFinish(pub FileHash);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespUploadFinish {
    pub done: bool,
}

impl Into<C2SReq> for ReqUploadFinish {
    fn into(self) -> C2SReq {
        return C2SReq::UploadFinish(self);
    }
}

impl C2SReqTrait for ReqUploadFinish {
    type Resp = RespUploadFinish;
}

// # Query
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqQuery {
    pub q: Query,
    pub parameters: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Debug, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryResVal {
    Scalar(Node),
    Array(Vec<Node>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespQuery {
    pub records: Vec<HashMap<String, QueryResVal>>,
}

impl Into<C2SReq> for ReqQuery {
    fn into(self) -> C2SReq {
        return C2SReq::Query(self);
    }
}

impl C2SReqTrait for ReqQuery {
    type Resp = RespQuery;
}

// # History
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqHistory {
    pub start_incl: DateTime<Utc>,
    pub end_excl: DateTime<Utc>,
}

impl Into<C2SReq> for ReqHistory {
    fn into(self) -> C2SReq {
        return C2SReq::History(self);
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespHistoryCommit {
    pub timestamp: DateTime<Utc>,
    pub desc: String,
    pub add: Vec<Triple>,
    pub remove: Vec<Triple>,
}

impl C2SReqTrait for ReqHistory {
    type Resp = Vec<RespHistoryCommit>;
}

// # Get Menu
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqGetMenu;

impl Into<C2SReq> for ReqGetMenu {
    fn into(self) -> C2SReq {
        return C2SReq::GetMenu(self);
    }
}

impl C2SReqTrait for ReqGetMenu {
    type Resp = Vec<MenuItem>;
}

// # Assemble
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum C2SReq {
    Commit(ReqCommit),
    UploadFinish(ReqUploadFinish),
    Query(ReqQuery),
    History(ReqHistory),
    GetMenu(ReqGetMenu),
}

// # ?
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
