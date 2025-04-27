use {
    crate::{
        async_::bg_val,
        el_general::{
            el_async,
            style_export::{
                self,
            },
        },
        ministate::{
            ministate_octothorpe,
            read_ministate,
            Ministate,
            MinistateForm,
            MinistateView,
        },
        playlist,
        state::{
            build_ministate,
            State,
            State_,
        },
        world::req_post_json,
    },
    gloo::{
        events::EventListener,
        utils::window,
    },
    lunk::{
        link,
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        el_from_raw,
        set_root,
        El,
        WeakEl,
    },
    shared::interface::{
        config::{
            menu::MenuItem,
            ClientConfig,
        },
        wire::ReqGetClientConfig,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlElement,
};

fn restore_ministate(pc: &mut ProcessingContext, state: &State) {
    let ministate = read_ministate().unwrap_or(Ministate::Home);
    build_ministate(pc, &state, &ministate);
    window().document().unwrap().set_title(match &ministate {
        Ministate::Home => "Sunwet",
        Ministate::View(s) => &s.title,
        Ministate::Form(s) => &s.title,
        Ministate::Edit(s) => &s.title,
    });
}

pub fn main_nonlink(pc: &mut ProcessingContext, base_url: String) {
    let stack =
        el_from_raw(style_export::cont_stack(style_export::ContStackArgs { children: vec![] }).root.into());
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

            fn build_menu_item(item: &MenuItem) -> HtmlElement {
                match item {
                    MenuItem::Section(item) => {
                        let mut children = vec![];
                        for child in &item.children {
                            children.push(build_menu_item(&child));
                        }
                        return style_export::cont_menu_group(style_export::ContMenuGroupArgs {
                            title: item.name.clone(),
                            children: children,
                        }).root;
                    },
                    MenuItem::View(item) => {
                        return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                            title: item.name.clone(),
                            href: ministate_octothorpe(&Ministate::View(MinistateView {
                                id: item.id.clone(),
                                title: item.name.clone(),
                                pos: None,
                            })),
                        }).root;
                    },
                    MenuItem::Form(item) => {
                        return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                            title: item.name.clone(),
                            href: ministate_octothorpe(&Ministate::Form(MinistateForm {
                                id: item.id.clone(),
                                title: item.name.clone(),
                            })),
                        }).root;
                    },
                }
            }

            let mut root = vec![];
            for item in &client_config.menu {
                root.push(build_menu_item(item));
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
    let root = el_from_raw(style_export::app_main(style_export::AppMainArgs {
        main_title: main_title.raw().dyn_into().unwrap(),
        main_body: main_body.raw().dyn_into().unwrap(),
        menu_body: menu_body.raw().dyn_into().unwrap(),
    }).root.into()).own(|_| (main_body.clone(), menu_body.clone()));
    let (playlist_state, playlist_root) = playlist::state_new(pc, base_url.clone());

    // Build app state
    let state = State::new(State_ {
        base_url: base_url.clone(),
        playlist: playlist_state,
        main_title: main_title,
        main_body: main_body,
        menu_body: menu_body,
        client_config: client_config,
    });

    // Load initial view
    restore_ministate(pc, &state);

    // React to further state changes
    EventListener::new(&window(), "popstate", {
        let eg = pc.eg();
        let state = state.clone();
        move |_e| eg.event(|pc| {
            restore_ministate(pc, &state);
        }).unwrap()
    }).forget();

    // Root and display
    set_root(vec![stack.own(|stack| (
        //. .
        playlist_root,
        link!(
            (pc = pc),
            (playing_i = state.playlist.0.playing_i.clone(), playing = state.playlist.0.playing.clone()),
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
                let e = state.playlist.0.playlist.borrow().get(playing_i.get().unwrap()).cloned().unwrap();
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
                            state.playlist.0.playing.set(pc, false);
                        }).unwrap()
                    })));
                *current.borrow_mut() = Some(modal.clone());
                stack.ref_push(modal);
                _ = e.media.pm_el().raw().request_fullscreen();
            }
        ),
    )).push(root)]);
}
