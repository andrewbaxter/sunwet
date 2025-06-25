use {
    super::{
        infinite::build_infinite,
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::MinistateQuery,
        state::set_page,
    },
    gloo::timers::callback::Timeout,
    js_sys::Math,
    lunk::ProcessingContext,
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

pub fn build_page_query(pc: &mut ProcessingContext, ms: &MinistateQuery) {
    let style_res =
        style_export::cont_page_query(
            style_export::ContPageQueryArgs {
                initial_query: ms.query.clone().unwrap_or_else(|| "\"hello world\" { => value }".to_string()),
            },
        );
    let results_group = style_res.results;
    style_res.query.ref_on("edit", {
        let debounce = RefCell::new(None);
        move |ev| {
            let ev_target = ev.target();
            *debounce.borrow_mut() = Some(Timeout::new(500, {
                let results_group = results_group.clone();
                move || {
                    let text =
                        ev_target.unwrap().dyn_ref::<HtmlElement>().unwrap().text_content().unwrap_or_default();
                    results_group.ref_clear();
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
                    build_infinite(results_group, None, {
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
                                        after: key,
                                    }),
                                }).await?;
                                let mut out = vec![];
                                for row in page_data.records {
                                    out.push(
                                        style_export::leaf_query_row(
                                            style_export::LeafQueryRowArgs {
                                                data: serde_json::to_string_pretty(&row).unwrap(),
                                            },
                                        ).root,
                                    );
                                }
                                return Ok((page_data.page_end.map(|x| Some(x)), out));
                            }
                        }
                    });
                }
            }));
        }
    });
    set_page(pc, "Query", style_res.root);
}
