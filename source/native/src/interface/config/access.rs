use {
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::iam::{
        IamTargetId,
        IamUserGroupId,
    },
    std::collections::HashSet,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AccessUserGroup {
    pub id: IamUserGroupId,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AccessTarget {
    pub id: IamTargetId,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AccessRule {
    pub target_id: IamTargetId,
    pub user_group_id: IamUserGroupId,
    pub read: bool,
    pub write: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Access {
    pub user_groups: Vec<AccessUserGroup>,
    pub targets: Vec<AccessTarget>,
    pub rules: Vec<AccessRule>,
    pub world_group_membership: Vec<IamUserGroupId>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserAccess {
    pub user_group_membership: HashSet<IamUserGroupId>,
}
