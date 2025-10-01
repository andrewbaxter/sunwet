use {
    super::{
        ministate::{
            record_new_ministate,
            record_replace_ministate,
            Ministate,
            PlaylistRestorePos,
        },
        page_form::build_page_form,
        page_history::build_page_history,
        page_node_edit::build_page_node_edit,
        page_node_view::build_page_node_view,
        page_view::build_page_view,
        playlist::{
            playlist_clear,
            PlaylistState,
        },
    },
    crate::libnonlink::{
        ministate::MinistateView,
        page_query::build_page_query,
    },
    gloo::utils::document,
    lunk::{
        EventGraph,
        Prim,
        ProcessingContext,
    },
    rooting::El,
    shared::interface::{
        config::{
            view::ViewId,
            ClientConfig,
        },
        triple::Node,
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
            Log,
            VecLog,
        },
    },
};

pub struct State_ {
    pub eg: EventGraph,
    pub ministate: RefCell<Ministate>,
    pub env: Env,
    pub playlist: PlaylistState,
    pub client_config: BgVal<Result<Rc<ClientConfig>, String>>,
    pub menu_open: Prim<bool>,
    // Arcmutex due to OnceLock, should El use sync alternatives?
    pub main_title: El,
    pub main_body: El,
    pub modal_stack: El,
    pub log: Rc<dyn Log>,
    pub log1: Rc<VecLog>,
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
    set_page_(pc, "Home", true, style_export::cont_page_home().root);
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    match s {
        Ministate::Home => {
            set_page_(pc, "Home", true, style_export::cont_page_home().root);
        },
        Ministate::View(v) => {
            set_page(pc, &v.title, el_async_(true, {
                let title = v.title.clone();
                let view_id = v.id.clone();
                let pos = v.pos.clone();
                let params = v.params.clone();
                let eg = pc.eg();
                async move {
                    let client_config = state().client_config.get().await?;
                    let Some(view) = client_config.views.get(&view_id) else {
                        return Err(format!("No view with id [{}] in config", view_id));
                    };
                    return build_page_view(eg, view_id, title, view.clone(), params, pos).map(|x| vec![x]);
                }
            }));
        },
        Ministate::Form(f) => {
            set_page(pc, &f.title, el_async_(true, {
                let title = f.title.clone();
                let form_id = f.id.clone();
                let params = f.params.clone();
                let eg = pc.eg();
                async move {
                    let client_config = state().client_config.get().await?;
                    let Some(form) = client_config.forms.get(&form_id) else {
                        return Err(format!("No menu item with id [{}] in config", form_id));
                    };
                    return build_page_form(eg, form_id, title, form.clone(), params).map(|x| vec![x]);
                }
            }));
        },
        Ministate::NodeEdit(ms) => {
            build_page_node_edit(pc, &ms.title, &ms.nodes);
        },
        Ministate::NodeView(ms) => {
            build_page_node_view(pc, &ms.title, &ms.node);
        },
        Ministate::History(ms) => {
            build_page_history(pc, ms);
        },
        Ministate::Query(ms) => {
            build_page_query(pc, ms);
        },
        Ministate::Logs => {
            set_page(
                pc,
                "Logs",
                style_export::cont_page_logs(
                    style_export::ContPageLogsArgs {
                        children: state()
                            .log1
                            .log
                            .borrow()
                            .iter()
                            .map(|x| style_export::leaf_logs_line(style_export::LeafLogsLineArgs {
                                stamp: x.0.to_rfc3339(),
                                text: x.1.clone(),
                            }).root)
                            .collect::<Vec<_>>(),
                    },
                ).root,
            );
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    record_new_ministate(&state().log, s);
    build_ministate(pc, s);
}

pub struct MinistateViewState_ {
    pub view_id: ViewId,
    pub title: String,
    pub pos: Option<PlaylistRestorePos>,
    pub params: HashMap<String, Node>,
}

#[derive(Clone)]
pub struct MinistateViewState(pub Rc<RefCell<MinistateViewState_>>);

impl MinistateViewState {
    pub fn set_pos(&self, pos: Option<PlaylistRestorePos>) {
        let mut s = self.0.borrow_mut();
        s.pos = pos;
        record_replace_ministate(&state().log, &Ministate::View(MinistateView {
            id: s.view_id.clone(),
            title: s.title.clone(),
            pos: s.pos.clone(),
            params: s.params.clone(),
        }));
    }

    pub fn set_param(&self, k: String, v: Node) {
        let mut s = self.0.borrow_mut();
        s.params.insert(k, v);
        record_replace_ministate(&state().log, &Ministate::View(MinistateView {
            id: s.view_id.clone(),
            title: s.title.clone(),
            pos: s.pos.clone(),
            params: s.params.clone(),
        }));
    }
}
