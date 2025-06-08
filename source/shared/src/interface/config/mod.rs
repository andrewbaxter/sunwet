pub mod form;
pub mod view;

use {
    crate::interface::triple::Node,
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
    view::ClientView,
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuSection {
    pub children: Vec<ClientMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientViewLink {
    pub view_id: String,
    /// Provide initial query parameters. These can be modified by the user.
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientFormLink {
    pub form_id: String,
    /// Provide initial parameters for fields, by field id.
    pub parameters: BTreeMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientPage {
    View(ClientViewLink),
    Form(ClientFormLink),
    History,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMenuItemDetail {
    Section(ClientMenuSection),
    Page(ClientPage),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuItem {
    /// The id of a menu item is used for permissions.
    pub id: String,
    pub name: String,
    pub detail: ClientMenuItemDetail,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<ClientMenuItem>,
    /// View ids to view definitions
    pub views: HashMap<String, ClientView>,
    /// Form ids to form definitions
    pub forms: HashMap<String, ClientForm>,
}
