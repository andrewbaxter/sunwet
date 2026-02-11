use {
    crate::libnonlink::{
        api::{
            req_file,
            req_post_json,
        },
        ministate::MinistateView,
        opfs::{
            OpfsDir,
            opfs_root,
            request_persistent,
        },
        state::state,
        viewutil::{
            DataStackLevel,
            maybe_get_field,
            maybe_get_field_or_literal,
            maybe_get_meta,
            unwrap_value_media_hash,
        },
    },
    chrono::Utc,
    gloo::utils::window,
    js_sys::Promise,
    lunk::EventGraph,
    rooting::{
        defer,
        spawn_rooted,
    },
    shared::interface::{
        config::view::{
            ClientView,
            DataRowsLayout,
            FieldOrLiteral,
            QueryOrField,
            Widget,
            WidgetRootDataRows,
        },
        derived::{
            COMIC_MANIFEST_FILENAME,
            ComicManifest,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            GEN_FILENAME_COMICMANIFEST,
            GENTYPE_CBZDIR,
            GENTYPE_EPUBHTML,
            GENTYPE_VTT,
            NodeMeta,
            ReqViewQuery,
            RespQuery,
            RespQueryRows,
            TRANSCODE_MIME_AAC,
            TRANSCODE_MIME_WEBM,
            TreeNode,
            gentype_transcode,
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
            env_preferred_audio_gentype,
            env_preferred_video_gentype,
            gen_video_subtitle_subpath,
        },
        world::{
            file_url,
            generated_file_url,
        },
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
};

// # Offline, incoming
const OPFS_OFFLINE_VIEWS_ROOT: &str = "offline_views";
const OPFS_OFFLINE_VIEWS_VIEW_FILENAME: &str = "view.json";
const OPFS_OFFLINE_VIEWS_DONE_FILENAME: &str = "done";
const OPFS_OFFLINE_FILES_ROOT: &str = "offline_files";
const OPFS_OFFLINE_FILES_META_FILENAME: &str = "meta.json";
const OPFS_OFFLINE_FILES_FILE_FILENAME: &str = "file";
const OPFS_OFFLINE_FILES_GEN_DIR: &str = "gen";
pub const OPFS_OFFLINE_FILES_COMIC_PAGES_DIR: &str = "pages";

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

pub async fn list_offline_views() -> Result<Vec<(String, MinistateView)>, String> {
    let mut out = vec![];
    let root_dir = &opfs_root().await.get_dir(vec![OPFS_OFFLINE_VIEWS_ROOT.to_string()]).await?;
    for (k, dir) in root_dir.list().await? {
        let dir = match dir.dir() {
            Ok(d) => d,
            Err(e) => {
                state().log.log(&e);
                continue;
            },
        };
        let view =
            match dir
                .get_file(vec![OPFS_OFFLINE_VIEWS_VIEW_FILENAME.to_string()])
                .await?
                .read_json::<MinistateView>()
                .await {
                Ok(v) => v,
                Err(e) => {
                    state()
                        .log
                        .log(&format!("Found invalid view main file in [{}], deleting and continuing: {}", k, &e));
                    dir.delete(&OPFS_OFFLINE_VIEWS_VIEW_FILENAME).await;
                    continue;
                },
            };
        out.push((k, view));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).reverse());
    return Ok(out);
}

pub async fn ensure_offline(eg: EventGraph, view: MinistateView) -> Result<(), String> {
    request_persistent().await;
    let key = Utc::now().to_rfc3339();
    let views_root = &opfs_root().await.ensure_dir(vec![OPFS_OFFLINE_VIEWS_ROOT.to_string(), key.clone()]).await?;
    views_root.ensure_file(vec![OPFS_OFFLINE_VIEWS_VIEW_FILENAME.to_string()]).await?.write_json(&view).await?;
    eg.event(|pc| {
        state().offline_list.splice(pc, 0, 0, vec![(key.clone(), view.clone())]);
    }).unwrap();
    trigger_offlining(eg);
    return Ok(());
}

pub async fn remove_offline(eg: EventGraph, key: &str) -> Result<(), String> {
    let views_dir = opfs_root().await.get_dir(vec![OPFS_OFFLINE_VIEWS_ROOT.to_string()]).await?;
    views_dir.delete(key).await;
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
    let Ok(hash) = unwrap_value_media_hash(&src) else {
        return None;
    };
    return Some(hash);
}

fn mime_filename(path: &str) -> String {
    return format!("{}.mime", path);
}

pub async fn get_opfs_url_with_colocated_mime(parent: &OpfsDir, mut path: Vec<String>) -> Result<String, String> {
    let Some(file) = path.pop() else {
        return Err(format!("get_file_url_with_mime called with empty path"));
    };
    let dir = parent.get_dir(path).await?;
    let mime: String = dir.get_file(vec![mime_filename(&file)]).await?.read_json().await?;
    let file = dir.get_file(vec![file]).await?;
    return Ok(file.url(&mime).await?);
}

pub async fn offline_file_url(h: &FileHash) -> Result<String, String> {
    return Ok(
        get_opfs_url_with_colocated_mime(
            &opfs_root().await,
            vec![OPFS_OFFLINE_FILES_ROOT.to_string(), h.to_string(), OPFS_OFFLINE_FILES_FILE_FILENAME.to_string()],
        ).await?,
    );
}

pub async fn offline_gen_dir(h: &FileHash, gentype: &str) -> Result<OpfsDir, String> {
    return Ok(
        opfs_root()
            .await
            .get_dir(
                vec![
                    OPFS_OFFLINE_FILES_ROOT.to_string(),
                    h.to_string(),
                    OPFS_OFFLINE_FILES_GEN_DIR.to_string(),
                    gentype.to_string()
                ],
            )
            .await?,
    );
}

pub async fn offline_gen_url(h: &FileHash, gentype: &str, subpath: &str) -> Result<String, String> {
    let root = opfs_root().await;
    if subpath == "" {
        return Ok(
            get_opfs_url_with_colocated_mime(
                &root,
                vec![
                    OPFS_OFFLINE_FILES_ROOT.to_string(),
                    h.to_string(),
                    OPFS_OFFLINE_FILES_GEN_DIR.to_string(),
                    gentype.to_string()
                ],
            ).await?,
        );
    } else {
        return Ok(
            get_opfs_url_with_colocated_mime(
                &root,
                vec![
                    OPFS_OFFLINE_FILES_ROOT.to_string(),
                    h.to_string(),
                    OPFS_OFFLINE_FILES_GEN_DIR.to_string(),
                    gentype.to_string(),
                    subpath.to_string()
                ],
            ).await?,
        );
    }
}

async fn fetch_media_file(config_at: &FieldOrLiteral, data_stack: &Vec<Rc<DataStackLevel>>) -> Result<(), String> {
    async fn download_colocate_mime(parent: &OpfsDir, seg: &str, url: String) -> Result<(), String> {
        let head =
            reqwasm::http::Request::new(&url)
                .send()
                .await
                .map_err(|e| format!("Error sending get request for offline-use view file [{}]: {}", url, e))?;
        parent
            .ensure_file(vec![mime_filename(seg)])
            .await?
            .write_json(&head.headers().get("Content-Type").unwrap_or("application/binary".to_string()))
            .await?;
        parent
            .ensure_file(vec![seg.to_string()])
            .await?
            .write_binary(
                &head.binary().await.map_err(|e| format!("Error downloading media file [{}]: {}", url, e))?,
            )
            .await?;
        return Ok(());
    }

    let Some(src) = maybe_get_field_or_literal(config_at, data_stack) else {
        return Ok(());
    };
    let TreeNode::Scalar(src) = src else {
        return Ok(());
    };
    let Some(meta) = maybe_get_meta(data_stack, &src) else {
        return Ok(());
    };
    let src = unwrap_value_media_hash(&src)?;
    let file_dir = opfs_root().await.ensure_dir(vec![OPFS_OFFLINE_FILES_ROOT.to_string(), src.to_string()]).await?;
    file_dir.ensure_file(vec![OPFS_OFFLINE_FILES_META_FILENAME.to_string()]).await?.write_json(meta).await?;
    let mime = meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("");
    let mime_parts = mime.split_once("/").unwrap_or((mime, ""));
    match mime_parts {
        ("image", _) => {
            download_colocate_mime(
                &file_dir,
                OPFS_OFFLINE_FILES_FILE_FILENAME,
                file_url(&state().env, &src),
            ).await?;
        },
        ("video", _) => {
            let gen_dir = file_dir.ensure_dir(vec![OPFS_OFFLINE_FILES_GEN_DIR.to_string()]).await?;
            if mime_parts.1 == "webm" {
                download_colocate_mime(
                    &file_dir,
                    OPFS_OFFLINE_FILES_FILE_FILENAME,
                    file_url(&state().env, &src),
                ).await?;
            } else {
                let gen_type = gentype_transcode(TRANSCODE_MIME_WEBM);
                download_colocate_mime(
                    &gen_dir,
                    &gen_type,
                    generated_file_url(&state().env, &src, &gen_type, ""),
                ).await?;
            }
            {
                let gentype = GENTYPE_VTT;
                let gen_dir = gen_dir.ensure_dir(vec![gentype.to_string()]).await?;
                for lang in &state().env.languages {
                    let subpath = gen_video_subtitle_subpath(lang);
                    if let Err(e) =
                        download_colocate_mime(
                            &gen_dir,
                            &subpath,
                            generated_file_url(&state().env, &src, &gentype, &subpath),
                        ).await {
                        state().log.log(&format!("Failed to offline subtitle file: {}", e));
                    }
                }
            }
        },
        ("audio", _) => {
            download_colocate_mime(
                &file_dir,
                OPFS_OFFLINE_FILES_FILE_FILENAME,
                file_url(&state().env, &src),
            ).await?;
            let gen_dir = file_dir.ensure_dir(vec![OPFS_OFFLINE_FILES_GEN_DIR.to_string()]).await?;
            let gentype = gentype_transcode(TRANSCODE_MIME_AAC);
            if let Err(e) =
                download_colocate_mime(
                    &gen_dir,
                    &gentype,
                    generated_file_url(&state().env, &src, &gentype, ""),
                ).await {
                state().log.log(&format!("Failed to offline aac transcode file: {}", e));
            }
        },
        ("application", "epub+zip") => {
            let gen_dir = file_dir.ensure_dir(vec![OPFS_OFFLINE_FILES_GEN_DIR.to_string()]).await?;
            let gentype = GENTYPE_EPUBHTML;
            download_colocate_mime(&gen_dir, gentype, generated_file_url(&state().env, &src, gentype, "")).await?;
        },
        ("application", "x-cbr") | ("application", "x-cbz") | ("application", "x-cb7") => {
            let dir_url = generated_file_url(&state().env, &src, GENTYPE_CBZDIR, "");
            let manifest_url = format!("{}/{}", dir_url, GEN_FILENAME_COMICMANIFEST);
            let manifest =
                serde_json::from_slice::<ComicManifest>(
                    &req_file(&manifest_url).await?,
                ).map_err(|e| format!("Error parsing comic manifest json at {}: {}", manifest_url, e))?;
            let gen_dir =
                file_dir.ensure_dir(vec![OPFS_OFFLINE_FILES_GEN_DIR.to_string(), GENTYPE_CBZDIR.to_string()]).await?;
            gen_dir.ensure_file(vec![COMIC_MANIFEST_FILENAME.to_string()]).await?.write_json(&manifest).await?;
            let pages_dir = gen_dir.ensure_dir(vec![OPFS_OFFLINE_FILES_COMIC_PAGES_DIR.to_string()]).await?;
            for page in manifest.pages {
                download_colocate_mime(&pages_dir, &page.path, format!("{}/{}", dir_url, page.path)).await?;
            }
        },
        _ => {
            return Ok(());
        },
    };
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

pub async fn retrieve_offline_query(
    key: &str,
    query_id: &str,
    params: &HashMap<String, Node>,
) -> Result<RespQuery, String> {
    let res: RespQuery =
        opfs_root()
            .await
            .get_file(
                vec![
                    OPFS_OFFLINE_VIEWS_ROOT.to_string(),
                    key.to_string(),
                    opfs_offline_views_query_filename(&query_id, &params)
                ],
            )
            .await?
            .read_json()
            .await?;
    return Ok(res);
}

pub async fn offline_audio_url(h: &FileHash) -> String {
    if let Some(gentype) = env_preferred_audio_gentype(&state().env) {
        return match offline_gen_url(h, &gentype, "").await {
            Ok(v) => v,
            Err(e) => {
                state().log.log(&format!("Error getting opfs generated url: {}", e));
                format!("")
            },
        };
    } else {
        return match offline_file_url(h).await {
            Ok(v) => v,
            Err(e) => {
                state().log.log(&format!("Error getting opfs file url: {}", e));
                format!("")
            },
        };
    }
}

pub async fn offline_video_url(h: &FileHash) -> String {
    return match offline_gen_url(h, &env_preferred_video_gentype(), "").await {
        Ok(x) => x,
        Err(e) => {
            state().log.log(&format!("Error determining offline video url: {}", e));
            format!("")
        },
    };
}

pub fn stop_offlining() {
    *state().offlining_bg.borrow_mut() = None;
}

pub fn trigger_offlining(eg: EventGraph) {
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
                    let _cleanup = defer({
                        let eg = eg.clone();
                        move || eg.event(|pc| {
                            *state().offlining_bg.borrow_mut() = None;
                            state().offlining.set(pc, false);
                        }).unwrap()
                    });

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
                    let offline_views_root = opfs_root().await.ensure_dir(vec![OPFS_OFFLINE_VIEWS_ROOT.to_string()]).await?;
                    for (key, task_dir) in offline_views_root.list().await? {
                        match async {
                            let task_dir = match task_dir.dir() {
                                Ok(d) => d,
                                Err(e) => {
                                    state().log.log(&e);
                                    return Ok(());
                                },
                            };

                            // # Handle creates/downloads
                            if task_dir.exists(OPFS_OFFLINE_VIEWS_DONE_FILENAME).await? {
                                return Ok(());
                            }
                            let view: MinistateView =
                                task_dir
                                    .get_file(vec![OPFS_OFFLINE_VIEWS_VIEW_FILENAME.to_string()])
                                    .await?
                                    .read_json()
                                    .await?;
                            let client_config = state().client_config.get().await.borrow().clone();
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
                                            let empty_node_meta: Rc<HashMap<Node, NodeMeta>> = Default::default();
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
                                    task_dir
                                        .ensure_file(vec![opfs_offline_views_query_filename(&query_id, &params)])
                                        .await?
                                        .write_json(&res)
                                        .await?;
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
                                            stack.push((RootOrWidget::Widget(&w.element_body), data_at.clone()));
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
                            task_dir
                                .ensure_file(vec![OPFS_OFFLINE_VIEWS_DONE_FILENAME.to_string()])
                                .await?
                                .write_binary(&[])
                                .await?;
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
                    for (
                        key,
                        task_dir,
                    ) in opfs_root().await.ensure_dir(vec![OPFS_OFFLINE_VIEWS_ROOT.to_string()]).await?.list().await? {
                        match async {
                            let task_dir = match task_dir.dir() {
                                Ok(d) => d,
                                Err(e) => {
                                    state().log.log(&e);
                                    return Ok(());
                                },
                            };
                            let view: MinistateView =
                                task_dir
                                    .get_file(vec![OPFS_OFFLINE_VIEWS_VIEW_FILENAME.to_string()])
                                    .await?
                                    .read_json()
                                    .await?;
                            let client_config = state().client_config.get().await.borrow().clone();
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
                                            let empty_node_meta: Rc<HashMap<Node, NodeMeta>> = Default::default();
                                            return Ok(res.into_iter().map(|x| DataStackLevel {
                                                data: x,
                                                node_meta: empty_node_meta.clone(),
                                            }).collect());
                                        },
                                        QueryOrField::Query(q) => q,
                                    };
                                    let params = data_to_query_params(view_def, query_id, data_at);
                                    let res: RespQuery =
                                        match task_dir
                                            .get_file(vec![opfs_offline_views_query_filename(&query_id, &params)])
                                            .await {
                                            Ok(r) => r,
                                            Err(_) => {
                                                return Ok(vec![]);
                                            },
                                        }.read_json().await?;
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
                    let files_root = opfs_root().await.ensure_dir(vec![OPFS_OFFLINE_FILES_ROOT.to_string()]).await?;
                    for (key, _) in files_root.list().await? {
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
                        files_root.delete(&key).await;
                    }

                    // Nothing left to do atm, exit
                    return Ok(JsValue::null());
                });
            });
            JsFuture::from(window().navigator().locks().request_with_callback("offline", cb.as_ref().unchecked_ref()))
                .await
                .log(&state().log, "Error doing work in `offline` lock");
        }));
    }
}
