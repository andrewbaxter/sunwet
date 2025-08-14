use {
    flowcontrol::{
        shed,
        ta_return,
    },
    gloo::{
        events::EventListener,
        timers::future::TimeoutFuture,
        utils::window,
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
        ElExt,
        Log,
    },
    wasm_bindgen::JsCast,
    web_sys::Element,
};

fn build_infinite_<
    K: 'static,
    T: Future<Output = Result<(Option<K>, Vec<El>), String>>,
    F: 'static + FnMut(K) -> T,
>(log: &Rc<dyn Log>, out: El, bg: Rc<RefCell<Option<ScopeValue>>>, initial_key: K, mut cb: F) {
    out.ref_push(el_async({
        let out = out.weak();
        let log = log.clone();
        async move {
            ta_return!(Vec < El >, String);
            let (next_key, children) = cb(initial_key).await?;
            if let Some(next_key) = next_key {
                *bg.borrow_mut() = Some(spawn_rooted({
                    let bg = bg.clone();
                    async move {
                        let (tx, mut rx) = mpsc::unbounded_channel();
                        _ = tx.send(());
                        TimeoutFuture::new(500).await;
                        let scroll_parent = {
                            let Some(at) = out.upgrade() else {
                                return;
                            };
                            let mut at = at.html().dyn_into::<Element>().unwrap();
                            loop {
                                if window()
                                    .get_computed_style(&at)
                                    .unwrap()
                                    .unwrap()
                                    .get_property_value("overflow-y")
                                    .as_ref()
                                    .map(|x| x.as_str()) ==
                                    Ok("auto") {
                                    break;
                                };
                                let Some(at1) = at.parent_element() else {
                                    log.log("Couldn't find scroll parent for infinite!");
                                    panic!();
                                };
                                at = at1;
                            }
                            at
                        };
                        let listener = EventListener::new(&scroll_parent, "scroll", move |_| {
                            _ = tx.send(());
                        });
                        shed!{
                            'trigger _;
                            while let Some(_) = rx.recv().await {
                                let scroll = scroll_parent.scroll_top();
                                let view_height = scroll_parent.client_height();
                                let full_height = scroll_parent.scroll_height();
                                if scroll + view_height * 3 / 2 > full_height {
                                    break 'trigger;
                                }
                            }
                            return;
                        }
                        drop(listener);
                        if let Some(out) = out.upgrade() {
                            build_infinite_(&log, out, bg, next_key, cb);
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
>(log: &Rc<dyn Log>, initial_key: K, cb: F) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let bg = Rc::new(RefCell::new(None));
    out.ref_own(|_| bg.clone());
    build_infinite_(log, out.clone(), bg, initial_key, cb);
    return out;
}
