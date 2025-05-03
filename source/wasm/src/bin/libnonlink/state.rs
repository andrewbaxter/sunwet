use {
    super::{
        ministate::{
            record_new_ministate,
            Ministate,
        },
        page_form::build_page_form_by_id,
        page_node_edit::build_page_node_edit,
        page_node_view::build_page_node_view,
        page_view::build_page_view,
        playlist::{
            playlist_clear,
            PlaylistState,
        },
    },
    gloo::utils::document,
    lunk::{
        EventGraph,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el_from_raw,
        El,
    },
    shared::interface::config::ClientConfig,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm::{
        async_::BgVal,
        js::style_export,
    },
};

pub struct State_ {
    pub eg: EventGraph,
    pub ministate: RefCell<Ministate>,
    // Ends with `/`
    pub base_url: String,
    pub playlist: PlaylistState,
    pub client_config: BgVal<Result<Rc<ClientConfig>, String>>,
    pub menu_open: Prim<bool>,
    // Arcmutex due to OnceLock, should El use sync alternatives?
    pub main_title: El,
    pub main_body: El,
    pub menu_body: El,
    pub modal_stack: El,
}

thread_local!{
    pub(crate) static STATE: RefCell<Option<Rc<State_>>> = RefCell::new(None);
}

pub fn state() -> Rc<State_> {
    return STATE.with(|x| x.borrow().clone()).unwrap();
}

pub fn set_page(pc: &mut ProcessingContext, title: &str, body: El) {
    set_page_(pc, title, false, body);
}

pub fn set_page_(pc: &mut ProcessingContext, title: &str, no_main_title: bool, body: El) {
    playlist_clear(pc, &state().playlist);
    document().set_title(&format!("{} - Sunwet", title));
    let state = state();
    state.modal_stack.ref_clear();
    if no_main_title {
        state.main_title.ref_text("");
    } else {
        state.main_title.ref_text(title);
    }
    state.main_body.ref_clear();
    state.main_body.ref_push(body);
    state.menu_open.set(pc, false);
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    match s {
        Ministate::Home => {
            set_page_(pc, "Home", true, el_from_raw(style_export::cont_page_home().root.into()));
        },
        Ministate::View(ms) => {
            build_page_view(pc, &ms.title, &ms.menu_item_id, ms.pos.clone());
        },
        Ministate::Form(ms) => {
            build_page_form_by_id(pc, &ms.title, &ms.menu_item_id);
        },
        Ministate::NodeEdit(ms) => {
            build_page_node_edit(pc, &ms.title, &ms.node);
        },
        Ministate::NodeView(ms) => {
            build_page_node_view(pc, &ms.title, &ms.node);
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    record_new_ministate(s);
    build_ministate(pc, s);
}
