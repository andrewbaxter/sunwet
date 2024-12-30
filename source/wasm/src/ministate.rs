use {
    gloo::utils::window,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::triple::Node,
    wasm_bindgen::JsValue,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlaylistEntryPath(pub Vec<Node>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlaylistPos {
    pub entry_path: PlaylistEntryPath,
    pub time: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateView {
    pub id: String,
    pub title: String,
    pub pos: Option<PlaylistPos>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateForm {
    pub id: String,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateEdit {
    pub title: String,
    pub node: Node,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Home,
    List(MinistateView),
    Form(MinistateForm),
    Edit(MinistateEdit),
}

pub fn record_new_ministate(s: &Ministate) {
    window()
        .history()
        .unwrap()
        .push_state_with_url(&JsValue::null(), "", Some(&format!("#{}", serde_json::to_string(&s).unwrap())))
        .unwrap();
}

pub fn record_replace_ministate(s: &Ministate) {
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::null(), "", Some(&format!("#{}", serde_json::to_string(&s).unwrap())))
        .unwrap();
}
