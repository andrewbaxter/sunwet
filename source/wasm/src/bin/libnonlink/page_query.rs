use {
    super::{
        infinite::build_infinite,
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        infinite::InfPageRes,
        ministate::{
            ministate_octothorpe,
            record_replace_ministate,
            MinistateNodeView,
            MinistateQuery,
        },
        node_edit::build_node_edit_contents,
        playlist::{
            categorize_mime_media,
            PlaylistEntryMediaType,
        },
        state::set_page,
    },
    flowcontrol::{
        shed,
        superif,
    },
    gloo::timers::callback::Timeout,
    js_sys::Math,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        El,
        WeakEl,
    },
    shared::{
        interface::{
            triple::{
                FileHash,
                Node,
            },
            wire::{
                NodeMeta,
                Pagination,
                ReqQuery,
                RespQueryRows,
                TreeNode,
            },
        },
        query_analysis::analyze_query,
        query_parser::compile_query,
        stringpattern::{
            node_to_text,
            Pattern,
            PatternPart,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
        u64,
    },
    wasm::{
        js::{
            copy,
            download,
            lazy_el_async,
            style_export::{
                self,
            },
        },
        world::file_url,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlElement,
    },
};

#[derive(Clone)]
struct QueryState {
    pretty_results_group: WeakEl,
    json_tab: El,
    download_tab: El,
    edit_tab: El,
    download_field: Rc<RefCell<Option<String>>>,
    download_pattern: Rc<RefCell<Option<String>>>,
}

struct PrettyElNormal {
    text: String,
    link: Option<String>,
}

struct PrettyElMedia {
    el: El,
    link: String,
}

enum PrettyEl {
    Normal(PrettyElNormal),
    Media(PrettyElMedia),
}

fn value_to_pretty_el(v: TreeNode, meta: &HashMap<Node, NodeMeta>) -> PrettyEl {
    let TreeNode::Scalar(n) = &v else {
        return PrettyEl::Normal(PrettyElNormal {
            text: serde_json::to_string(&v).unwrap(),
            link: None,
        });
    };
    shed!{
        let Some(meta) = meta.get(n) else {
            break;
        };
        let Node::File(file) = n else {
            break;
        };
        let src_url = file_url(&state().env, file);
        let link = ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
            title: node_to_text(n),
            node: n.clone(),
        }));
        match categorize_mime_media(meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("")) {
            Some(PlaylistEntryMediaType::Audio) => {
                return PrettyEl::Media(PrettyElMedia {
                    el: style_export::leaf_media_audio(style_export::LeafMediaAudioArgs { src: src_url }).root,
                    link: link,
                });
            },
            Some(PlaylistEntryMediaType::Video) => {
                return PrettyEl::Media(PrettyElMedia {
                    el: style_export::leaf_media_video(style_export::LeafMediaVideoArgs { src: src_url }).root,
                    link: link,
                });
            },
            Some(PlaylistEntryMediaType::Image) => {
                return PrettyEl::Media(PrettyElMedia {
                    el: style_export::leaf_media_img(style_export::LeafMediaImgArgs { src: src_url }).root,
                    link: link,
                });
            },
            _ => {
                break;
            },
        }
    }
    let text = node_to_text(n);
    let link = ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
        title: text.clone(),
        node: n.clone(),
    }));
    return PrettyEl::Normal(PrettyElNormal {
        text: text,
        link: Some(link),
    });
}

fn refresh_query(eg: EventGraph, qstate: QueryState, text: &str) {
    record_replace_ministate(
        &state().log,
        &super::ministate::Ministate::Query(MinistateQuery { query: Some(text.to_string()) }),
    );
    {
        let Some(pretty_results_group) = qstate.pretty_results_group.upgrade() else {
            return;
        };
        pretty_results_group.ref_clear();
    }
    qstate.edit_tab.ref_clear();
    qstate.download_tab.ref_clear();
    qstate.json_tab.ref_clear();
    let query = match compile_query(&text) {
        Ok(q) => q,
        Err(e) => {
            let Some(pretty_results_group) = qstate.pretty_results_group.upgrade() else {
                return;
            };
            pretty_results_group.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                data: e,
                in_root: false,
            }).root);
            return;
        },
    };
    if let Some(pretty_results_group) = qstate.pretty_results_group.upgrade() {
        pretty_results_group.ref_push(build_infinite(&state().log, None, {
            let seed = (Math::random() * u64::MAX as f64) as u64;
            let query = query.clone();
            move |key| {
                let query = query.clone();
                async move {
                    let page_data = req_post_json(ReqQuery {
                        query: query.clone(),
                        parameters: Default::default(),
                        pagination: Some(Pagination {
                            count: 100,
                            seed: Some(seed),
                            key: key,
                        }),
                    }).await?;
                    let meta = page_data.meta.into_iter().collect::<HashMap<_, _>>();
                    let mut out = vec![];
                    match page_data.rows {
                        RespQueryRows::Scalar(rows) => {
                            for row in rows {
                                out.push(
                                    style_export::cont_query_pretty_row(
                                        style_export::ContQueryPrettyRowArgs {
                                            children: vec![match value_to_pretty_el(TreeNode::Scalar(row), &meta) {
                                                PrettyEl::Normal(v) => style_export::leaf_query_pretty_v(
                                                    style_export::LeafQueryPrettyVArgs {
                                                        value: v.text,
                                                        link: v.link,
                                                    },
                                                ).root,
                                                PrettyEl::Media(v) => style_export::leaf_query_pretty_media_v(
                                                    style_export::LeafQueryPrettyMediaVArgs {
                                                        value: v.el,
                                                        link: v.link,
                                                    },
                                                ).root,
                                            }],
                                        },
                                    ).root,
                                );
                            }
                        },
                        RespQueryRows::Record(rows) => {
                            for row in rows {
                                let mut fields = vec![];
                                for (k, v) in row {
                                    let field;
                                    match value_to_pretty_el(v, &meta) {
                                        PrettyEl::Normal(v) => {
                                            field =
                                                style_export::leaf_query_pretty_inline_kv(
                                                    style_export::LeafQueryPrettyInlineKvArgs {
                                                        key: k,
                                                        value: v.text,
                                                        link: v.link,
                                                    },
                                                ).root;
                                        },
                                        PrettyEl::Media(v) => {
                                            field =
                                                style_export::leaf_query_pretty_media_kv(
                                                    style_export::LeafQueryPrettyMediaKvArgs {
                                                        key: k,
                                                        value: v.el,
                                                        link: v.link,
                                                    },
                                                ).root;
                                        },
                                    }
                                    fields.push(field);
                                }
                                out.push(
                                    style_export::cont_query_pretty_row(
                                        style_export::ContQueryPrettyRowArgs { children: fields },
                                    ).root,
                                );
                            }
                        },
                    }
                    return Ok(InfPageRes {
                        next_key: page_data.next_page_key.map(|x| Some(x)),
                        page_els: out,
                        immediate_advance: false,
                    });
                }
            }
        }));
    };
    qstate.json_tab.ref_push(lazy_el_async({
        let query = query.clone();
        async move || -> Result<Vec<El>, String> {
            let data = req_post_json(ReqQuery {
                query: query.clone(),
                parameters: Default::default(),
                pagination: None,
            }).await?;
            let out = style_export::cont_page_query_tab_json();
            let data = Rc::new(data.rows);
            out.json_results.ref_text(&serde_json::to_string_pretty(&data).unwrap());
            out.copy_button.ref_on("click", {
                let data = data.clone();
                move |_| {
                    copy(&state().log, &data);
                }
            });
            out.download_button.ref_on("click", {
                let data = data.clone();
                move |_| {
                    download("sunwet_query_results.json".to_string(), &data);
                }
            });
            return Ok(vec![out.root]);
        }
    }));
    qstate.download_tab.ref_push(lazy_el_async({
        let query = query.clone();
        async move || -> Result<Vec<El>, String> {
            let data = req_post_json(ReqQuery {
                query: query.clone(),
                parameters: Default::default(),
                pagination: None,
            }).await?;
            let meta = data.meta.into_iter().filter_map(|x| {
                let Node::File(k) = x.0 else {
                    return None;
                };
                let Some(v) = x.1.mime else {
                    return None;
                };
                return Some((k, v));
            }).collect::<HashMap<_, _>>();

            fn determine_ext(row: &FileHash, mimes: &HashMap<FileHash, String>) -> &'static str {
                superif!({
                    let Some(mime) = mimes.get(row) else {
                        break 'bad;
                    };
                    let Some(ext) = mime2ext::mime2ext(mime) else {
                        break 'bad;
                    };
                    ext
                } 'bad {
                    return "bin";
                })
            }

            match data.rows {
                RespQueryRows::Scalar(rows) => {
                    let out = style_export::cont_page_query_tab_download_v().root;
                    for row in rows {
                        let row_el;
                        if let Node::File(row) = row {
                            row_el = style_export::leaf_query_download_row(style_export::LeafQueryDownloadRowArgs {
                                filename: format!("{}.{}", row.to_string(), determine_ext(&row, &meta)),
                                link: file_url(&state().env, &row),
                            }).root;
                        } else {
                            row_el = style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                data: format!("Row is not a file: {}", serde_json::to_string(&row).unwrap()),
                                in_root: false,
                            }).root;
                        }
                        out.ref_push(row_el);
                    }
                    return Ok(vec![out]);
                },
                RespQueryRows::Record(rows) => {
                    let out = style_export::cont_page_query_tab_download_kv();
                    let initial_field;
                    shed!{
                        'found_initial_field _;
                        let df = qstate.download_field.borrow();
                        if let Some(f) = df.as_ref() {
                            initial_field = f.clone();
                            break 'found_initial_field;
                        }
                        for row in &rows {
                            for (k, v) in row {
                                let TreeNode::Scalar(Node::File(_)) = v else {
                                    continue;
                                };
                                initial_field = k.clone();
                                break 'found_initial_field;
                            }
                        }
                        for row in &rows {
                            for k in row.keys() {
                                initial_field = k.clone();
                                break 'found_initial_field;
                            }
                        }
                        initial_field = format!("");
                    }
                    let initial_pattern;
                    shed!{
                        'found_initial_pattern _;
                        let dp = qstate.download_pattern.borrow();
                        if let Some(p) = dp.as_ref() {
                            initial_pattern = p.clone();
                            break 'found_initial_pattern;
                        }
                        if let Some(r) = rows.first() {
                            let mut parts = vec![];
                            for k in r.keys() {
                                if *k == initial_field {
                                    continue;
                                }
                                if !parts.is_empty() {
                                    parts.push(PatternPart::Lit(format!("_")));
                                }
                                parts.push(PatternPart::Field(k.clone()));
                            }
                            if parts.is_empty() {
                                if let Some(k) = r.keys().next() {
                                    parts.push(PatternPart::Field(k.clone()));
                                }
                            }
                            initial_pattern = Pattern { parts: parts }.to_string();
                            break 'found_initial_pattern;
                        }
                        initial_pattern = format!("");
                    }
                    out.download_field.ref_text(&initial_field);
                    out.download_pattern.ref_text(&initial_pattern);
                    let field = Rc::new(RefCell::new(initial_field));
                    let pattern = Rc::new(RefCell::new(initial_pattern));
                    let refresh_download_results = {
                        let results = out.download_results.clone();
                        let field = field.clone();
                        let pattern = pattern.clone();
                        Rc::new(move || {
                            results.ref_clear();
                            let field = field.borrow();
                            let pattern = Pattern::from(pattern.borrow().as_str());
                            for row in &rows {
                                let row_el;
                                if let Some(field) = row.get(&*field) {
                                    if let TreeNode::Scalar(Node::File(field)) = field {
                                        row_el =
                                            style_export::leaf_query_download_row(
                                                style_export::LeafQueryDownloadRowArgs {
                                                    link: file_url(&state().env, field),
                                                    filename: format!(
                                                        "{}.{}",
                                                        pattern.interpolate(row),
                                                        determine_ext(field, &meta)
                                                    ),
                                                },
                                            ).root;
                                    } else {
                                        row_el = style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                            data: format!(
                                                "Row field is not a file: {}",
                                                serde_json::to_string(&row).unwrap()
                                            ),
                                            in_root: false,
                                        }).root;
                                    }
                                } else {
                                    row_el = style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                        data: format!(
                                            "Row missing file field: {}",
                                            serde_json::to_string(&row).unwrap()
                                        ),
                                        in_root: false,
                                    }).root;
                                }
                                results.ref_push(row_el);
                            }
                        })
                    };
                    refresh_download_results();
                    out.download_field.ref_on("input", {
                        let mut db = debounce_cb({
                            let self_ = out.download_field.weak();
                            let refresh_download_results = refresh_download_results.clone();
                            move |_| {
                                let Some(self_) = self_.upgrade() else {
                                    return;
                                };
                                let text = self_.raw().text_content().unwrap_or_default();
                                *field.borrow_mut() = text.clone();
                                if text.is_empty() {
                                    *qstate.download_field.borrow_mut() = None;
                                } else {
                                    *qstate.download_field.borrow_mut() = Some(text);
                                }
                                refresh_download_results();
                            }
                        });
                        move |_| db(())
                    });
                    out.download_pattern.ref_on("input", {
                        let mut db = debounce_cb({
                            let self_ = out.download_pattern.weak();
                            move |_| {
                                let Some(self_) = self_.upgrade() else {
                                    return;
                                };
                                let text = self_.raw().text_content().unwrap_or_default();
                                *pattern.borrow_mut() = text.clone();
                                if text.is_empty() {
                                    *qstate.download_pattern.borrow_mut() = None;
                                } else {
                                    *qstate.download_pattern.borrow_mut() = Some(text);
                                }
                                refresh_download_results();
                            }
                        });
                        move |_| db(())
                    });
                    return Ok(vec![out.root]);
                },
            }
        }
    }));
    qstate.edit_tab.ref_push(lazy_el_async({
        let query = query.clone();
        let query_text = text.to_string();
        let eg = eg.clone();
        async move || -> Result<Vec<El>, String> {
            if analyze_query(&query).r#struct.is_some() {
                return Ok(vec![style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                    data: format!("To edit nodes, use a suffix-less query that returns all the nodes to edit"),
                    in_root: false,
                }).root]);
            }
            let data = req_post_json(ReqQuery {
                query: query.clone(),
                parameters: Default::default(),
                pagination: None,
            }).await?;
            let RespQueryRows::Scalar(nodes) = data.rows else {
                panic!();
            };
            let contents = build_node_edit_contents(eg, format!("Query batch edit: {}", query_text), nodes).await?;
            let out = style_export::cont_page_query_tab_edit(style_export::ContPageQueryTabEditArgs {
                children: contents.children,
                bar_children: contents.bar_children,
            });
            return Ok(vec![out.edit_bar, out.root]);
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
    let json_stack = style_export::cont_stack(style_export::ContStackArgs { children: vec![] }).root;
    let download_stack = style_export::cont_stack(style_export::ContStackArgs { children: vec![] }).root;
    let edit_stack = style_export::cont_stack(style_export::ContStackArgs { children: vec![] }).root;
    let style_res = style_export::cont_page_query(style_export::ContPageQueryArgs {
        initial_query: initial_query.clone(),
        json_tab: vec![json_stack.clone()],
        download_tab: vec![download_stack.clone()],
        edit_tab: vec![edit_stack.clone()],
    });
    let qstate = QueryState {
        pretty_results_group: style_res.pretty_results.weak(),
        json_tab: json_stack,
        download_tab: download_stack,
        edit_tab: edit_stack,
        download_field: Rc::new(RefCell::new(None)),
        download_pattern: Rc::new(RefCell::new(None)),
    };
    refresh_query(pc.eg(), qstate.clone(), &initial_query);
    style_res.query.ref_on("input", {
        let mut db = debounce_cb({
            let query_input = style_res.query.weak();
            let eg = pc.eg();
            move |_| {
                let Some(query_input) = query_input.upgrade() else {
                    return;
                };
                let query_text =
                    query_input.raw().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default();
                refresh_query(eg.clone(), qstate.clone(), &query_text);
            }
        });
        move |_| db(())
    });
    set_page(pc, "Query", style_res.root);
}
