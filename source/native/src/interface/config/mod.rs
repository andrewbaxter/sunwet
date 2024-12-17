use {
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        config::menu::MenuItem,
        iam::{
            IamTargetId,
            UserIdentityId,
        },
    },
    std::{
        collections::HashMap,
        net::SocketAddr,
        path::PathBuf,
    },
};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GlobalConfig {
    pub admin_token: Option<String>,
    pub public_access: Option<Vec<IamTargetId>>,
    pub menu: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum UserAccess {
    ReadWrite,
    Read(Vec<IamTargetId>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserConfig {
    pub access: UserAccess,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UsersConfig {
    pub users: HashMap<UserIdentityId, UserConfig>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MaybeFdap<T> {
    /// Get the config from FDAP, with this path.
    Fdap(Vec<String>),
    /// The config is specified directly here.
    Local(T),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OidcConfig {
    pub provider_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FdapConfig {
    pub url: String,
    pub token: String,
}

#[derive(Serialize, Deserialize)]
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
