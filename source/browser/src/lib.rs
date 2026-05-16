use {
    js_sys::{
        Function,
        Promise,
    },
    lunk::EventGraph,
    serde::Deserialize,
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
            store_commit,
            trigger_onlining_no_lock,
            OnliningState,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    tsify_next::Tsify,
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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["browser", "storage", "local"], js_name = "get")]
    fn browser_storage_get(keys: &JsValue) -> Promise;

    #[wasm_bindgen(js_namespace = ["browser", "storage", "local"], js_name = "set")]
    fn browser_storage_set(items: &JsValue) -> Promise;
}

pub async fn get_setting(key: &str) -> Option<String> {
    let keys = js_sys::Array::new();
    keys.push(&JsValue::from_str(key));
    let result = match JsFuture::from(browser_storage_get(&keys)).await {
        Ok(v) => v,
        Err(e) => {
            web_sys::console::error_1(
                &JsValue::from_str(&format!("sunwet get_setting({}) storage get error: {:?}", key, e)),
            );
            return None;
        },
    };
    match js_sys::Reflect::get(&result, &JsValue::from_str(key)) {
        Ok(v) => v.as_string(),
        Err(e) => {
            web_sys::console::error_1(
                &JsValue::from_str(&format!("sunwet get_setting({}) reflect get error: {:?}", key, e)),
            );
            None
        },
    }
}

pub async fn set_setting(key: &str, value: &str) -> Result<(), JsValue> {
    let items = js_sys::Object::new();
    js_sys::Reflect::set(&items, &JsValue::from_str(key), &JsValue::from_str(value))?;
    JsFuture::from(browser_storage_set(&items.into())).await?;
    Ok(())
}

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

#[derive(Deserialize, Tsify)]
pub struct CaptureFile {
    #[serde(with = "serde_bytes")]
    #[tsify(type = "Uint8Array")]
    pub data: Vec<u8>,
    pub mimetype: String,
    pub parameter: String,
}

#[derive(Deserialize, Tsify)]
pub struct CaptureCallbackResult {
    pub form_id: String,
    #[tsify(type = "Record<string, string>")]
    pub parameters: HashMap<String, String>,
    pub files: Vec<CaptureFile>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_CAPTURE_BUTTON: &str = "export function create_capture_button(id: string, view_query: string, callback: (id: string) => Promise<CaptureCallbackResult>): HTMLElement;";

async fn get_settings() -> (Option<String>, Option<String>) {
    let url = get_setting(KEY_SERVER_URL).await;
    let token = get_setting(KEY_TOKEN).await;
    (url, token)
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
            if let Err(e) = class_list.remove_1("fade") {
                web_sys::console::error_1(
                    &JsValue::from_str(&format!("sunwet update_button_state remove fade error: {:?}", e)),
                );
            }
        },
        Existence::Exists => {
            if let Err(e) = class_list.add_1("fade") {
                web_sys::console::error_1(
                    &JsValue::from_str(&format!("sunwet update_button_state add fade error: {:?}", e)),
                );
            }
        },
    }
    match error {
        ErrorState::None => {
            if let Err(e) = class_list.remove_1("error") {
                web_sys::console::error_1(
                    &JsValue::from_str(&format!("sunwet update_button_state remove error class error: {:?}", e)),
                );
            }
        },
        ErrorState::Error => {
            if let Err(e) = class_list.add_1("error") {
                web_sys::console::error_1(
                    &JsValue::from_str(&format!("sunwet update_button_state add error class error: {:?}", e)),
                );
            }
        },
    }
}

async fn check_existence(button: &HtmlButtonElement, id: &str, view_query: &str) {
    let (url, token) = get_settings().await;
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
        let result: CaptureCallbackResult =
            serde_wasm_bindgen::from_value(js_value).map_err(|e| format!("{}", e))?;
        let form_id = result.form_id;
        let mut parameters: HashMap<String, TreeNode> =
            result
                .parameters
                .into_iter()
                .map(|(k, v)| (k, TreeNode::Scalar(Node::from(v))))
                .collect();
        let mut commit_files = vec![];
        let mut upload_files = vec![];
        let mut param_files: HashMap<String, Vec<TreeNode>> = HashMap::new();
        for file in result.files {
            let hash = FileHash::from_sha256(Sha256::digest(&file.data));
            param_files
                .entry(file.parameter)
                .or_default()
                .push(TreeNode::Scalar(Node::File(hash.clone())));
            commit_files.push(CommitFile {
                hash: hash.clone(),
                size: file.data.len() as u64,
                mimetype: file.mimetype,
            });
            upload_files.push(UploadFile { data: file.data, hash });
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
                match store_commit(&log, ReqCommit::Form(form), upload_files).await {
                    Ok(_) => {
                        trigger_onlining_no_lock(&onlining, eg, &log, &base_url);
                        update_button_state(&button, Existence::Exists, ErrorState::None);
                    },
                    Err(e) => {
                        web_sys::console::error_1(
                            &JsValue::from_str(&format!("sunwet store_commit error: {}", e)),
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

#[wasm_bindgen(skip_typescript)]
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
