use {
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IamUserGroupId(pub i64);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IamTargetId(pub i64);

pub const IAM_TARGET_ADMIN: IamTargetId = IamTargetId(0);
pub const IAM_TARGET_WORLD_RO: IamTargetId = IamTargetId(1);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserIdentityId(pub String);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IdentityId {
    Public,
    User(String),
}
