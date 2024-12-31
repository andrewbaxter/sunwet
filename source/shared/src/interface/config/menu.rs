use {
    super::{
        form::Form,
        view::View,
    },
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItemSection {
    pub name: String,
    pub children: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuItem {
    Section(MenuItemSection),
    View(View),
    Form(Form),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Menu {
    pub list: Vec<MenuItem>,
    pub unlisted_views: Vec<View>,
    pub unlisted_forms: Vec<View>,
}
