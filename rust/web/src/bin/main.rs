use std::{
    cell::RefCell,
    collections::HashMap,
    panic,
    rc::Rc,
};
use gloo::{
    events::EventListener,
    utils::window,
};
use lunk::{
    link,
    EventGraph,
    HistPrim,
    ProcessingContext,
};
use rooting::{
    set_root,
    spawn_rooted,
    El,
};
use shared::{
    bb,
    model::{
        C2SReq,
        View,
    },
};
use wasm_bindgen::{
    UnwrapThrowExt,
};
use web::{
    async_::bg_val,
    el_general::{
        el_async,
        el_button_icon,
        el_button_icon_switch_auto,
        el_group,
        el_hbox,
        el_stack,
        el_vbox,
        log,
        CSS_GROW,
        ICON_CLOSE,
        ICON_MENU,
        ICON_NOMENU,
    },
    world::req_post_json,
};
use crate::mainlib::{
    ministate::Ministate,
    playlist,
    state::{
        self,
        build_ministate,
        el_ministate_button,
        State,
        State_,
    },
};

pub mod mainlib;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let origin = window().location().origin().unwrap_throw();
        let show_sidebar = HistPrim::new(pc, false);
        let mobile_vert_title_group = el_group().classes(&["s_vert_title"]);
        let title_group = el_group().classes(&["s_title"]);
        let body_group = el_group().classes(&["s_body"]);
        let (playlist_state, playlist_root) = playlist::state_new(pc);
        let views = bg_val({
            let origin = origin.clone();
            async move {
                match req_post_json::<HashMap<String, View>>(&origin, C2SReq::ViewsList).await {
                    Ok(q) => return Rc::new(RefCell::new(q)),
                    Err(e) => {
                        log(format!("Error retrieving stored views: {:?}", e));
                        return Rc::new(RefCell::new(HashMap::new()));
                    },
                };
            }
        });
        let stack = el_stack();
        let state = State::new(State_ {
            origin: origin.clone(),
            playlist: playlist_state,
            views: views,
            stack: stack.weak(),
            mobile_vert_title_group: mobile_vert_title_group.weak(),
            title_group: title_group.weak(),
            body_group: body_group.weak(),
        });
        let sidebar_group = el_group();

        fn restore_ministate(pc: &mut ProcessingContext, state: &State) {
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
        }

        restore_ministate(pc, &state);
        EventListener::new(&window(), "popstate", {
            let eg = pc.eg();
            let state = state.clone();
            move |_e| eg.event(|pc| {
                restore_ministate(pc, &state);
            })
        }).forget();
        stack.ref_push(el_hbox().classes(&["s_root", CSS_GROW]).extend(vec![
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
        ]).own(|e| (
            playlist_root,
            link!(
                //. .
                (pc = pc),
                (show_sidebar = show_sidebar.clone()),
                (),
                (root = e.weak(), sidebar_group = sidebar_group.weak(), state = state.clone()) {
                    let root = root.upgrade()?;
                    root.ref_modify_classes(&[("sidebar", *show_sidebar.borrow())]);
                    root.ref_modify_classes(&[("no_sidebar", !*show_sidebar.borrow())]);
                    let sidebar_group = sidebar_group.upgrade()?;
                    if *show_sidebar.borrow() {
                        sidebar_group.ref_push(
                            el_vbox().classes(&["s_sidebar", CSS_GROW]).push(el_async().own(|e| spawn_rooted({
                                let state = state.clone();
                                let eg = pc.eg();
                                let placeholder = e.weak();
                                async move {
                                    let views = state.views.get().await;
                                    let Some(placeholder) = placeholder.upgrade() else {
                                        return;
                                    };
                                    eg.event(|pc| {
                                        for (view_id, view) in &*views.borrow() {
                                            placeholder.ref_replace(
                                                vec![el_ministate_button(pc, &state, &view.name, Ministate::View {
                                                    id: view_id.clone(),
                                                    title: view.name.clone(),
                                                    pos: None,
                                                })],
                                            );
                                        }
                                    });
                                }
                            }))),
                        );
                    } else {
                        sidebar_group.ref_clear();
                    }
                }
            ),
            link!(
                (pc = pc),
                (playing_i = state.playlist.0.playing_i.clone(), playing = state.playlist.0.playing.clone()),
                (),
                (state = state.clone(), stack = stack.clone(), current = Rc::new(RefCell::new(None as Option<El>))) {
                    if !playing.get() {
                        return None;
                    }
                    if !(!playing.get_old() || playing_i.get() != playing_i.get_old()) {
                        return None;
                    }
                    let e = state.playlist.0.playlist.borrow().get(playing_i.get().unwrap()).cloned().unwrap();
                    if !e.media.pm_display() {
                        return None;
                    }
                    if let Some(current) = current.borrow_mut().take() {
                        current.ref_replace(vec![]);
                    }
                    let new_player = el_vbox().classes(&["s_player_modal"]);
                    new_player.ref_extend(
                        vec![el_hbox().classes(&["titlebar"]).extend(vec![el_button_icon(pc, ICON_CLOSE, "Close", {
                            let state = state.clone();
                            let current = Rc::downgrade(&current);
                            move |pc| {
                                let Some(current) = current.upgrade() else {
                                    return;
                                };
                                if let Some(current) = current.borrow_mut().take() {
                                    current.ref_replace(vec![]);
                                }
                                state.playlist.0.playing.set(pc, false);
                            }
                        })]), e.media.pm_el().clone()],
                    );
                    *current.borrow_mut() = Some(new_player.clone());
                    stack.ref_push(new_player);
                    _ = e.media.pm_media().request_fullscreen();
                }
            ),
        )));
        set_root(vec![stack]);
    });
}
