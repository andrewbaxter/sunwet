use std::path::PathBuf;
use serde::{
    Deserialize,
    Serialize,
};
use super::FileHash;

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
