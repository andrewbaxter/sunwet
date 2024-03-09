use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};
use lunk::{
    ProcessingContext,
};
use rooting::{
    el,
    El,
    WeakEl,
};
use shared::model::{
    View,
};
use web::{
    async_::BgVal,
    el_general::{
        CSS_BUTTON,
        CSS_BUTTON_ICON_TEXT,
    },
};
use crate::{
    playlist::PlaylistState,
};
use super::{
    ministate::{
        record_new_ministate,
        Ministate,
        PlaylistEntryPath,
    },
    page_query::{
        build_page_view_by_id,
        BuildPlaylistPos,
    },
};

pub struct State_ {
    pub origin: String,
    pub playlist: PlaylistState,
    pub views: BgVal<Rc<RefCell<HashMap<String, View>>>>,
    pub stack: WeakEl,
    pub mobile_vert_title_group: WeakEl,
    pub title_group: WeakEl,
    pub body_group: WeakEl,
}

pub type State = Rc<State_>;

pub fn el_ministate_button(pc: &mut ProcessingContext, state: &State, text: &str, ministate: Ministate) -> El {
    return el("a")
        .classes(&[CSS_BUTTON, CSS_BUTTON_ICON_TEXT])
        .attr("href", &format!("#{}", serde_json::to_string(&ministate).unwrap()))
        .push(el("span").text(text))
        .on("click", {
            let eg = pc.eg();
            let state = state.clone();
            move |ev| eg.event(|pc| {
                ev.stop_propagation();
                change_ministate(pc, &state, &ministate);
            })
        });
}

pub fn build_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    match s {
        Ministate::Home => {
            state.mobile_vert_title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text("Sunwet"));
            state.title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text("Sunwet"));
            state.body_group.upgrade().unwrap().ref_clear();
        },
        Ministate::View { id, title, pos } => {
            build_page_view_by_id(pc, state, title, id, &BuildPlaylistPos {
                view_id: id.clone(),
                view_title: title.clone(),
                entry_path: Some(PlaylistEntryPath(vec![])),
            }, pos);
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    record_new_ministate(s);
    build_ministate(pc, state, s);
}
