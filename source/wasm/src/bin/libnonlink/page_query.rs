use {
    super::{
        infinite::build_infinite,
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::{
            record_replace_ministate,
            MinistateQuery,
        },
        state::set_page,
    },
    gloo::timers::callback::Timeout,
    js_sys::Math,
    lunk::ProcessingContext,
    rooting::El,
    shared::{
        interface::wire::{
            Pagination,
            ReqQuery,
        },
        query_parser::compile_query,
    },
    std::{
        cell::RefCell,
        u64,
    },
    wasm::js::style_export::{
        self,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlElement,
};

fn refresh(results_group: El, text: &str) {
    record_replace_ministate(
        &state().log,
        &super::ministate::Ministate::Query(MinistateQuery { query: Some(text.to_string()) }),
    );
    let query = match compile_query(None, &text) {
        Ok(q) => q,
        Err(e) => {
            results_group.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                data: e,
                in_root: false,
            }).root);
            return;
        },
    };
    results_group.ref_push(build_infinite(&state().log, None, {
        let seed = (Math::random() * u64::MAX as f64) as u64;
        move |key| {
            let query = query.clone();
            async move {
                let page_data = req_post_json(&state().env.base_url, ReqQuery {
                    query: query.clone(),
                    parameters: Default::default(),
                    pagination: Some(Pagination {
                        count: 100,
                        seed: Some(seed),
                        key: key,
                    }),
                }).await?;
                let mut out = vec![];
                for row in page_data.records {
                    out.push(
                        style_export::leaf_query_row(
                            style_export::LeafQueryRowArgs { data: serde_json::to_string_pretty(&row).unwrap() },
                        ).root,
                    );
                }
                return Ok((page_data.next_page_key.map(|x| Some(x)), out));
            }
        }
    }));
}

pub fn build_page_query(pc: &mut ProcessingContext, ms: &MinistateQuery) {
    let initial_query = ms.query.clone().unwrap_or_else(|| "\"hello world\" { => value }".to_string());
    let style_res =
        style_export::cont_page_query(style_export::ContPageQueryArgs { initial_query: initial_query.clone() });
    let results_group = style_res.results;
    refresh(results_group.clone(), &initial_query);
    style_res.query.ref_on("input", {
        let debounce = RefCell::new(None);
        move |ev| {
            let ev_target = ev.target();
            *debounce.borrow_mut() = Some(Timeout::new(500, {
                let results_group = results_group.clone();
                move || {
                    let text =
                        ev_target.unwrap().dyn_ref::<HtmlElement>().unwrap().text_content().unwrap_or_default();
                    results_group.ref_clear();
                    refresh(results_group, &text);
                }
            }));
        }
    });
    set_page(pc, "Query", style_res.root);
}
