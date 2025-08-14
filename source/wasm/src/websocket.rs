use {
    crate::{
        async_::WaitVal,
        js::Log,
    },
    flowcontrol::shed,
    gloo::{
        events::EventListener,
        timers::future::TimeoutFuture,
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
    std::{
        cell::Cell,
        rc::{
            Rc,
            Weak,
        },
    },
    wasm_bindgen::JsCast,
    web_sys::{
        MessageEvent,
        WebSocket,
    },
};

pub struct Ws_<S: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned> {
    url: String,
    ws: WaitVal<WebSocket>,
    ws_state: Cell<ScopeValue>,
    handler: Box<dyn Fn(&Ws<S, R>, R) -> ()>,
    log: Rc<dyn Log>,
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

    pub fn new(
        log: Rc<dyn Log>,
        base_url: &str,
        path: impl ToString,
        notify_handler: impl 'static + Fn(&Ws<S, R>, R) -> (),
    ) -> Self {
        let path = path.to_string();
        let noschema_base_url = shed!{
            if let Some(u) = base_url.strip_prefix("http://") {
                break u;
            };
            if let Some(u) = base_url.strip_prefix("https://") {
                break u;
            };
            panic!();
        };
        let url = format!("wss://{}{}", noschema_base_url, path);

        fn connect<
            S: 'static + Serialize + DeserializeOwned,
            R: 'static + Serialize + DeserializeOwned,
        >(state: Ws<S, R>) {
            let ws = match WebSocket::new(&state.0.url) {
                Ok(ws) => ws,
                Err(e) => {
                    state.0.log.log_js("Error creating websocket", &e);
                    delay_reconnect(state);
                    return;
                },
            };
            state.0.ws_state.set(scope_any((
                //. .
                EventListener::once(&ws, "open", {
                    let ws = ws.clone();
                    let state = state.weak();
                    move |ev| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        state.log.log_js("DEBUG Got websocket open event", ev);
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
                        state.log.log_js("DEBUG Got websocket message event", e);
                        let state = Ws(state);
                        let ev = e.dyn_ref::<MessageEvent>().unwrap();
                        let body = match ev.data().dyn_into::<js_sys::JsString>() {
                            Ok(v) => v,
                            Err(e) => {
                                state.0.log.log_js2("Received non-string message", &e, &ev.data());
                                return;
                            },
                        };
                        let body = body.as_string().unwrap();
                        let message = match serde_json::from_str::<R>(&body) {
                            Ok(v) => v,
                            Err(e) => {
                                state.0.log.log(&format!("Failed to deserialize message: {}\nMessage: {}", e, body));
                                return;
                            },
                        };
                        (state.0.handler)(&state, message);
                    }
                }),
                EventListener::once(&ws, "error", {
                    let state = state.weak();
                    move |e| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        state.log.log_js("Websocket closed with error (reconnecting)", e);
                    }
                }),
                EventListener::once(&ws, "close", {
                    let state = state.weak();
                    move |ev| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        state.log.log_js("DEBUG Got websocket close event", ev);
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
                let state = state.weak();
                async move {
                    TimeoutFuture::new(1000).await;
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    connect(Ws(state));
                }
            }));
        }

        let out = Ws(Rc::new(Ws_ {
            url: url,
            ws: WaitVal::new(),
            ws_state: Cell::new(scope_any(())),
            handler: Box::new(notify_handler),
            log: log.clone(),
        }));
        connect(out.clone());
        return out;
    }

    pub async fn send(&self, data: S) {
        loop {
            match self.0.ws.get().await.send_with_str(&serde_json::to_string(&data).unwrap()) {
                Ok(_) => break,
                Err(e) => {
                    self.0.log.log_js("Error sending notification; retrying", &e);
                    TimeoutFuture::new(1000).await;
                },
            }
        }
    }
}
