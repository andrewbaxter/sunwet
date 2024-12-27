use {
    async_trait::async_trait,
    chrono::Utc,
    futures::{
        Future,
        FutureExt,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::window,
    },
    lunk::{
        link,
        EventGraph,
        Prim,
    },
    rooting::{
        el,
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
    wasm_bindgen::JsCast,
    web::{
        el_general::{
            async_event,
            el_audio,
            el_hbox,
            el_icon,
            el_vbox,
            el_video,
            ICON_VOLUME,
        },
        websocket::Ws,
        world::file_url,
    },
    web_sys::{
        HtmlInputElement,
        HtmlMediaElement,
    },
};

trait PlaylistMedia {
    fn wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn seek(&self, time: f64);
    fn set_volume(&self, volume: f64);
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

    fn set_volume(&self, _volume: f64) {
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

    fn set_volume(&self, volume: f64) {
        self.media.set_volume(volume);
    }

    fn play(&self) {
        _ = self.media.play().unwrap();
    }

    fn pause(&self) {
        self.media.pause().unwrap();
    }
}

struct State_ {
    base_url: String,
    display: El,
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
        let hash = window().location().hash().unwrap();
        let Some(sess_id) = hash.strip_prefix("#") else {
            panic!("Missing session id");
        };
        let sess_id = sess_id.to_string();
        let display = el("div").classes(&["s_display"]);
        let album = el("span").classes(&["s_album"]);
        let artist = el("span").classes(&["s_author"]);
        let name = el("span").classes(&["s_name"]);
        let volume = Prim::new(pc, 1.);
        let state = State(Rc::new(State_ {
            base_url: origin,
            display: display.clone(),
            album: album.clone(),
            artist: artist.clone(),
            name: name.clone(),
            message_bg: Cell::new(scope_any(())),
            media: Prim::new(pc, None),
        }));
        let ws = Ws::<WsL2S, WsS2L>::new(format!("link/{}", sess_id), {
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
                                let media: Rc<dyn PlaylistMedia>;
                                match prepare.media {
                                    PrepareMedia::Audio(audio) => {
                                        state.0.display.ref_push(match &audio.cover {
                                            Some(cover) => el(
                                                "img",
                                            ).attr("src", &file_url(&state.0.base_url, cover)),
                                            None => el_icon(ICON_AUDIO),
                                        }).ref_attr("preload", "auto");
                                        let media_el = el_audio(&file_url(&state.0.base_url, &audio.audio));
                                        media =
                                            Rc::new(
                                                PlaylistMediaAudioVideo {
                                                    media: media_el.raw().dyn_into::<HtmlMediaElement>().unwrap(),
                                                },
                                            );
                                    },
                                    PrepareMedia::Video(video_file) => {
                                        let media_el =
                                            el_video(
                                                &file_url(&state.0.base_url, &video_file),
                                            ).attr("preload", "auto");
                                        state.0.display.ref_push(media_el.clone());
                                        media =
                                            Rc::new(
                                                PlaylistMediaAudioVideo {
                                                    media: media_el.raw().dyn_into::<HtmlMediaElement>().unwrap(),
                                                },
                                            );
                                    },
                                    PrepareMedia::Image(image_file) => {
                                        let media_el =
                                            el("img").attr("src", &file_url(&state.0.base_url, &image_file));
                                        state.0.display.ref_push(media_el);
                                        media = Rc::new(PlaylistMediaImage {});
                                    },
                                }
                                eg.event(|pc| {
                                    state.0.media.set(pc, Some(media.clone()));
                                });
                                media.wait_until_seekable().await;
                                media.seek(prepare.media_time);
                                media.wait_until_buffered().await;
                                ws.send(WsL2S::Ready(Utc::now())).await;
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
        set_root(vec![
            //. .
            el_vbox().extend(vec![
                //. .
                display,
                album,
                el_hbox().extend(vec![artist, name]),
                el_hbox().extend(
                    vec![
                        el_icon(ICON_VOLUME).attr("title", "Volume"),
                        el("input").attr("type", "range").on("input", {
                            let volume = volume.clone();
                            let eg = pc.eg();
                            move |ev| eg.event(|pc| {
                                let input = ev.dyn_ref::<HtmlInputElement>().unwrap();
                                volume.set(pc, input.value_as_number());
                            })
                        })
                    ],
                )
            ]).own(|_| (ws, link!((_pc = pc), (volume = volume, media = state.0.media.clone()), (), () {
                let Some(media) = &*media.borrow() else {
                    return None;
                };
                media.set_volume(*volume.borrow());
            })))
        ]);
    });
}
