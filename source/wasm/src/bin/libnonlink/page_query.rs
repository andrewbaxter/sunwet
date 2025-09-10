use {
    super::{
        infinite::build_infinite,
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        infinite::InfPageRes,
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
    web_sys::{
        Event,
        HtmlElement,
    },
};

fn refresh_query(results_group: El, text: &str) {
    record_replace_ministate(
        &state().log,
        &super::ministate::Ministate::Query(MinistateQuery { query: Some(text.to_string()) }),
    );
    let query = match compile_query(&text) {
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
                        style_export::leaf_query_json_row(
                            style_export::LeafQueryJsonRowArgs { data: serde_json::to_string_pretty(&row).unwrap() },
                        ).root,
                    );
                }
                return Ok(InfPageRes {
                    next_key: page_data.next_page_key.map(|x| Some(x)),
                    page_els: out,
                    immediate_advance: false,
                });
            }
        }
    }));
}

fn debounce_cb<I: 'static, F: 'static + Clone + FnMut(I) -> ()>(f: F) -> impl FnMut(I) -> () {
    let debounce = RefCell::new(None);
    return move |i| {
        *debounce.borrow_mut() = Some(Timeout::new(500, {
            let mut f = f.clone();
            move || f(i)
        }));
    };
}

pub fn build_page_query(pc: &mut ProcessingContext, ms: &MinistateQuery) {
    let initial_query = ms.query.clone().unwrap_or_else(|| "\"hello world\" { => value }".to_string());
    let style_res =
        style_export::cont_page_query(style_export::ContPageQueryArgs { initial_query: initial_query.clone() });
    let json_results_group = style_res.json_results;
    let downloads_group = style_res.download_results;
    refresh_query(json_results_group.clone(), &initial_query);
    style_res.query.ref_on("input", debounce_cb({
        let query = style_res.query.weak();
        let downloads_group = downloads_group.clone();
        let download_field = style_res.download_field.weak();
        let download_pattern = style_res.download_pattern.weak();
        move |ev: &Event| {
            let Some(query) = query.upgrade() else {
                return;
            };
            let Some(download_field) = download_field.upgrade() else {
                return;
            };
            let Some(download_pattern) = download_pattern.upgrade() else {
                return;
            };
            let ev_target = ev.target();
            let query_text = query.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            let download_field_text =
                download_field.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            let download_pattern_text =
                download_pattern.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            edit_group.ref_clear();
            json_results_group.ref_clear();
            pretty_results_group.ref_clear();
            downloads_group.ref_clear();
            refresh_query(pretty_results_group, json_results_group, &query_text);
            refresh_downloads(downloads_group, &text);
        }
    }));
    style_res.download_pattern.ref_on("input", debounce_cb({
        let downloads_group = downloads_group.clone();
        let download_field = style_res.download_field.weak();
        let download_pattern = style_res.download_pattern.weak();
        move |ev: &Event| {
            let Some(download_field) = download_field.upgrade() else {
                return;
            };
            let Some(download_pattern) = download_pattern.upgrade() else {
                return;
            };
            let download_field_text =
                download_field.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            let download_pattern_text =
                download_pattern.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            downloads_group.ref_clear();
            refresh_downloads(downloads_group, &text);
        }
    }));
    style_res.download_field.ref_on("input", debounce_cb({
        let downloads_group = downloads_group.clone();
        let download_field = style_res.download_field.weak();
        let download_pattern = style_res.download_pattern.weak();
        move |ev: &Event| {
            let Some(download_field) = field.upgrade() else {
                return;
            };
            let Some(download_pattern) = pattern.upgrade() else {
                return;
            };
            let download_field_text =
                download_field.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            let download_pattern_text =
                download_pattern.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
            downloads_group.ref_clear();
            refresh_downloads(downloads_group, &text);
        }
    }));
    set_page(pc, "Query", style_res.root);
}
