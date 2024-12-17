use {
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IamTargetId(pub i64);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserIdentityId(pub String);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IdentityId {
    Public,
    User(String),
}
