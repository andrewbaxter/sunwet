use {
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, JsonSchema, TS)]
#[serde(
    //. rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct UserIdentityId(pub String);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IdentityId {
    Public,
    User(String),
}
