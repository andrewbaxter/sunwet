use {
    crate::libnonlink::{
        ministate::MinistateOfflineView,
        node_button::STORAGE_CURRENT_LIST,
        offline::{
            list_offline_views,
            stop_offlining,
            trigger_offlining,
        },
        online::{
            stop_onlining,
            trigger_onlining,
        },
        seekbar::setup_seekbar,
        state::CurrentList,
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    gloo::{
        events::EventListener,
        storage::{
            LocalStorage,
            SessionStorage,
            Storage,
        },
        utils::{
            document,
            format::JsValueSerdeExt,
            window,
        },
    },
    libnonlink::{
        api::{
            redirect_login,
            redirect_logout,
            req_post_json,
            set_want_logged_in,
            unset_want_logged_in,
            want_logged_in,
        },
        ministate::{
            LOCALSTORAGE_PWA_MINISTATE,
            Ministate,
            MinistateForm,
            MinistateHistory,
            MinistateQuery,
            MinistateView,
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            ministate_octothorpe,
            read_ministate,
            record_replace_ministate,
        },
        page_view::LOCALSTORAGE_SHARE_SESSION_ID,
        playlist::{
            self,
            playlist_set_link,
        },
        state::{
            STATE,
            State_,
            build_ministate,
            state,
        },
    },
    lunk::{
        EventGraph,
        List,
        Prim,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        set_root,
    },
    serde::Deserialize,
    shared::{
        interface::{
            config::{
                ClientConfig,
                ClientMenuItem,
                ClientMenuItemDetail,
                ClientPage,
            },
            wire::{
                ReqGetClientConfig,
                ReqWhoAmI,
                RespWhoAmI,
            },
        },
        stringpattern::node_to_text,
    },
    std::{
        cell::RefCell,
        collections::BTreeMap,
        panic,
        rc::Rc,
    },
    wasm::{
        async_::{
            WaitVal,
        },
        js::{
            Log,
            LogJsErr,
            VecLog,
            el_async_,
            scan_env,
            style_export::{
                self,
            },
        },
    },
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::spawn_local,
    web_sys::{
        HtmlElement,
        HtmlInputElement,
        MessageEvent,
    },
};

pub mod libnonlink;

pub const LOCALSTORAGE_CLIENTCONFIG: &str = "clientconfig";

pub fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    let log1 = Rc::new(VecLog { log: Default::default() });
    let log = log1.clone() as Rc<dyn Log>;
    eg.event(|pc| {
        let env = scan_env(&log);
        let main_title_ = style_export::leaf_title(style_export::LeafTitleArgs { text: "Sunwet".to_string() });
        let main_title = main_title_.root;
        let main_body = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
        let modal_stack = style_export::cont_root_stack(style_export::ContRootStackArgs { children: vec![] }).root;

        // Build app state
        let (playlist_state, playlist_root) = playlist::state_new(pc, log.clone(), env.clone());
        STATE.with(|s| *s.borrow_mut() = Some(Rc::new(State_ {
            eg: pc.eg(),
            ministate: RefCell::new(shed!{
                'found _;
                shed!{
                    let m = match SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE) {
                        Ok(m) => m,
                        Err(e) => match e {
                            gloo::storage::errors::StorageError::KeyNotFound(_) => {
                                break;
                            },
                            gloo::storage::errors::StorageError::SerdeError(..) |
                            gloo::storage::errors::StorageError::JsError(..) => {
                                log.log(
                                    &format!("Error reading post-redirect ministate from session storage: {}", e),
                                );
                                break;
                            },
                        },
                    };
                    SessionStorage::delete(SESSIONSTORAGE_POST_REDIRECT_MINISTATE);
                    record_replace_ministate(&log, &m);
                    break 'found m;
                }
                shed!{
                    if !env.pwa {
                        break;
                    }
                    let m = match LocalStorage::get::<Ministate>(LOCALSTORAGE_PWA_MINISTATE) {
                        Ok(m) => m,
                        Err(e) => match e {
                            gloo::storage::errors::StorageError::KeyNotFound(_) => {
                                break;
                            },
                            gloo::storage::errors::StorageError::SerdeError(..) |
                            gloo::storage::errors::StorageError::JsError(..) => {
                                log.log(&format!("Error reading pwa ministate from local storage: {}", e));
                                break;
                            },
                        },
                    };
                    record_replace_ministate(&log, &m);
                }
                break 'found read_ministate(&log);
            }),
            menu_open: Prim::new(false),
            env: env,
            playlist: playlist_state,
            onlining: Prim::new(false),
            onlining_bg: Default::default(),
            offlining: Prim::new(false),
            offlining_bg: Default::default(),
            offline_list: List::new(vec![]),
            modal_stack: modal_stack.clone(),
            main_title: main_title.clone(),
            menu_page_buttons: style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root,
            main_body: main_body.clone(),
            client_config: WaitVal::new(),
            whoami: Prim::new(None),
            log1: log1,
            log: log.clone(),
            current_list: Prim::new(shed!{
                if let Ok(m) = SessionStorage::get::<CurrentList>(STORAGE_CURRENT_LIST) {
                    break Some(m);
                };
                if let Ok(m) = LocalStorage::get::<CurrentList>(STORAGE_CURRENT_LIST) {
                    break Some(m);
                };
                break None;
            }),
        })));
        spawn_local({
            let eg = pc.eg();
            async move {
                let offline = match list_offline_views().await {
                    Ok(l) => l,
                    Err(e) => {
                        state().log.log(&format!("Error listing existing offline views: {}", e));
                        return;
                    },
                };
                eg.event(|pc| {
                    state().offline_list.extend(pc, offline);
                }).unwrap();
            }
        });
        spawn_local({
            let eg = eg.clone();
            async move {
                match async {
                    ta_return!((), String);
                    if let Ok(c) = LocalStorage::get(LOCALSTORAGE_CLIENTCONFIG) {
                        let prim = Prim::new(Rc::new(c));
                        state().client_config.set(Some(prim.clone()));
                        let c = Rc::new(req_post_json(ReqGetClientConfig).await?);
                        LocalStorage::set(
                            LOCALSTORAGE_CLIENTCONFIG,
                            &c,
                        ).log(&state().log, "Error storing client config in localstorage");
                        eg.event(|pc| {
                            prim.set(pc, c);
                        }).unwrap();
                    } else {
                        let c = Rc::new(req_post_json(ReqGetClientConfig).await?);
                        LocalStorage::set(
                            LOCALSTORAGE_CLIENTCONFIG,
                            &c,
                        ).log(&state().log, "Error storing client config in localstorage");
                        state().client_config.set(Some(Prim::new(c)));
                    }
                    return Ok(());
                }.await {
                    Ok(_) => { },
                    Err(e) => {
                        state().log.log(&format!("Error requesting menu: {}", e));
                    },
                }
            }
        });
        spawn_local({
            let eg = eg.clone();
            async move {
                match async {
                    ta_return!((), String);
                    let whoami = req_post_json(ReqWhoAmI).await?;
                    if want_logged_in() && whoami == RespWhoAmI::Public {
                        redirect_login(&state().env.base_url);
                    }
                    eg.event(|pc| {
                        state().whoami.set(pc, Some(whoami));
                    }).unwrap();
                    return Ok(());
                }.await {
                    Ok(_) => { },
                    Err(e) => {
                        state().log.log(&format!("Error requesting user info: {}", e));
                    },
                }
            }
        });

        // Restore share state
        {
            if let Ok(sess_id) = LocalStorage::get::<String>(LOCALSTORAGE_SHARE_SESSION_ID) {
                playlist_set_link(pc, &state().playlist, &sess_id);
            };
        }

        // Load initial view
        build_ministate(pc, &state().ministate.borrow());

        // React to further state changes
        EventListener::new(&window(), "popstate", {
            let eg = pc.eg();
            move |_e| eg.event(|pc| {
                let ministate = read_ministate(&state().log);
                *state().ministate.borrow_mut() = ministate.clone();
                LocalStorage::set(
                    LOCALSTORAGE_PWA_MINISTATE,
                    ministate_octothorpe(&ministate),
                ).log(&state().log, "Error storing PWA ministate");
                build_ministate(pc, &ministate);
            }).unwrap()
        }).forget();
        const LOCALSTORAGE_ENABLE_ONLINING: &str = "enable_onlining";
        let enable_onlining = Prim::new(LocalStorage::get::<bool>(LOCALSTORAGE_ENABLE_ONLINING).unwrap_or(true));
        const LOCALSTORAGE_ENABLE_OFFLINING: &str = "enable_offlining";
        let enable_offlining = Prim::new(LocalStorage::get::<bool>(LOCALSTORAGE_ENABLE_OFFLINING).unwrap_or(true));

        // Root and display
        set_root(vec![style_export::cont_root_stack(style_export::ContRootStackArgs { children: vec![{
            let app_res = style_export::app_main(style_export::AppMainArgs {
                main_title: main_title,
                main_body: main_body,
                menu_body: {
                    let mut root = vec![];
                    root.push(el_async_(false, {
                        let eg = eg.clone();
                        async move {
                            ta_return!(Vec < El >, String);
                            let client_config = state().client_config.get().await;
                            return Ok(eg.event(|pc| {
                                let menu_dynamic =
                                    style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
                                menu_dynamic.ref_own(
                                    |g| link!((pc = pc), (client_config = client_config), (), (g = g.weak()) {
                                        fn build_menu_item(
                                            pc: &mut ProcessingContext,
                                            config: &ClientConfig,
                                            carry_titles: &Vec<String>,
                                            item: &ClientMenuItem,
                                        ) -> El {
                                            match &item.detail {
                                                ClientMenuItemDetail::Section(section) => {
                                                    let mut sub_carry_titles = carry_titles.clone();
                                                    sub_carry_titles.push(item.name.clone());
                                                    let mut children = vec![];
                                                    for child in &section.children {
                                                        children.push(
                                                            build_menu_item(pc, config, &sub_carry_titles, &child)
                                                        );
                                                    }
                                                    return style_export::cont_menu_group(
                                                        style_export::ContMenuGroupArgs {
                                                            title: item.name.clone(),
                                                            children: children,
                                                        }
                                                    ).root;
                                                },
                                                ClientMenuItemDetail::Page(page) => {
                                                    match page {
                                                        ClientPage::View(page) => {
                                                            return style_export::leaf_menu_link(
                                                                style_export::LeafMenuLinkArgs {
                                                                    title: item.name.clone(),
                                                                    href: ministate_octothorpe(
                                                                        &Ministate::View(MinistateView {
                                                                            id: page.view_id.clone(),
                                                                            title: format!(
                                                                                "{}, {}",
                                                                                carry_titles.join(", "),
                                                                                item.name
                                                                            ),
                                                                            pos: None,
                                                                            params: page
                                                                                .parameters
                                                                                .iter()
                                                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                                                .collect(),
                                                                        })
                                                                    ),
                                                                }
                                                            ).root;
                                                        },
                                                        ClientPage::Form(page) => {
                                                            return style_export::leaf_menu_link(
                                                                style_export::LeafMenuLinkArgs {
                                                                    title: item.name.clone(),
                                                                    href: ministate_octothorpe(
                                                                        &Ministate::Form(MinistateForm {
                                                                            id: page.form_id.clone(),
                                                                            title: format!(
                                                                                "{}, {}",
                                                                                carry_titles.join(", "),
                                                                                item.name
                                                                            ),
                                                                            params: page
                                                                                .parameters
                                                                                .iter()
                                                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                                                .collect(),
                                                                        })
                                                                    ),
                                                                }
                                                            ).root;
                                                        },
                                                        ClientPage::History => {
                                                            return style_export::leaf_menu_link(
                                                                style_export::LeafMenuLinkArgs {
                                                                    title: "History".to_string(),
                                                                    href: ministate_octothorpe(
                                                                        &Ministate::History(
                                                                            MinistateHistory::default()
                                                                        ),
                                                                    ),
                                                                }
                                                            ).root;
                                                        },
                                                        ClientPage::Query => {
                                                            return style_export::leaf_menu_link(
                                                                style_export::LeafMenuLinkArgs {
                                                                    title: "Query".to_string(),
                                                                    href: ministate_octothorpe(
                                                                        &Ministate::Query(
                                                                            MinistateQuery { query: None }
                                                                        ),
                                                                    ),
                                                                }
                                                            ).root;
                                                        },
                                                    }
                                                },
                                            }
                                        }

                                        let g = g.upgrade()?;
                                        let client_config = client_config.borrow().clone();
                                        g.ref_clear();
                                        for item in &client_config.menu {
                                            g.ref_push(build_menu_item(pc, &client_config, &vec![], item));
                                        }
                                    })
                                );
                                return vec![menu_dynamic];
                            }).unwrap());
                        }
                    }));
                    root.push({
                        let group = style_export::cont_menu_group(style_export::ContMenuGroupArgs {
                            title: format!("Offline"),
                            children: vec![]
                        });
                        group
                            .body
                            .ref_own(
                                |b| link!((_pc = pc), (offline = state().offline_list.clone()), (), (b = b.weak(),) {
                                    let b = b.upgrade()?;

                                    fn build(key: &String, view: &MinistateView) -> El {
                                        let sorted_params = view.params.iter().collect::<BTreeMap<_, _>>();
                                        return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                            title: format!(
                                                "{}: {}",
                                                view.title,
                                                sorted_params
                                                    .iter()
                                                    .map(|(k, v)| format!("{}={}", k, node_to_text(&v)))
                                                    .collect::<Vec<_>>()
                                                    .join(", ")
                                            ),
                                            href: ministate_octothorpe(&Ministate::OfflineView(MinistateOfflineView {
                                                id: view.id.clone(),
                                                pos: view.pos.clone(),
                                                key: key.clone(),
                                                title: view.title.clone(),
                                                params: view.params.clone(),
                                            }))
                                        }).root;
                                    }

                                    if b.raw().children().length() == 0 {
                                        let mut add = vec![];
                                        for (key, view) in offline.borrow_values().iter() {
                                            add.push(build(key, view));
                                        }
                                        b.ref_extend(add);
                                    } else {
                                        for change in offline.borrow_changes().iter() {
                                            b.ref_splice(
                                                change.offset,
                                                change.remove,
                                                change.add.iter().map(|(k, v)| build(k, v)).collect()
                                            );
                                        }
                                    }
                                })
                            );
                        group.root
                    });
                    root.push({
                        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                            title: "Logs".to_string(),
                            href: ministate_octothorpe(&Ministate::Logs),
                        }).root
                    });
                    let mut bar_children = vec![];
                    bar_children.push(
                        style_export::cont_group(style_export::ContGroupArgs { children: vec![] })
                            .root
                            .own(|g| link!((pc = pc), (whoami = state().whoami.clone()), (), (g = g.weak()) {
                                let g = g.upgrade()?;
                                let whoami = whoami.borrow().clone();
                                let whoami = whoami.as_ref()?;
                                match whoami {
                                    RespWhoAmI::Public => {
                                        let button = style_export::leaf_menu_bar_button_login().root;
                                        button.ref_on("click", {
                                            let eg = pc.eg();
                                            move |_| eg.event(|_pc| {
                                                set_want_logged_in();
                                                redirect_login(&state().env.base_url);
                                            }).unwrap()
                                        });
                                        g.ref_push(button);
                                    },
                                    RespWhoAmI::User(_) => {
                                        let button = style_export::leaf_menu_bar_button_logout().root;
                                        button.ref_on("click", {
                                            let eg = pc.eg();
                                            move |_| eg.event(|_pc| {
                                                unset_want_logged_in();
                                                redirect_logout(&state().env.base_url);
                                            }).unwrap()
                                        });
                                        g.ref_push(button);
                                    },
                                    RespWhoAmI::Token => { },
                                }
                            }))
                    );
                    let menu_body = style_export::cont_menu_body(style_export::ContMenuBodyArgs {
                        children: root,
                        bar_children: bar_children,
                        page_button_children: vec![state().menu_page_buttons.clone()],
                    });
                    menu_body.user.ref_own(|u| link!((_pc = pc), (whoami = state().whoami.clone()), (), (u = u.weak()) {
                        let u = u.upgrade()?;
                        let whoami = whoami.borrow().clone();
                        let whoami = whoami.as_ref()?;
                        u.ref_text(&match whoami {
                            RespWhoAmI::Public => "Guest".to_string(),
                            RespWhoAmI::User(u) => u.clone(),
                            RespWhoAmI::Token => "Token".to_string(),
                        });
                    }));
                    for (
                        root,
                        checkbox,
                        enable,
                        thinking,
                    ) in [
                        (
                            menu_body.onlining,
                            menu_body.onlining_checkbox,
                            enable_onlining.clone(),
                            state().onlining.clone(),
                        ),
                        (
                            menu_body.offlining,
                            menu_body.offlining_checkbox,
                            enable_offlining.clone(),
                            state().offlining.clone(),
                        ),
                    ] {
                        checkbox.raw().dyn_ref::<HtmlInputElement>().unwrap().set_checked(*enable.borrow());
                        checkbox.ref_on("change", {
                            let b = checkbox.weak();
                            let enable = enable.clone();
                            let eg = pc.eg();
                            move |_| {
                                let Some(b) = b.upgrade() else {
                                    return;
                                };
                                let b = b.raw().dyn_into::<HtmlInputElement>().unwrap();
                                eg.event(|pc| {
                                    enable.set(pc, b.checked());
                                }).unwrap();
                            }
                        });
                        root.own(|e_| link!((_pc = pc), (thinking = thinking), (), (e_ = e_.weak()) {
                            let e_ = e_.upgrade()?;
                            e_.ref_modify_classes(
                                &[(&style_export::class_state_thinking().value, *thinking.borrow())]
                            )
                        }));
                    }
                    menu_body.root
                }
            });
            app_res.admenu_button.on("click", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    let current_open = *state().menu_open.borrow();
                    state().menu_open.set(pc, !current_open);
                }).unwrap()
            });
            app_res.root
        }, modal_stack.clone()] }).root.own(|_| (
            //. .
            playlist_root,
            EventListener::new(&window(), "message", |ev| {
                let ev = ev.dyn_ref::<MessageEvent>().unwrap();

                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case", deny_unknown_fields)]
                enum Message {
                    Log(String),
                    Reload,
                }

                let message = match JsValueSerdeExt::into_serde::<Message>(&ev.data()) {
                    Ok(m) => m,
                    Err(e) => {
                        state().log.log(&format!("Error parsing js message: {}", e));
                        return;
                    },
                };
                match message {
                    Message::Log(m) => {
                        state().log.log(&format!("From service worker: {}", m));
                    },
                    Message::Reload => {
                        window()
                            .location()
                            .reload()
                            .log(&state().log, "Error executing reload triggered by web worker.");
                    },
                }
            }),
            link!((_pc = pc), (playing_i = state().playlist.0.playing_i.clone()), (), () {
                let class = style_export::class_state_element_selected().value;
                {
                    let old_focused = document().get_elements_by_class_name(&class);
                    let mut old_focused1 = vec![];
                    for i in 0 .. old_focused.length() {
                        old_focused1.push(old_focused.item(i).unwrap());
                    }
                    for o in old_focused1 {
                        o
                            .class_list()
                            .remove_1(&class)
                            .log(&state().log, "Error removing selected class from play button");
                    }
                }
                if let Some(e_i) = playing_i.get() {
                    let e = state().playlist.0.playlist.borrow().get(&e_i).cloned().unwrap();
                    for b in &e.play_buttons {
                        b
                            .class_list()
                            .add_1(&class)
                            .log(&state().log, "Error setting selected class from play button");
                    }
                }
            }),
            link!(
                (pc = pc),
                (playing_i = state().playlist.0.playing_i.clone(), playing = state().playlist.0.playing.clone()),
                (),
                (modal_stack = modal_stack.weak(), current = Rc::new(RefCell::new(None as Option<El>))) {
                    if !playing.get() {
                        return None;
                    }
                    if !(!playing.get_old() || playing_i.get() != playing_i.get_old()) {
                        return None;
                    }
                    let Some(modal_stack) = modal_stack.upgrade() else {
                        return None;
                    };
                    let e = state().playlist.0.playlist.borrow().get(&playing_i.get().unwrap()).cloned().unwrap();
                    if !e.media.pm_display() {
                        return None;
                    }
                    if let Some(current) = current.borrow_mut().take() {
                        current.ref_replace(vec![]);
                    }
                    let media_display = e.media.pm_el(&state().log, pc);
                    let media_display_raw = media_display.raw();
                    let modal =
                        style_export::cont_media_fullscreen(
                            style_export::ContMediaFullscreenArgs { media: media_display },
                        );
                    setup_seekbar(pc, modal.seekbar, modal.seekbar_fill, modal.seekbar_label);
                    modal.button_close.on("click", {
                        let current = Rc::downgrade(&current);
                        let eg = pc.eg();
                        move |_| eg.event(|pc| {
                            let Some(current) = current.upgrade() else {
                                return;
                            };
                            if let Some(current) = current.borrow_mut().take() {
                                current.ref_replace(vec![]);
                            }
                            state().playlist.0.playing.set(pc, false);
                        }).unwrap()
                    });
                    modal.button_fullscreen.on("click", {
                        let media_display_raw = media_display_raw.clone();
                        move |_| {
                            media_display_raw
                                .request_fullscreen()
                                .log(&state().log, "Error making media display fullscreen");
                        }
                    });
                    let modal = modal.root;
                    *current.borrow_mut() = Some(modal.clone());
                    modal_stack.ref_push(modal);
                    media_display_raw.request_fullscreen().log(&state().log, "Error making media display fullscreen");
                }
            ),
            link!((_pc = pc), (menu_open = state().menu_open.clone()), (), () {
                let new_open = *menu_open.borrow();
                let state_open = style_export::class_menu_state_open().value;
                let x = document().get_elements_by_class_name(&style_export::class_menu_want_state_open().value);
                let mut y = vec![];
                for i in 0 .. x.length() {
                    y.push(x.item(i).unwrap().dyn_into::<HtmlElement>().unwrap());
                }
                for ele in y {
                    ele.class_list().toggle_with_force(&state_open, new_open).unwrap();
                }
            }),
            link!((pc = pc), (enable_onlining = enable_onlining), (), () {
                let v = *enable_onlining.borrow();
                LocalStorage::set(LOCALSTORAGE_ENABLE_ONLINING, v).log(&state().log, "Error storing onlining setting");
                if v {
                    trigger_onlining(pc.eg());
                } else {
                    stop_onlining();
                }
            }),
            link!((pc = pc), (enable_offlining = enable_offlining), (), () {
                let v = *enable_offlining.borrow();
                LocalStorage::set(
                    LOCALSTORAGE_ENABLE_OFFLINING,
                    v
                ).log(&state().log, "Error storing offlining setting");
                if v {
                    trigger_offlining(pc.eg());
                } else {
                    stop_offlining();
                }
            }),
        ))]);
    });
}
