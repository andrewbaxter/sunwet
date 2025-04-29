use {
    super::{
        ministate::{
            record_new_ministate,
            Ministate,
            PlaylistEntryPath,
        },
        page_edit::build_page_edit,
        page_form::build_page_form_by_id,
        page_view::{
            build_page_view,
            BuildPlaylistPos,
        },
        playlist::PlaylistState,
    },
    gloo::utils::document,
    lunk::{
        EventGraph,
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
        el_general::style_export,
    },
};

pub struct State_ {
    pub eg: EventGraph,
    pub ministate: RefCell<Ministate>,
    // Ends with `/`
    pub base_url: String,
    pub playlist: PlaylistState,
    pub client_config: BgVal<Result<Rc<ClientConfig>, String>>,
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

pub fn set_page(title: &str, body: El) {
    document().set_title(&format!("{} - Sunwet", title));
    let state = state();
    state.modal_stack.ref_clear();
    state.main_title.ref_text(title);
    state.main_body.ref_clear();
    state.main_body.ref_push(body);
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    match s {
        Ministate::Home => {
            set_page(
                "Home",
                el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into()),
            );
        },
        Ministate::View(ms) => {
            build_page_view(pc, &ms.title, &ms.menu_item_id, &BuildPlaylistPos {
                list_id: ms.menu_item_id.clone(),
                list_title: ms.title.clone(),
                entry_path: Some(PlaylistEntryPath(vec![])),
            }, &ms.pos);
        },
        Ministate::Form(ms) => {
            build_page_form_by_id(pc, &ms.title, &ms.menu_item_id);
        },
        Ministate::Edit(ms) => {
            build_page_edit(pc, &ms.title, &ms.node);
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    record_new_ministate(s);
    build_ministate(pc, s);
}
