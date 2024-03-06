use std::{
    cell::{
        Cell,
        RefCell,
    },
    panic,
    rc::Rc,
};
use chrono::{
    Utc,
};
use gloo::{
    timers::future::TimeoutFuture,
    utils::window,
};
use lunk::{
    link,
    EventGraph,
    Prim,
};
use rooting::{
    el,
    scope_any,
    set_root,
    spawn_rooted,
    El,
    ScopeValue,
};
use shared::model::link::{
    PrepareMedia,
    WsL2S,
    WsS2L,
};
use wasm_bindgen::JsCast;
use web::{
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
};
use web_sys::{
    HtmlInputElement,
    HtmlMediaElement,
};

struct State_ {
    display: El,
    album: El,
    artist: El,
    name: El,
    volume: Prim<f64>,
    message_bg: Cell<ScopeValue>,
    media: RefCell<Option<HtmlMediaElement>>,
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
            display: display.clone(),
            album: album.clone(),
            artist: artist.clone(),
            name: name.clone(),
            volume: volume.clone(),
            message_bg: Cell::new(scope_any(())),
            media: RefCell::new(None),
        }));
        let ws = Ws::<WsL2S, WsS2L>::new(format!("link/{}", sess_id), {
            let state = state.clone();
            move |ws, message| {
                state.0.message_bg.set(scope_any(spawn_rooted({
                    let ws = ws.clone();
                    let state = state.clone();
                    async move {
                        match message {
                            WsS2L::Prepare(prepare) => {
                                state.0.album.ref_text(&prepare.album);
                                state.0.artist.ref_text(&prepare.artist);
                                state.0.name.ref_text(&prepare.name);
                                state.0.display.ref_clear();
                                let media_el;
                                match prepare.media {
                                    PrepareMedia::Audio(audio) => {
                                        state.0.display.ref_push(el("img").attr("src", &audio.cover_url));
                                        media_el = el_audio(&audio.audio_url);
                                    },
                                    PrepareMedia::Video(url) => {
                                        media_el = el_video(&url);
                                        state.0.display.ref_push(media_el.clone());
                                    },
                                }
                                let media_el2 = media_el.raw().dyn_into::<HtmlMediaElement>().unwrap();
                                *state.0.media.borrow_mut() = Some(media_el2.clone());
                                media_el.ref_attr("preload", "auto");
                                let raw_media_el = media_el.raw().dyn_into::<HtmlMediaElement>().unwrap();

                                // `HAVE_METADATA`
                                if raw_media_el.ready_state() < 1 {
                                    async_event(&raw_media_el, "loadedmetadata").await;
                                }
                                raw_media_el.set_current_time(prepare.media_time);

                                // `HAVE_ENOUGH_DATA`
                                if raw_media_el.ready_state() < 4 {
                                    async_event(&raw_media_el, "canplaythrough").await;
                                }
                                ws.send(WsL2S::Ready(Utc::now())).await;
                            },
                            WsS2L::Play(play_at) => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    TimeoutFuture::new(
                                        (play_at - Utc::now()).num_milliseconds().max(0) as u32,
                                    ).await;
                                    media.set_volume(*state.0.volume.borrow());
                                    _ = media.play().unwrap();
                                }
                            },
                            WsS2L::Pause => {
                                if let Some(media) = &*state.0.media.borrow() {
                                    media.pause().unwrap();
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
            ]).own(|_| (ws, link!((_pc = pc), (volume = volume), (), (state = state.clone()) {
                let media = state.0.media.borrow();
                let media = media.as_ref()?;
                media.set_volume(*volume.borrow());
            })))
        ]);
    });
}
