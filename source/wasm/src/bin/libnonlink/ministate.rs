use {
    super::playlist::PlaylistIndex,
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::window,
    },
    js_sys::decode_uri,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        config::{
            form::FormId,
            view::ViewId,
        },
        triple::Node,
    },
    std::{
        collections::HashMap,
        rc::Rc,
    },
    wasm::js::{
        get_dom_octothorpe,
        Log,
        LogJsErr,
    },
    wasm_bindgen::JsValue,
};

pub const LOCALSTORAGE_PWA_MINISTATE: &str = "pwa_ministate";
pub const SESSIONSTORAGE_POST_REDIRECT_MINISTATE: &str = "post_redirect";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlaylistRestorePos {
    pub index: PlaylistIndex,
    pub time: f64,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub play: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateView {
    pub id: ViewId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<PlaylistRestorePos>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateOfflineView {
    pub key: String,
    pub id: ViewId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<PlaylistRestorePos>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateForm {
    pub id: FormId,
    pub title: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, Node>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateNodeEdit {
    pub title: String,
    pub nodes: Vec<Node>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateListEdit {
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
pub enum MinistateHistoryPredicate {
    Incoming(String),
    Outgoing(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateHistoryFilter {
    pub node: Node,
    pub predicate: Option<MinistateHistoryPredicate>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateHistory {
    pub filter: Option<MinistateHistoryFilter>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateQuery {
    pub query: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Home,
    View(MinistateView),
    OfflineView(MinistateOfflineView),
    Form(MinistateForm),
    NodeEdit(MinistateNodeEdit),
    NodeView(MinistateNodeView),
    ListEdit(MinistateListEdit),
    History(MinistateHistory),
    Query(MinistateQuery),
    Logs,
}

pub fn ministate_octothorpe(s: &Ministate) -> String {
    return format!("#{}", serde_json::to_string(&s).unwrap());
}

pub fn ministate_title(s: &Ministate) -> String {
    match s {
        Ministate::Home => return format!("Home"),
        Ministate::View(s) => return s.title.clone(),
        Ministate::OfflineView(s) => return s.title.clone(),
        Ministate::Form(s) => return s.title.clone(),
        Ministate::NodeEdit(s) => return s.title.clone(),
        Ministate::NodeView(s) => return s.title.clone(),
        Ministate::ListEdit(s) => return s.title.clone(),
        Ministate::History(_) => return format!("History"),
        Ministate::Query(_) => return format!("Query"),
        Ministate::Logs => return format!("Logs"),
    }
}

pub fn save_pwa_ministate(log: &Rc<dyn Log>, s: &Ministate) {
    LocalStorage::set(LOCALSTORAGE_PWA_MINISTATE, s).log(log, &"Error storing PWA ministate");
}

/// Replaces current state in history, no page change
pub fn record_replace_ministate(log: &Rc<dyn Log>, s: &Ministate) {
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::null(), "", Some(&ministate_octothorpe(s)))
        .unwrap();
    save_pwa_ministate(log, s);
}

pub fn read_ministate(log: &Rc<dyn Log>) -> Ministate {
    let Some(s) = get_dom_octothorpe(log) else {
        return Ministate::Home;
    };
    match serde_json::from_str::<Ministate>(s.as_ref()) {
        Ok(s) => {
            return s;
        },
        Err(e) => {
            log.log(&format!("Unable to parse url anchor state (1/2, no urldecode) [{}]: {}", s, e));
        },
    };
    match serde_json::from_str::<Ministate>(&decode_uri(s.as_str()).unwrap().as_string().unwrap()) {
        Ok(s) => {
            return s;
        },
        Err(e) => {
            log.log(&format!("Unable to parse url anchor state (2/2, urldecode) [{}]: {}", s, e));
        },
    }
    return Ministate::Home;
}
