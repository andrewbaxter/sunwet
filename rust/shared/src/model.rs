use std::{
    collections::HashMap,
    str::FromStr,
};
use serde::{
    Deserialize,
    Serialize,
};

pub const HEADER_OFFSET: &'static str = "x-file-offset";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Node {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Triple {
    pub subject: Node,
    pub predicate: String,
    pub object: Node,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CommitFile {
    pub hash: FileHash,
    pub size: u64,
    pub mimetype: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Commit {
    pub add: Vec<Triple>,
    pub remove: Vec<Triple>,
    pub files: Vec<CommitFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommitResp {
    pub incomplete: Vec<FileHash>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidthUnit {
    Physical(f64),
    Percent(f64),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Width {
    desired: WidthUnit,
    min: Option<WidthUnit>,
    max: Option<WidthUnit>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewListArea {
    pub children: Vec<ViewList>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewList {
    pub query: String,
    pub width: Width,
    pub display: Vec<ViewListArea>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewDefinition {
    pub id: String,
    pub path: Vec<String>,
    pub name: String,
    pub top: ViewList,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FileHash {
    Sha256(String),
}

const HASH_PREFIX_SHA256: &'static str = "sha256";

impl ToString for FileHash {
    fn to_string(&self) -> String {
        let prefix;
        let hash;
        match self {
            FileHash::Sha256(v) => {
                prefix = HASH_PREFIX_SHA256;
                hash = v;
            },
        }
        return format!("{}:{}", prefix, hash);
    }
}

impl FromStr for FileHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((prefix, suffix)) = s.split_once(':') else {
            return Err("Invalid file hash; missing colon separating prefix and suffix".to_string());
        };
        match prefix {
            HASH_PREFIX_SHA256 => {
                return Ok(FileHash::Sha256(suffix.to_string()));
            },
            _ => {
                return Err(format!("Invalid file hash; unknown hash prefix [{}]", prefix));
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Query {
    pub query: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum C2SReq {
    Commit(Commit),
    UploadFinish(FileHash),
    Query(Query),
}
