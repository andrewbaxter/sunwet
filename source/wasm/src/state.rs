use {
    super::{
        ministate::{
            record_new_ministate,
            Ministate,
            PlaylistEntryPath,
        },
        page_edit::build_page_edit,
        page_form::build_page_form_by_id,
        page_query::{
            build_page_list_by_id,
            BuildPlaylistPos,
        },
    },
    crate::{
        async_::BgVal,
        el_general::{
            CSS_BUTTON,
            CSS_BUTTON_ICON_TEXT,
        },
        playlist::PlaylistState,
    },
    lunk::ProcessingContext,
    rooting::{
        el,
        El,
        WeakEl,
    },
    shared::interface::config::{
        form::Form,
        menu::MenuItem,
        view::View,
    },
    std::{
        collections::HashMap,
        rc::Rc,
    },
};

#[derive(Clone)]
pub struct Menu {
    pub menu: Vec<MenuItem>,
    pub views: HashMap<String, View>,
    pub forms: HashMap<String, Form>,
}

pub struct State_ {
    pub base_url: String,
    pub playlist: PlaylistState,
    pub menu: BgVal<Result<Rc<Menu>, String>>,
    pub stack: WeakEl,
    pub page_title: WeakEl,
    pub page_body: WeakEl,
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
            state.page_title.upgrade().unwrap().ref_clear().ref_push(el("h1").text("Sunwet"));
            state.page_body.upgrade().unwrap().ref_clear();
        },
        Ministate::List(ms) => {
            build_page_list_by_id(pc, state, &ms.title, &ms.id, &BuildPlaylistPos {
                list_id: ms.id.clone(),
                list_title: ms.title.clone(),
                entry_path: Some(PlaylistEntryPath(vec![])),
            }, &ms.pos);
        },
        Ministate::Form(ms) => {
            build_page_form_by_id(pc, state, &ms.title, &ms.id);
        },
        Ministate::Edit(ms) => {
            build_page_edit(pc, state, &ms.title, &ms.node);
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    record_new_ministate(s);
    build_ministate(pc, state, s);
}
