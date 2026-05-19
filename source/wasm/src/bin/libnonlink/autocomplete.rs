use {
    super::api::req_post_json,
    gloo::timers::callback::Timeout,
    rooting::El,
    shared::interface::wire::C2SReqTrait,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlInputElement,
};

/// Wire up a datalist to an input element. When the input changes, after a 1s
/// debounce, calls `build_req` with the (prefix, suffix) split at cursor position,
/// then populates the datalist with the response strings.
pub fn wire_autocomplete<
    R: 'static + C2SReqTrait<Resp = Vec<String>> + Clone,
    F: 'static + Fn(String, String) -> R + Clone,
>(input: &El, datalist: &El, build_req: F) {
    let debounce: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    input.ref_on("input", {
        let datalist = datalist.weak();
        let debounce = debounce.clone();
        move |ev| {
            let Some(target) = ev.target() else {
                return;
            };
            let Ok(input_el) = target.dyn_into::<HtmlInputElement>() else {
                return;
            };
            let value = input_el.value();
            let cursor_pos = input_el.selection_start().ok().flatten().unwrap_or(value.len() as u32) as usize;
            let cursor_pos = cursor_pos.min(value.len());
            let prefix = value[..cursor_pos].to_string();
            let suffix = value[cursor_pos..].to_string();
            let datalist = datalist.clone();
            let build_req = build_req.clone();
            *debounce.borrow_mut() = Some(Timeout::new(1_000, move || {
                let Some(datalist) = datalist.upgrade() else {
                    return;
                };
                wasm_bindgen_futures::spawn_local(async move {
                    let req = build_req(prefix, suffix);
                    match req_post_json(req).await {
                        Ok(suggestions) => {
                            datalist.ref_clear();
                            for s in suggestions {
                                let option =
                                    web_sys::window()
                                        .unwrap()
                                        .document()
                                        .unwrap()
                                        .create_element("option")
                                        .unwrap();
                                option.set_attribute("value", &s).unwrap();
                                datalist
                                    .raw()
                                    .dyn_ref::<web_sys::Element>()
                                    .unwrap()
                                    .append_child(&option)
                                    .unwrap();
                            }
                        },
                        Err(_) => {
                            // Silently ignore autocomplete errors
                        },
                    }
                });
            }));
        }
    });
}
