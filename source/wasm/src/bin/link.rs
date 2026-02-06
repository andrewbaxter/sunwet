use {
    chrono::Utc,
    flowcontrol::{
        shed,
        ta_return,
    },
    futures::{
        FutureExt,
        future::join_all,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::document,
    },
    lunk::{
        EventGraph,
        Prim,
    },
    reqwasm::http::Request,
    rooting::{
        El,
        ScopeValue,
        el,
        el_from_raw,
        scope_any,
        set_root,
        spawn_rooted,
    },
    shared::interface::{
        derived::ComicManifest,
        wire::{
            GENTYPE_DIR,
            link::{
                COOKIE_LINK_SESSION,
                PrepareMedia,
                WsL2S,
                WsS2L,
            },
        },
    },
    std::{
        cell::Cell,
        panic,
        rc::Rc,
        time::Duration,
    },
    tokio::time::sleep,
    wasm::{
        constants::LINK_HASH_PREFIX,
        js::{
            ConsoleLog,
            Log,
            get_dom_octothorpe,
            scan_env,
            style_export,
        },
        media::{
            PlaylistMedia,
            PlaylistMediaAudioVideo,
            PlaylistMediaBook,
            PlaylistMediaComic,
            PlaylistMediaImage,
            pm_share_ready_prep,
        },
        websocket::Ws,
        world::{
            file_url,
            generated_file_url,
        },
    },
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::{
        JsFuture,
        spawn_local,
    },
    web_sys::{
        DomException,
        HtmlDocument,
        HtmlElement,
        HtmlMediaElement,
    },
};

struct State_ {
    media_el_audio: El,
    media_el_video: El,
    media_el_image: El,
    display: El,
    display_over: El,
    display_under: El,
    album_artist: El,
    name: El,
    message_bg: Cell<ScopeValue>,
    media: Prim<Option<Rc<dyn PlaylistMedia>>>,
    log: Rc<dyn Log>,
}

#[derive(Clone)]
struct State(Rc<State_>);

fn build_link(log: &Rc<dyn Log>, media_audio_el: HtmlMediaElement, media_video_el: HtmlMediaElement) {
    let eg = EventGraph::new();
    eg.event(|pc| {
        let env = scan_env(&log);
        let class_state_hide = style_export::class_state_hide().value;
        let hash = get_dom_octothorpe(&log).unwrap();
        let link_id = hash.strip_prefix(LINK_HASH_PREFIX).unwrap();
        document()
            .dyn_into::<HtmlDocument>()
            .unwrap()
            .set_cookie(&format!("{}={}", COOKIE_LINK_SESSION, link_id))
            .unwrap();
        let style_res = style_export::app_link();
        let state = State(Rc::new(State_ {
            media_el_audio: el_from_raw(media_audio_el.clone().into()),
            media_el_video: el_from_raw(media_video_el.into()),
            media_el_image: el("img").on("click", |ev| {
                if document().fullscreen_element().is_none() {
                    let img = ev.target().unwrap().dyn_ref::<HtmlElement>().unwrap().clone();
                    _ = img.request_fullscreen().unwrap();
                } else {
                    document().exit_fullscreen();
                }
            }),
            display: style_res.display,
            display_under: style_res.display_under.clone(),
            display_over: style_res.display_over.clone(),
            album_artist: style_res.album_artist.clone(),
            name: style_res.title.clone(),
            message_bg: Cell::new(scope_any(())),
            media: Prim::new(None),
            log: log.clone(),
        }));
        let ws = Ws::<WsL2S, WsS2L>::new(log.clone(), &env.base_url, format!("link/{}", link_id), {
            let state = state.clone();
            let eg = pc.eg();
            let env = env.clone();
            move |ws, message| {
                state.0.message_bg.set(scope_any(spawn_rooted({
                    let eg = eg.clone();
                    let ws = ws.clone();
                    let state = state.clone();
                    let class_state_hide = class_state_hide.clone();
                    let env = env.clone();
                    async move {
                        match message {
                            WsS2L::Prepare(prepare) => {
                                state.0.album_artist.ref_text(&format!("{} — {}", prepare.album, prepare.artist));
                                state.0.name.ref_text(&prepare.name);
                                state.0.display.ref_clear();
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, true)]);
                                let media: Rc<dyn PlaylistMedia>;
                                match prepare.media {
                                    PrepareMedia::Audio(audio) => {
                                        match &audio.cover_source_url {
                                            Some(cover_source_url) => {
                                                state
                                                    .0
                                                    .display_under
                                                    .ref_modify_classes(&[(&class_state_hide, true)]);
                                                state
                                                    .0
                                                    .display
                                                    .ref_push(
                                                        el("img").attr("src", &file_url(&env, cover_source_url)),
                                                    )
                                                    .ref_attr("preload", "auto");
                                            },
                                            None => {
                                                state
                                                    .0
                                                    .display_under
                                                    .ref_modify_classes(&[(&class_state_hide, false)]);
                                            },
                                        }
                                        media =
                                            Rc::new(
                                                PlaylistMediaAudioVideo::new_audio(
                                                    state.0.media_el_audio.clone(),
                                                    audio.source_url.clone(),
                                                    0.,
                                                ),
                                            );
                                    },
                                    PrepareMedia::Video(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        let media_el = state.0.media_el_video.clone();
                                        state.0.display.ref_push(media_el.clone());
                                        media =
                                            Rc::new(PlaylistMediaAudioVideo::new_video(media_el, source_url, 0.));
                                    },
                                    PrepareMedia::Image(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        let media_el = state.0.media_el_image.clone();
                                        state.0.display.ref_push(media_el.clone());
                                        media = Rc::new(PlaylistMediaImage {
                                            element: media_el,
                                            src: source_url.clone(),
                                        });
                                    },
                                    PrepareMedia::Comic(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        media =
                                            Rc::new(
                                                PlaylistMediaComic::new(
                                                    &generated_file_url(&env, &source_url, GENTYPE_DIR, ""),
                                                    Rc::new({
                                                        let log = state.0.log.clone();
                                                        move |url| {
                                                            let log = log.clone();
                                                            async move {
                                                                loop {
                                                                    match async {
                                                                        ta_return!(ComicManifest, String);
                                                                        let r =
                                                                            Request::get(&url)
                                                                                .send()
                                                                                .await
                                                                                .map_err(
                                                                                    |e| format!(
                                                                                        "Error requesting comic manifest: {}",
                                                                                        e
                                                                                    ),
                                                                                )?
                                                                                .binary()
                                                                                .await
                                                                                .map_err(
                                                                                    |e| format!(
                                                                                        "Error reading comic manifest response: {}",
                                                                                        e
                                                                                    ),
                                                                                )?;
                                                                        return Ok(
                                                                            serde_json::from_slice::<ComicManifest>(
                                                                                &r,
                                                                            ).map_err(
                                                                                |e| format!(
                                                                                    "Error reading comic manifest: {}",
                                                                                    e
                                                                                ),
                                                                            )?,
                                                                        );
                                                                    }.await {
                                                                        Ok(r) => return Ok(r),
                                                                        Err(e) => {
                                                                            log.log(
                                                                                &format!(
                                                                                    "Request failed, retrying: {}",
                                                                                    e
                                                                                ),
                                                                            );
                                                                            sleep(Duration::from_secs(1)).await;
                                                                        },
                                                                    }
                                                                }
                                                            }
                                                        }.boxed_local()
                                                    }),
                                                    0,
                                                ),
                                            );
                                        eg
                                            .event(
                                                |pc| state.0.display.ref_push(media.pm_el(&state.0.log, pc).clone()),
                                            )
                                            .unwrap();
                                    },
                                    PrepareMedia::Book(source_url) => {
                                        state.0.display_under.ref_modify_classes(&[(&class_state_hide, true)]);
                                        media =
                                            Rc::new(
                                                PlaylistMediaBook::new(
                                                    &generated_file_url(&env, &source_url, GENTYPE_DIR, ""),
                                                    0,
                                                ),
                                            );
                                        eg
                                            .event(
                                                |pc| state.0.display.ref_push(media.pm_el(&state.0.log, pc).clone()),
                                            )
                                            .unwrap();
                                    },
                                }
                                eg.event(|pc| {
                                    if let Some(old) = &*state.0.media.borrow() {
                                        old.pm_stop();
                                    }
                                    state.0.media.set(pc, Some(media.clone()));
                                });
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, false)]);
                                pm_share_ready_prep(eg, &state.0.log, &env, media.as_ref(), prepare.media_time).await;
                                ws.send(WsL2S::Ready(Utc::now())).await;
                                state.0.display_over.ref_modify_classes(&[(&class_state_hide, true)]);
                            },
                            WsS2L::Play(play_at) => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    TimeoutFuture::new(
                                        (play_at - Utc::now()).num_milliseconds().max(0) as u32,
                                    ).await;
                                    media.pm_play(&state.0.log);
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
        set_root(vec![style_res.root.own(|_| ws)]);
    });
}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    spawn_local(async move {
        let log = Rc::new(ConsoleLog {}) as Rc<dyn Log>;

        // Work around ios safari alone blocking audio-playing media despite the users'
        // wishes. Supposedly if you keep a single media element around that got
        // permission you don't need to interactively trigger permission again...
        let audio_el = document().create_element("audio").unwrap().dyn_into::<HtmlMediaElement>().unwrap();
        audio_el.set_attribute("src", "audiotest.mp3").unwrap();
        let video_el = document().create_element("video").unwrap().dyn_into::<HtmlMediaElement>().unwrap();
        video_el.set_attribute("src", "videotest.webm").unwrap();
        match JsFuture::from(audio_el.play().unwrap()).await {
            Ok(_) => {
                build_link(&log, audio_el, video_el);
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
                    style_res.button.on("click", move |_| {
                        let bg =
                            vec![JsFuture::from(audio_el.play().unwrap()), JsFuture::from(video_el.play().unwrap())];
                        spawn_local({
                            let audio_el = audio_el.clone();
                            let video_el = video_el.clone();
                            let log = log.clone();
                            async move {
                                for res in join_all(bg).await {
                                    if let Err(e) = res {
                                        log.log_js("Error confirming media element permissions", &e);
                                    }
                                }
                                build_link(&log, audio_el, video_el)
                            }
                        });
                    });
                    set_root(vec![style_res.root]);
                    return;
                }
                log.log_js("Error playing media to guage permissions", &e);
                panic!("");
            },
        }
    });
}
