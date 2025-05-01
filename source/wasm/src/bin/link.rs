use {
    async_trait::async_trait,
    chrono::Utc,
    futures::{
        Future,
        FutureExt,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::{
            document,
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
        pin::Pin,
        rc::Rc,
    },
    wasm::{
        constants::LINK_HASH_PREFIX,
        js::{
            async_event,
            el_async,
            el_audio,
            el_video,
            get_dom_octothorpe,
            style_export,
        },
        websocket::Ws,
    },
    wasm_bindgen::{
        JsCast,
    },
    web_sys::{
        HtmlElement,
        HtmlMediaElement,
    },
};

trait PlaylistMedia {
    fn wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn seek(&self, time: f64);
    fn play(&self);
    fn pause(&self);
}

#[derive(Clone)]
struct PlaylistMediaImage {}

impl PlaylistMedia for PlaylistMediaImage {
    fn wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }

    fn wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }

    fn seek(&self, _time: f64) {
        // nop
    }

    fn play(&self) {
        // nop
    }

    fn pause(&self) {
        // nop
    }
}

#[derive(Clone)]
struct PlaylistMediaAudioVideo {
    media: HtmlMediaElement,
}

#[async_trait]
impl PlaylistMedia for PlaylistMediaAudioVideo {
    fn wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.media.clone();
        return async move {
            // `HAVE_METADATA`
            if m.ready_state() < 1 {
                async_event(&m, "loadedmetadata").await;
            }
        }.boxed_local();
    }

    fn wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.media.clone();
        return async move {
            // `HAVE_ENOUGH_DATA`
            if m.ready_state() < 4 {
                async_event(&m, "canplaythrough").await;
            }
        }.boxed_local();
    }

    fn seek(&self, time: f64) {
        self.media.set_current_time(time);
    }

    fn play(&self) {
        _ = self.media.play().unwrap();
    }

    fn pause(&self) {
        self.media.pause().unwrap();
    }
}

struct State_ {
    display: El,
    display_over: El,
    album: El,
    artist: El,
    name: El,
    message_bg: Cell<ScopeValue>,
    media: Prim<Option<Rc<dyn PlaylistMedia>>>,
}

#[derive(Clone)]
struct State(Rc<State_>);

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let hash = get_dom_octothorpe().unwrap();
        let link_id = hash.strip_prefix(LINK_HASH_PREFIX).unwrap();
        let style_res = style_export::app_link();
        let state = State(Rc::new(State_ {
            display: el_from_raw(style_res.display.into()),
            display_over: el_from_raw(style_res.display_over.into()).clone(),
            album: el_from_raw(style_res.album.into()).clone(),
            artist: el_from_raw(style_res.artist.into()).clone(),
            name: el_from_raw(style_res.title.into()).clone(),
            message_bg: Cell::new(scope_any(())),
            media: Prim::new(None),
        }));
        let ws = Ws::<WsL2S, WsS2L>::new(format!("link/{}", link_id), {
            let state = state.clone();
            let eg = pc.eg();
            move |ws, message| {
                state.0.message_bg.set(scope_any(spawn_rooted({
                    let eg = eg.clone();
                    let ws = ws.clone();
                    let state = state.clone();
                    async move {
                        match message {
                            WsS2L::Prepare(prepare) => {
                                state.0.album.ref_text(&prepare.album);
                                state.0.artist.ref_text(&prepare.artist);
                                state.0.name.ref_text(&prepare.name);
                                state.0.display.ref_clear();
                                state.0.display_over.ref_clear();
                                let media: Rc<dyn PlaylistMedia>;
                                match prepare.media {
                                    PrepareMedia::Audio(audio) => {
                                        state
                                            .0
                                            .display
                                            .ref_push(el("img").attr("src", match &audio.cover_source_url {
                                                Some(cover_source_url) => cover_source_url.url.as_str(),
                                                None => "static/fallback_cover.png",
                                            }))
                                            .ref_attr("preload", "auto");
                                        let media_el = el_audio(&audio.source_url.url);
                                        media =
                                            Rc::new(
                                                PlaylistMediaAudioVideo {
                                                    media: media_el.raw().dyn_into::<HtmlMediaElement>().unwrap(),
                                                },
                                            );
                                    },
                                    PrepareMedia::Video(source_url) => {
                                        let media_el = el_video(&source_url.url).attr("preload", "auto");
                                        state.0.display.ref_push(media_el.clone());
                                        media =
                                            Rc::new(
                                                PlaylistMediaAudioVideo {
                                                    media: media_el.raw().dyn_into::<HtmlMediaElement>().unwrap(),
                                                },
                                            );
                                    },
                                    PrepareMedia::Image(source_url) => {
                                        let media_el = el("img").attr("src", &source_url.url).on("click", |ev| {
                                            if document().fullscreen_element().is_none() {
                                                let img =
                                                    ev.target().unwrap().dyn_ref::<HtmlElement>().unwrap().clone();
                                                _ = img.request_fullscreen().unwrap();
                                            } else {
                                                document().exit_fullscreen();
                                            }
                                        });
                                        state.0.display.ref_push(media_el);
                                        media = Rc::new(PlaylistMediaImage {});
                                    },
                                }
                                eg.event(|pc| {
                                    state.0.media.set(pc, Some(media.clone()));
                                });
                                state.0.display_over.ref_push(el_async(async move {
                                    media.wait_until_seekable().await;
                                    media.seek(prepare.media_time);
                                    media.wait_until_buffered().await;
                                    ws.send(WsL2S::Ready(Utc::now())).await;
                                    return Ok(el("div")) as Result<_, String>;
                                }));
                            },
                            WsS2L::Play(play_at) => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    TimeoutFuture::new(
                                        (play_at - Utc::now()).num_milliseconds().max(0) as u32,
                                    ).await;
                                    _ = media.play();
                                }
                            },
                            WsS2L::Pause => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    media.pause();
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
