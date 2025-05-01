use {
    std::{
        cell::{
            Cell,
        },
        rc::{
            Rc,
            Weak,
        },
    },
    gloo::{
        events::EventListener,
        timers::future::TimeoutFuture,
        utils::window,
    },
    rooting::{
        scope_any,
        spawn_rooted,
        ScopeValue,
    },
    serde::{
        de::DeserializeOwned,
        Serialize,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        MessageEvent,
        WebSocket,
    },
    crate::{
        async_::WaitVal,
        js::{
            log,
            log_js,
            log_js2,
        },
    },
};

pub struct Ws_<S: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned> {
    path: String,
    ws: WaitVal<WebSocket>,
    ws_state: Cell<ScopeValue>,
    handler: Box<dyn Fn(&Ws<S, R>, R) -> ()>,
}

pub struct Ws<S: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned>(Rc<Ws_<S, R>>);

impl<S: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned> Clone for Ws<S, R> {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}

impl<S: 'static + Serialize + DeserializeOwned, R: 'static + Serialize + DeserializeOwned> Ws<S, R> {
    pub fn weak(&self) -> Weak<Ws_<S, R>> {
        return Rc::downgrade(&self.0);
    }

    pub fn new(path: impl ToString, notify_handler: impl 'static + Fn(&Ws<S, R>, R) -> ()) -> Self {
        let path = path.to_string();

        fn connect<
            S: 'static + Serialize + DeserializeOwned,
            R: 'static + Serialize + DeserializeOwned,
        >(state: Ws<S, R>) {
            let ws =
                match WebSocket::new(
                    &format!("wss://{}/{}", window().location().host().unwrap().as_str(), state.0.path),
                ) {
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
                        state.0.ws.set(Some(ws));
                    }
                }),
                EventListener::new(&ws, "message", {
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
                        let message = match serde_json::from_str::<R>(&body) {
                            Ok(v) => v,
                            Err(e) => {
                                log(format!("Failed to deserialize message: {}\nMessage: {}", e, body));
                                return;
                            },
                        };
                        (state.0.handler)(&state, message);
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
            S: 'static + Serialize + DeserializeOwned,
            R: 'static + Serialize + DeserializeOwned,
        >(state: Ws<S, R>) {
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
            path: path,
            ws: WaitVal::new(),
            ws_state: Cell::new(scope_any(())),
            handler: Box::new(notify_handler),
        }));
        connect(out.clone());
        return out;
    }

    pub async fn send(&self, data: S) {
        loop {
            match self.0.ws.get().await.send_with_str(&serde_json::to_string(&data).unwrap()) {
                Ok(_) => break,
                Err(e) => {
                    log_js("Error sending notification; retrying", &e);
                    TimeoutFuture::new(1000).await;
                },
            }
        }
    }
}
