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
        El,
        ScopeValue,
        spawn_rooted,
    },
    shared_wasm::log::Log,
    std::{
        cell::RefCell,
        future::Future,
        rc::Rc,
    },
    tokio::sync::{
        mpsc,
        oneshot,
    },
    wasm::js::{
        ElExt,
        el_async,
        style_export,
    },
    wasm_bindgen::JsCast,
    web_sys::Element,
};

pub struct InfPageRes<K> {
    pub next_key: Option<K>,
    pub page_els: Vec<El>,
    pub immediate_advance: bool,
}

pub struct InfAdvanceMsg {
    /// false = check scroll position, true = force advance
    pub force: bool,
    /// If provided, signaled after the next page has loaded
    pub loaded: Option<oneshot::Sender<()>>,
}

pub type InfAdvanceSlot = Rc<RefCell<Option<mpsc::UnboundedSender<InfAdvanceMsg>>>>;

fn build_infinite_<
    K: 'static,
    T: Future<Output = Result<InfPageRes<K>, String>>,
    F: 'static + FnMut(K) -> T,
>(
    log: &Rc<dyn Log>,
    out: El,
    bg: Rc<RefCell<Option<ScopeValue>>>,
    initial_key: K,
    advance_tx_slot: Option<InfAdvanceSlot>,
    loaded_signal: Option<oneshot::Sender<()>>,
    mut cb: F,
) {
    out.ref_push(el_async({
        let out = out.weak();
        let log = log.clone();
        async move {
            ta_return!(Vec < El >, String);
            let page_res = cb(initial_key).await?;
            if let Some(loaded_signal) = loaded_signal {
                _ = loaded_signal.send(());
            }
            if let Some(next_key) = page_res.next_key {
                let immediate_advance = page_res.immediate_advance;
                *bg.borrow_mut() = Some(spawn_rooted({
                    let bg = bg.clone();
                    let advance_tx_slot = advance_tx_slot.clone();
                    async move {
                        let loaded_signal;
                        if immediate_advance {
                            loaded_signal = None;
                        } else {
                            let (tx, mut rx) = mpsc::unbounded_channel::<InfAdvanceMsg>();
                            _ = tx.send(InfAdvanceMsg { force: false, loaded: None });
                            if let Some(slot) = &advance_tx_slot {
                                *slot.borrow_mut() = Some(tx.clone());
                            }
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
                            let listener = EventListener::new(&scroll_parent, "scroll", {
                                let tx = tx.clone();
                                move |_| {
                                    _ = tx.send(InfAdvanceMsg { force: false, loaded: None });
                                }
                            });
                            loaded_signal = shed!{
                                'trigger _;
                                while let Some(msg) = rx.recv().await {
                                    if msg.force {
                                        break 'trigger msg.loaded;
                                    }
                                    let scroll = scroll_parent.scroll_top();
                                    let view_height = scroll_parent.client_height();
                                    let full_height = scroll_parent.scroll_height();
                                    if scroll + view_height * 3 / 2 > full_height {
                                        break 'trigger msg.loaded;
                                    }
                                }
                                return;
                            };
                            drop(listener);
                        }
                        if let Some(out) = out.upgrade() {
                            build_infinite_(&log, out, bg, next_key, advance_tx_slot, loaded_signal, cb);
                        }
                    }
                }));
            } else {
                // No more pages — clear the advance slot
                if let Some(slot) = &advance_tx_slot {
                    *slot.borrow_mut() = None;
                }
            }
            return Ok(page_res.page_els);
        }
    }));
}

pub fn build_infinite<
    K: 'static,
    T: Future<Output = Result<InfPageRes<K>, String>>,
    F: 'static + FnMut(K) -> T,
>(log: &Rc<dyn Log>, initial_key: K, advance_tx_slot: Option<InfAdvanceSlot>, cb: F) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let bg = Rc::new(RefCell::new(None));
    out.ref_own(|_| bg.clone());
    build_infinite_(log, out.clone(), bg, initial_key, advance_tx_slot, None, cb);
    return out;
}
