use {
    js_sys::{
        Function,
        Promise,
    },
    serde::Deserialize,
    std::collections::HashMap,
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

    #[wasm_bindgen(js_namespace = ["browser", "runtime"], js_name = "sendMessage")]
    fn browser_send_message(msg: &JsValue) -> Promise;
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

#[derive(Deserialize, Tsify)]
pub struct CaptureCallbackResult {
    pub form_id: String,
    #[tsify(type = "Record<string, string>")]
    pub parameters: HashMap<String, String>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_CAPTURE_FILE: &str = "export interface CaptureCallbackResult { form_id: string; parameters: Record<string, string>; files: Array<{data: Uint8Array, mimetype: string, parameter: string}>; }";

#[wasm_bindgen(typescript_custom_section)]
const TS_CAPTURE_BUTTON: &str = "export function create_capture_button(id: string, view_query: string, callback: (id: string) => Promise<CaptureCallbackResult>): HTMLElement;";

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

async fn send_to_background(msg: &JsValue) -> Result<JsValue, String> {
    let resp =
        JsFuture::from(browser_send_message(msg))
            .await
            .map_err(|e| format!("sendMessage failed: {:?}", e))?;
    if let Ok(err) = js_sys::Reflect::get(&resp, &JsValue::from_str("error")) {
        if let Some(err_str) = err.as_string() {
            return Err(err_str);
        }
    }
    Ok(resp)
}

async fn check_existence(button: &HtmlButtonElement, id: &str, view_query: &str) {
    let msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&msg, &JsValue::from_str("type"), &JsValue::from_str("check_existence"));
    let _ = js_sys::Reflect::set(&msg, &JsValue::from_str("id"), &JsValue::from_str(id));
    let _ = js_sys::Reflect::set(&msg, &JsValue::from_str("view_query"), &JsValue::from_str(view_query));
    match send_to_background(&msg.into()).await {
        Ok(resp) => {
            let exists =
                js_sys::Reflect::get(&resp, &JsValue::from_str("exists"))
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
            update_button_state(button, if exists {
                Existence::Exists
            } else {
                Existence::New
            }, ErrorState::None);
        },
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!("sunwet existence check error: {}", e)));
            update_button_state(button, Existence::New, ErrorState::Error);
        },
    }
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

    // Send the callback result to background for processing.
    // Add "type": "capture" and forward the whole object.
    let _ = js_sys::Reflect::set(&js_value, &JsValue::from_str("type"), &JsValue::from_str("capture"));
    match send_to_background(&js_value).await {
        Ok(_) => {
            update_button_state(button, Existence::Exists, ErrorState::None);
        },
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&format!("sunwet capture error: {}", e)));
            update_button_state(button, Existence::New, ErrorState::Error);
        },
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
