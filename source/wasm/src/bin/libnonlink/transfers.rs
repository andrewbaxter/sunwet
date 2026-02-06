use {
    crate::libnonlink::{
        api::{
            file_post_json,
            req_post_json,
        },
        commit::UploadFile,
        ministate::MinistateView,
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
    flowcontrol::ta_return,
    gloo::{
        timers::future::TimeoutFuture,
        utils::window,
    },
    js_sys::{
        Array,
        Promise,
        Uint8Array,
    },
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        ScopeValue,
        defer,
        spawn_rooted,
    },
    serde::{
        Serialize,
        de::DeserializeOwned,
    },
    shared::interface::{
        config::view::{
            DataRowsLayout,
            FieldOrLiteral,
            QueryOrField,
            ViewId,
            Widget,
            WidgetRootDataRows,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            NodeMeta,
            ReqCommit,
            ReqUploadFinish,
            ReqViewQuery,
            RespQuery,
            RespQueryRows,
            TreeNode,
        },
    },
    std::{
        cell::RefCell,
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        rc::Rc,
        str::FromStr,
    },
    tokio_stream::StreamExt,
    wasm::{
        js::{
            self,
            Log,
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
        stream::JsStream,
    },
    web_sys::{
        ConnectionType,
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
        FileSystemWritableFileStream,
    },
};

// # Opfs utils
pub async fn opfs_root() -> FileSystemDirectoryHandle {
    return JsFuture::from(window().navigator().storage().get_directory())
        .await
        .expect("Error getting opfs root")
        .dyn_into::<FileSystemDirectoryHandle>()
        .unwrap();
}

pub async fn opfs_ensure_dir(parent: &FileSystemDirectoryHandle, seg: &str) -> FileSystemDirectoryHandle {
    return JsFuture::from(parent.get_directory_handle_with_options(seg, &{
        let x = FileSystemGetDirectoryOptions::new();
        x.set_create(true);
        x
    }))
        .await
        .expect("Error getting/creating opfs dir")
        .dyn_into::<FileSystemDirectoryHandle>()
        .expect("Opfs get dir handle result wasn't file system dir handle");
}

/// Each JsValue is either FileSystemDirectoryHandle or FileSystemFileHandle
pub async fn opfs_list_dir(parent: &FileSystemDirectoryHandle) -> Vec<(String, JsValue)> {
    let mut entries = vec![];
    let mut entries0 = JsStream::from(parent.entries());
    while let Some(e) = entries0.next().await {
        let e = match e {
            Ok(e) => e,
            Err(e) => {
                state().log.log_js2("Error reading directory entry", parent, &e);
                continue;
            },
        };
        let e = e.dyn_into::<Array>().unwrap();
        let name = e.get(0).as_string().unwrap();
        let handle = e.get(1);
        entries.push((name, handle));
    }
    return entries;
}

pub async fn opfs_read_binary(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<Vec<u8>, String> {
    return Ok(
        JsFuture::from(
            web_sys::File::from(
                JsFuture::from(
                    FileSystemFileHandle::from(
                        JsFuture::from(parent.get_file_handle(seg))
                            .await
                            .map_err(|e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()))?,
                    ).get_file(),
                )
                    .await
                    .map_err(
                        |e| format!("Error getting file from file handle at seg [{}]: {:?}", seg, e.as_string()),
                    )?,
            ).text(),
        )
            .await
            .map_err(|e| format!("Error getting string contents of file at seg [{}]: {:?}", seg, e.as_string()))?
            .dyn_into::<Uint8Array>()
            .unwrap()
            .to_vec(),
    );
}

pub async fn opfs_read_json<
    T: DeserializeOwned,
>(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<T, String> {
    return Ok(
        serde_json::from_slice::<T>(
            &opfs_read_binary(parent, seg).await?,
        ).map_err(|e| format!("Error parsing json file from opfs at seg [{}]: {}", seg, e))?,
    );
}

pub async fn opfs_write_binary(parent: &FileSystemDirectoryHandle, seg: &str, data: &Vec<u8>) -> Result<(), String> {
    let f =
        FileSystemFileHandle::from(
            JsFuture::from(parent.get_file_handle(seg))
                .await
                .map_err(|e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    let w =
        FileSystemWritableFileStream::from(
            JsFuture::from(f.create_writable())
                .await
                .map_err(|e| format!("Error getting file handle writable at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    JsFuture::from(
        w
            .write_with_u8_array(data)
            .map_err(|e| format!("Error writing message to opfs file at seg [{}]: {:?}", seg, e.as_string()))?,
    )
        .await
        .map_err(|e| format!("Error writing message to opfs file at seg [{}] (2): {:?}", seg, e.as_string()))?;
    return Ok(());
}

pub async fn opfs_write_json<
    T: Serialize,
>(parent: &FileSystemDirectoryHandle, seg: &str, data: T) -> Result<(), String> {
    return opfs_write_binary(parent, seg, &serde_json::to_vec(&data).unwrap()).await;
}

pub async fn opfs_delete(parent: &FileSystemDirectoryHandle, seg: &str) {
    if let Err(e) = JsFuture::from(parent.remove_entry(seg)).await {
        state().log.log_js(&format!("Error deleting opfs entry at [{}]", seg), &e);
    }
}

pub async fn opfs_exists(parent: &FileSystemDirectoryHandle, seg: &str) -> bool {
    // Bikeshedding https://github.com/whatwg/fs/issues/80
    //
    // This is mostly used by offline, offline is mostly for mobile devices with
    // limited storage and/or in small directories - so hopefully this hack doesn't
    // blow up for typical use cases.
    for (k, _) in opfs_list_dir(parent).await {
        if k == seg {
            return true;
        }
    }
    return false;
}

// # Outgoing
const OPFS_OUT_COMMITS_ROOT: &str = "out_commits";
const OPFS_OUT_COMMITS_MAIN_FILENAME: &str = "commit.json";

async fn opfs_commits_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, OPFS_OUT_COMMITS_ROOT).await;
}

pub async fn ensure_commit(eg: EventGraph, commit: ReqCommit, files: Vec<UploadFile>) -> Result<(), String> {
    let key = Utc::now().to_rfc3339();
    let commit_dir = opfs_ensure_dir(&opfs_commits_root().await, &key).await;
    opfs_write_json(&commit_dir, OPFS_OUT_COMMITS_MAIN_FILENAME, &commit).await?;
    for file in files {
        opfs_write_binary(&commit_dir, &file.hash.to_string(), &file.data).await?;
    }
    trigger_transfers(eg);
    return Ok(());
}

// # Offline, incoming
const OPFS_OFFLINE_VIEWS_ROOT: &str = "offline_views";
const OPFS_OFFLINE_VIEWS_VIEW_FILENAME: &str = "view.json";
const OPFS_OFFLINE_VIEWS_DONE_FILENAME: &str = "done";
const OPFS_OFFLINE_FILES_ROOT: &str = "offline_files";
const OPFS_OFFLINE_FILES_META_FILENAME: &str = "meta.json";
const OPFS_OFFLINE_FILES_FILE_FILENAME: &str = "file";

async fn opfs_offline_views_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, OPFS_OFFLINE_VIEWS_ROOT).await;
}

async fn opfs_offline_files_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, OPFS_OFFLINE_FILES_ROOT).await;
}

fn opfs_offline_views_query_filename(index: &Vec<i32>) -> String {
    return format!("req_{}.json", index.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("_"));
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
        state().transfers_offline.splice(pc, 0, 0, vec![(key.clone(), view.clone())]);
    }).unwrap();
    trigger_transfers(eg);
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
        let o = state().transfers_offline.clone();
        let index = o.borrow_values().iter().enumerate().filter_map(|x| if x.1.0 == key {
            Some(x.0)
        } else {
            None
        }).next();
        if let Some(index) = index {
            o.splice(pc, index, 1, vec![]);
        }
    }).unwrap();
    trigger_transfers(eg);
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

pub fn trigger_transfers(eg: EventGraph) {
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
        *state().transfers_bg.borrow_mut() = None;
    } else {
        let state1 = state();
        let mut bg = state1.transfers_bg.borrow_mut();
        if bg.is_none() {
            *bg = Some(spawn_rooted(async move {
                let eg = eg.clone();
                let cb = Closure::<dyn Fn(JsValue) -> Promise>::new(move |_| {
                    let eg = eg.clone();
                    return future_to_promise(async move {
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

                        fn counter(outer: &Rc<Vec<i32>>) -> impl FnMut() -> Rc<Vec<i32>> {
                            let mut child_index = 0;
                            let outer = outer.clone();
                            return move || {
                                let mut out = outer.as_ref().clone();
                                out.push(child_index);
                                child_index += 1;
                                return Rc::new(out);
                            };
                        }

                        // Do one task at a time (upload or download), always prioritizing uploads
                        'next : loop {
                            // Upload the next commit
                            {
                                let commit_root = opfs_commits_root().await;
                                for (key, task_dir) in opfs_list_dir(&commit_root).await {
                                    let task_dir = match task_dir.dyn_into::<FileSystemDirectoryHandle>() {
                                        Ok(d) => d,
                                        Err(e) => {
                                            state()
                                                .log
                                                .log_js(
                                                    &format!(
                                                        "Found non-directory entry in upload root at [{}], deleting and continuing",
                                                        key
                                                    ),
                                                    &e,
                                                );
                                            opfs_delete(&commit_root, &key).await;
                                            continue;
                                        },
                                    };
                                    eg.event(|pc| {
                                        state().transfers_uploading.set(pc, true);
                                    }).unwrap();
                                    let res = async {
                                        ta_return!((), String);
                                        let req: ReqCommit =
                                            opfs_read_json(&commit_root, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await?;
                                        let need_files = req_post_json(req).await?;
                                        for file in need_files.incomplete {
                                            let data = opfs_read_binary(&task_dir, &file.to_string()).await?;
                                            const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
                                            let file_size = data.len() as u64;
                                            let chunks = file_size.div_ceil(CHUNK_SIZE);
                                            for i in 0 .. chunks {
                                                let chunk_start = i * CHUNK_SIZE;
                                                let chunk_size = file_size.min(CHUNK_SIZE);
                                                file_post_json(
                                                    &file,
                                                    chunk_start,
                                                    &data[chunk_start as usize .. (chunk_start + chunk_size) as usize],
                                                ).await?;
                                            }
                                            loop {
                                                let resp = req_post_json(ReqUploadFinish(file.clone())).await?;
                                                if resp.done {
                                                    break;
                                                }
                                                TimeoutFuture::new(1000).await;
                                            }
                                        }
                                        opfs_delete(&commit_root, &key).await;
                                        return Ok(());
                                    }.await;
                                    eg.event(|pc| {
                                        state().transfers_uploading.set(pc, false);
                                    }).unwrap();
                                    match res {
                                        Ok(_) => {
                                            continue 'next;
                                        },
                                        Err(e) => {
                                            state()
                                                .log
                                                .log(
                                                    &format!(
                                                        "Error uploading queued commit [at opfs {}]: {}",
                                                        task_dir.to_string().as_string().unwrap(),
                                                        e
                                                    ),
                                                );
                                        },
                                    }
                                }
                            }

                            // Download
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
                                    eg.event(|pc| {
                                        state().transfers_downloading.set(pc, true);
                                    }).unwrap();
                                    let _clear_prim = defer(|| {
                                        eg.event(|pc| {
                                            state().transfers_downloading.set(pc, false);
                                        }).unwrap();
                                    });
                                    let view: MinistateView =
                                        opfs_read_json(&task_dir, OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await?;
                                    let client_config = state().client_config.borrow().as_ref().unwrap().get().await?;
                                    let Some(view_def) = client_config.views.get(&view.id) else {
                                        return Err(format!("No view with id [{}] in config", view.id));
                                    };
                                    let fetch_query_or_field =
                                        async |
                                            config_at: &QueryOrField,
                                            index: &Vec<i32>,
                                            data_at: &Vec<Rc<DataStackLevel>>,
                                        | ->
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
                                            let mut params = HashMap::new();
                                            if let Some(query_params) = view_def.query_parameter_keys.get(query_id) {
                                                for k in query_params {
                                                    let Some(TreeNode::Scalar(v)) =
                                                        maybe_get_field(k, &data_at) else {
                                                            return Err(
                                                                format!(
                                                                    "Parameters must be scalars, but query paramter [{}] is missing or not a scalar",
                                                                    k
                                                                ),
                                                            );
                                                        };
                                                    params.insert(k.clone(), v);
                                                }
                                            }
                                            let res = req_post_json(ReqViewQuery {
                                                view_id: view.id.clone(),
                                                query: query_id.clone(),
                                                parameters: params.clone(),
                                                pagination: None,
                                            }).await?;
                                            opfs_write_json(
                                                &task_dir,
                                                &opfs_offline_views_query_filename(&index),
                                                &res,
                                            ).await?;
                                            return Ok(resp_query_to_rows(res));
                                        };
                                    let mut stack =
                                        vec![
                                            (
                                                Rc::new(vec![]),
                                                RootOrWidget::Root(&view_def.root),
                                                Rc::new(vec![Rc::new(DataStackLevel {
                                                    data: TreeNode::Record(
                                                        view
                                                            .params
                                                            .iter()
                                                            .map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone())))
                                                            .collect(),
                                                    ),
                                                    node_meta: Default::default(),
                                                })]),
                                            )
                                        ];
                                    while let Some((index, config_at, data_at)) = stack.pop() {
                                        match config_at {
                                            RootOrWidget::Root(w) => {
                                                let mut child_indexes = counter(&index);
                                                for row in fetch_query_or_field(
                                                    &w.data,
                                                    index.as_ref(),
                                                    &data_at,
                                                ).await? {
                                                    let data_at = stack_data(&data_at, row);
                                                    stack.push(
                                                        (
                                                            child_indexes(),
                                                            RootOrWidget::Widget(&w.element_body),
                                                            data_at.clone(),
                                                        ),
                                                    );
                                                    if let Some(ext) = &w.element_expansion {
                                                        stack.push(
                                                            (
                                                                child_indexes(),
                                                                RootOrWidget::Widget(ext),
                                                                data_at.clone(),
                                                            ),
                                                        )
                                                    }
                                                }
                                            },
                                            RootOrWidget::Widget(w) => match w {
                                                Widget::Layout(w) => {
                                                    let mut child_index = counter(&index);
                                                    for w in &w.elements {
                                                        stack.push(
                                                            (child_index(), RootOrWidget::Widget(w), data_at.clone()),
                                                        );
                                                    }
                                                },
                                                Widget::DataRows(w) => {
                                                    let mut child_index1 = counter(&index);
                                                    for row in fetch_query_or_field(&w.data, &index, &data_at).await? {
                                                        let row_params = stack_data(&data_at, row);
                                                        match w.row_widget {
                                                            DataRowsLayout::Unaligned(w) => {
                                                                stack.push(
                                                                    (
                                                                        child_index1(),
                                                                        RootOrWidget::Widget(&w.widget),
                                                                        row_params.clone(),
                                                                    ),
                                                                );
                                                            },
                                                            DataRowsLayout::Table(w) => {
                                                                let child_index2 = counter(&child_index1());
                                                                for e in &w.elements {
                                                                    stack.push(
                                                                        (
                                                                            child_index2(),
                                                                            RootOrWidget::Widget(e),
                                                                            row_params.clone(),
                                                                        ),
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
                                    Ok(_) => {
                                        continue 'next;
                                    },
                                    Err(e) => {
                                        state()
                                            .log
                                            .log(
                                                &format!("Error preparing view for offline viewing [{}]: {}", key, e),
                                            );
                                    },
                                };
                            }

                            // No more uploads/downloads
                            break;
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
                                    async |index: &Vec<i32>| -> Result<Vec<DataStackLevel>, String> {
                                        let res: RespQuery =
                                            match opfs_read_json(
                                                &task_dir,
                                                &opfs_offline_views_query_filename(&index),
                                            ).await {
                                                Ok(r) => r,
                                                Err(e) => {
                                                    return Ok(vec![]);
                                                },
                                            };
                                        return Ok(resp_query_to_rows(res));
                                    };

                                // Walk tree to find/add all referenced files for this task
                                let mut stack = vec![(Rc::new(vec![]), RootOrWidget::Root(&view_def.root), Rc::new(vec![Rc::new(DataStackLevel {
                                    data: TreeNode::Record(
                                        view
                                            .params
                                            .iter()
                                            .map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone())))
                                            .collect(),
                                    ),
                                    node_meta: Default::default(),
                                })]))];
                                while let Some((index, config_at, data_at)) = stack.pop() {
                                    match config_at {
                                        RootOrWidget::Root(w) => {
                                            let mut next_index = counter(&index);
                                            for row in retrieve_query_or_field(&index).await? {
                                                let child_params = stack_data(&data_at, row);
                                                stack.push(
                                                    (
                                                        next_index(),
                                                        RootOrWidget::Widget(&w.element_body),
                                                        child_params.clone(),
                                                    ),
                                                );
                                                if let Some(ext) = &w.element_expansion {
                                                    stack.push(
                                                        (
                                                            next_index(),
                                                            RootOrWidget::Widget(ext),
                                                            child_params.clone(),
                                                        ),
                                                    )
                                                }
                                            }
                                        },
                                        RootOrWidget::Widget(w) => match w {
                                            Widget::Layout(w) => {
                                                let mut next_index = counter(&index);
                                                for w in &w.elements {
                                                    stack.push(
                                                        (next_index(), RootOrWidget::Widget(w), data_at.clone()),
                                                    );
                                                }
                                            },
                                            Widget::DataRows(w) => {
                                                let mut next_index = counter(&index);
                                                for row in retrieve_query_or_field(&index).await? {
                                                    let row_params = stack_data(&data_at, row);
                                                    match w.row_widget {
                                                        DataRowsLayout::Unaligned(w) => {
                                                            stack.push(
                                                                (
                                                                    next_index(),
                                                                    RootOrWidget::Widget(&w.widget),
                                                                    row_params.clone(),
                                                                ),
                                                            );
                                                        },
                                                        DataRowsLayout::Table(w) => {
                                                            let mut next_index = counter(&next_index());
                                                            for e in &w.elements {
                                                                stack.push(
                                                                    (
                                                                        next_index(),
                                                                        RootOrWidget::Widget(e),
                                                                        row_params.clone(),
                                                                    ),
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
                                Err(e) => {
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
                        return Ok(JsValue::null());
                    });
                });
                JsFuture::from(
                    window().navigator().locks().request_with_callback("transfers", cb.as_ref().unchecked_ref()),
                ).await;
            }));
        }
    }
}
