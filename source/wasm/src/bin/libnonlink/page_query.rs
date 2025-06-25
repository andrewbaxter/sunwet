use {
    super::{
        infinite::build_infinite,
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::MinistateQuery,
    },
    gloo::timers::callback::Timeout,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::El,
    shared::interface::wire::ReqQuery,
    std::cell::RefCell,
    wasm::js::style_export::{
        self,
    },
    web_sys::HtmlElement,
};

pub fn build_page_query(pc: &mut ProcessingContext, ms: &MinistateQuery) -> Result<El, String> {
    let style_res = style_export::cont_page_query(style_export::ContPageQueryArgs {})?;
    let results_group = style_res.results;
    style_res.query.ref_on("edit", {
        let debounce = RefCell::new(None);
        move |ev| {
            *debounce.borrow_mut() = Some(Timeout::new(500, {
                move || {
                    let text = ev.target().dyn_ref::<HtmlElement>().unwrap().text_content().unwrap_or_default();
                    results_group.ref_clear();
                    let query = match parse_query(text) {
                        Ok(q) => q,
                        Err(e) => {
                            results_group.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                data: e,
                                in_root: false,
                            }).root);
                            return;
                        },
                    };
                    results_group.ref_push(build_infinite(results_group, None, {
                        move |key| async move {
                            let page_data = req_post_json(&state().env.base_url, ReqQuery {
                                query: query,
                                parameters: Default::default(),
                                pagination: key,
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
                    }));
                }
            }));
        }
    });
    return Ok(style_res.root);
}
