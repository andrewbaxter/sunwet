use {
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        config::{
            form::{
                FormField,
                FormOutput,
            },
            view::{
                ClientViewParam,
                WidgetRootDataRows,
            },
        },
        iam::UserIdentityId,
        query::Query,
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        net::SocketAddr,
        path::PathBuf,
    },
};

pub type MenuItemId = String;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItemSection {
    pub name: String,
    pub children: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuItemSub {
    Section(MenuItemSection),
    View(View),
    Form(Form),
    History,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItem {
    pub id: MenuItemId,
    pub sub: MenuItemSub,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct View {
    pub name: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, ClientViewParam>,
    pub queries: BTreeMap<String, Query>,
    pub root: WidgetRootDataRows,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Form {
    pub name: String,
    pub fields: Vec<FormField>,
    pub outputs: Vec<FormOutput>,
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GlobalConfig {
    pub api_tokens: HashMap<String, IamGrants>,
    #[serde(default)]
    pub public_iam_grants: HashSet<MenuItemId>,
    pub menu: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IamGrants {
    Admin,
    Limited(HashSet<MenuItemId>),
}

impl IamGrants {
    pub fn match_set(&self, target_set: &HashSet<MenuItemId>) -> bool {
        match self {
            IamGrants::Admin => {
                return true;
            },
            IamGrants::Limited(self_set) => {
                for target_id in target_set {
                    if self_set.contains(target_id) {
                        return true;
                    }
                }
            },
        }
        return false;
    }
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
    pub temp_dir: PathBuf,
    pub graph_dir: PathBuf,
    pub files_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub bind_addr: SocketAddr,
    pub oidc: Option<OidcConfig>,
    pub fdap: Option<FdapConfig>,
    pub global: MaybeFdap<GlobalConfig>,
    pub user: MaybeFdap<UsersConfig>,
}
