use {
    flowcontrol::ta_return,
    gloo::{
        events::EventListener,
        utils::{
            document,
            window,
        },
    },
    rooting::{
        spawn_rooted,
        El,
        ScopeValue,
    },
    std::{
        cell::RefCell,
        future::Future,
        rc::Rc,
    },
    tokio::sync::mpsc,
    wasm::js::el_async,
};

fn build_infinite_<
    K: 'static,
    T: Future<Output = Result<(Option<K>, Vec<El>), String>>,
    F: 'static + FnMut(K) -> T,
>(out: El, bg: Rc<RefCell<Option<ScopeValue>>>, initial_key: K, mut cb: F) {
    out.ref_push(el_async({
        let out = out.weak();
        async move {
            ta_return!(Vec < El >, String);
            let (next_key, children) = cb(initial_key).await?;
            if let Some(next_key) = next_key {
                *bg.borrow_mut() = Some(spawn_rooted({
                    let bg = bg.clone();
                    async move {
                        let (tx, mut rx) = mpsc::unbounded_channel();
                        _ = tx.send(());
                        let _listener = EventListener::new(&window(), "scroll", move |_| {
                            _ = tx.send(());
                        });
                        while let Some(_) = rx.recv().await {
                            let html = document().body().unwrap().parent_element().unwrap();
                            if html.scroll_top() + html.client_height() * 3 / 2 > html.scroll_height() {
                                break;
                            }
                        }
                        if let Some(out) = out.upgrade() {
                            build_infinite_(out, bg, next_key, cb);
                        }
                    }
                }));
            }
            return Ok(children);
        }
    }));
}

pub fn build_infinite<
    K: 'static,
    T: Future<Output = Result<(Option<K>, Vec<El>), String>>,
    F: 'static + FnMut(K) -> T,
>(out: El, initial_key: K, cb: F) {
    let bg = Rc::new(RefCell::new(None));
    out.ref_own(|_| bg.clone());
    build_infinite_(out, bg, initial_key, cb);
}
