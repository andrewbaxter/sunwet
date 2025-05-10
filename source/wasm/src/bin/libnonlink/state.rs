use {
    super::{
        ministate::{
            record_new_ministate,
            Ministate,
        },
        page_form::{
            build_page_form,
        },
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
    shared::interface::config::{
        menu::ClientMenuItem,
        ClientConfig,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm::{
        async_::BgVal,
        js::{
            el_async_,
            style_export,
            Env,
        },
    },
};

pub struct State_ {
    pub eg: EventGraph,
    pub ministate: RefCell<Ministate>,
    pub env: Env,
    pub playlist: PlaylistState,
    pub client_config: BgVal<Result<Rc<(ClientConfig, HashMap<String, ClientMenuItem>)>, String>>,
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

pub fn build_home_page(pc: &mut ProcessingContext) {
    set_page_(pc, "Home", true, el_from_raw(style_export::cont_page_home().root.into()));
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    match s {
        Ministate::Home => {
            set_page_(pc, "Home", true, el_from_raw(style_export::cont_page_home().root.into()));
        },
        Ministate::MenuItem(ms) => {
            set_page(pc, &ms.title, el_async_(true, {
                let title = ms.title.clone();
                let menu_item_id = ms.menu_item_id.clone();
                let pos = ms.pos.clone();
                let eg = pc.eg();
                async move {
                    let client_config = state().client_config.get().await?;
                    let Some(menu_item) = client_config.1.get(&menu_item_id) else {
                        return Err(format!("No menu item with id [{}] in config", menu_item_id));
                    };
                    match menu_item {
                        ClientMenuItem::Section(_) => {
                            return Err(format!("Menu item [{}] is a section, nothing to display.", menu_item_id));
                        },
                        ClientMenuItem::View(menu_item) => {
                            return build_page_view(eg, &client_config.0, title, menu_item.clone(), pos);
                        },
                        ClientMenuItem::Form(menu_item) => {
                            return build_page_form(eg, client_config.0.clone(), title, menu_item.clone());
                        },
                    }
                }
            }));
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
