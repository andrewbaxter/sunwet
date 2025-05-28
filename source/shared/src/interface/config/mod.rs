pub mod form;
pub mod view;

use {
    form::ClientForm,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    view::ClientView,
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientMenuSection {
    pub name: String,
    pub children: Vec<ClientMenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMenuItem {
    Section(ClientMenuSection),
    View(ClientView),
    Form(ClientForm),
    History,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<ClientMenuItem>,
}
