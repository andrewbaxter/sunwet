use {
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    sunwet_browser::{
        KEY_SERVER_URL,
        KEY_TOKEN,
    },
    wasm_bindgen::prelude::*,
    web_sys::{
        HtmlButtonElement,
        HtmlInputElement,
        MouseEvent,
    },
};

fn main() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let container = document.create_element("div").unwrap();
    container.set_class_name("sunwet-settings-container");

    let title = document.create_element("h1").unwrap();
    title.set_class_name("sunwet-settings-title");
    title.set_text_content(Some("Sunwet Settings"));
    container.append_child(&title).unwrap();

    // Helper to create a labeled input
    let create_field = |label_text: &str, input_type: &str, storage_key: &str| {
        let wrapper = document.create_element("div").unwrap();
        wrapper.set_class_name("sunwet-settings-field");

        let label = document.create_element("label").unwrap();
        label.set_text_content(Some(label_text));
        label.set_class_name("sunwet-settings-label");
        wrapper.append_child(&label).unwrap();

        let input = document
            .create_element("input")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        input.set_type(input_type);
        input.set_id(storage_key);
        input.set_class_name("sunwet-settings-input");
        if let Ok(val) = LocalStorage::get::<String>(storage_key) {
            input.set_value(&val);
        }
        wrapper.append_child(&input).unwrap();

        wrapper
    };

    let url_wrapper = create_field("Server URL", "text", KEY_SERVER_URL);
    container.append_child(&url_wrapper).unwrap();

    let token_wrapper = create_field("Access Token", "password", KEY_TOKEN);
    container.append_child(&token_wrapper).unwrap();

    let error_block = document.create_element("div").unwrap();
    error_block.set_class_name("sunwet-settings-error");
    container.append_child(&error_block).unwrap();

    let save_btn = document
        .create_element("button")
        .unwrap()
        .dyn_into::<HtmlButtonElement>()
        .unwrap();
    save_btn.set_class_name("sunwet-settings-button");
    save_btn.set_text_content(Some("Save"));

    let error_block_closure = error_block.clone();
    let save_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        error_block_closure.set_text_content(None);
        let document = web_sys::window().unwrap().document().unwrap();
        let url_input = document
            .get_element_by_id(KEY_SERVER_URL)
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let token_input = document
            .get_element_by_id(KEY_TOKEN)
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let url = url_input.value();
        let token = token_input.value();
        if let Err(e) = LocalStorage::set(KEY_SERVER_URL, &url) {
            error_block_closure.set_text_content(Some(&format!("Error saving server URL: {}", e)));
            return;
        }
        if let Err(e) = LocalStorage::set(KEY_TOKEN, &token) {
            error_block_closure.set_text_content(Some(&format!("Error saving token: {}", e)));
            return;
        }
        error_block_closure.set_text_content(Some("Settings saved!"));
    }) as Box<dyn FnMut(_)>);
    save_btn
        .add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();

    container.append_child(&save_btn).unwrap();
    body.append_child(&container).unwrap();
}
