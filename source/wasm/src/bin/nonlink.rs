use {
    flowcontrol::{
        shed,
        superif,
    },
    gloo::{
        events::EventListener,
        utils::{
            format::JsValueSerdeExt,
            window,
        },
    },
    libnonlink::{
        api::req_post_json,
        ministate::{
            ministate_octothorpe,
            read_ministate,
            Ministate,
            MinistateEdit,
            MinistateForm,
            MinistateView,
        },
        playlist::{
            self,
            playlist_push,
            playlist_toggle_play,
            AudioPlaylistMedia,
            PlaylistEntry,
            PlaylistEntryMediaType,
            VideoPlaylistMedia,
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
        ProcessingContext,
    },
    rooting::{
        el,
        el_from_raw,
        set_root,
        El,
        WeakEl,
    },
    serde::Deserialize,
    shared::interface::{
        config::{
            menu::ClientMenuItem,
            ClientConfig,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqGetClientConfig,
            ReqViewQuery,
            TreeNode,
        },
    },
    std::{
        cell::RefCell,
        collections::{
            BTreeMap,
            HashMap,
        },
        panic,
        rc::Rc,
        str::FromStr,
    },
    wasm::{
        async_::bg_val,
        constants::LINK_HASH_PREFIX,
        el_general::{
            el_async,
            el_audio,
            el_video,
            get_dom_octothorpe,
            style_export::{
                self,
            },
        },
        world::file_url,
    },
    wasm_bindgen::{
        prelude::wasm_bindgen,
        JsCast,
        JsValue,
        UnwrapThrowExt,
    },
    web_sys::HtmlElement,
};

pub mod libnonlink;

pub fn main() { }

#[wasm_bindgen]
pub async fn export_query(id: String, data: JsValue) -> Result<JsValue, String> {
    let data =
        <JsValue as gloo::utils::format::JsValueSerdeExt>::into_serde::<HashMap<String, Node>>(
            &data,
        ).map_err(|e| e.to_string())?;
    let Ministate::View(ministate) = state().ministate.borrow().clone() else {
        return Err(format!("ASSERT! Current state is not view"));
    };
    let res = req_post_json(&state().base_url, ReqViewQuery {
        menu_item_id: ministate.menu_item_id.clone(),
        query: id,
        parameters: data,
    }).await?;
    return Ok(<JsValue as JsValueSerdeExt>::from_serde(&res.records).unwrap());
}

#[wasm_bindgen(js_name = "importBuildFileUrl")]
pub fn export_file_url(file: String) -> Result<String, String> {
    let file = FileHash::from_str(&file)?;
    return Ok(file_url(&state().base_url, &file));
}

#[wasm_bindgen(js_name = "importBuildEditUrl")]
pub fn export_edit_url(title: String, node: JsValue) -> Result<String, String> {
    let node =
        <JsValue as gloo::utils::format::JsValueSerdeExt>::into_serde::<Node>(&node).map_err(|e| e.to_string())?;
    return Ok(ministate_octothorpe(&Ministate::Edit(MinistateEdit {
        title: title,
        node: node,
    })));
}

#[wasm_bindgen(js_name = "importSetPlaylist")]
pub fn export_set_playlist(playlist: JsValue) -> Result<(), String> {
    #[derive(Deserialize)]
    pub struct JsPlaylistEntry {
        pub name: Option<String>,
        pub album: Option<String>,
        pub artist: Option<String>,
        pub cover: Option<FileHash>,
        pub file: FileHash,
        pub media_type: PlaylistEntryMediaType,
    }

    let playlist =
        <JsValue as gloo::utils::format::JsValueSerdeExt>::into_serde::<Vec<JsPlaylistEntry>>(
            &playlist,
        ).map_err(|e| e.to_string())?;
    for entry in playlist {
        let setup_media_element = |pc: &mut ProcessingContext, i: usize, media: &El| {
            media.ref_on("ended", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state().playlist, Some(i));
                }).unwrap()
            });
            media.ref_on("pause", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    state().0.playing.set(pc, false);
                }).unwrap()
            });
            media.ref_on("play", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    state().0.playing.set(pc, true);
                }).unwrap()
            });
            media.ref_on("volumechange", {
                let eg = pc.eg();
                let volume = state().playlist.0.volume.clone();
                let debounce = state().playlist.0.volume_debounce.clone();
                move |ev| {
                    if Utc::now().signed_duration_since(debounce.get()) < Duration::milliseconds(200) {
                        return;
                    }
                    eg.event(|pc| {
                        let v = ev.target().unwrap().dyn_ref::<HtmlMediaElement>().unwrap().volume();
                        volume.set(pc, (v / 2., v / 2.));
                    })
                }
            });
            let restore_pos = shed!{
                'restore_pos _;
                shed!{
                    let Some(init) = restore_playlist_pos else {
                        break;
                    };
                    media.ref_on("loadedmetadata", {
                        let time = init.time;
                        move |e| {
                            e.target().unwrap().dyn_into::<HtmlMediaElement>().unwrap().set_current_time(time);
                        }
                    });
                    break 'restore_pos true;
                };
                break 'restore_pos false;
            };
            if restore_pos {
                state().playlist.0.playing_i.set(pc, Some(i));
            }
        };
        let box_media;
        match entry.media_type {
            PlaylistEntryMediaType::Audio => {
                let media = el_audio(&file_url(&state().base_url, &entry.file)).attr("controls", "true");
                setup_media_element(pc, i, &media);
                box_media = Box::new(AudioPlaylistMedia {
                    element: media.clone(),
                    ministate_id: build_playlist_pos.list_id.clone(),
                    ministate_title: build_playlist_pos.list_title.clone(),
                    ministate_path: build_playlist_pos.entry_path.clone(),
                });
            },
            PlaylistEntryMediaType::Video => {
                let mut sub_tracks = vec![];
                for lang in window().navigator().languages() {
                    let lang = lang.as_string().unwrap();
                    sub_tracks.push((generated_file_url(&state().base_url, &source, &format!("webvtt_{}", {
                        let lang = if let Some((lang, _)) = lang.split_once("-") {
                            lang
                        } else {
                            &lang
                        };
                        match lang {
                            "en" => "eng",
                            "jp" => "jpn",
                            _ => {
                                log(format!("Unhandled subtitle translation for language {}", lang));
                                continue;
                            },
                        }
                    }), "text/vtt"), lang));
                }
                let media =
                    el_video(
                        &generated_file_url(&state().base_url, &source, "", "video/webm"),
                    ).attr("controls", "true");
                setup_media_element(pc, i, &media);
                for (i, (url, lang)) in sub_tracks.iter().enumerate() {
                    let track = el("track").attr("kind", "subtitles").attr("src", url).attr("srclang", lang);
                    if i == 0 {
                        track.ref_attr("default", "default");
                    }
                    media.ref_push(track);
                }
                box_media = Box::new(VideoPlaylistMedia {
                    element: media.clone(),
                    ministate_id: build_playlist_pos.list_id.clone(),
                    ministate_title: build_playlist_pos.list_title.clone(),
                    ministate_path: build_playlist_pos.entry_path.clone(),
                });
            },
            PlaylistEntryMediaType::Image => {
                let source;
                let Ok(n) = query_res_as_file(v) else {
                    return el_media_button_err(
                        format!("Field contents wasn't string value node or string: {:?}", v),
                    );
                };
                source = n;
                let media = el("img").attr("src", &file_url(&state.base_url, &source)).attr("loading", "lazy");
                box_media = Box::new(ImagePlaylistMedia {
                    element: media.clone(),
                    ministate_id: build_playlist_pos.list_id.clone(),
                    ministate_title: build_playlist_pos.list_title.clone(),
                    ministate_path: build_playlist_pos.entry_path.clone(),
                });
            },
        }
        playlist_push(&state().playlist, Rc::new(PlaylistEntry {
            name: entry.name,
            album: entry.album,
            artist: entry.artist,
            cover: entry.cover,
            file: entry.file,
            media_type: entry.media_type,
            media: box_media,
        }));
    }
    return Ok(());
}

#[wasm_bindgen(js_name = "importTogglePlay")]
pub fn export_toggle_play(index: usize) {
    state().eg.event(|pc| {
        playlist_toggle_play(pc, &state().playlist, Some(index));
    });
}

#[wasm_bindgen(js_name = "main")]
pub fn export_real_main() {
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
        let root = el_from_raw(style_export::app_main(style_export::AppMainArgs {
            main_title: main_title.raw().dyn_into().unwrap(),
            main_body: main_body.raw().dyn_into().unwrap(),
            menu_body: menu_body.raw().dyn_into().unwrap(),
        }).root.into()).own(|_| (main_body.clone(), menu_body.clone()));
        let (playlist_state, playlist_root) = playlist::state_new(pc, base_url.clone());

        // Build app state
        STATE.with(|s| *s.borrow_mut() = Some(Rc::new(State_ {
            eg: pc.eg(),
            ministate: RefCell::new(read_ministate()),
            base_url: base_url.clone(),
            playlist: playlist_state,
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
                    let e = state().playlist.0.playlist.borrow().get(playing_i.get().unwrap()).cloned().unwrap();
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
        )).push(root)]);
    });
}
