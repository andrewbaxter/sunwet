use std::collections::HashMap;
use gloo::utils::window;
use lunk::ProcessingContext;
use rooting::{
    el,
    El,
};
use rooting_forms::BigString;
use serde::{
    Deserialize,
    Serialize,
};
use wasm_bindgen::JsValue;
use crate::{
    page_query::{
        build_page_view,
        definition::{
            Align,
            Layout,
            LayoutIndividual,
            Orientation,
            QueryOrField,
            WidgetList,
            WidgetNest,
        },
    },
    state::{
        State,
        View,
    },
    testdata::testdata_albums,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ministate {
    Home,
    View {
        id: usize,
        title: String,
        play_entry: Vec<HashMap<String, serde_json::Value>>,
        play_time: f64,
    },
    NewView,
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
