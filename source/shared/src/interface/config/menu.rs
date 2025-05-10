use {
    crate::interface::triple::Node,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuItemSection {
    pub name: String,
    pub children: Vec<ClientMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuItemView {
    pub id: String,
    pub name: String,
    pub view_id: String,
    #[serde(default)]
    pub arguments: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuItemForm {
    pub id: String,
    pub name: String,
    pub form_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMenuItem {
    Section(ClientMenuItemSection),
    View(ClientMenuItemView),
    Form(ClientMenuItemForm),
}
