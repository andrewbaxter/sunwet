pub mod form;
pub mod menu;

use {
    form::ClientForm,
    menu::ClientMenuItem,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::{
        HashMap,
    },
};

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientView {
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<ClientMenuItem>,
    pub views: HashMap<String, ClientView>,
    pub forms: HashMap<String, ClientForm>,
}
