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
    },
    crate::{
        async_::BgVal,
        el_general::style_export,
        playlist::PlaylistState,
    },
    gloo::utils::{
        document,
    },
    lunk::ProcessingContext,
    rooting::{
        el_from_raw,
        El,
    },
    shared::interface::config::ClientConfig,
    std::rc::Rc,
};

pub struct State_ {
    // Ends with `/`
    pub base_url: String,
    pub playlist: PlaylistState,
    pub client_config: BgVal<Result<Rc<ClientConfig>, String>>,
    pub main_title: El,
    pub main_body: El,
    pub menu_body: El,
}

pub type State = Rc<State_>;

pub fn set_page(state: &State, title: &str, body: El) {
    document().set_title(title);
    state.main_title.ref_text(title);
    state.main_body.ref_clear();
    state.main_body.ref_push(body);
}

pub fn build_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    match s {
        Ministate::Home => {
            set_page(
                state,
                "Home",
                el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into()),
            );
        },
        Ministate::View(ms) => {
            build_page_view(state, &ms.title, &ms.id, &BuildPlaylistPos {
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
