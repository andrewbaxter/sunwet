use {
    super::{
        config::ClientConfig,
        query::Query,
    },
    crate::interface::triple::{
        FileHash,
        Node,
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
    std::{
        collections::{
            BTreeMap,
            HashMap,
        },
    },
};

pub mod link;

pub const HEADER_OFFSET: &'static str = "x-file-offset";

pub trait C2SReqTrait: Serialize + DeserializeOwned + Into<C2SReq> {
    type Resp: Serialize + DeserializeOwned;
}

// # Commit
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Triple {
    pub subject: Node,
    pub predicate: String,
    pub object: Node,
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

// # Form commit
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqFormCommit {
    pub menu_item_id: String,
    pub parameters: HashMap<String, TreeNode>,
}

impl Into<C2SReq> for ReqFormCommit {
    fn into(self) -> C2SReq {
        return C2SReq::FormCommit(self);
    }
}

impl C2SReqTrait for ReqFormCommit {
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
    pub query: Query,
    #[serde(default)]
    pub parameters: HashMap<String, Node>,
}

/// A tree node is like a json node but it can also encode files.  So the root of
/// the returned query is generic data w/ a file type, then once you reach the
/// nodes it's just generic data. This allows users to select on files directly,
/// rather than try to re-parse json.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Debug, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TreeNode {
    Scalar(Node),
    Array(Vec<TreeNode>),
    Record(BTreeMap<String, TreeNode>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespQuery {
    pub records: Vec<BTreeMap<String, TreeNode>>,
}

impl Into<C2SReq> for ReqQuery {
    fn into(self) -> C2SReq {
        return C2SReq::Query(self);
    }
}

impl C2SReqTrait for ReqQuery {
    type Resp = RespQuery;
}

// # View query
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqViewQuery {
    pub menu_item_id: String,
    pub query: String,
    pub parameters: HashMap<String, Node>,
}

impl Into<C2SReq> for ReqViewQuery {
    fn into(self) -> C2SReq {
        return C2SReq::ViewQuery(self);
    }
}

impl C2SReqTrait for ReqViewQuery {
    type Resp = RespQuery;
}

// # Get triples from
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqGetTriplesAround {
    pub node: Node,
}

impl Into<C2SReq> for ReqGetTriplesAround {
    fn into(self) -> C2SReq {
        return C2SReq::GetTriplesAround(self);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespGetTriplesAround {
    pub incoming: Vec<Triple>,
    pub outgoing: Vec<Triple>,
}

impl C2SReqTrait for ReqGetTriplesAround {
    type Resp = RespGetTriplesAround;
}

// # History
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqHistory {
    pub end_excl: DateTime<Utc>,
    pub start_incl: DateTime<Utc>,
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

// # History, commits
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqHistoryCommitCount {
    pub end_excl: DateTime<Utc>,
}

impl Into<C2SReq> for ReqHistoryCommitCount {
    fn into(self) -> C2SReq {
        return C2SReq::HistoryCommitCount(self);
    }
}

impl C2SReqTrait for ReqHistoryCommitCount {
    type Resp = Vec<RespHistoryCommit>;
}

// # Get Menu
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqGetClientConfig;

impl Into<C2SReq> for ReqGetClientConfig {
    fn into(self) -> C2SReq {
        return C2SReq::GetClientConfig(self);
    }
}

impl C2SReqTrait for ReqGetClientConfig {
    type Resp = ClientConfig;
}

// # Who am I
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqWhoAmI;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RespWhoAmI {
    Public,
    User(String),
    Token,
}

impl Into<C2SReq> for ReqWhoAmI {
    fn into(self) -> C2SReq {
        return C2SReq::WhoAmI(self);
    }
}

impl C2SReqTrait for ReqWhoAmI {
    type Resp = RespWhoAmI;
}

// # Assemble
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum C2SReq {
    /// Make changes to the graph
    Commit(ReqCommit),
    /// Make changes to the graph via a form (uses form permissions)
    FormCommit(ReqFormCommit),
    UploadFinish(ReqUploadFinish),
    /// Read from the graph
    Query(ReqQuery),
    /// Read from the graph via a view (uses view permissions)
    ViewQuery(ReqViewQuery),
    GetTriplesAround(ReqGetTriplesAround),
    History(ReqHistory),
    HistoryCommitCount(ReqHistoryCommitCount),
    GetClientConfig(ReqGetClientConfig),
    WhoAmI(ReqWhoAmI),
}

pub fn alphanumeric_only(s: &str) -> String {
    return s.chars().map(|c| match c {
        'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' => c,
        _ => '_',
    }).collect::<String>();
}

pub fn gentype_transcode(mime: &str) -> String {
    return format!("mime_{}", alphanumeric_only(mime));
}

// Lang is as given by VTT
pub fn gentype_vtt(lang: &str) -> String {
    return format!("vtt_{}", alphanumeric_only(lang));
}

// Why not ACCEPT header? Accept header only accepts mime types, we need more high
// level data and I don't want to do extra-standard stuff.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileUrlQuery {
    /// Derivation name, like `mime-image/jpg` or `vtt-eng`, and whether it's required.
    /// If not required and the derivation is missing, respond with the original file
    /// data, otherwise 404.
    #[serde(skip_serializing_if = "Option::is_none", rename = "d", default)]
    pub derivation: Option<(String, bool)>,
}
