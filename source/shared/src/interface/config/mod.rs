pub mod form;
pub mod view;
pub mod menu;

use {
    form::Form,
    menu::MenuItem,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct View {
    pub name: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<MenuItem>,
    pub views: HashMap<String, View>,
    pub forms: HashMap<String, Form>,
}
