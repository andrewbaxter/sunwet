use {
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    js_sys::{
        Array,
        Function,
        JSON,
        Promise,
        Reflect,
        Uint8Array,
    },
    lunk::EventGraph,
    shared::interface::{
        config::view::ViewId,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            C2SReq,
            CommitFile,
            ReqCommit,
            ReqCommitFree,
            ReqViewQuery,
            RespQuery,
            RespQueryRows,
            Triple,
        },
    },
    shared_wasm::{
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
        str::FromStr,
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

thread_local! {
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
    let url: Result<String, _> = LocalStorage::get("sunwet_server_url");
    let token: Result<String, _> = LocalStorage::get("sunwet_token");
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

fn update_button_state(
    button: &HtmlButtonElement,
    existence: Existence,
    error: ErrorState,
) {
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

async fn api_request<Req, Resp>(base_url: &str, token: Option<&str>, req: Req) -> Result<Resp, String>
where
    Req: serde::Serialize,
    Resp: serde::de::DeserializeOwned,
{
    let window = web_sys::window().ok_or("no window")?;
    let url = format!("{}api", base_url);
    let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.mode(web_sys::RequestMode::Cors);
    opts.body(Some(&JsValue::from_str(&body)));

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("failed to create request: {:?}", e))?;
    let headers = request.headers();
    let _ = headers.set("Content-type", "application/json");
    if let Some(t) = token {
        let _ = headers.set("Authorization", &format!("Bearer {}", t));
    }

    let resp = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {:?}", e))?;
    let resp: web_sys::Response = resp
        .dyn_into()
        .map_err(|e| format!("invalid response: {:?}", e))?;

    if resp.status() == 401 {
        return Err("Unauthorized".to_string());
    }
    if resp.status() >= 400 {
        let text = JsFuture::from(resp.text().unwrap())
            .await
            .map_err(|e| format!("failed to read error body: {:?}", e))?;
        return Err(format!("HTTP {}: {:?}", resp.status(), text));
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| format!("failed to parse json: {:?}", e))?;
    let json_str = JSON::stringify(&json).unwrap().as_string().unwrap();
    serde_json::from_str(&json_str).map_err(|e| format!("json decode error: {}", e))
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

    let req: C2SReq = ReqViewQuery {
        view_id: ViewId(view_query.to_string()),
        query: "".to_string(),
        parameters: {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Node::from(id));
            map
        },
        pagination: None,
    }.into();

    let result: Result<RespQuery, String> = api_request(&base_url, token.as_deref(), req).await;
    match result {
        Ok(resp) => {
            let exists = match resp.rows {
                RespQueryRows::Scalar(v) => !v.is_empty(),
                RespQueryRows::Record(v) => !v.is_empty(),
            };
            update_button_state(
                button,
                if exists {
                    Existence::Exists
                } else {
                    Existence::New
                },
                ErrorState::None,
            );
        },
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "sunwet existence check error: {}",
                e
            )));
            update_button_state(button, Existence::New, ErrorState::Error);
        },
    }
}

fn parse_callback_result(js_value: &JsValue) -> Result<(ReqCommitFree, Vec<UploadFile>), String> {
    let comment = Reflect::get(js_value, &JsValue::from_str("comment"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "Browser capture".to_string());

    let triples_arr = Reflect::get(js_value, &JsValue::from_str("triples"))
        .map_err(|_| "missing 'triples' field")?;
    let triples_arr: Array = triples_arr
        .dyn_into()
        .map_err(|_| "'triples' must be an array")?;
    let mut triples = Vec::with_capacity(triples_arr.length() as usize);
    for i in 0..triples_arr.length() {
        let item = triples_arr.get(i);
        let subject = Reflect::get(&item, &JsValue::from_str("subject"))
            .map_err(|_| "missing subject")?
            .as_string()
            .ok_or("subject must be a string")?;
        let predicate = Reflect::get(&item, &JsValue::from_str("predicate"))
            .map_err(|_| "missing predicate")?
            .as_string()
            .ok_or("predicate must be a string")?;
        let object = Reflect::get(&item, &JsValue::from_str("object"))
            .map_err(|_| "missing object")?
            .as_string()
            .ok_or("object must be a string")?;
        triples.push(Triple {
            subject: Node::from(subject),
            predicate,
            object: Node::from(object),
        });
    }

    let mut upload_files = Vec::new();
    let mut commit_files = Vec::new();
    if let Ok(files_arr) = Reflect::get(js_value, &JsValue::from_str("files")) {
        if !files_arr.is_undefined() && !files_arr.is_null() {
            let files_arr: Array = files_arr
                .dyn_into()
                .map_err(|_| "'files' must be an array")?;
            for i in 0..files_arr.length() {
                let item = files_arr.get(i);
                let data = Reflect::get(&item, &JsValue::from_str("data"))
                    .map_err(|_| "missing file data")?;
                let data: Uint8Array = data
                    .dyn_into()
                    .map_err(|_| "file data must be Uint8Array")?;
                let hash = Reflect::get(&item, &JsValue::from_str("hash"))
                    .map_err(|_| "missing file hash")?
                    .as_string()
                    .ok_or("file hash must be a string")?;
                let mimetype = Reflect::get(&item, &JsValue::from_str("mimetype"))
                    .map_err(|_| "missing file mimetype")?
                    .as_string()
                    .ok_or("file mimetype must be a string")?;

                let hash = FileHash::from_str(&hash).map_err(|e| e.to_string())?;
                let data_vec = data.to_vec();
                let size = data_vec.len() as u64;
                upload_files.push(UploadFile {
                    data: data_vec,
                    hash: hash.clone(),
                });
                commit_files.push(CommitFile {
                    hash,
                    size,
                    mimetype,
                });
            }
        }
    }

    Ok((
        ReqCommitFree {
            comment,
            add: triples,
            remove: vec![],
            files: commit_files,
        },
        upload_files,
    ))
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
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "sunwet capture callback error: {:?}",
                e
            )));
            update_button_state(button, Existence::New, ErrorState::Error);
            return;
        },
    };

    let result = JsFuture::from(promise).await;
    match result {
        Ok(js_value) => {
            let (commit_free, upload_files) = match parse_callback_result(&js_value) {
                Ok(v) => v,
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "sunwet parse callback result error: {}",
                        e
                    )));
                    update_button_state(button, Existence::New, ErrorState::Error);
                    return;
                },
            };

            let commit = ReqCommit::Free(commit_free);
            let (url, _) = get_settings();
            let Some(base_url) = url else {
                update_button_state(button, Existence::New, ErrorState::Error);
                return;
            };
            let base_url = if base_url.ends_with('/') {
                base_url
            } else {
                format!("{}/", base_url)
            };

            APP_STATE.with(|state| {
                let state = state.borrow();
                let Some(app_state) = state.as_ref() else {
                    update_button_state(button, Existence::New, ErrorState::Error);
                    return;
                };

                let onlining = app_state.onlining.clone();
                let eg = app_state.eg.clone();
                let log = app_state.log.clone();
                let button = button.clone();

                spawn_local(async move {
                    match ensure_commit(&onlining, eg, &log, &base_url, commit, upload_files).await {
                        Ok(_) => {
                            update_button_state(&button, Existence::Exists, ErrorState::None);
                        },
                        Err(e) => {
                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                "sunwet ensure_commit error: {}",
                                e
                            )));
                            update_button_state(&button, Existence::New, ErrorState::Error);
                        },
                    }
                });
            });
        },
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "sunwet capture promise rejected: {:?}",
                e
            )));
            update_button_state(button, Existence::New, ErrorState::Error);
        },
    }
}

#[wasm_bindgen]
pub fn create_capture_button(
    id: String,
    view_query: String,
    callback: Function,
) -> Result<HtmlElement, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;

    let button = document
        .create_element("button")?
        .dyn_into::<HtmlButtonElement>()?;
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
