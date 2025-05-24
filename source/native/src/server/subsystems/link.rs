use {
    crate::{
        server::state::{
            LinkSessionState,
            State,
            WsLinkState,
        },
        spawn_scoped,
    },
    by_address::ByAddress,
    chrono::{
        Duration,
        Utc,
    },
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
        SourceUrl,
        WsC2S,
        WsL2S,
        WsS2C,
        WsS2L,
    },
    std::{
        future::Future,
        sync::{
            Arc,
            Mutex,
        },
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

pub async fn handle_ws_main(state: Arc<State>, session: String, websocket: HyperWebsocket) {
    let (s2c_tx, mut s2c_rx) = mpsc::channel(10);
    let session_state = state.link_sessions.entry(session.clone()).and_upsert_with(|x| async {
        match x {
            Some(x) => {
                return x.into_value();
            },
            None => {
                return Arc::new(LinkSessionState {
                    links: Default::default(),
                    public_files: Default::default(),
                });
            },
        }
    }).await.into_value();
    tokio::spawn(async move {
        match async {
            ta_return!((), loga::Error);
            let mut websocket = websocket.await?;
            let main_ready = Arc::new(Mutex::new(None));
            let s2c_tx = Arc::new(s2c_tx);
            loop {
                let s2c_tx = s2c_tx.clone();
                match async {
                    ta_return!(bool, loga::Error);
                    select!{
                        m = s2c_rx.recv() => {
                            // Need to do this to avoid websocket movement issues.
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
                                                // Make referenced files temporarily public
                                                {
                                                    let mut link_public = session_state.public_files.lock().unwrap();
                                                    link_public.clear();
                                                    match &prepare.media {
                                                        shared::interface::wire::link::PrepareMedia::Audio(m) => {
                                                            if let SourceUrl::File(file) = &m.source_url {
                                                                link_public.insert(file.clone());
                                                            }
                                                            if let Some(SourceUrl::File(file)) = &m.cover_source_url {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                        shared::interface::wire::link::PrepareMedia::Video(m) => {
                                                            if let SourceUrl::File(file) = m {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                        shared::interface::wire::link::PrepareMedia::Image(m) => {
                                                            if let SourceUrl::File(file) = m {
                                                                link_public.insert(file.clone());
                                                            }
                                                        },
                                                    }
                                                }

                                                // .
                                                let links = session_state.links.lock().unwrap().clone();

                                                // Start waiting for ready reports
                                                let (main_ready_tx, main_ready_rx) = oneshot::channel();
                                                *main_ready.lock().unwrap() = Some(main_ready_tx);
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
                                                        let delay = delays.into_iter().max().unwrap_or(Duration::zero());
                                                        let start_at = Utc::now() + delay * 5;
                                                        let mut bg =
                                                            vec![
                                                                s2c_tx
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
                                                    let Some(ready) = main_ready.lock().unwrap().take() else {
                                                        break;
                                                    };
                                                    _ = ready.send(Utc::now() - sent_at);
                                                }
                                            },
                                            WsC2S::Pause => {
                                                let links = session_state.links.lock().unwrap().clone();
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
    });
}

pub async fn handle_ws_link(state: Arc<State>, session: String, websocket: HyperWebsocket) {
    let Some(session_state) = state.link_sessions.get(&session).await else {
        return;
    };
    let (tx, mut rx) = mpsc::channel(10);
    let link_state = Arc::new(WsLinkState {
        send: tx,
        ready: Mutex::new(None),
    });
    session_state.links.lock().unwrap().insert(ByAddress::from(link_state.clone()));
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
                                                let Some(ready) = link_state.ready.lock().unwrap().take() else {
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
        session_state.links.lock().unwrap().remove(&ByAddress::from(link_state));
    });
}

pub async fn handle_link_ws<
    F: Future<Output = ()>,
>(
    state: Arc<State>,
    session_id: String,
    head: http::request::Parts,
    upgrade:
        Result<
            (hyper::Response<Full<Bytes>>, HyperWebsocket),
            hyper_tungstenite::tungstenite::error::ProtocolError,
        >,
    handle: fn(Arc<State>, String, HyperWebsocket) -> F,
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
    handle(state, session_id, websocket).await;
    let (resp_header, resp_body) = response.into_parts();
    return Response::from_parts(resp_header, resp_body.map_err(|_| std::io::Error::other("")).boxed());
}
