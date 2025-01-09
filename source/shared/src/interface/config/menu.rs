use {
    super::{
        form::Form,
        view::View,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItemSection {
    pub name: String,
    pub children: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuItem {
    Section(MenuItemSection),
    View(View),
    Form(Form),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Menu {
    /// The root items in the menu
    pub list: Vec<MenuItem>,
    /// Views that aren't in the menu, but may be accessed by links
    pub unlisted_views: Vec<View>,
    /// Forms that aren't in the menu, but may be accessed by links
    pub unlisted_forms: Vec<Form>,
}
