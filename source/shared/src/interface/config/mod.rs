pub mod form;
pub mod menu;
pub mod view;

use {
    form::ClientForm,
    menu::ClientMenuItem,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
    view::WidgetRootDataRows,
};

#[derive(Clone, Serialize, Deserialize, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientView {
    pub config: WidgetRootDataRows,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientConfig {
    pub menu: Vec<ClientMenuItem>,
    pub views: HashMap<String, ClientView>,
    pub forms: HashMap<String, ClientForm>,
}
