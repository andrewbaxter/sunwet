use {
    gloo::storage::{
        LocalStorage,
        Storage,
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
    let style = container.dyn_ref::<web_sys::HtmlElement>().unwrap().style();
    style.set_property("max-width", "480px").unwrap();
    style.set_property("margin", "40px auto").unwrap();
    style.set_property("padding", "24px").unwrap();
    style.set_property("font-family", "sans-serif").unwrap();

    let title = document.create_element("h1").unwrap();
    title.set_text_content(Some("Sunwet Settings"));
    let title_style = title.dyn_ref::<web_sys::HtmlElement>().unwrap().style();
    title_style.set_property("font-size", "24px").unwrap();
    title_style.set_property("margin-bottom", "24px").unwrap();
    container.append_child(&title).unwrap();

    // Helper to create a labeled input
    let create_field = |label_text: &str, input_type: &str, storage_key: &str| {
        let wrapper = document.create_element("div").unwrap();
        let w_style = wrapper.dyn_ref::<web_sys::HtmlElement>().unwrap().style();
        w_style.set_property("margin-bottom", "16px").unwrap();

        let label = document.create_element("label").unwrap();
        label.set_text_content(Some(label_text));
        let l_style = label.dyn_ref::<web_sys::HtmlElement>().unwrap().style();
        l_style.set_property("display", "block").unwrap();
        l_style.set_property("margin-bottom", "4px").unwrap();
        l_style.set_property("font-weight", "600").unwrap();
        l_style.set_property("font-size", "14px").unwrap();
        wrapper.append_child(&label).unwrap();

        let input = document
            .create_element("input")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        input.set_type(input_type);
        input.set_id(storage_key);
        let i_style = input.style();
        i_style.set_property("width", "100%").unwrap();
        i_style.set_property("padding", "8px").unwrap();
        i_style.set_property("font-size", "14px").unwrap();
        i_style.set_property("border", "1px solid rgb(149,148,148)").unwrap();
        i_style.set_property("border-radius", "4px").unwrap();
        i_style.set_property("box-sizing", "border-box").unwrap();
        i_style.set_property("background", "rgb(53,55,58)").unwrap();
        i_style.set_property("color", "rgb(237,231,223)").unwrap();
        if let Ok(val) = LocalStorage::get::<String>(storage_key) {
            input.set_value(&val);
        }
        wrapper.append_child(&input).unwrap();

        wrapper
    };

    let url_wrapper = create_field("Server URL", "text", "sunwet_server_url");
    container.append_child(&url_wrapper).unwrap();

    let token_wrapper = create_field("Access Token", "password", "sunwet_token");
    container.append_child(&token_wrapper).unwrap();

    let save_btn = document
        .create_element("button")
        .unwrap()
        .dyn_into::<HtmlButtonElement>()
        .unwrap();
    save_btn.set_text_content(Some("Save"));
    let b_style = save_btn.style();
    b_style.set_property("padding", "10px 20px").unwrap();
    b_style.set_property("font-size", "14px").unwrap();
    b_style.set_property("font-weight", "600").unwrap();
    b_style.set_property("color", "#fff").unwrap();
    b_style.set_property("background", "#1565c0").unwrap();
    b_style.set_property("border", "none").unwrap();
    b_style.set_property("border-radius", "4px").unwrap();
    b_style.set_property("cursor", "pointer").unwrap();

    let save_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let document = web_sys::window().unwrap().document().unwrap();
        let url_input = document
            .get_element_by_id("sunwet_server_url")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let token_input = document
            .get_element_by_id("sunwet_token")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let url = url_input.value();
        let token = token_input.value();
        let _ = LocalStorage::set("sunwet_server_url", &url);
        let _ = LocalStorage::set("sunwet_token", &token);
        web_sys::window()
            .unwrap()
            .alert_with_message("Settings saved!")
            .unwrap();
    }) as Box<dyn FnMut(_)>);
    save_btn
        .add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();

    container.append_child(&save_btn).unwrap();
    body.append_child(&container).unwrap();
}
