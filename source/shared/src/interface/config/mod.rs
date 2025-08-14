pub mod form;
pub mod view;

use {
    crate::interface::{
        config::{
            form::FormId,
            view::ViewId,
        },
        triple::Node,
    },
    form::ClientForm,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::{
        BTreeMap,
        HashMap,
    },
    ts_rs::TS,
    view::ClientView,
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(
    //. rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct MenuItemId(pub String);

impl std::fmt::Display for MenuItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return self.0.fmt(f);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuSection {
    pub children: Vec<ClientMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientViewLink {
    pub view_id: ViewId,
    /// Provide initial query parameters. These can be modified by the user.
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientFormLink {
    pub form_id: FormId,
    /// Provide initial parameters for fields, by field id.
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientPage {
    View(ClientViewLink),
    Form(ClientFormLink),
    History,
    Query,
    Logs,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMenuItemDetail {
    Section(ClientMenuSection),
    Page(ClientPage),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuItem {
    /// The id of a menu item is used for permissions.
    pub id: MenuItemId,
    pub name: String,
    pub detail: ClientMenuItemDetail,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<ClientMenuItem>,
    /// View ids to view definitions
    pub views: HashMap<ViewId, ClientView>,
    /// Form ids to form definitions
    pub forms: HashMap<FormId, ClientForm>,
}
