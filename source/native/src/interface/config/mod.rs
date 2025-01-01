use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        config::menu::MenuItem,
        iam::UserIdentityId,
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        net::SocketAddr,
        path::PathBuf,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PageAccess {
    View(String),
    Form(String),
}

impl GoodOrmningCustomString<PageAccess> for PageAccess {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(serde_json::from_str::<Self>(&value).map_err(|e| e.to_string())?);
    }
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GlobalConfig {
    pub admin_token: Option<String>,
    #[serde(default)]
    pub public_iam_grants: HashSet<PageAccess>,
    pub menu: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IamGrants {
    Admin,
    Limited(HashSet<PageAccess>),
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserConfig {
    pub iam_grants: IamGrants,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UsersConfig {
    pub users: HashMap<UserIdentityId, UserConfig>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MaybeFdap<T> {
    /// Get the config from FDAP, with this path.
    Fdap(Vec<String>),
    /// The config is specified directly here.
    Local(T),
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OidcConfig {
    pub provider_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FdapConfig {
    pub url: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub debug: bool,
    pub graph_dir: PathBuf,
    pub files_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub bind_addr: SocketAddr,
    pub oidc: Option<OidcConfig>,
    pub fdap: Option<FdapConfig>,
    pub global: MaybeFdap<GlobalConfig>,
    pub user: MaybeFdap<UsersConfig>,
}
