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
        triple::Node,
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
    pub children: Vec<ServerConfigMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ViewLink {
    pub view_id: String,
    /// Provide initial query parameters. These can be modified by the user.
    #[serde(default)]
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormLink {
    pub form_id: String,
    /// Provide initial parameters for fields, by field id.
    #[serde(default)]
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NodeLink {
    pub node: Node,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuItemPage {
    View(ViewLink),
    Form(FormLink),
    History,
    Query,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerConfigMenuItemDetail {
    Section(MenuItemSection),
    Page(MenuItemPage),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ServerConfigMenuItem {
    /// The id of a menu item is used for permissions.
    pub id: String,
    pub name: String,
    pub detail: ServerConfigMenuItemDetail,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct View {
    #[serde(default)]
    pub parameters: BTreeMap<String, ClientViewParam>,
    pub queries: BTreeMap<String, Query>,
    pub display: WidgetRootDataRows,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Form {
    pub fields: Vec<FormField>,
    pub outputs: Vec<FormOutput>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IamGrantsLimited {
    /// For every menu item id listed here, give the user access to the menu item, all
    /// child menu items (if a section) transitively, and any forms or views directly
    /// linked by leaf menu items.
    #[serde(default)]
    pub menu_items: HashSet<MenuItemId>,
    /// Give the user access to all these views.
    #[serde(default)]
    pub views: HashSet<String>,
    /// Give the user access to all these forms.
    #[serde(default)]
    pub forms: HashSet<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GlobalConfig {
    pub api_tokens: HashMap<String, IamGrants>,
    #[serde(default)]
    pub public_iam_grants: IamGrantsLimited,
    pub menu: Vec<ServerConfigMenuItem>,
    /// View ids to view definitions
    pub views: HashMap<String, View>,
    /// Form ids to form definitions
    pub forms: HashMap<String, Form>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IamGrants {
    Admin,
    Limited(IamGrantsLimited),
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
