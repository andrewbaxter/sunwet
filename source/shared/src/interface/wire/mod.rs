use {
    super::{
        config::ClientConfig,
        query::Query,
    },
    crate::interface::{
        config::{
            form::FormId,
            view::ViewId,
        },
        triple::{
            FileHash,
            Node,
        },
    },
    chrono::{
        DateTime,
        Utc,
    },
    schemars::JsonSchema,
    serde::{
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    std::collections::{
        BTreeMap,
        HashMap,
    },
};

pub mod link;

pub const HEADER_OFFSET: &'static str = "x-file-offset";

pub trait C2SReqTrait: Serialize + DeserializeOwned + Into<C2SReq> {
    type Resp: Serialize + DeserializeOwned;
}

// # Commit
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Triple {
    pub subject: Node,
    pub predicate: String,
    pub object: Node,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CommitFile {
    pub hash: FileHash,
    pub size: u64,
    pub mimetype: String,
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqCommit {
    pub comment: String,
    pub add: Vec<Triple>,
    pub remove: Vec<Triple>,
    pub files: Vec<CommitFile>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
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
#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqFormCommit {
    pub form_id: FormId,
    #[serde(default)]
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqUploadFinish(pub FileHash);

#[derive(Serialize, Deserialize, JsonSchema)]
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Pagination {
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Node>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqQuery {
    pub query: Query,
    #[serde(default)]
    pub parameters: HashMap<String, Node>,
    pub pagination: Option<Pagination>,
}

/// A tree node is like a json node but it can also encode files.  So the root of
/// the returned query is generic data w/ a file type, then once you reach the
/// nodes it's just generic data. This allows users to select on files directly,
/// rather than try to re-parse json.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TreeNode {
    Scalar(Node),
    Array(Vec<TreeNode>),
    Record(BTreeMap<String, TreeNode>),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RespQueryRows {
    /// For queries with no struct `{}` suffix
    Scalar(Vec<Node>),
    /// For queries with a struct `{}` suffix
    Record(Vec<BTreeMap<String, TreeNode>>),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespQuery {
    pub rows: RespQueryRows,
    pub meta: Vec<(Node, NodeMeta)>,
    pub next_page_key: Option<Node>,
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqViewQuery {
    pub view_id: ViewId,
    pub query: String,
    pub parameters: HashMap<String, Node>,
    pub pagination: Option<Pagination>,
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqGetTriplesAround {
    pub nodes: Vec<Node>,
}

impl Into<C2SReq> for ReqGetTriplesAround {
    fn into(self) -> C2SReq {
        return C2SReq::GetTriplesAround(self);
    }
}

impl C2SReqTrait for ReqGetTriplesAround {
    type Resp = Vec<Triple>;
}

// # Get node meta
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqGetNodeMeta {
    pub node: Node,
}

impl Into<C2SReq> for ReqGetNodeMeta {
    fn into(self) -> C2SReq {
        return C2SReq::GetNodeMeta(self);
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NodeMeta {
    pub mime: Option<String>,
}

impl C2SReqTrait for ReqGetNodeMeta {
    type Resp = Option<NodeMeta>;
}

// # History
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqHistory {
    pub page_key: Option<(DateTime<Utc>, Triple)>,
    pub filter: Option<ReqHistoryFilter>,
}

impl Into<C2SReq> for ReqHistory {
    fn into(self) -> C2SReq {
        return C2SReq::History(self);
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespHistoryEvent {
    pub delete: bool,
    pub commit: DateTime<Utc>,
    pub triple: Triple,
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RespHistory {
    pub events: Vec<RespHistoryEvent>,
    pub commit_descriptions: HashMap<DateTime<Utc>, String>,
}

impl C2SReqTrait for ReqHistory {
    type Resp = RespHistory;
}

// # History, commits
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReqHistoryFilterPredicate {
    Incoming(String),
    Outgoing(String),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqHistoryFilter {
    pub node: Node,
    pub predicate: Option<ReqHistoryFilterPredicate>,
}

// # Get Menu
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReqWhoAmI;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum C2SReq {
    /// Make changes to the graph
    Commit(ReqCommit),
    /// Make changes to the graph via a form (uses form permissions)
    FormCommit(ReqFormCommit),
    /// Tell the server to verify and commit a file after uploading all chunks
    UploadFinish(ReqUploadFinish),
    /// Read from the graph
    Query(ReqQuery),
    /// Read from the graph via a view (uses view permissions)
    ViewQuery(ReqViewQuery),
    /// Get all triples where the subject/object is a given node
    GetTriplesAround(ReqGetTriplesAround),
    /// Get metadata associated with nodes (ex: mime type for files)
    GetNodeMeta(ReqGetNodeMeta),
    History(ReqHistory),
    /// Request the config for the web UI for this user
    GetClientConfig(ReqGetClientConfig),
    /// Check authentication status
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
pub const GENTYPE_VTT: &str = "vtt";

pub fn gentype_vtt_subpath(lang: &str) -> String {
    return alphanumeric_only(lang);
}

pub const GENTYPE_DIR: &str = "dir";
