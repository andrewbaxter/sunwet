use {
    flowcontrol::superif,
    gloo::{
        events::EventListener,
        storage::{
            SessionStorage,
            Storage,
        },
        utils::{
            document,
            window,
        },
    },
    libnonlink::{
        api::req_post_json,
        ministate::{
            ministate_octothorpe,
            read_ministate,
            record_replace_ministate,
            Ministate,
            MinistateForm,
            MinistateView,
            SESSIONSTORAGE_POST_REDIRECT,
        },
        playlist::{
            self,
        },
        state::{
            build_ministate,
            state,
            State_,
            STATE,
        },
    },
    lunk::{
        link,
        EventGraph,
        Prim,
    },
    rooting::{
        el_from_raw,
        set_root,
        El,
    },
    shared::interface::{
        config::{
            menu::ClientMenuItem,
            ClientConfig,
        },
        wire::ReqGetClientConfig,
    },
    std::{
        cell::RefCell,
        panic,
        rc::Rc,
    },
    wasm::{
        async_::bg_val,
        js::{
            el_async,
            style_export::{
                self,
            },
        },
    },
    wasm_bindgen::{
        JsCast,
        UnwrapThrowExt,
    },
    web_sys::HtmlElement,
};

pub mod libnonlink;

pub fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let base_url;
        {
            let loc = window().location();
            base_url = format!("{}{}", loc.origin().unwrap_throw(), loc.pathname().unwrap_throw());
        }
        let stack =
            el_from_raw(style_export::cont_stack(style_export::ContStackArgs { children: vec![] }).root.into());
        let modal_stack =
            el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into());
        let main_body =
            el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into());
        let client_config = bg_val({
            let base_url = base_url.clone();
            async move {
                let config = match req_post_json(&base_url, ReqGetClientConfig).await {
                    Ok(menu) => menu,
                    Err(e) => {
                        return Err(format!("Error retrieving menu: {:?}", e));
                    },
                };
                return Ok(Rc::new(config));
            }
        });
        let menu_body = el_async({
            let client_config = client_config.clone();
            async move {
                let client_config = client_config.get().await?;

                fn build_menu_item(config: &ClientConfig, item: &ClientMenuItem) -> HtmlElement {
                    match item {
                        ClientMenuItem::Section(item) => {
                            let mut children = vec![];
                            for child in &item.children {
                                children.push(build_menu_item(config, &child));
                            }
                            return style_export::cont_menu_group(style_export::ContMenuGroupArgs {
                                title: item.name.clone(),
                                children: children,
                            }).root;
                        },
                        ClientMenuItem::View(item) => {
                            return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                title: item.name.clone(),
                                href: ministate_octothorpe(&Ministate::View(MinistateView {
                                    menu_item_id: item.id.clone(),
                                    title: item.name.clone(),
                                    pos: None,
                                })),
                            }).root;
                        },
                        ClientMenuItem::Form(item) => {
                            return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                title: item.name.clone(),
                                href: ministate_octothorpe(&Ministate::Form(MinistateForm {
                                    menu_item_id: item.id.clone(),
                                    title: item.name.clone(),
                                })),
                            }).root;
                        },
                    }
                }

                let mut root = vec![];
                for item in &client_config.menu {
                    root.push(build_menu_item(&client_config, item));
                }
                return Ok(
                    el_from_raw(
                        style_export::cont_menu_body(style_export::ContMenuBodyArgs { children: root }).root.into(),
                    ),
                ) as
                    Result<_, String>;
            }
        });
        let main_title =
            el_from_raw(
                style_export::leaf_title(style_export::LeafTitleArgs { text: "Sunwet".to_string() }).root.into(),
            );
        let root_res = style_export::app_main(style_export::AppMainArgs {
            main_title: main_title.raw().dyn_into().unwrap(),
            main_body: main_body.raw().dyn_into().unwrap(),
            menu_body: menu_body.raw().dyn_into().unwrap(),
        });
        let admenu_button = el_from_raw(root_res.admenu_button.into()).on("click", {
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                let current_open = *state().menu_open.borrow();
                state().menu_open.set(pc, !current_open);
            }).unwrap()
        });
        let root =
            el_from_raw(
                root_res.root.into(),
            ).own(|_| (main_title.clone(), main_body.clone(), menu_body.clone(), admenu_button));
        let (playlist_state, playlist_root) = playlist::state_new(pc, base_url.clone());

        // Build app state
        STATE.with(|s| *s.borrow_mut() = Some(Rc::new(State_ {
            eg: pc.eg(),
            ministate: RefCell::new(superif!({
                let Ok(m) = SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT) else {
                    break 'not_redirect;
                };
                SessionStorage::delete(SESSIONSTORAGE_POST_REDIRECT);
                record_replace_ministate(&m);
                m
            } 'not_redirect {
                read_ministate()
            })),
            menu_open: Prim::new(false),
            base_url: base_url.clone(),
            playlist: playlist_state,
            modal_stack: modal_stack.clone(),
            main_title: main_title,
            main_body: main_body,
            menu_body: menu_body,
            client_config: client_config,
        })));

        // Load initial view
        build_ministate(pc, &state().ministate.borrow());

        // React to further state changes
        EventListener::new(&window(), "popstate", {
            let eg = pc.eg();
            let state = state.clone();
            move |_e| eg.event(|pc| {
                let ministate = read_ministate();
                *state().ministate.borrow_mut() = ministate.clone();
                build_ministate(pc, &ministate);
            }).unwrap()
        }).forget();

        // Root and display
        set_root(vec![stack.own(|stack| (
            //. .
            playlist_root,
            link!(
                (pc = pc),
                (playing_i = state().playlist.0.playing_i.clone(), playing = state().playlist.0.playing.clone()),
                (),
                (state = state.clone(), stack = stack.weak(), current = Rc::new(RefCell::new(None as Option<El>))) {
                    if !playing.get() {
                        return None;
                    }
                    if !(!playing.get_old() || playing_i.get() != playing_i.get_old()) {
                        return None;
                    }
                    let Some(stack) = stack.upgrade() else {
                        return None;
                    };
                    let e = state().playlist.0.playlist.borrow().get(&playing_i.get().unwrap()).cloned().unwrap();
                    if !e.media.pm_display() {
                        return None;
                    }
                    if let Some(current) = current.borrow_mut().take() {
                        current.ref_replace(vec![]);
                    }
                    let media_el = e.media.pm_el();
                    let modal =
                        style_export::cont_media_fullscreen(
                            style_export::ContMediaFullscreenArgs { media: media_el.raw().dyn_into().unwrap() },
                        );
                    let modal =
                        el_from_raw(modal.root.into()).own(|_| (el_from_raw(modal.button_close.into()).on("click", {
                            let state = state.clone();
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
                        })));
                    *current.borrow_mut() = Some(modal.clone());
                    stack.ref_push(modal);
                    _ = e.media.pm_el().raw().request_fullscreen();
                }
            ),
            link!((_pc = pc), (menu_open = state().menu_open.clone()), (), () {
                let new_open = *menu_open.borrow();
                let state_open = style_export::class_menu_state_open().value;
                let x = document().get_elements_by_class_name(&style_export::class_menu_want_state_open().value);
                for i in 0 .. x.length() {
                    let ele = x.item(i).unwrap().dyn_into::<HtmlElement>().unwrap();
                    ele.class_list().toggle_with_force(&state_open, new_open).unwrap();
                }
            }),
        )).push(root).push(modal_stack)]);
    });
}
