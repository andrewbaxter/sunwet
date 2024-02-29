use std::{
    any::Any,
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    panic,
    rc::Rc,
    str::FromStr,
};
use chrono::{
    DateTime,
    Duration,
    Utc,
};
use futures::{
    Future,
    FutureExt,
};
use gloo::{
    console::{
        console,
        warn,
    },
    events::EventListener,
    timers::future::TimeoutFuture,
    utils::{
        document,
        window,
    },
};
use js_sys::Function;
use lunk::{
    link,
    EventGraph,
    HistPrim,
    Prim,
    ProcessingContext,
};
use reqwasm::http::Request;
use rooting::{
    el,
    scope_any,
    set_root,
    spawn_rooted,
    El,
    ScopeValue,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use shared::{
    bb,
    model::{
        link::{
            PrepareMedia,
            WsL2SReq,
            WsS2LNotify,
        },
        view::{
            LayoutIndividual,
            ViewPartList,
            WidgetNest,
        },
        C2SReq,
        FileHash,
        Node,
        Query,
    },
    unenum,
};
use tokio::sync::{
    broadcast,
    OnceCell,
};
use wasm_bindgen::{
    closure::Closure,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use wasm_bindgen_futures::spawn_local;
use web::{
    el_general::{
        async_event,
        el_audio,
        el_hbox,
        el_icon,
        el_vbox,
        el_video,
        log,
        ICON_VOLUME,
    },
    websocket::Ws,
};
use web_sys::{
    console::log_2,
    HtmlAudioElement,
    HtmlInputElement,
    HtmlMediaElement,
    MediaMetadata,
    MediaSession,
    MessageEvent,
    Url,
    WebSocket,
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
        let ws = Ws::<(), WsS2LNotify, WsL2SReq>::new({
            let state = state.clone();
            move |ws, message| {
                state.0.message_bg.set(scope_any(spawn_rooted({
                    let ws = ws.clone();
                    let state = state.clone();
                    async move {
                        match message {
                            WsS2LNotify::Prepare(prepare) => {
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
                                let play_at =
                                    match ws.request::<DateTime<Utc>>(WsL2SReq::Ready(Utc::now())).await {
                                        Ok(Some(r)) => r,
                                        Ok(None) => {
                                            return;
                                        },
                                        Err(e) => {
                                            log(format!("Received error from ready request: {}", e));
                                            return;
                                        },
                                    };
                                TimeoutFuture::new((play_at - Utc::now()).num_milliseconds().max(0) as u32).await;
                                media_el2.set_volume(*state.0.volume.borrow());
                                _ = media_el2.play().unwrap();
                            },
                            WsS2LNotify::Pause => {
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
                    vec![el_icon(ICON_VOLUME, "Volume"), el("input").attr("type", "range").on("input", {
                        let volume = volume.clone();
                        let eg = pc.eg();
                        move |ev| eg.event(|pc| {
                            let input = ev.dyn_ref::<HtmlInputElement>().unwrap();
                            volume.set(pc, input.value_as_number());
                        })
                    })],
                )
            ]).own(|_| (ws, link!((_pc = pc), (volume = volume), (), (state = state.clone()) {
                let media = state.0.media.borrow();
                let media = media.as_ref()?;
                media.set_volume(*volume.borrow());
            })))
        ]);
    });
}
