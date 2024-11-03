use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize)]
pub struct IamTargetId(pub usize);

#[derive(Serialize, Deserialize)]
pub struct IamTarget {
    pub id: IamTargetId,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct IamRoleId(pub usize);

#[derive(Serialize, Deserialize)]
pub struct IamRole {
    pub id: IamRoleId,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct IamAccess {
    pub target: IamTargetId,
    pub role: IamRoleId,
    pub read: bool,
    pub write: bool,
}

type UserId = String;

#[derive(Serialize, Deserialize)]
pub enum IamPrincipalId {
    World,
    User(UserId),
}

#[derive(Serialize, Deserialize)]
pub struct IamRoleMember {
    pub role: IamRoleId,
    pub principal: IamPrincipalId,
}

#[derive(Serialize, Deserialize)]
pub struct IamConfig {
    pub targets: Vec<IamTarget>,
    pub access: Vec<IamAccess>,
    pub roles: Vec<IamRole>,
    pub members: Vec<IamRoleMember>,
}

impl GoodOrmningCustomString<IamConfig> for IamConfig {
    fn to_sql<'a>(value: &'a IamConfig) -> std::borrow::Cow<'a, str> {
        return serde_json::to_string(value).unwrap().into();
    }

    fn from_sql(value: String) -> Result<IamConfig, String> {
        return Ok(serde_json::from_str::<IamConfig>(&value).map_err(|e| e.to_string())?.into());
    }
}
