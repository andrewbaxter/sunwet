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
    /// Restrict which users can see this menu section - if missing, only admins can
    /// see it.
    pub allow_target: Option<IamTargetId>,
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
