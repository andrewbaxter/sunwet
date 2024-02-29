use std::{
    collections::HashMap,
    rc::Rc,
};
use gloo::utils::window;
use lunk::ProcessingContext;
use rooting::{
    el,
    El,
};
use serde::{
    Deserialize,
    Serialize,
};
use wasm_bindgen::JsValue;
use crate::{
    state::{
        State,
        View,
    },
};
use shared::model::view::{
    Align,
    Layout,
    LayoutIndividual,
    Orientation,
    QueryOrField,
    ViewPartList,
    WidgetNest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaylistEntryPath(pub Vec<serde_json::Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaylistPos {
    pub entry_path: PlaylistEntryPath,
    pub time: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ministate {
    Home,
    View {
        id: String,
        title: String,
        pos: Option<PlaylistPos>,
    },
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
