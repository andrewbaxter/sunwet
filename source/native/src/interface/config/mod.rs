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
                FormId,
                FormOutput,
            },
            view::{
                ClientViewParam,
                ViewId,
                WidgetRootDataRows,
            },
            MenuItemId,
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
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItemSection {
    /// Items to show in the group.
    pub children: Vec<ServerConfigMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ViewLink {
    /// This is the key of a view in the root global config.
    pub view_id: ViewId,
    /// Provide initial query parameters. These can be modified by the user.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormLink {
    /// This is the key of a form in the root global config.
    pub form_id: FormId,
    /// Provide initial parameters for fields, by field id.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuItemPage {
    /// Take the user to the specified view page.
    View(ViewLink),
    /// Take the user to the specified form page.
    Form(FormLink),
    /// Take the user to the commit history page.
    History,
    /// Take the user to the free query page.
    Query,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerConfigMenuItemDetail {
    /// This shows an expandable group of menu items.
    Section(MenuItemSection),
    /// This is a leaf menu item, a link to a page.
    Page(MenuItemPage),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ServerConfigMenuItem {
    /// The id of a menu item is used for permissions.
    pub id: MenuItemId,
    /// Text to show in the menu.
    pub name: String,
    /// The type of menu item.
    pub detail: ServerConfigMenuItemDetail,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct View {
    /// The user will be presented inputs to provide values to the query. The values
    /// entered will be available as variables with the mapped name during query
    /// evaluation.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub parameters: BTreeMap<String, ClientViewParam>,
    /// The queries the view can execute, named. These are referred to by the display.
    pub queries: BTreeMap<String, Query>,
    /// How to build the view.
    pub display: WidgetRootDataRows,
}

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Form {
    /// What the user must input.
    pub fields: Vec<FormField>,
    /// How to construct the commit from the input fields.
    pub outputs: Vec<FormOutput>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS, Debug, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ConfigIamGrantsLimited {
    /// For every menu item id listed here, give the user access to the menu item, all
    /// child menu items (if a section) transitively, and any forms or views directly
    /// linked by leaf menu items.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub menu_items: HashSet<MenuItemId>,
    /// Give the user access to all these views.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub views: HashSet<ViewId>,
    /// Give the user access to all these forms.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub forms: HashSet<FormId>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigIamGrants {
    /// Access everything, including running free queries.
    Admin,
    /// Access only specified views and forms.
    Limited(ConfigIamGrantsLimited),
}

impl Default for ConfigIamGrants {
    fn default() -> Self {
        return ConfigIamGrants::Limited(Default::default());
    }
}

#[derive(Serialize, Deserialize, Default, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GlobalConfig {
    /// Define access for non-authenticated users.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub public_iam_grants: ConfigIamGrants,
    /// A map of api tokens (the token is the key) to access to grant the bearer of
    /// that token.
    pub api_tokens: HashMap<String, ConfigIamGrants>,
    pub menu: Vec<ServerConfigMenuItem>,
    /// View ids to view definitions
    pub views: HashMap<ViewId, View>,
    /// Form ids to form definitions
    pub forms: HashMap<FormId, Form>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UserConfig {
    /// What the user is allowed to access.
    pub iam_grants: ConfigIamGrants,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UsersConfig {
    /// Configure access based on the identity provided by the identity server
    /// (`subject` field in the identity token).
    pub users: HashMap<UserIdentityId, UserConfig>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MaybeFdap<T> {
    /// Get the config from FDAP, with this path.
    Fdap(Vec<String>),
    /// The config is specified directly here.
    Local(T),
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OidcConfig {
    // Standard OIDC parameter, see OIDC provider documentation.
    pub provider_url: String,
    // Standard OIDC parameter, see OIDC provider documentation.
    pub client_id: String,
    // Standard OIDC parameter, see OIDC provider documentation.
    pub client_secret: Option<String>,
}

/// This describes a config value that can either be provided directly in the
/// config, or fetched from FDAP at the provided path (in the globally configured
/// FDAP server).
#[derive(Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FdapConfig {
    /// Standard FDAP parameter, see FDAP provider documentation.
    pub url: String,
    /// Standard FDAP parameter, see FDAP provider documentation.
    pub token: String,
}

#[derive(Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[ts(export)]
pub struct Config {
    /// Verbose logging.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub debug: bool,
    /// This directory stores partial files during upload. Completed files are moved
    /// into the "files" directory atomically, so it must be on the same mount as the
    /// files_dir.
    pub temp_dir: PathBuf,
    /// This directory contains the graph (triples). Back this directory up (stop
    /// Sunwet first).
    pub graph_dir: PathBuf,
    /// This directory contains the files directly upload, that are referenced by the
    /// graph. Back this directory up (stop Sunwet first).
    pub files_dir: PathBuf,
    /// This directory contains generated files. Everything can be re-created if this
    /// directory is lost.
    pub cache_dir: PathBuf,
    /// Access the server via this address (both web and CLI).
    pub bind_addr: SocketAddr,
    /// Allow users to identify via OIDC
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub oidc: Option<OidcConfig>,
    /// Define access for users (as identified by OIDC).
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub users: Option<MaybeFdap<UsersConfig>>,
    /// If you have configs you want to fetch from an FDAP server (global or users) you
    /// must configure how to access the FDAP server here.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub fdap: Option<FdapConfig>,
    /// Everything else.
    pub global: MaybeFdap<GlobalConfig>,
}
