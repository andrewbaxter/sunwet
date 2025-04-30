use {
    super::playlist::PlaylistIndex,
    gloo::utils::window,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::triple::Node,
    wasm::el_general::{
        get_dom_octothorpe,
        log,
    },
    wasm_bindgen::JsValue,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlaylistRestorePos {
    pub index: PlaylistIndex,
    pub time: f64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateView {
    pub menu_item_id: String,
    pub title: String,
    pub pos: Option<PlaylistRestorePos>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateForm {
    pub menu_item_id: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateEdit {
    pub title: String,
    pub node: Node,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Home,
    View(MinistateView),
    Form(MinistateForm),
    Edit(MinistateEdit),
}

pub fn ministate_octothorpe(s: &Ministate) -> String {
    return format!("#{}", serde_json::to_string(&s).unwrap());
}

pub fn ministate_title(s: &Ministate) -> String {
    match s {
        Ministate::Home => return format!("Home"),
        Ministate::View(s) => return s.title.clone(),
        Ministate::Form(s) => return s.title.clone(),
        Ministate::Edit(s) => return s.title.clone(),
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
    let s = match serde_json::from_str::<Ministate>(s.as_ref()) {
        Ok(s) => s,
        Err(e) => {
            log(format!("Unable to parse url anchor state: {:?}\nAnchor: {}", e, s));
            return Ministate::Home;
        },
    };
    return s;
}
