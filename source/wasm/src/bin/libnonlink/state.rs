use {
    super::{
        ministate::{
            Ministate,
            PlaylistRestorePos,
            record_replace_ministate,
        },
        page_form::build_page_form,
        page_history::build_page_history,
        page_node_edit::build_page_node_edit,
        page_node_view::build_page_node_view,
        page_view::build_page_view,
        playlist::{
            PlaylistState,
            playlist_clear,
        },
    },
    crate::libnonlink::{
        ministate::{
            LOCALSTORAGE_PWA_MINISTATE,
            MinistateOfflineView,
            MinistateView,
            ministate_octothorpe,
        },
        page_list_edit::build_page_list_edit,
        page_query::build_page_query,
    },
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::{
            document,
            window,
        },
    },
    lunk::{
        EventGraph,
        List,
        Prim,
        ProcessingContext,
    },
    rooting::{
        El,
        ScopeValue,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        config::{
            ClientConfig,
            view::ViewId,
        },
        triple::Node,
        wire::RespWhoAmI,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm::{
        async_::WaitVal,
        js::{
            Env,
            Log,
            LogJsErr,
            VecLog,
            el_async_,
            style_export,
        },
    },
    wasm_bindgen::JsValue,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CurrentList {
    pub name: String,
    pub node: Node,
}

pub struct State_ {
    pub eg: EventGraph,
    pub ministate: RefCell<Ministate>,
    pub env: Env,
    pub playlist: PlaylistState,
    pub onlining: Prim<bool>,
    pub onlining_bg: RefCell<Option<ScopeValue>>,
    pub offlining: Prim<bool>,
    pub offlining_bg: RefCell<Option<ScopeValue>>,
    pub offline_list: List<(String, MinistateView)>,
    pub client_config: WaitVal<Prim<Rc<ClientConfig>>>,
    pub whoami: Prim<Option<RespWhoAmI>>,
    pub menu_open: Prim<bool>,
    pub main_title: El,
    pub menu_page_buttons: El,
    pub main_body: El,
    pub modal_stack: El,
    pub log: Rc<dyn Log>,
    pub log1: Rc<VecLog>,
    pub current_list: Prim<Option<CurrentList>>,
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
    state().menu_page_buttons.ref_clear();
    match s {
        Ministate::Home => {
            playlist_clear(pc, &state().playlist, false);
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
                    let client_config = state().client_config.get().await.borrow().clone();
                    let Some(view) = client_config.views.get(&view_id) else {
                        return Err(format!("No view with id [{}] in config", view_id));
                    };
                    return build_page_view(eg, view_id, title, view.clone(), params, pos, None).map(|x| vec![x]);
                }
            }));
        },
        Ministate::OfflineView(v) => {
            set_page(pc, &v.title, el_async_(true, {
                let title = v.title.clone();
                let view_id = v.id.clone();
                let pos = v.pos.clone();
                let params = v.params.clone();
                let key = v.key.clone();
                let eg = pc.eg();
                async move {
                    let client_config = state().client_config.get().await.borrow().clone();
                    let Some(view) = client_config.views.get(&view_id) else {
                        return Err(format!("No view with id [{}] in config", view_id));
                    };
                    return build_page_view(
                        eg,
                        view_id,
                        title,
                        view.clone(),
                        params,
                        pos,
                        Some(key),
                    ).map(|x| vec![x]);
                }
            }));
        },
        Ministate::Form(f) => {
            playlist_clear(pc, &state().playlist, false);
            set_page(pc, &f.title, el_async_(true, {
                let title = f.title.clone();
                let form_id = f.id.clone();
                let params = f.params.clone();
                let eg = pc.eg();
                async move {
                    let client_config = state().client_config.get().await.borrow().clone();
                    let Some(form) = client_config.forms.get(&form_id) else {
                        return Err(format!("No menu item with id [{}] in config", form_id));
                    };
                    return build_page_form(eg, form_id, title, form.clone(), params).map(|x| vec![x]);
                }
            }));
        },
        Ministate::NodeEdit(ms) => {
            playlist_clear(pc, &state().playlist, false);
            build_page_node_edit(pc, &ms.title, &ms.nodes);
        },
        Ministate::NodeView(ms) => {
            playlist_clear(pc, &state().playlist, false);
            build_page_node_view(pc, &ms.title, &ms.node);
        },
        Ministate::ListEdit(ms) => {
            playlist_clear(pc, &state().playlist, false);
            build_page_list_edit(pc, &ms.title, &ms.node);
        },
        Ministate::History(ms) => {
            playlist_clear(pc, &state().playlist, false);
            build_page_history(pc, ms);
        },
        Ministate::Query(ms) => {
            playlist_clear(pc, &state().playlist, false);
            build_page_query(pc, ms);
        },
        Ministate::Logs => {
            playlist_clear(pc, &state().playlist, false);
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
                            .rev()
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

pub fn goto_replace_ministate(pc: &mut ProcessingContext, log: &Rc<dyn Log>, s: &Ministate) {
    window()
        .history()
        .unwrap()
        .push_state_with_url(&JsValue::null(), "", Some(&serde_json::to_string(s).unwrap()))
        .log(log, &"Error pushing history");
    log.log(&format!("DEBUG set ministate to (goto): {}", serde_json::to_string(&s).unwrap()));
    LocalStorage::set(LOCALSTORAGE_PWA_MINISTATE, s).log(log, &"Error storing PWA ministate");
    build_ministate(pc, s);
}

pub struct MinistateViewState_ {
    pub view_id: ViewId,
    pub title: String,
    pub pos: Option<PlaylistRestorePos>,
    pub params: HashMap<String, Node>,
    pub offline: Option<String>,
}

#[derive(Clone)]
pub struct MinistateViewState(pub Rc<RefCell<MinistateViewState_>>);

impl MinistateViewState {
    pub fn set_pos(&self, pos: Option<PlaylistRestorePos>) {
        let mut s = self.0.borrow_mut();
        s.pos = pos;
        if let Some(key) = &s.offline {
            record_replace_ministate(&state().log, &Ministate::OfflineView(MinistateOfflineView {
                id: s.view_id.clone(),
                title: s.title.clone(),
                pos: s.pos.clone(),
                params: s.params.clone(),
                key: key.clone(),
            }));
        } else {
            record_replace_ministate(&state().log, &Ministate::View(MinistateView {
                id: s.view_id.clone(),
                title: s.title.clone(),
                pos: s.pos.clone(),
                params: s.params.clone(),
            }));
        }
    }

    pub fn set_param(&self, k: String, v: Node) {
        let mut s = self.0.borrow_mut();
        s.params.insert(k, v);
        if let Some(key) = &s.offline {
            record_replace_ministate(&state().log, &Ministate::OfflineView(MinistateOfflineView {
                id: s.view_id.clone(),
                title: s.title.clone(),
                pos: s.pos.clone(),
                params: s.params.clone(),
                key: key.clone(),
            }));
        } else {
            record_replace_ministate(&state().log, &Ministate::View(MinistateView {
                id: s.view_id.clone(),
                title: s.title.clone(),
                pos: s.pos.clone(),
                params: s.params.clone(),
            }));
        }
    }
}
