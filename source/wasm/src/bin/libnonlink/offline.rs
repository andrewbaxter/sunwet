use {
    crate::libnonlink::{
        api::req_post_json,
        ministate::MinistateView,
        opfs::{
            opfs_delete,
            opfs_ensure_dir,
            opfs_exists,
            opfs_list_dir,
            opfs_read_json,
            opfs_root,
            opfs_write_binary,
            opfs_write_json,
        },
        state::state,
        viewutil::{
            DataStackLevel,
            maybe_get_field,
            maybe_get_field_or_literal,
            maybe_get_meta,
            unwrap_value_media_url,
        },
    },
    chrono::Utc,
    gloo::utils::window,
    js_sys::Promise,
    lunk::EventGraph,
    rooting::spawn_rooted,
    shared::interface::{
        config::view::{
            ClientView,
            DataRowsLayout,
            FieldOrLiteral,
            QueryOrField,
            Widget,
            WidgetRootDataRows,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            NodeMeta,
            ReqViewQuery,
            RespQuery,
            RespQueryRows,
            TreeNode,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        rc::Rc,
        str::FromStr,
    },
    wasm::{
        js::{
            LogJsErr,
            env_preferred_audio_url,
            env_preferred_video_url,
        },
        world::file_url,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        prelude::Closure,
    },
    wasm_bindgen_futures::{
        JsFuture,
        future_to_promise,
    },
    web_sys::{
        ConnectionType,
        FileSystemDirectoryHandle,
    },
};

// # Offline, incoming
const OPFS_OFFLINE_VIEWS_VIEW_FILENAME: &str = "view.json";
const OPFS_OFFLINE_VIEWS_DONE_FILENAME: &str = "done";
const OPFS_OFFLINE_FILES_META_FILENAME: &str = "meta.json";
const OPFS_OFFLINE_FILES_FILE_FILENAME: &str = "file";

async fn opfs_offline_views_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, "offline_views").await;
}

async fn opfs_offline_files_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, "offline_files").await;
}

fn data_to_query_params(
    view_def: &ClientView,
    query_id: &str,
    data_at: &Vec<Rc<DataStackLevel>>,
) -> HashMap<String, Node> {
    let mut params = HashMap::new();
    if let Some(query_params) = view_def.query_parameter_keys.get(query_id) {
        for k in query_params {
            let Some(TreeNode::Scalar(v)) = maybe_get_field(k, &data_at) else {
                return Default::default();
            };
            params.insert(k.clone(), v);
        }
    }
    return params;
}

fn opfs_offline_views_query_filename(query_id: &str, params: &HashMap<String, Node>) -> String {
    // Canonical encoding of filename (btreemap everywhere)
    let params = params.iter().collect::<BTreeMap<_, _>>();
    return format!("req_{}_{}.json", query_id, urlencoding::encode(&serde_json::to_string(&params).unwrap()));
}

pub async fn list_offline_views() -> Vec<(String, MinistateView)> {
    let mut out = vec![];
    let root_dir = opfs_offline_views_root().await;
    for (k, dir) in opfs_list_dir(&root_dir).await {
        let dir = match dir.dyn_into::<FileSystemDirectoryHandle>() {
            Ok(d) => d,
            Err(e) => {
                state()
                    .log
                    .log_js(
                        &format!("Found non-directory entry in offline views root at [{}], deleting and continuing", k),
                        &e,
                    );
                opfs_delete(&root_dir, &k).await;
                continue;
            },
        };
        let view = match opfs_read_json(&dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await {
            Ok(v) => v,
            Err(e) => {
                state()
                    .log
                    .log(&format!("Found invalid view main file in [{}], deleting and continuing: {}", k, &e));
                opfs_delete(&dir, &OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await;
                continue;
            },
        };
        out.push((k, view));
    }
    return out;
}

pub async fn ensure_offline(eg: EventGraph, view: MinistateView) -> Result<(), String> {
    let key = Utc::now().to_rfc3339();
    let views_root = opfs_ensure_dir(&opfs_offline_views_root().await, &key).await;
    opfs_write_json(&views_root, OPFS_OFFLINE_VIEWS_VIEW_FILENAME, &view).await?;
    eg.event(|pc| {
        state().offline_list.splice(pc, 0, 0, vec![(key.clone(), view.clone())]);
    }).unwrap();
    trigger_offlining(eg);
    return Ok(());
}

pub async fn remove_offline(eg: EventGraph, key: &str) -> Result<(), String> {
    let views_root = opfs_ensure_dir(&opfs_offline_views_root().await, &key).await;
    let Ok(view_dir) =
        JsFuture::from(views_root.get_directory_handle(key))
            .await
            .and_then(|v| v.dyn_into::<FileSystemDirectoryHandle>()) else {
            return Ok(());
        };
    opfs_delete(&view_dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await;
    eg.event(|pc| {
        let o = state().offline_list.clone();
        let index = o.borrow_values().iter().enumerate().filter_map(|x| if x.1.0 == key {
            Some(x.0)
        } else {
            None
        }).next();
        if let Some(index) = index {
            o.splice(pc, index, 1, vec![]);
        }
    }).unwrap();
    trigger_offlining(eg);
    return Ok(());
}

fn media_file_hash(config_at: &FieldOrLiteral, data_stack: &Vec<Rc<DataStackLevel>>) -> Option<FileHash> {
    let Some(src) = maybe_get_field_or_literal(config_at, data_stack) else {
        return None;
    };
    let TreeNode::Scalar(src) = src else {
        return None;
    };
    let Ok(hash) = unwrap_value_media_url(&src) else {
        return None;
    };
    return Some(hash);
}

async fn fetch_media_file(config_at: &FieldOrLiteral, data_stack: &Vec<Rc<DataStackLevel>>) -> Result<(), String> {
    let Some(src) = maybe_get_field_or_literal(config_at, data_stack) else {
        return Ok(());
    };
    let TreeNode::Scalar(src) = src else {
        return Ok(());
    };
    let Some(meta) = maybe_get_meta(data_stack, &src) else {
        return Ok(());
    };
    let src = unwrap_value_media_url(&src)?;
    let file_dir = opfs_ensure_dir(&opfs_offline_files_root().await, &src.to_string()).await;
    let media_url = match meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("").split("/").next().unwrap() {
        "image" => file_url(&state().env, &src),
        "video" => env_preferred_video_url(&state().env, &src),
        "audio" => env_preferred_audio_url(&state().env, &src),
        _ => {
            return Ok(());
        },
    };
    opfs_write_json(&file_dir, OPFS_OFFLINE_FILES_META_FILENAME, meta).await?;
    opfs_write_binary(
        &file_dir,
        OPFS_OFFLINE_FILES_FILE_FILENAME,
        &reqwasm::http::Request::new(&media_url)
            .send()
            .await
            .map_err(|e| format!("Error sending get request for offline-use view file [{}]: {}", media_url, e))?
            .binary()
            .await
            .map_err(|e| format!("Error downloading media file [{}]: {}", media_url, e))?,
    ).await?;
    return Ok(());
}

fn resp_query_to_rows(res: RespQuery) -> Vec<DataStackLevel> {
    let node_meta = Rc::new(res.meta.into_iter().collect::<HashMap<_, _>>());
    let mut out = vec![];
    match res.rows {
        RespQueryRows::Scalar(rows) => {
            for v in rows {
                out.push(DataStackLevel {
                    data: TreeNode::Scalar(v),
                    node_meta: node_meta.clone(),
                });
            }
        },
        RespQueryRows::Record(rows) => {
            for v in rows {
                out.push(DataStackLevel {
                    data: TreeNode::Record(v),
                    node_meta: node_meta.clone(),
                });
            }
        },
    }
    return out;
}

pub fn trigger_offlining(eg: EventGraph) {
    let go = if let Ok(c) = window().navigator().connection() {
        match c.type_() {
            ConnectionType::Cellular | ConnectionType::None => {
                false
            },
            _ => {
                true
            },
        }
    } else {
        true
    };
    if go {
        *state().offlining_bg.borrow_mut() = None;
    } else {
        let state1 = state();
        let mut bg = state1.offlining_bg.borrow_mut();
        if bg.is_none() {
            *bg = Some(spawn_rooted(async move {
                let eg = eg.clone();
                let cb = Closure::<dyn Fn(JsValue) -> Promise>::new(move |_| {
                    let eg = eg.clone();
                    return future_to_promise(async move {
                        eg.event(|pc| {
                            state().offlining.set(pc, true);
                        }).unwrap();

                        enum RootOrWidget<'a> {
                            Root(&'a WidgetRootDataRows),
                            Widget(&'a Widget),
                        }

                        fn stack_data(
                            parent: &Rc<Vec<Rc<DataStackLevel>>>,
                            row: DataStackLevel,
                        ) -> Rc<Vec<Rc<DataStackLevel>>> {
                            let mut child_params = parent.as_ref().clone();
                            child_params.push(Rc::new(row));
                            return Rc::new(child_params);
                        }

                        // Do one task at a time (upload or download), always prioritizing uploads
                        let offline_views_root = opfs_offline_views_root().await;
                        for (key, task_dir) in opfs_list_dir(&offline_views_root).await {
                            match async {
                                let task_dir = match task_dir.dyn_into::<FileSystemDirectoryHandle>() {
                                    Ok(d) => d,
                                    Err(e) => {
                                        state()
                                            .log
                                            .log_js(
                                                &format!(
                                                    "Found non-directory entry in offline views root at [{}], deleting and continuing",
                                                    key
                                                ),
                                                &e,
                                            );
                                        opfs_delete(&offline_views_root, &key).await;
                                        return Ok(());
                                    },
                                };

                                // # Handle deletes
                                if !opfs_exists(&task_dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await {
                                    opfs_delete(&offline_views_root, &key).await;
                                    return Ok(());
                                }

                                // # Handle creates/downloads
                                if opfs_exists(&task_dir, OPFS_OFFLINE_VIEWS_DONE_FILENAME).await {
                                    return Ok(());
                                }
                                let view: MinistateView =
                                    opfs_read_json(&task_dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await?;
                                let client_config = state().client_config.borrow().as_ref().unwrap().get().await?;
                                let Some(view_def) = client_config.views.get(&view.id) else {
                                    return Err(format!("No view with id [{}] in config", view.id));
                                };
                                let fetch_query_or_field =
                                    async |config_at: &QueryOrField, data_at: &Vec<Rc<DataStackLevel>>| ->
                                        Result<Vec<DataStackLevel>, String> {
                                        let query_id = match config_at {
                                            QueryOrField::Field(config_at) => {
                                                let Some(TreeNode::Array(res)) =
                                                    maybe_get_field(&config_at, &data_at) else {
                                                        return Ok(vec![]);
                                                    };
                                                let empty_node_meta: Rc<HashMap<Node, NodeMeta>> =
                                                    Default::default();
                                                return Ok(res.into_iter().map(|x| DataStackLevel {
                                                    data: x,
                                                    node_meta: empty_node_meta.clone(),
                                                }).collect());
                                            },
                                            QueryOrField::Query(q) => q,
                                        };
                                        let params = data_to_query_params(view_def, query_id, data_at);
                                        let res = req_post_json(ReqViewQuery {
                                            view_id: view.id.clone(),
                                            query: query_id.clone(),
                                            parameters: params.clone(),
                                            pagination: None,
                                        }).await?;
                                        opfs_write_json(
                                            &task_dir,
                                            &opfs_offline_views_query_filename(&query_id, &params),
                                            &res,
                                        ).await?;
                                        return Ok(resp_query_to_rows(res));
                                    };
                                let mut stack =
                                    vec![(RootOrWidget::Root(&view_def.root), Rc::new(vec![Rc::new(DataStackLevel {
                                        data: TreeNode::Record(
                                            view
                                                .params
                                                .iter()
                                                .map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone())))
                                                .collect(),
                                        ),
                                        node_meta: Default::default(),
                                    })]))];
                                while let Some((config_at, data_at)) = stack.pop() {
                                    match config_at {
                                        RootOrWidget::Root(w) => {
                                            for row in fetch_query_or_field(&w.data, &data_at).await? {
                                                let data_at = stack_data(&data_at, row);
                                                stack.push(
                                                    (RootOrWidget::Widget(&w.element_body), data_at.clone()),
                                                );
                                                if let Some(ext) = &w.element_expansion {
                                                    stack.push((RootOrWidget::Widget(ext), data_at.clone()))
                                                }
                                            }
                                        },
                                        RootOrWidget::Widget(w) => match w {
                                            Widget::Layout(w) => {
                                                for w in &w.elements {
                                                    stack.push((RootOrWidget::Widget(w), data_at.clone()));
                                                }
                                            },
                                            Widget::DataRows(w) => {
                                                for row in fetch_query_or_field(&w.data, &data_at).await? {
                                                    let row_params = stack_data(&data_at, row);
                                                    match &w.row_widget {
                                                        DataRowsLayout::Unaligned(w) => {
                                                            stack.push(
                                                                (RootOrWidget::Widget(&w.widget), row_params.clone()),
                                                            );
                                                        },
                                                        DataRowsLayout::Table(w) => {
                                                            for e in &w.elements {
                                                                stack.push(
                                                                    (RootOrWidget::Widget(e), row_params.clone()),
                                                                );
                                                            }
                                                        },
                                                    }
                                                }
                                            },
                                            Widget::Text(_) => { },
                                            Widget::Date(_) => { },
                                            Widget::Time(_) => { },
                                            Widget::Datetime(_) => { },
                                            Widget::Color(_) => { },
                                            Widget::Media(config_at) => {
                                                fetch_media_file(&config_at.data, &data_at).await?;
                                            },
                                            Widget::Icon(_) => { },
                                            Widget::PlayButton(config_at) => {
                                                fetch_media_file(
                                                    &FieldOrLiteral::Field(config_at.media_file_field.clone()),
                                                    &data_at,
                                                ).await?;
                                            },
                                            Widget::Space => { },
                                            Widget::Node(_) => { },
                                        },
                                    }
                                }
                                opfs_write_binary(&task_dir, OPFS_OFFLINE_VIEWS_DONE_FILENAME, &vec![]).await?;
                                return Ok(());
                            }.await {
                                Ok(_) => { },
                                Err(e) => {
                                    state()
                                        .log
                                        .log(&format!("Error preparing view for offline viewing [{}]: {}", key, e));
                                },
                            };
                        }

                        // # GC files from deleted downloads
                        //
                        // Prepare set of live files.
                        let mut live_files = HashSet::new();
                        let offline_views_root = opfs_offline_views_root().await;
                        for (key, task_dir) in opfs_list_dir(&offline_views_root).await {
                            match async {
                                let task_dir = match task_dir.dyn_into::<FileSystemDirectoryHandle>() {
                                    Ok(d) => d,
                                    Err(e) => {
                                        state()
                                            .log
                                            .log_js(
                                                &format!(
                                                    "Found non-directory entry in offline views root at [{}], deleting and continuing",
                                                    key
                                                ),
                                                &e,
                                            );
                                        opfs_delete(&offline_views_root, &key).await;
                                        return Ok(());
                                    },
                                };
                                let view: MinistateView =
                                    opfs_read_json(&task_dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await?;
                                let client_config = state().client_config.borrow().as_ref().unwrap().get().await?;
                                let Some(view_def) = client_config.views.get(&view.id) else {
                                    return Err(format!("No view with id [{}] in config", view.id));
                                };
                                let retrieve_query_or_field =
                                    async |config_at: &QueryOrField, data_at: &Vec<Rc<DataStackLevel>>| ->
                                        Result<Vec<DataStackLevel>, String> {
                                        let query_id = match config_at {
                                            QueryOrField::Field(config_at) => {
                                                let Some(TreeNode::Array(res)) =
                                                    maybe_get_field(&config_at, &data_at) else {
                                                        return Ok(vec![]);
                                                    };
                                                let empty_node_meta: Rc<HashMap<Node, NodeMeta>> =
                                                    Default::default();
                                                return Ok(res.into_iter().map(|x| DataStackLevel {
                                                    data: x,
                                                    node_meta: empty_node_meta.clone(),
                                                }).collect());
                                            },
                                            QueryOrField::Query(q) => q,
                                        };
                                        let params = data_to_query_params(view_def, query_id, data_at);
                                        let res: RespQuery =
                                            match opfs_read_json(
                                                &task_dir,
                                                &opfs_offline_views_query_filename(&query_id, &params),
                                            ).await {
                                                Ok(r) => r,
                                                Err(_) => {
                                                    return Ok(vec![]);
                                                },
                                            };
                                        return Ok(resp_query_to_rows(res));
                                    };

                                // Walk tree to find/add all referenced files for this task
                                let mut stack = vec![(RootOrWidget::Root(&view_def.root), Rc::new(vec![Rc::new(DataStackLevel {
                                    data: TreeNode::Record(
                                        view
                                            .params
                                            .iter()
                                            .map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone())))
                                            .collect(),
                                    ),
                                    node_meta: Default::default(),
                                })]))];
                                while let Some((config_at, data_at)) = stack.pop() {
                                    match config_at {
                                        RootOrWidget::Root(w) => {
                                            for row in retrieve_query_or_field(&w.data, &data_at).await? {
                                                let child_params = stack_data(&data_at, row);
                                                stack.push(
                                                    (RootOrWidget::Widget(&w.element_body), child_params.clone()),
                                                );
                                                if let Some(ext) = &w.element_expansion {
                                                    stack.push((RootOrWidget::Widget(ext), child_params.clone()))
                                                }
                                            }
                                        },
                                        RootOrWidget::Widget(w) => match w {
                                            Widget::Layout(w) => {
                                                for w in &w.elements {
                                                    stack.push((RootOrWidget::Widget(w), data_at.clone()));
                                                }
                                            },
                                            Widget::DataRows(w) => {
                                                for row in retrieve_query_or_field(&w.data, &data_at).await? {
                                                    let row_params = stack_data(&data_at, row);
                                                    match &w.row_widget {
                                                        DataRowsLayout::Unaligned(w) => {
                                                            stack.push(
                                                                (RootOrWidget::Widget(&w.widget), row_params.clone()),
                                                            );
                                                        },
                                                        DataRowsLayout::Table(w) => {
                                                            for e in &w.elements {
                                                                stack.push(
                                                                    (RootOrWidget::Widget(e), row_params.clone()),
                                                                );
                                                            }
                                                        },
                                                    }
                                                }
                                            },
                                            Widget::Text(_) => { },
                                            Widget::Date(_) => { },
                                            Widget::Time(_) => { },
                                            Widget::Datetime(_) => { },
                                            Widget::Color(_) => { },
                                            Widget::Media(config_at) => {
                                                if let Some(h) = media_file_hash(&config_at.data, &data_at) {
                                                    live_files.insert(h);
                                                }
                                            },
                                            Widget::Icon(_) => { },
                                            Widget::PlayButton(config_at) => {
                                                if let Some(h) =
                                                    media_file_hash(
                                                        &FieldOrLiteral::Field(config_at.media_file_field.clone()),
                                                        &data_at,
                                                    ) {
                                                    live_files.insert(h);
                                                }
                                            },
                                            Widget::Space => { },
                                            Widget::Node(_) => { },
                                        },
                                    }
                                }
                                return Ok(());
                            }.await {
                                Ok(_) => { },
                                Err(e) => {
                                    state().log.log(&format!("Error doing offline file GC scan [{}]: {}", key, e));
                                },
                            };
                        }

                        // Delete any downloaded files not referenced by any downloaded view
                        let files_root = opfs_offline_files_root().await;
                        for (key, _) in opfs_list_dir(&files_root).await {
                            match FileHash::from_str(&key) {
                                Ok(hash) => {
                                    if live_files.contains(&hash) {
                                        continue;
                                    }
                                },
                                Err(_) => {
                                    state()
                                        .log
                                        .log(
                                            &format!(
                                                "File in offline files directory has an invalid (non-file-hash) name. Deleting."
                                            ),
                                        );
                                },
                            }
                            opfs_delete(&files_root, &key).await;
                        }

                        // Nothing left to do atm, exit
                        eg.event(|pc| {
                            state().offlining.set(pc, false);
                        }).unwrap();
                        return Ok(JsValue::null());
                    });
                });
                JsFuture::from(
                    window().navigator().locks().request_with_callback("offline", cb.as_ref().unchecked_ref()),
                )
                    .await
                    .log(&state().log, "Error doing work in `offline` lock");
            }));
        }
    }
}
