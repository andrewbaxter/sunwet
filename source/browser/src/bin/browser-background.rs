use {
    js_sys::{
        Function,
        Promise,
        Uint8Array,
    },
    lunk::{
        EventGraph,
        Prim,
    },
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::{
        config::{
            form::FormId,
            view::ViewId,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            CommitFile,
            ReqCommit,
            ReqCommitForm,
            ReqViewQuery,
            RespQueryRows,
            TreeNode,
        },
    },
    shared_wasm::{
        api::req_post_json_with_headers,
        commit::UploadFile,
        log::{
            ConsoleLog,
            Log,
        },
        online::{
            OnliningState,
            store_commit,
            trigger_onlining_no_lock,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    sunwet_browser::{
        KEY_SERVER_URL,
        KEY_TOKEN,
        get_setting,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        prelude::Closure,
    },
    wasm_bindgen_futures::{
        JsFuture,
        future_to_promise,
        spawn_local,
    },
};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = ["browser", "runtime", "onMessage"], js_name = "addListener")]
    fn on_message_add_listener(callback: &Function);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = "sunwetFetchWithReferer")]
    fn sunwet_fetch_with_referer(url: &str, referer: &str) -> Promise;
}

thread_local!{
    static BG_STATE: RefCell<Option<BgState>> = RefCell::new(None);
}

struct BgState {
    onlining: Rc<OnliningState>,
    eg: EventGraph,
    log: Rc<dyn Log>,
}

async fn get_settings() -> (Option<String>, Option<String>) {
    let url = get_setting(KEY_SERVER_URL).await;
    let token = get_setting(KEY_TOKEN).await;
    (url, token)
}

fn format_base_url(url: String) -> String {
    if url.ends_with('/') {
        url
    } else {
        format!("{}/", url)
    }
}

async fn handle_check_existence(msg: &JsValue) -> Result<JsValue, String> {
    let id =
        js_sys::Reflect::get(msg, &JsValue::from_str("id"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("missing id")?;
    let view_query =
        js_sys::Reflect::get(msg, &JsValue::from_str("view_query"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("missing view_query")?;
    let (url, token) = get_settings().await;
    let Some(base_url) = url else {
        return Err("no server URL configured".to_string());
    };
    let base_url = format_base_url(base_url);
    let mut headers = HashMap::new();
    if let Some(t) = token {
        headers.insert("Authorization".to_string(), format!("Bearer {}", t));
    }
    let log: Rc<dyn Log> = Rc::new(ConsoleLog {});
    let req = ReqViewQuery {
        view_id: ViewId(view_query),
        query: "root".to_string(),
        parameters: {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Node::from(id));
            map
        },
        pagination: None,
    };
    let resp = req_post_json_with_headers(&log, &base_url, &headers, req).await?;
    let rows = match resp.rows {
        RespQueryRows::Scalar(_) => {
            return Err("expected record rows from existence query, got scalar".to_string());
        },
        RespQueryRows::Record(v) => v,
    };
    let result = js_sys::Object::new();
    let Some(row) = rows.into_iter().next() else {
        js_sys::Reflect::set(
            &result,
            &JsValue::from_str("exists"),
            &JsValue::from_bool(false),
        ).map_err(|e| format!("{:?}", e))?;
        return Ok(result.into());
    };
    let id_node = match row.get("id") {
        Some(TreeNode::Scalar(node)) => node.clone(),
        Some(other) => {
            return Err(format!("expected scalar id in existence query result, got {:?}", other));
        },
        None => {
            return Err("existence query result row missing id field".to_string());
        },
    };
    let Node::Value(serde_json::Value::String(id_str)) = &id_node else {
        return Err(format!("expected string id in existence query result, got {:?}", id_node));
    };
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("exists"),
        &JsValue::from_bool(true),
    ).map_err(|e| format!("{:?}", e))?;
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("existing_id"),
        &JsValue::from_str(id_str),
    ).map_err(|e| format!("{:?}", e))?;
    Ok(result.into())
}

async fn handle_capture(msg: &JsValue) -> Result<JsValue, String> {
    let form_id =
        js_sys::Reflect::get(msg, &JsValue::from_str("form_id"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("missing form_id")?;
    let params_js =
        js_sys::Reflect::get(
            msg,
            &JsValue::from_str("parameters"),
        ).map_err(|_| "missing parameters".to_string())?;
    let mut parameters: HashMap<String, TreeNode> = HashMap::new();
    let params_obj = js_sys::Object::from(params_js);
    let keys = js_sys::Object::keys(&params_obj);
    for i in 0 .. keys.length() {
        let key = keys.get(i).as_string().unwrap_or_default();
        let val =
            js_sys::Reflect::get(&params_obj, &keys.get(i)).ok().and_then(|v| v.as_string()).unwrap_or_default();
        parameters.insert(key, TreeNode::Scalar(Node::from(val)));
    }
    let existing_id =
        js_sys::Reflect::get(msg, &JsValue::from_str("existing_id")).ok().and_then(|v| v.as_string());
    parameters.entry("id".to_string()).or_insert_with(|| {
        let id_value = existing_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        TreeNode::Scalar(Node::Value(serde_json::Value::String(id_value)))
    });
    parameters.entry("stamp".to_string()).or_insert_with(|| {
        TreeNode::Scalar(Node::Value(serde_json::Value::String(chrono::Utc::now().to_rfc3339())))
    });
    let files_js =
        js_sys::Reflect::get(msg, &JsValue::from_str("files")).map_err(|_| "missing files".to_string())?;
    let files_arr = js_sys::Array::from(&files_js);
    let mut commit_files = vec![];
    let mut upload_files = vec![];
    let mut param_files: HashMap<String, Vec<TreeNode>> = HashMap::new();
    for i in 0 .. files_arr.length() {
        let file_js = files_arr.get(i);
        let data_js =
            js_sys::Reflect::get(
                &file_js,
                &JsValue::from_str("data"),
            ).map_err(|e| format!("missing data in file {}: {:?}", i, e))?;
        let data = Uint8Array::new(&data_js).to_vec();
        let mimetype =
            js_sys::Reflect::get(&file_js, &JsValue::from_str("mimetype"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());
        let parameter =
            js_sys::Reflect::get(&file_js, &JsValue::from_str("parameter"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
        let hash = FileHash::from_sha256(Sha256::digest(&data));
        param_files.entry(parameter).or_default().push(TreeNode::Scalar(Node::File(hash.clone())));
        commit_files.push(CommitFile {
            hash: hash.clone(),
            size: data.len() as u64,
            mimetype,
        });
        upload_files.push(UploadFile {
            data,
            hash,
        });
    }
    for (param_name, nodes) in param_files {
        parameters.insert(param_name, if nodes.len() == 1 {
            nodes.into_iter().next().unwrap()
        } else {
            TreeNode::Array(nodes)
        });
    }
    let (url, _) = get_settings().await;
    let Some(base_url) = url else {
        return Err("no server URL configured".to_string());
    };
    let base_url = format_base_url(base_url);
    let form = ReqCommitForm {
        form_id: FormId(form_id),
        parameters,
        files: commit_files,
    };
    let log: Rc<dyn Log> = Rc::new(ConsoleLog {});
    store_commit(&log, ReqCommit::Form(form), upload_files).await?;
    BG_STATE.with(|state| {
        let state = state.borrow();
        if let Some(bg) = state.as_ref() {
            trigger_onlining_no_lock(&bg.onlining, bg.eg.clone(), &bg.log, &base_url);
        }
    });
    let result = js_sys::Object::new();
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(true),
    ).map_err(|e| format!("{:?}", e))?;
    Ok(result.into())
}

async fn handle_fetch_media(msg: &JsValue) -> Result<JsValue, String> {
    let url =
        js_sys::Reflect::get(msg, &JsValue::from_str("url"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("missing url")?;
    let referer =
        js_sys::Reflect::get(msg, &JsValue::from_str("referer"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
    let result =
        JsFuture::from(sunwet_fetch_with_referer(&url, &referer))
            .await
            .map_err(|e| format!("fetch error: {:?}", e))?;
    Ok(result)
}

async fn handle_message(msg: JsValue) -> Result<JsValue, JsValue> {
    let msg_type =
        js_sys::Reflect::get(&msg, &JsValue::from_str("type"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
    let result = match msg_type.as_str() {
        "check_existence" => handle_check_existence(&msg).await,
        "capture" => handle_capture(&msg).await,
        "fetch_media" => handle_fetch_media(&msg).await,
        other => Err(format!("unknown message type: {}", other)),
    };
    match result {
        Ok(v) => Ok(v),
        Err(e) => {
            let err_obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&err_obj, &JsValue::from_str("error"), &JsValue::from_str(&e));
            Ok(err_obj.into())
        },
    }
}

fn main() {
    let log: Rc<dyn Log> = Rc::new(ConsoleLog {});
    let state = Rc::new(OnliningState {
        bg: Default::default(),
        running: Prim::new(false),
    });
    let eg = EventGraph::new();
    BG_STATE.with(|s| {
        *s.borrow_mut() = Some(BgState {
            onlining: state.clone(),
            eg: eg.clone(),
            log: log.clone(),
        });
    });

    // Set up message listener
    let handler = Closure::wrap(Box::new(move |msg: JsValue| -> Promise {
        future_to_promise(async move {
            handle_message(msg).await
        })
    }) as Box<dyn FnMut(JsValue) -> Promise>);
    on_message_add_listener(handler.as_ref().unchecked_ref());
    handler.forget();
    log.log("sunwet background script initialized");

    // Try to online any pending commits on startup
    spawn_local(async move {
        let Some(base_url) = get_setting(KEY_SERVER_URL).await else {
            return;
        };
        let base_url = format_base_url(base_url);
        trigger_onlining_no_lock(&state, eg, &log, &base_url);
    });
}
