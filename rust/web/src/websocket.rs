use std::{
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    rc::{
        Rc,
        Weak,
    },
};
use futures::channel::oneshot::{
    channel,
    Sender,
};
use gloo::{
    events::EventListener,
    timers::future::TimeoutFuture,
    utils::window,
};
use rooting::{
    scope_any,
    spawn_rooted,
    ScopeValue,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use shared::model::link::WsMessage;
use wasm_bindgen::JsCast;
use web_sys::{
    MessageEvent,
    WebSocket,
};
use crate::{
    async_::WaitVal,
    el_general::{
        log,
        log_js,
        log_js2,
    },
};

pub struct Ws_<SN: Serialize + DeserializeOwned, RN: Serialize + DeserializeOwned, SR: Serialize + DeserializeOwned> {
    ws: WaitVal<WebSocket>,
    ws_state: Cell<ScopeValue>,
    send_index: Cell<usize>,
    history: RefCell<Vec<WsMessage>>,
    requests: RefCell<HashMap<usize, Sender<serde_json::Value>>>,
    notify_handler: Box<dyn Fn(&Ws<SN, RN, SR>, RN) -> ()>,
}

pub struct Ws<SN: Serialize + DeserializeOwned, RN: Serialize + DeserializeOwned, SR: Serialize + DeserializeOwned>(
    Rc<Ws_<SN, RN, SR>>,
);

impl<
    SN: Serialize + DeserializeOwned,
    RN: Serialize + DeserializeOwned,
    SR: Serialize + DeserializeOwned,
> Clone for Ws<SN, RN, SR> {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}

impl<
    SN: 'static + Serialize + DeserializeOwned,
    RN: 'static + Serialize + DeserializeOwned,
    SR: 'static + Serialize + DeserializeOwned,
> Ws<SN, RN, SR> {
    pub fn weak(&self) -> Weak<Ws_<SN, RN, SR>> {
        return Rc::downgrade(&self.0);
    }

    pub fn new(notify_handler: impl 'static + Fn(&Ws<SN, RN, SR>, RN) -> ()) -> Self {
        fn connect<
            SN: 'static + Serialize + DeserializeOwned,
            RN: 'static + Serialize + DeserializeOwned,
            SR: 'static + Serialize + DeserializeOwned,
        >(state: Ws<SN, RN, SR>) {
            let ws = match WebSocket::new(&format!("wss://{}", window().location().host().unwrap().as_str())) {
                Ok(ws) => ws,
                Err(e) => {
                    log_js("Error creating websocket", &e);
                    delay_reconnect(state);
                    return;
                },
            };
            state.0.ws_state.set(scope_any((
                //. .
                EventListener::once(&ws, "open", {
                    let ws = ws.clone();
                    let state = state.weak();
                    move |_| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        let state = Ws(state);
                        let mut send = vec![];
                        for message in &*state.0.history.borrow() {
                            send.push(serde_json::to_string(&message).unwrap());
                        }
                        for message in send {
                            match ws.send_with_str(&message) {
                                Ok(_) => { },
                                Err(e) => {
                                    log_js("Error resending unacked history message", &e);
                                    delay_reconnect(state);
                                    return;
                                },
                            }
                        }
                        state.0.ws.set(Some(ws));
                    }
                }),
                EventListener::new(&ws, "message", {
                    let ws = ws.clone();
                    let state = state.weak();
                    move |e| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        let state = Ws(state);
                        let ev = e.dyn_ref::<MessageEvent>().unwrap();
                        let body = match ev.data().dyn_into::<js_sys::JsString>() {
                            Ok(v) => v,
                            Err(e) => {
                                log_js2("Received non-string message", &e, &ev.data());
                                return;
                            },
                        };
                        let body = body.as_string().unwrap();
                        let message = match serde_json::from_str::<WsMessage>(&body) {
                            Ok(v) => v,
                            Err(e) => {
                                log(format!("Failed to deserialize message: {}\nMessage: {}", e, body));
                                return;
                            },
                        };
                        match message {
                            WsMessage::Ack(index) => {
                                state.0.history.borrow_mut().retain(|message| match message {
                                    WsMessage::Ack(_) => unreachable!(),
                                    WsMessage::Notify((i, _)) => return *i > index,
                                    WsMessage::Request((i, _)) => return *i > index,
                                });
                            },
                            WsMessage::Notify((index, data)) => {
                                match ws.send_with_str(&serde_json::to_string(&WsMessage::Ack(index)).unwrap()) {
                                    Ok(_) => { },
                                    Err(e) => {
                                        log_js("Error sending ack", &e);
                                        return;
                                    },
                                };
                                let data = match serde_json::from_value(data.clone()) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        log(
                                            format!(
                                                "Failed deserializing ws notification, discarding: {}\nBody: {:?}",
                                                e.to_string(),
                                                data
                                            ),
                                        );
                                        return;
                                    },
                                };
                                (state.0.notify_handler)(&state, data);
                            },
                            WsMessage::Request((index, data)) => {
                                match ws.send_with_str(&serde_json::to_string(&WsMessage::Ack(index)).unwrap()) {
                                    Ok(_) => { },
                                    Err(_) => {
                                        log_js("Error sending ack", ev);
                                        return;
                                    },
                                };
                                if let Some(req) = state.0.requests.borrow_mut().remove(&index) {
                                    _ = req.send(data);
                                }
                            },
                        }
                    }
                }),
                EventListener::once(&ws, "error", {
                    move |e| {
                        log_js("Websocket closed with error (reconnecting)", e);
                    }
                }),
                EventListener::once(&ws, "close", {
                    let state = state.weak();
                    move |_| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        let state = Ws(state);
                        delay_reconnect(state);
                    }
                }),
            )));
        }

        fn delay_reconnect<
            SN: 'static + Serialize + DeserializeOwned,
            RN: 'static + Serialize + DeserializeOwned,
            SR: 'static + Serialize + DeserializeOwned,
        >(state: Ws<SN, RN, SR>) {
            state.0.ws.set(None);
            state.0.ws_state.set(spawn_rooted({
                let state = state.clone();
                async move {
                    TimeoutFuture::new(1000).await;
                    connect(state);
                }
            }));
        }

        let out = Ws(Rc::new(Ws_ {
            ws: WaitVal::new(),
            ws_state: Cell::new(scope_any(())),
            send_index: Cell::new(0),
            history: RefCell::new(vec![]),
            requests: RefCell::new(HashMap::new()),
            notify_handler: Box::new(notify_handler),
        }));
        connect(out.clone());
        return out;
    }

    pub async fn notify(&self, data: SN) {
        let index = self.0.send_index.get();
        self.0.send_index.set(index + 1);
        let message = WsMessage::Notify((index, serde_json::to_value(&data).unwrap()));
        let message_str = serde_json::to_string(&message).unwrap();
        self.0.history.borrow_mut().push(message);
        loop {
            match self.0.ws.get().await.send_with_str(&message_str) {
                Ok(_) => break,
                Err(e) => {
                    log_js("Error sending notification; retrying", &e);
                    TimeoutFuture::new(1000).await;
                },
            }
        }
    }

    // Resp is None if shutting down (dropped).
    pub async fn request<RR: DeserializeOwned>(&self, req: SR) -> Result<Option<RR>, String> {
        let index = self.0.send_index.get();
        self.0.send_index.set(index + 1);
        let message = WsMessage::Request((index, serde_json::to_value(&req).unwrap()));
        let message_str = serde_json::to_string(&message).unwrap();
        self.0.history.borrow_mut().push(message);
        let (tx, rx) = channel();
        self.0.requests.borrow_mut().insert(index, tx);
        loop {
            match self.0.ws.get().await.send_with_str(&message_str) {
                Ok(_) => break,
                Err(e) => {
                    log_js("Error sending request; retrying", &e);
                    TimeoutFuture::new(1000).await;
                },
            }
        }
        match rx.await {
            Ok(v) => match serde_json::from_value(v.clone()) {
                Ok(v) => return Ok(Some(v)),
                Err(e) => {
                    return Err(
                        format!(
                            "Received invalid response to request {:?}: {}\nBody: {:?}",
                            serde_json::to_value(&req),
                            e.to_string(),
                            v
                        ),
                    );
                },
            },
            Err(_) => return Ok(None),
        }
    }
}
