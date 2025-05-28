use {
    super::playlist::PlaylistIndex,
    gloo::utils::window,
    js_sys::decode_uri,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::triple::Node,
    wasm::js::{
        get_dom_octothorpe,
        log,
    },
    wasm_bindgen::JsValue,
};

pub const SESSIONSTORAGE_POST_REDIRECT: &str = "post_redirect";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlaylistRestorePos {
    pub index: PlaylistIndex,
    pub time: f64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateMenuItem {
    pub menu_item_id: String,
    pub title: String,
    pub pos: Option<PlaylistRestorePos>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateNodeEdit {
    pub title: String,
    pub node: Node,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateNodeView {
    pub title: String,
    pub node: Node,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Home,
    MenuItem(MinistateMenuItem),
    NodeEdit(MinistateNodeEdit),
    NodeView(MinistateNodeView),
    History,
}

pub fn ministate_octothorpe(s: &Ministate) -> String {
    return format!("#{}", serde_json::to_string(&s).unwrap());
}

pub fn ministate_title(s: &Ministate) -> String {
    match s {
        Ministate::Home => return format!("Home"),
        Ministate::MenuItem(s) => return s.title.clone(),
        Ministate::NodeEdit(s) => return s.title.clone(),
        Ministate::NodeView(s) => return s.title.clone(),
        Ministate::History => return format!("History"),
    }
}

pub fn record_new_ministate(s: &Ministate) {
    window()
        .history()
        .unwrap()
        .push_state_with_url(
            &JsValue::null(),
            &format!("{} - Sunwet", ministate_title(s)),
            Some(&ministate_octothorpe(s)),
        )
        .unwrap();
}

pub fn record_replace_ministate(s: &Ministate) {
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::null(), "", Some(&ministate_octothorpe(s)))
        .unwrap();
}

pub fn read_ministate() -> Ministate {
    let Some(s) = get_dom_octothorpe() else {
        return Ministate::Home;
    };
    match serde_json::from_str::<Ministate>(s.as_ref()) {
        Ok(s) => return s,
        Err(_) => {
            // nop
        },
    };
    match serde_json::from_str::<Ministate>(&decode_uri(s.as_str()).unwrap().as_string().unwrap()) {
        Ok(s) => return s,
        Err(_) => {
            // nop
        },
    }
    log(format!("Unable to parse url anchor state: [{}]", s));
    return Ministate::Home;
}
