use {
    crate::libnonlink::state::{
        set_page,
        state,
    },
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::window,
    },
    lunk::ProcessingContext,
    shared_wasm::log::LogJsErr,
    wasm::js::{
        on_thinking,
        style_export,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlInputElement,
};

pub const LOCALSTORAGE_OFFLINE_ENABLED: &str = "offline_enabled";

pub fn build_page_settings(pc: &mut ProcessingContext) {
    let offline_enabled = LocalStorage::get::<bool>(LOCALSTORAGE_OFFLINE_ENABLED).unwrap_or(false);
    let offline_pair = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
        id: "offline_enabled".to_string(),
        title: "Offline".to_string(),
        value: offline_enabled,
    });
    let save_button = style_export::leaf_button_big_commit().root;
    on_thinking(&save_button, {
        let input = offline_pair.input.weak();
        move || {
            let input = input.clone();
            async move {
                let Some(input) = input.upgrade() else {
                    return;
                };
                let checked = input.raw().dyn_into::<HtmlInputElement>().unwrap().checked();
                LocalStorage::set(
                    LOCALSTORAGE_OFFLINE_ENABLED,
                    checked,
                ).log(&state().log, "Error storing offline_enabled setting");
                window().location().reload().log(&state().log, "Error reloading page");
            }
        }
    });
    let page = style_export::cont_page_form(style_export::ContPageFormArgs {
        bar_children: vec![save_button],
        entries: vec![offline_pair.root],
    });
    set_page(pc, "Settings", page.root);
}
