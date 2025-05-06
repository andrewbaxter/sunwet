use {
    crate::{
        server::state::{
            State,
            WsState,
        },
        spawn_scoped,
    },
    chrono::Utc,
    flowcontrol::{
        shed,
        ta_return,
    },
    futures::{
        future::join_all,
        FutureExt,
        SinkExt,
    },
    http::Response,
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
        Full,
    },
    hyper::body::Bytes,
    hyper_tungstenite::HyperWebsocket,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        ResultContext,
    },
    shared::interface::wire::link::{
        WsC2S,
        WsL2S,
        WsS2C,
        WsS2L,
    },
    std::sync::{
        atomic::Ordering,
        Arc,
        Mutex,
    },
    tokio::{
        select,
        sync::{
            mpsc,
            oneshot,
        },
    },
    tokio_stream::StreamExt,
};

pub fn handle_ws_main(state: Arc<State>, websocket: HyperWebsocket) {
    let (tx, mut rx) = mpsc::channel(10);
    *state.link_main.lock().unwrap() = Some(Arc::new(WsState {
        send: tx,
        ready: Mutex::new(None),
    }));
    tokio::spawn(async move {
        match async {
            ta_return!((), loga::Error);
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_return!(bool, loga::Error);
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
                            shed!{
                                match &from_remote {
                                    hyper_tungstenite::tungstenite::Message::Text(m) => {
                                        eprintln!("got main ws message {}", m);
                                        match serde_json::from_str::<WsC2S>(
                                            &m,
                                        ).context("Error parsing message json")? {
                                            WsC2S::Prepare(prepare) => {
                                                let Some(main) = state.link_main.lock().unwrap().clone() else {
                                                    continue;
                                                };

                                                // Make referenced files temporarily public
                                                {
                                                    let mut link_public = state.link_public_files.lock().unwrap();
                                                    link_public.clear();
                                                    match &prepare.media {
                                                        shared::interface::wire::link::PrepareMedia::Audio(m) => {
                                                            if let Some(file) = m.source_url.file.as_ref() {
                                                                link_public.insert(file.clone());
                                                            }
                                                            if let Some(file) =
                                                                m
                                                                    .cover_source_url
                                                                    .as_ref()
                                                                    .and_then(|x| x.file.as_ref()) {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                        shared::interface::wire::link::PrepareMedia::Video(m) => {
                                                            if let Some(file) = m.file.as_ref() {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                        shared::interface::wire::link::PrepareMedia::Image(m) => {
                                                            if let Some(file) = m.file.as_ref() {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                    }
                                                }

                                                // .
                                                let links = state.link_links.lock().unwrap().values().cloned().collect::<Vec<_>>();

                                                // Start waiting for ready reports
                                                let (main_ready_tx, main_ready_rx) = oneshot::channel();
                                                *main.ready.lock().unwrap() = Some(main_ready_tx);
                                                let mut link_readies = vec![];
                                                for link in &links {
                                                    let (ready_tx, ready_rx) = oneshot::channel();
                                                    *link.ready.lock().unwrap() = Some(ready_tx);
                                                    link_readies.push(ready_rx);
                                                }
                                                *state.link_bg.lock().unwrap() = Some(spawn_scoped({
                                                    let links = links.clone();
                                                    async move {
                                                        let mut delays = vec![];
                                                        eprintln!(
                                                            "waiting for readies (link {}) {}",
                                                            link_readies.len(),
                                                            Utc::now().to_rfc3339()
                                                        );
                                                        if let Ok(delay) = main_ready_rx.await {
                                                            eprintln!("ready - got main");
                                                            delays.push(delay);
                                                        }
                                                        for ready in link_readies {
                                                            let Ok(delay) = ready.await else {
                                                                eprintln!("ready - dc");
                                                                continue;
                                                            };
                                                            eprintln!("ready - got");
                                                            delays.push(delay);
                                                        }
                                                        eprintln!("all readies ready {}", Utc::now().to_rfc3339());

                                                        // All readies received, trigger start
                                                        let delay = delays.into_iter().max().unwrap();
                                                        let start_at = Utc::now() + delay * 5;
                                                        let mut bg =
                                                            vec![
                                                                main
                                                                    .send
                                                                    .send(WsS2C::Play(start_at))
                                                                    .map(|_| ())
                                                                    .boxed()
                                                            ];
                                                        for link in &links {
                                                            bg.push(
                                                                link
                                                                    .send
                                                                    .send(WsS2L::Play(start_at))
                                                                    .map(|_| ())
                                                                    .boxed(),
                                                            );
                                                        }
                                                        _ = join_all(bg).await;
                                                    }
                                                }));

                                                // Forward prepare data
                                                {
                                                    let mut bg = vec![];
                                                    for link in &links {
                                                        bg.push(link.send.send(WsS2L::Prepare(prepare.clone())));
                                                    }
                                                    _ = join_all(bg).await;
                                                }
                                            },
                                            WsC2S::Ready(sent_at) => {
                                                shed!{
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
                                                {
                                                    let mut bg = vec![];
                                                    for link in &links {
                                                        bg.push(link.send.send(WsS2L::Pause));
                                                    }
                                                    _ = join_all(bg).await;
                                                }
                                            },
                                        }
                                    },
                                    _ => {
                                        state
                                            .log
                                            .log_with(
                                                loga::DEBUG,
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
                        state.log.log_err(loga::DEBUG, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(loga::DEBUG, e.context("Error in websocket connection"));
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
            ta_return!((), loga::Error);
            let mut websocket = websocket.await?;
            loop {
                match async {
                    ta_return!(bool, loga::Error);
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
                                    eprintln!("got link ws message {}", m);
                                    match serde_json::from_str::<WsL2S>(&m).context("Error parsing message json")? {
                                        WsL2S::Ready(sent_at) => {
                                            shed!{
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
                                            loga::DEBUG,
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
                        state.log.log_err(loga::DEBUG, e.context("Error handling event in websocket task"));
                    },
                }
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                state.log.log_err(loga::DEBUG, e.context("Error in link websocket connection"));
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
            state.log.log_err(loga::WARN, e.context_with("Error serving response", ea!(url = head.uri)));
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
