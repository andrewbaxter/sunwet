use {
    chrono::Utc,
    flowcontrol::shed,
    futures::{
        future::join_all,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::{
            document,
            window,
        },
    },
    lunk::{
        EventGraph,
        Prim,
    },
    rooting::{
        el,
        el_from_raw,
        scope_any,
        set_root,
        spawn_rooted,
        El,
        ScopeValue,
    },
    shared::interface::wire::link::{
        PrepareMedia,
        WsL2S,
        WsS2L,
    },
    std::{
        cell::Cell,
        panic,
        rc::Rc,
    },
    wasm::{
        constants::LINK_HASH_PREFIX,
        js::{
            self,
            get_dom_octothorpe,
            log_js,
            style_export,
        },
        js_media::{
            PlaylistMedia,
            PlaylistMediaAudio,
            PlaylistMediaImage,
            PlaylistMediaVideo,
        },
        websocket::Ws,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        UnwrapThrowExt,
    },
    wasm_bindgen_futures::{
        spawn_local,
        JsFuture,
    },
    web_sys::{
        console::log_1,
        DomException,
        HtmlElement,
        HtmlMediaElement,
    },
};

struct State_ {
    media_audio_el: El,
    media_video_el: El,
    display: El,
    display_over: El,
    display_under: El,
    album_artist: El,
    name: El,
    message_bg: Cell<ScopeValue>,
    media: Prim<Option<Rc<dyn PlaylistMedia>>>,
}

#[derive(Clone)]
struct State(Rc<State_>);

fn build_link(media_audio_el: HtmlMediaElement, media_video_el: HtmlMediaElement) {
    let eg = EventGraph::new();
    eg.event(|pc| {
        let base_url;
        {
            let loc = window().location();
            base_url =
                format!(
                    "{}{}",
                    loc.origin().unwrap_throw(),
                    loc.pathname().unwrap_throw().strip_suffix("link.html").unwrap_throw()
                );
        }
        let is_ios = js::is_ios();
        if is_ios {
            log_1(&JsValue::from("Detected mobile ios, activating webkit workarounds."));
        }
        let class_state_hide = style_export::class_state_hide().value;
        let hash = get_dom_octothorpe().unwrap();
        let link_id = hash.strip_prefix(LINK_HASH_PREFIX).unwrap();
        let style_res = style_export::app_link();
        let state = State(Rc::new(State_ {
            media_audio_el: el_from_raw(media_audio_el.into()),
            media_video_el: el_from_raw(media_video_el.into()),
            display: el_from_raw(style_res.display.into()),
            display_under: el_from_raw(style_res.display_under.into()).clone(),
            display_over: el_from_raw(style_res.display_over.into()).clone(),
            album_artist: el_from_raw(style_res.album_artist.into()).clone(),
            name: el_from_raw(style_res.title.into()).clone(),
            message_bg: Cell::new(scope_any(())),
            media: Prim::new(None),
        }));
        let ws = Ws::<WsL2S, WsS2L>::new(&base_url, format!("link/{}", link_id), {
            let state = state.clone();
            let eg = pc.eg();
            move |ws, message| {
                state.0.message_bg.set(scope_any(spawn_rooted({
                    let eg = eg.clone();
                    let ws = ws.clone();
                    let state = state.clone();
                    let class_state_hide = class_state_hide.clone();
                    async move {
                        match message {
                            WsS2L::Prepare(prepare) => {
                                log_1(&JsValue::from("x1"));
                                state.0.album_artist.ref_text(&format!("{} — {}", prepare.album, prepare.artist));
                                log_1(&JsValue::from("x2"));
                                state.0.name.ref_text(&prepare.name);
                                log_1(&JsValue::from("x3"));
                                state.0.display.ref_clear();
                                log_1(&JsValue::from("x4"));
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, true)]);
                                log_1(&JsValue::from("x5"));
                                let media: Rc<dyn PlaylistMedia>;
                                log_1(&JsValue::from("x6"));
                                match prepare.media {
                                    PrepareMedia::Audio(audio) => {
                                        match &audio.cover_source_url {
                                            Some(cover_source_url) => {
                                                log_1(&JsValue::from("x7"));
                                                state
                                                    .0
                                                    .display_under
                                                    .ref_modify_classes(&[(&class_state_hide, true)]);
                                                log_1(&JsValue::from("x8"));
                                                state
                                                    .0
                                                    .display
                                                    .ref_push(el("img").attr("src", cover_source_url.url.as_str()))
                                                    .ref_attr("preload", "auto");
                                                log_1(&JsValue::from("x9"));
                                            },
                                            None => {
                                                log_1(&JsValue::from("x10"));
                                                state
                                                    .0
                                                    .display_under
                                                    .ref_modify_classes(&[(&class_state_hide, false)]);
                                                log_1(&JsValue::from("x11"));
                                            },
                                        }
                                        log_1(&JsValue::from("x12"));
                                        let media_el = state.0.media_audio_el.clone();
                                        media_el.ref_attr("src", &audio.source_url.url);
                                        log_1(&JsValue::from("x13"));
                                        media = Rc::new(PlaylistMediaAudio { element: media_el });
                                        log_1(&JsValue::from("x14"));
                                    },
                                    PrepareMedia::Video(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        let media_el = state.0.media_video_el.clone();
                                        media_el.ref_attr("src", &source_url.url);
                                        state.0.media_video_el.ref_attr("preload", "auto");
                                        state.0.display.ref_push(media_el.clone());
                                        media = Rc::new(PlaylistMediaVideo { element: media_el });
                                    },
                                    PrepareMedia::Image(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        let media_el = el("img").attr("src", &source_url.url).on("click", |ev| {
                                            if document().fullscreen_element().is_none() {
                                                let img =
                                                    ev.target().unwrap().dyn_ref::<HtmlElement>().unwrap().clone();
                                                _ = img.request_fullscreen().unwrap();
                                            } else {
                                                document().exit_fullscreen();
                                            }
                                        });
                                        state.0.display.ref_push(media_el.clone());
                                        media = Rc::new(PlaylistMediaImage { element: media_el });
                                    },
                                }
                                log_1(&JsValue::from("x15"));
                                eg.event(|pc| {
                                    if let Some(old) = &*state.0.media.borrow() {
                                        old.pm_stop();
                                    }
                                    state.0.media.set(pc, Some(media.clone()));
                                });
                                log_1(&JsValue::from("x16"));
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, false)]);
                                log_1(&JsValue::from("x17"));
                                media.pm_wait_until_seekable().await;
                                log_1(&JsValue::from("x18"));
                                media.pm_seek(prepare.media_time);
                                log_1(&JsValue::from("x19"));
                                media.pm_wait_until_buffered().await;
                                log_1(&JsValue::from("x20"));
                                if is_ios {
                                    // Ios safari can't seek until canplaythrough event and then we need to wait again
                                    // to make sure it can playthrough from the new position... ugh
                                    log_1(&JsValue::from("x18"));
                                    media.pm_seek(prepare.media_time);
                                    log_1(&JsValue::from("x19"));
                                    media.pm_wait_until_buffered().await;
                                }
                                log_1(&JsValue::from("x20b"));
                                ws.send(WsL2S::Ready(Utc::now())).await;
                                log_1(&JsValue::from("x21"));
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, true)]);
                                log_1(&JsValue::from("x22"));
                            },
                            WsS2L::Play(play_at) => {
                                log_1(&JsValue::from("x23"));
                                if let Some(media) = &*state.0.media.borrow() {
                                    log_1(&JsValue::from("x24"));
                                    TimeoutFuture::new(
                                        (play_at - Utc::now()).num_milliseconds().max(0) as u32,
                                    ).await;
                                    log_1(&JsValue::from("x25"));
                                    _ = media.pm_play();
                                    log_1(&JsValue::from("x26"));
                                }
                            },
                            WsS2L::Pause => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    media.pm_stop();
                                }
                            },
                        }
                    }
                })));
            }
        });
        set_root(vec![el_from_raw(style_res.root.into()).own(|_| ws)]);
    });
}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    spawn_local(async move {
        // Work around ios safari alone blocking audio-playing media despite the users'
        // wishes. Supposedly if you keep a single media element around that got
        // permission you don't need to interactively trigger permission again...
        let audio_el = document().create_element("audio").unwrap().dyn_into::<HtmlMediaElement>().unwrap();
        audio_el.set_attribute("src", "audiotest.mp3").unwrap();
        let video_el = document().create_element("video").unwrap().dyn_into::<HtmlMediaElement>().unwrap();
        video_el.set_attribute("src", "videotest.webm").unwrap();
        match JsFuture::from(audio_el.play().unwrap()).await {
            Ok(_) => {
                build_link(audio_el, video_el);
            },
            Err(e) => {
                shed!{
                    let Some(e) = e.dyn_ref::<DomException>() else {
                        break;
                    };
                    if e.name() != "NotAllowedError" {
                        break;
                    }

                    // Work around autoplay blocking (ios safari, desktop firefox) by making it a
                    // non-auto play
                    let style_res = style_export::app_link_perms();
                    let button = el_from_raw(style_res.button.into()).on("click", move |_| {
                        let bg =
                            vec![
                                JsFuture::from(audio_el.play().unwrap()),
                                JsFuture::from(video_el.play().unwrap())
                            ];
                        spawn_local({
                            let audio_el = audio_el.clone();
                            let video_el = video_el.clone();
                            async move {
                                for res in join_all(bg).await {
                                    if let Err(e) = res {
                                        log_js("Error confirming media element permissions", &e);
                                    }
                                }
                                build_link(audio_el, video_el)
                            }
                        });
                    });
                    set_root(vec![el_from_raw(style_res.root.into()).own(|_| button)]);
                    return;
                }
                log_js("Error playing media to guage permissions", &e);
                panic!("");
            },
        }
    });
}
