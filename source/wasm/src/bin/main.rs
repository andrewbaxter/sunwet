use {
    web::{
        playlist,
        state::{
            Menu,
            build_ministate,
            el_ministate_button,
            State,
            State_,
        },
        ministate::{
            Ministate,
            MinistateForm,
            MinistateView,
        },
    },
    flowcontrol::{
        shed,
        superif,
    },
    gloo::{
        events::EventListener,
        utils::window,
    },
    lunk::{
        link,
        EventGraph,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el,
        set_root,
        spawn_rooted,
        El,
    },
    shared::interface::{
        config::{
            form::Form,
            menu::MenuItem,
            view::View,
        },
        wire::ReqGetMenu,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        panic,
        rc::Rc,
    },
    wasm_bindgen::{
        JsCast,
        UnwrapThrowExt,
    },
    web::{
        async_::bg_val,
        constants::LINK_HASH_PREFIX,
        el_general::{
            el_async,
            el_button_icon,
            el_err_block,
            el_group,
            el_hbox,
            el_icon,
            el_stack,
            el_vbox,
            log,
            CSS_STATE_GROW,
            CSS_S_BODY,
            CSS_S_MENU,
            CSS_S_ROOT,
            CSS_S_TITLE,
            CSS_S_VIEW,
            ICON_CLOSE,
        },
        main_link::main_link,
        world::req_post_json,
    },
    web_sys::HtmlElement,
};

fn get_dom_hash() -> Option<String> {
    let hash = window().location().hash().unwrap();
    let Some(s) = hash.strip_prefix("#") else {
        return None;
    };
    let s = match urlencoding::decode(s) {
        Ok(s) => s,
        Err(e) => {
            log(format!("Unable to url-decode anchor state: {:?}\nAnchor: {}", e, s));
            return None;
        },
    };
    return Some(s.to_string());
}

fn restore_ministate(pc: &mut ProcessingContext, state: &State) {
    build_ministate(pc, &state, &shed!{
        'ret_ministate _;
        shed!{
            let Some(s) = get_dom_hash() else {
                break;
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

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let base_url;
        {
            let loc = window().location();
            base_url = format!("{}{}", loc.origin().unwrap_throw(), loc.pathname().unwrap_throw());
        }

        // Short circuit to link mode
        superif!({
            let Some(hash) = get_dom_hash() else {
                break;
            };
            let Some(link_id) = hash.strip_prefix(LINK_HASH_PREFIX) else {
                break;
            };
            break 'is_link link_id.to_string();
        } link_id = 'is_link {
            main_link(pc, base_url, link_id);
            return;
        });

        // Non-link
        let page_title = el_group().classes(&[CSS_S_TITLE]);
        let page_body = el_group().classes(&[CSS_S_BODY]);
        let (playlist_state, playlist_root) = playlist::state_new(pc, base_url.clone());

        // Async load menu
        let menu = bg_val({
            let base_url = base_url.clone();
            async move {
                let menu = match req_post_json(&base_url, ReqGetMenu).await {
                    Ok(menu) => menu,
                    Err(e) => {
                        return Err(format!("Error retrieving menu: {:?}", e));
                    },
                };
                let mut views = HashMap::new();
                let mut forms = HashMap::new();

                fn walk_menu(views: &mut HashMap<String, View>, forms: &mut HashMap<String, Form>, i: &MenuItem) {
                    match i {
                        MenuItem::Section(s) => {
                            for c in &s.children {
                                walk_menu(views, forms, c);
                            }
                        },
                        MenuItem::View(v) => {
                            views.insert(v.id.clone(), v.clone());
                        },
                        MenuItem::Form(f) => {
                            forms.insert(f.id.clone(), f.clone());
                        },
                    }
                }

                for i in &menu {
                    walk_menu(&mut views, &mut forms, i);
                }
                return Ok(Rc::new(Menu {
                    menu: menu,
                    views: views,
                    forms: forms,
                }));
            }
        });

        // Build app state
        let stack = el_stack().classes(&[CSS_S_ROOT, CSS_STATE_GROW]);
        let state = State::new(State_ {
            base_url: base_url.clone(),
            playlist: playlist_state,
            menu: menu,
            stack: stack.weak(),
            page_title: page_title.weak(),
            page_body: page_body.weak(),
        });
        let menu_visible = Prim::new(true);

        // Load initial view
        restore_ministate(pc, &state);

        // React to further state changes
        EventListener::new(&window(), "popstate", {
            let eg = pc.eg();
            let state = state.clone();
            move |_e| eg.event(|pc| {
                restore_ministate(pc, &state);
            })
        }).forget();

        // Create view
        stack.ref_push(el_vbox().classes(&[CSS_S_VIEW]).extend(vec![
            //. .
            el_hbox().classes(&["s_titlebar"]).extend(vec![
                //. .
                el_button_icon(pc, el("img").attr("src", "logo_off.svg"), "Menu", {
                    let menu_visible = menu_visible.clone();
                    move |pc| {
                        menu_visible.set(pc, true);
                    }
                }),
                page_title
            ]),
            page_body
        ]));

        // Create menu
        stack.ref_push(el_vbox()
            .classes(&[CSS_S_MENU])
            .extend(vec![el_hbox().classes(&["s_titlebar"]).extend(vec![
                //. .
                el_button_icon(pc, el("img").attr("src", "logo_off.svg"), "Back", {
                    let menu_visible = menu_visible.clone();
                    move |pc| {
                        menu_visible.set(pc, false);
                    }
                }),
                el("h1").classes(&["Sunwet"])
            ]), el_async().own(|async_el| spawn_rooted({
                let state = state.clone();
                let eg = pc.eg();
                let async_el = async_el.weak();
                async move {
                    let menu = state.menu.get().await;
                    let Some(async_el) = async_el.upgrade() else {
                        return;
                    };
                    let menu = match menu {
                        Ok(m) => m,
                        Err(e) => {
                            async_el.ref_replace(vec![el_err_block(e)]);
                            return;
                        },
                    };
                    eg.event(|pc| {
                        let mut els = vec![];

                        fn build_menu_item(state: &State, pc: &mut ProcessingContext, i: &MenuItem) -> El {
                            match i {
                                MenuItem::Section(s) => {
                                    let out = el("details");
                                    out.ref_push(el("summary").text(&s.name));
                                    let mut children = vec![];
                                    for child in &s.children {
                                        children.push(build_menu_item(state, pc, &child));
                                    }
                                    out.ref_push(el("div").classes(&["g_menu_section_body"]).extend(children));
                                    return out;
                                },
                                MenuItem::View(view) => {
                                    return el_ministate_button(pc, &state, &view.name, Ministate::List(MinistateView {
                                        id: view.id.clone(),
                                        title: view.name.clone(),
                                        pos: None,
                                    }));
                                },
                                MenuItem::Form(form) => {
                                    return el_ministate_button(pc, &state, &form.name, Ministate::Form(MinistateForm {
                                        id: form.id.clone(),
                                        title: form.name.clone(),
                                    }));
                                },
                            }
                        }

                        for item in &menu.menu {
                            els.push(build_menu_item(&state, pc, item));
                        }
                        async_el.ref_replace(els);
                    });
                }
            }))])
            .own(|menu_el| link!((_pc = pc), (menu_visible = menu_visible.clone()), (), (menu_el = menu_el.weak()), {
                let menu_el = menu_el.upgrade()?;
                let style = menu_el.raw().dyn_into::<HtmlElement>().unwrap().style();
                match *menu_visible.borrow() {
                    true => {
                        style.remove_property("display").unwrap();
                    },
                    false => {
                        style.set_property("display", "none").unwrap();
                    },
                }
            })));

        // Set up playback handling (including making video overlay)
        stack.ref_own(
            |_| (
                playlist_root,
                link!(
                    (pc = pc),
                    (playing_i = state.playlist.0.playing_i.clone(), playing = state.playlist.0.playing.clone()),
                    (),
                    (
                        state = state.clone(),
                        stack = stack.clone(),
                        current = Rc::new(RefCell::new(None as Option<El>))
                    ) {
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
                            vec![
                                el_hbox()
                                    .classes(&["titlebar"])
                                    .extend(vec![el_button_icon(pc, el_icon(ICON_CLOSE), "Close", {
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
                                    })]),
                                e.media.pm_el().clone()
                            ],
                        );
                        *current.borrow_mut() = Some(new_player.clone());
                        stack.ref_push(new_player);
                        _ = e.media.pm_el().raw().request_fullscreen();
                    }
                ),
            ),
        );

        // Display
        set_root(vec![stack]);
    });
}
