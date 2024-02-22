use std::{
    any::Any,
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    panic,
    rc::Rc,
    str::FromStr,
};
use el_general::{
    el_button_icon_switch,
    el_button_icon_switch_auto,
    el_button_icon_text,
    el_group,
    el_hbox,
    el_vbox,
    log,
};
use futures::{
    Future,
    FutureExt,
};
use gloo::{
    console::{
        console,
        log,
        warn,
    },
    utils::{
        document,
        window,
    },
};
use js_sys::Function;
use lunk::{
    link,
    EventGraph,
    HistPrim,
    Prim,
    ProcessingContext,
};
use ministate::Ministate;
use page_query::{
    build_page_view,
    definition::{
        LayoutIndividual,
        WidgetList,
        WidgetNest,
    },
};
use playlist::PlaylistState;
use reqwasm::http::Request;
use rooting::{
    el,
    set_root,
    spawn_rooted,
    El,
    ScopeValue,
};
use rooting_forms::{
    BigString,
    Form,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use shared::{
    bb,
    model::{
        C2SReq,
        FileHash,
        Node,
        Query,
    },
    unenum,
};
use state::{
    build_ministate,
    change_ministate,
    State,
    State_,
    View,
};
use tokio::sync::{
    broadcast,
    OnceCell,
};
use util::{
    bg_val,
    CSS_GROW,
    ICON_MENU,
    ICON_NOMENU,
};
use wasm_bindgen::{
    closure::Closure,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    HtmlAudioElement,
    HtmlMediaElement,
    MediaMetadata,
    MediaSession,
    Url,
};
use world::req_post_json;
use crate::{
    ministate::{
        PlaylistEntryPath,
        PlaylistPos,
    },
    state::el_ministate_button,
    testdata::testdata_albums,
};

pub mod el_general;
pub mod page_query;
pub mod testdata;
pub mod playlist;
pub mod state;
pub mod world;
pub mod util;
pub mod ministate;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let origin = window().location().origin().unwrap_throw();
        let show_sidebar = Prim::new(pc, false);
        let mobile_vert_title_group = el_group().classes(&["s_vert_title"]);
        let title_group = el_group().classes(&["s_title"]);
        let body_group = el_group().classes(&["s_body"]);
        let (playlist_state, playlist_root) = playlist::state_new(pc);
        let views = bg_val({
            let origin = origin.clone();
            async move {
                //.                match req_post_json::<HashMap<usize, state::View>>(&origin, C2SReq::ListStoredViews).await {
                //.                    Ok(q) => return Rc::new(RefCell::new(q)),
                //.                    Err(e) => {
                //.                        log(format!("Error retrieving stored views: {:?}", e));
                //.                        return Rc::new(RefCell::new(HashMap::new()));
                //.                    },
                //.                };
                return Rc::new(RefCell::new({
                    let mut m = HashMap::new();
                    m.insert("albums".to_string(), View {
                        name: "Albums".to_string(),
                        def: testdata_albums(),
                    });
                    m
                }));
            }
        });
        let state = State::new(State_ {
            origin: origin.clone(),
            playlist: playlist_state,
            views: views,
            mobile_vert_title_group: mobile_vert_title_group.weak(),
            title_group: title_group.weak(),
            body_group: body_group.weak(),
        });
        let sidebar_group = el_group();
        build_ministate(pc, &state, &bb!{
            'ret_ministate _;
            bb!{
                let hash = window().location().hash().unwrap();
                let Some(s) = hash.strip_prefix("#") else {
                    break;
                };
                let s = match urlencoding::decode(s) {
                    Ok(s) => s,
                    Err(e) => {
                        log(format!("Unable to url-decode anchor state: {:?}\nAnchor: {}", e, s));
                        break;
                    },
                };
                let s = match serde_json::from_str::<Ministate>(s.as_ref()) {
                    Ok(s) => s,
                    Err(e) => {
                        log(format!("Unable to parse url anchor state: {:?}\nAnchor: {}", e, s));
                        break;
                    },
                };
                break 'ret_ministate s;
            }
            break 'ret_ministate Ministate::Home;
        });
        set_root(vec![el_hbox().classes(&["s_root", CSS_GROW]).extend(vec![
            // Sidebar
            sidebar_group.clone(),
            // Main content
            el_vbox().classes(&["s_main", CSS_GROW]).extend(vec![
                //. .
                el_hbox()
                    .classes(&["s_titlebar"])
                    .extend(
                        vec![
                            el_button_icon_switch_auto(
                                pc,
                                ICON_MENU,
                                "Show menu",
                                ICON_NOMENU,
                                "Hide menu",
                                &show_sidebar,
                            ),
                            mobile_vert_title_group,
                            title_group
                        ],
                    ),
                body_group
            ])
        ]).own(|e| (playlist_root, link!(
            //. .
            (pc = pc),
            (show_sidebar = show_sidebar.clone()),
            (),
            (root = e.weak(), sidebar_group = sidebar_group.weak(), state = state.clone(), bg = Cell::new(None)) {
                let root = root.upgrade()?;
                root.ref_modify_classes(&[("sidebar", *show_sidebar.borrow())]);
                root.ref_modify_classes(&[("no_sidebar", !*show_sidebar.borrow())]);
                let sidebar_group = sidebar_group.upgrade()?;
                if *show_sidebar.borrow() {
                    let sidebar = el_vbox().classes(&["s_sidebar", CSS_GROW]);
                    bg.set(Some(spawn_rooted({
                        let state = state.clone();
                        let eg = pc.eg();
                        let sidebar = sidebar.clone();
                        async move {
                            let views = state.views.get().await;
                            eg.event(|pc| {
                                for (view_id, view) in &*views.borrow() {
                                    sidebar.ref_push(el_ministate_button(pc, &state, &view.name, Ministate::View {
                                        id: view_id.clone(),
                                        title: view.name.clone(),
                                        pos: None,
                                    }));
                                }
                            });
                        }
                    })));
                    sidebar_group.ref_push(sidebar);
                } else {
                    sidebar_group.ref_clear();
                }
            }
        )))]);
    });
}
