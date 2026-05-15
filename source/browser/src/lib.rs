use {
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    js_sys::{
        Array,
        Function,
        Object,
        Promise,
        Reflect,
        Uint8Array,
    },
    lunk::EventGraph,
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
            RespQuery,
            RespQueryRows,
            TreeNode,
        },
    },
    shared_wasm::{
        api::req_post_json_with_headers,
        commit::UploadFile,
        log::Log,
        online::{
            ensure_commit,
            OnliningState,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::{
        spawn_local,
        JsFuture,
    },
    web_sys::{
        HtmlButtonElement,
        HtmlElement,
        MouseEvent,
    },
};

pub const KEY_SERVER_URL: &str = "sunwet_server_url";
pub const KEY_TOKEN: &str = "sunwet_token";

thread_local!{
    static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None);
}

struct AppState {
    onlining: Rc<OnliningState>,
    eg: EventGraph,
    log: Rc<dyn Log>,
}

pub fn init_app_state(onlining: Rc<OnliningState>, eg: EventGraph, log: Rc<dyn Log>) {
    APP_STATE.with(|s| {
        *s.borrow_mut() = Some(AppState {
            onlining,
            eg,
            log,
        });
    });
}

fn get_settings() -> (Option<String>, Option<String>) {
    let url: Result<String, _> = LocalStorage::get(KEY_SERVER_URL);
    let token: Result<String, _> = LocalStorage::get(KEY_TOKEN);
    (url.ok(), token.ok())
}

#[derive(Clone, Copy)]
enum Existence {
    New,
    Exists,
}

#[derive(Clone, Copy)]
enum ErrorState {
    None,
    Error,
}

fn update_button_state(button: &HtmlButtonElement, existence: Existence, error: ErrorState) {
    let class_list = button.class_list();
    match existence {
        Existence::New => {
            let _ = class_list.remove_1("fade");
        },
        Existence::Exists => {
            let _ = class_list.add_1("fade");
        },
    }
    match error {
        ErrorState::None => {
            let _ = class_list.remove_1("error");
        },
        ErrorState::Error => {
            let _ = class_list.add_1("error");
        },
    }
}

async fn check_existence(button: &HtmlButtonElement, id: &str, view_query: &str) {
    let (url, token) = get_settings();
    let Some(base_url) = url else {
        update_button_state(button, Existence::New, ErrorState::Error);
        return;
    };
    let base_url = if base_url.ends_with('/') {
        base_url
    } else {
        format!("{}/", base_url)
    };
    let mut headers = HashMap::new();
    if let Some(t) = token {
        headers.insert("Authorization".to_string(), format!("Bearer {}", t));
    }
    let req = ReqViewQuery {
        view_id: ViewId(view_query.to_string()),
        query: "".to_string(),
        parameters: {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Node::from(id));
            map
        },
        pagination: None,
    };
    APP_STATE.with(|state| {
        let state = state.borrow();
        let Some(app_state) = state.as_ref() else {
            update_button_state(button, Existence::New, ErrorState::Error);
            return;
        };
        let log = app_state.log.clone();
        let button = button.clone();
        spawn_local(async move {
            let result: Result<RespQuery, String> =
                req_post_json_with_headers(&log, &base_url, &headers, req).await;
            match result {
                Ok(resp) => {
                    let exists = match resp.rows {
                        RespQueryRows::Scalar(v) => !v.is_empty(),
                        RespQueryRows::Record(v) => !v.is_empty(),
                    };
                    update_button_state(&button, if exists {
                        Existence::Exists
                    } else {
                        Existence::New
                    }, ErrorState::None);
                },
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!("sunwet existence check error: {}", e)));
                    update_button_state(&button, Existence::New, ErrorState::Error);
                },
            }
        });
    });
}

fn js_value_to_treenode(value: &JsValue) -> Result<TreeNode, String> {
    if let Some(s) = value.as_string() {
        return Ok(TreeNode::Scalar(Node::from(s)));
    }
    if let Some(arr) = value.dyn_ref::<Array>() {
        let mut out = Vec::with_capacity(arr.length() as usize);
        for i in 0 .. arr.length() {
            out.push(js_value_to_treenode(&arr.get(i))?);
        }
        return Ok(TreeNode::Array(out));
    }
    if let Some(obj) = value.dyn_ref::<Object>() {
        let mut out = std::collections::BTreeMap::new();
        let keys = Object::keys(obj);
        for i in 0 .. keys.length() {
            let key = keys.get(i).as_string().ok_or("object key must be string")?;
            let val = Reflect::get(obj, &JsValue::from_str(&key)).map_err(|_| format!("missing key {}", key))?;
            out.insert(key, js_value_to_treenode(&val)?);
        }
        return Ok(TreeNode::Record(out));
    }
    Err("unsupported value type for TreeNode".to_string())
}

async fn handle_click(button: &HtmlButtonElement, id: &str, callback: &Function) {
    update_button_state(button, Existence::New, ErrorState::None);
    let this = JsValue::null();
    let id_js = JsValue::from_str(id);
    let promise = match callback.call1(&this, &id_js) {
        Ok(v) => {
            if let Ok(p) = v.clone().dyn_into::<Promise>() {
                p
            } else {
                Promise::resolve(&v)
            }
        },
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!("sunwet capture callback error: {:?}", e)));
            update_button_state(button, Existence::New, ErrorState::Error);
            return;
        },
    };
    let js_value = match JsFuture::from(promise).await {
        Ok(v) => v,
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!("sunwet capture promise rejected: {:?}", e)));
            update_button_state(button, Existence::New, ErrorState::Error);
            return;
        },
    };
    let res: Result<(), String> = async {
        let form_id =
            Reflect::get(&js_value, &JsValue::from_str("form_id"))
                .map_err(|_| "missing 'form_id' field")?
                .as_string()
                .ok_or("form_id must be a string")?;
        let mut parameters = HashMap::new();
        let params =
            Reflect::get(&js_value, &JsValue::from_str("parameters")).map_err(|_| "missing 'parameters' field")?;
        if !params.is_undefined() && !params.is_null() {
            let params: Object = params.dyn_into().map_err(|_| "'parameters' must be an object")?;
            let keys = Object::keys(&params);
            for i in 0 .. keys.length() {
                let key = keys.get(i).as_string().ok_or("parameter key must be string")?;
                let val =
                    Reflect::get(&params, &JsValue::from_str(&key)).map_err(|_| format!("missing parameter {}", key))?;
                parameters.insert(key, js_value_to_treenode(&val)?);
            }
        }
        let mut commit_files = vec![];
        let mut upload_files = vec![];
        let files_val =
            Reflect::get(&js_value, &JsValue::from_str("files")).map_err(|_| "missing 'files' field")?;
        if !files_val.is_undefined() && !files_val.is_null() {
            let files_arr: Array = files_val.dyn_into().map_err(|_| "'files' must be an array")?;
            let mut param_files: HashMap<String, Vec<TreeNode>> = HashMap::new();
            for i in 0 .. files_arr.length() {
                let file_obj = files_arr.get(i);
                let data: Uint8Array =
                    Reflect::get(&file_obj, &JsValue::from_str("data"))
                        .map_err(|_| "missing file 'data'")?
                        .dyn_into()
                        .map_err(|_| "file 'data' must be a Uint8Array")?;
                let data = data.to_vec();
                let mimetype =
                    Reflect::get(&file_obj, &JsValue::from_str("mimetype"))
                        .map_err(|_| "missing file 'mimetype'")?
                        .as_string()
                        .ok_or("file 'mimetype' must be a string")?;
                let parameter =
                    Reflect::get(&file_obj, &JsValue::from_str("parameter"))
                        .map_err(|_| "missing file 'parameter'")?
                        .as_string()
                        .ok_or("file 'parameter' must be a string")?;
                let hash = FileHash::from_sha256(Sha256::digest(&data));
                param_files
                    .entry(parameter)
                    .or_default()
                    .push(TreeNode::Scalar(Node::File(hash.clone())));
                commit_files.push(CommitFile {
                    hash: hash.clone(),
                    size: data.len() as u64,
                    mimetype,
                });
                upload_files.push(UploadFile { data, hash });
            }
            for (param_name, nodes) in param_files {
                parameters.insert(param_name, if nodes.len() == 1 {
                    nodes.into_iter().next().unwrap()
                } else {
                    TreeNode::Array(nodes)
                });
            }
        }
        let (url, _) = get_settings();
        let Some(base_url) = url else {
            return Err("no server URL configured".to_string());
        };
        let base_url = if base_url.ends_with('/') {
            base_url
        } else {
            format!("{}/", base_url)
        };
        let form = ReqCommitForm {
            form_id: FormId(form_id),
            parameters,
            files: commit_files,
        };
        APP_STATE.with(|state| {
            let state = state.borrow();
            let Some(app_state) = state.as_ref() else {
                return Err("app state not initialized".to_string());
            };
            let onlining = app_state.onlining.clone();
            let eg = app_state.eg.clone();
            let log = app_state.log.clone();
            let button = button.clone();
            spawn_local(async move {
                match ensure_commit(&onlining, eg, &log, &base_url, ReqCommit::Form(form), upload_files).await {
                    Ok(_) => {
                        update_button_state(&button, Existence::Exists, ErrorState::None);
                    },
                    Err(e) => {
                        web_sys::console::error_1(
                            &JsValue::from_str(&format!("sunwet ensure_commit error: {}", e)),
                        );
                        update_button_state(&button, Existence::New, ErrorState::Error);
                    },
                }
            });
            Ok(())
        })
    }.await;
    if let Err(e) = res {
        web_sys::console::error_1(&JsValue::from_str(&format!("sunwet capture error: {}", e)));
        update_button_state(button, Existence::New, ErrorState::Error);
    }
}

#[wasm_bindgen]
pub fn create_capture_button(id: String, view_query: String, callback: Function) -> Result<HtmlElement, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let button = document.create_element("button")?.dyn_into::<HtmlButtonElement>()?;
    button.set_type("button");
    button.set_class_name("sunwet-import-button");
    update_button_state(&button, Existence::New, ErrorState::None);
    let button_check = button.clone();
    let id_check = id.clone();
    let view_query_check = view_query.clone();
    spawn_local(async move {
        check_existence(&button_check, &id_check, &view_query_check).await;
    });
    let button_click = button.clone();
    let id_click = id.clone();
    let callback_click = callback.clone();
    let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let button = button_click.clone();
        let id = id_click.clone();
        let callback = callback_click.clone();
        spawn_local(async move {
            handle_click(&button, &id, &callback).await;
        });
    }) as Box<dyn FnMut(_)>);
    button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(button.dyn_into::<HtmlElement>()?)
}
