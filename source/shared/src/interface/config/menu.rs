use {
    super::{
        form::Form,
        view::View,
    },
    crate::interface::iam::IamTargetId,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MenuItemSection {
    pub iam_target: IamTargetId,
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
