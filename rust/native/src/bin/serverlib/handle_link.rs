use std::sync::{
    atomic::Ordering,
    Arc,
    Mutex,
};
use chrono::Utc;
use futures::SinkExt;
use http::Response;
use http_body_util::{
    combinators::BoxBody,
    BodyExt,
    Full,
};
use hyper::body::Bytes;
use hyper_tungstenite::HyperWebsocket;
use loga::{
    ea,
    DebugDisplay,
    ErrContext,
    ResultContext,
};
use native::{
    ta_res,
    util::{
        spawn_scoped,
        Flag,
    },
};
use shared::{
    bb,
    model::link::{
        WsC2S,
        WsL2S,
        WsS2C,
        WsS2L,
    },
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
};
use tokio_stream::StreamExt;
use super::state::{
    State,
    WsState,
};

pub fn handle_ws_main(state: Arc<State>, websocket: HyperWebsocket) {
    let (tx, mut rx) = mpsc::channel(10);
    *state.link_main.lock().unwrap() = Some(Arc::new(WsState {
        send: tx,
        ready: Mutex::new(None),
    }));
    tokio::spawn(async move {
        match async {
            ta_res!(());
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_res!(bool);

                    select!{
                        m = rx.recv() => {
                            let Some(to_remote) = m else {
                                return Ok(false);
                            };
                            websocket
                                .send(
                                    hyper_tungstenite::tungstenite::Message::text(
                                        serde_json::to_string(&to_remote).unwrap(),
                                    ),
                                )
                                .await?;
                        },
                        m = websocket.next() => {
                            let Some(from_remote) = m else {
                                return Ok(false);
                            };
                            let from_remote = from_remote?;

                            bb!{
                                match &from_remote {
                                    hyper_tungstenite::tungstenite::Message::Text(m) => {
                                        match serde_json::from_str::<WsC2S>(
                                            &m,
                                        ).context("Error parsing message json")? {
                                            WsC2S::Prepare(prepare) => {
                                                let (main_ready_tx, main_ready_rx) = oneshot::channel();
                                                let Some(main) = state.link_main.lock().unwrap().clone() else {
                                                    continue;
                                                };
                                                *main.ready.lock().unwrap() = Some(main_ready_tx);
                                                let mut link_readies = vec![];
                                                let links =
                                                    state
                                                        .link_links
                                                        .lock()
                                                        .unwrap()
                                                        .values()
                                                        .cloned()
                                                        .collect::<Vec<_>>();
                                                for link in &links {
                                                    let link = link.clone();
                                                    let (ready_tx, ready_rx) = oneshot::channel();
                                                    *link.ready.lock().unwrap() = Some(ready_tx);
                                                    _ = link.send.send(WsS2L::Prepare(prepare.clone())).await;
                                                    link_readies.push(ready_rx);
                                                }
                                                *state.link_bg.lock().unwrap() = Some(spawn_scoped(async move {
                                                    let mut delays = vec![];
                                                    if let Ok(delay) = main_ready_rx.await {
                                                        delays.push(delay);
                                                    }
                                                    for ready in link_readies {
                                                        if let Ok(delay) = ready.await {
                                                            delays.push(delay);
                                                        }
                                                    }
                                                    delays.sort();
                                                    let delay = delays.last().unwrap();
                                                    let start_at = Utc::now() + *delay * 5;
                                                    _ = main.send.send(WsS2C::Play(start_at)).await;
                                                    for link in links {
                                                        _ = link.send.send(WsS2L::Play(start_at)).await;
                                                    }
                                                }));
                                            },
                                            WsC2S::Ready(sent_at) => {
                                                bb!{
                                                    let Some(main) = state.link_main.lock().unwrap().clone() else {
                                                        break;
                                                    };
                                                    let Some(ready) = main.ready.lock().unwrap().take() else {
                                                        break;
                                                    };
                                                    _ = ready.send(Utc::now() - sent_at);
                                                }
                                            },
                                            WsC2S::Pause => {
                                                let links =
                                                    state
                                                        .link_links
                                                        .lock()
                                                        .unwrap()
                                                        .values()
                                                        .cloned()
                                                        .collect::<Vec<_>>();
                                                for link in links {
                                                    _ = link.send.send(WsS2L::Pause);
                                                }
                                            },
                                        }
                                    },
                                    _ => {
                                        state
                                            .log
                                            .log_with(
                                                Flag::Debug,
                                                "Received unhandled websocket message type",
                                                ea!(message = from_remote.dbg_str()),
                                            );
                                    },
                                }
                            }
                        }
                    }

                    return Ok(true);
                }.await {
                    Ok(live) => {
                        if !live {
                            break;
                        }
                    },
                    Err(e) => {
                        state.log.log_err(Flag::Debug, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(Flag::Debug, e.context("Error in websocket connection"));
            },
        }
        *state.link_main.lock().unwrap() = None;
        *state.link_session.lock().unwrap() = None;
    });
}

pub fn handle_ws_link(state: Arc<State>, websocket: HyperWebsocket) {
    let id = state.link_ids.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = mpsc::channel(10);
    state.link_links.lock().unwrap().insert(id, Arc::new(WsState {
        send: tx,
        ready: Mutex::new(None),
    }));
    tokio::spawn(async move {
        match async {
            ta_res!(());
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_res!(bool);

                    select!{
                        m = rx.recv() => {
                            let Some(to_remote) = m else {
                                return Ok(false);
                            };
                            websocket
                                .send(
                                    hyper_tungstenite::tungstenite::Message::text(
                                        serde_json::to_string(&to_remote).unwrap(),
                                    ),
                                )
                                .await?;
                        },
                        m = websocket.next() => {
                            let Some(from_remote) = m else {
                                return Ok(false);
                            };
                            let from_remote = from_remote?;
                            match &from_remote {
                                hyper_tungstenite::tungstenite::Message::Text(m) => {
                                    match serde_json::from_str::<WsL2S>(
                                        &m,
                                    ).context("Error parsing message json")? {
                                        WsL2S::Ready(sent_at) => {
                                            bb!{
                                                let links = state.link_links.lock().unwrap();
                                                let Some(link) = links.get(&id) else {
                                                    break;
                                                };
                                                let Some(ready) = link.ready.lock().unwrap().take() else {
                                                    break;
                                                };
                                                _ = ready.send(Utc::now() - sent_at);
                                            }
                                        },
                                    }
                                },
                                _ => {
                                    state
                                        .log
                                        .log_with(
                                            Flag::Debug,
                                            "Received unhandled websocket message type",
                                            ea!(message = from_remote.dbg_str()),
                                        );
                                },
                            }
                        }
                    }

                    return Ok(true);
                }.await {
                    Ok(live) => {
                        if !live {
                            break;
                        }
                    },
                    Err(e) => {
                        state.log.log_err(Flag::Debug, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(Flag::Debug, e.context("Error in websocket connection"));
            },
        }
        state.link_links.lock().unwrap().remove(&id);
    });
}

pub fn handle_ws(
    state: Arc<State>,
    head: http::request::Parts,
    upgrade:
        Result<
            (hyper::Response<Full<Bytes>>, HyperWebsocket),
            hyper_tungstenite::tungstenite::error::ProtocolError,
        >,
    handle: fn(Arc<State>, HyperWebsocket) -> (),
) -> Response<BoxBody<Bytes, std::io::Error>> {
    let (response, websocket) = match upgrade {
        Ok(x) => x,
        Err(e) => {
            state.log.log_err(Flag::Warn, e.context_with("Error serving response", ea!(url = head.uri)));
            return Response::builder()
                .status(503)
                .body(http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed())
                .unwrap();
        },
    };
    handle(state, websocket);
    let (resp_header, resp_body) = response.into_parts();
    return Response::from_parts(resp_header, resp_body.map_err(|_| std::io::Error::other("")).boxed());
}
