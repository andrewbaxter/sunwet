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
    wasm::js::{
        el_async,
        style_export,
    },
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
                        let listener = EventListener::new(&window(), "scroll", move |_| {
                            _ = tx.send(());
                        });
                        while let Some(_) = rx.recv().await {
                            let html = document().body().unwrap().parent_element().unwrap();
                            let scroll = html.scroll_top();
                            let view_height = html.client_height();
                            let full_height = html.scroll_height();
                            if scroll + view_height * 3 / 2 > full_height {
                                break;
                            }
                        }
                        drop(listener);
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
>(initial_key: K, cb: F) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let bg = Rc::new(RefCell::new(None));
    out.ref_own(|_| bg.clone());
    build_infinite_(out.clone(), bg, initial_key, cb);
    return out;
}
