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
    lunk::{
        Prim,
        ProcessingContext,
        link,
    },
    rooting::El,
    shared_wasm::log::LogJsErr,
    wasm::js::{
        on_thinking,
        style_export,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlElement,
        HtmlInputElement,
    },
};

pub const LOCALSTORAGE_OFFLINE_ENABLED: &str = "offline_enabled";
pub const LOCALSTORAGE_BOOK_FONT_SIZE: &str = "book_font_size";
pub const DEFAULT_BOOK_FONT_SIZE: &str = "14";

pub fn build_page_settings(pc: &mut ProcessingContext) {
    let offline_enabled = LocalStorage::get::<bool>(LOCALSTORAGE_OFFLINE_ENABLED).unwrap_or(false);
    let offline_pair = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
        id: "offline_enabled".to_string(),
        title: "Offline".to_string(),
        value: offline_enabled,
    });
    let book_font_size =
        LocalStorage::get::<String>(LOCALSTORAGE_BOOK_FONT_SIZE).unwrap_or(DEFAULT_BOOK_FONT_SIZE.to_string());
    let font_size_pair = style_export::leaf_input_pair_range(style_export::LeafInputPairRangeArgs {
        id: "book_font_size".to_string(),
        title: "Book font size".to_string(),
        value: book_font_size.clone(),
        min: "6".to_string(),
        max: "50".to_string(),
        preview: "Sphinx of black quartz, judge my vow.".to_string(),
    });
    let font_size_value = Prim::new(book_font_size);
    font_size_pair.input.ref_on("input", {
        let eg = pc.eg();
        let font_size_value = font_size_value.clone();
        move |ev| {
            let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            eg.event(|pc| {
                font_size_value.set(pc, e.value());
            }).unwrap();
        }
    });
    font_size_pair.preview.ref_own({
        let preview = font_size_pair.preview.weak();
        |_el| link!((pc = pc), (font_size_value = font_size_value.clone()), (), (preview = preview), {
            let preview = preview.upgrade()?;
            let size = font_size_value.borrow().clone();
            preview
                .raw()
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .style()
                .set_property("font-size", &format!("{}pt", size))
                .ok();
        })
    });
    let save_button = style_export::leaf_button_big_commit().root;
    on_thinking(&save_button, {
        let input = offline_pair.input.weak();
        let font_size_input = font_size_pair.input.weak();
        move || {
            let input = input.clone();
            let font_size_input = font_size_input.clone();
            async move {
                let Some(input) = input.upgrade() else {
                    return;
                };
                let checked = input.raw().dyn_into::<HtmlInputElement>().unwrap().checked();
                LocalStorage::set(
                    LOCALSTORAGE_OFFLINE_ENABLED,
                    checked,
                ).log(&state().log, "Error storing offline_enabled setting");
                let Some(font_size_input) = font_size_input.upgrade() else {
                    return;
                };
                let font_size = font_size_input.raw().dyn_into::<HtmlInputElement>().unwrap().value();
                LocalStorage::set(
                    LOCALSTORAGE_BOOK_FONT_SIZE,
                    font_size,
                ).log(&state().log, "Error storing book_font_size setting");
                window().location().reload().log(&state().log, "Error reloading page");
            }
        }
    });
    let page = style_export::cont_page_form(style_export::ContPageFormArgs {
        bar_children: vec![save_button],
        entries: vec![offline_pair.root, font_size_pair.root],
    });
    set_page(pc, "Settings", page.root);
}
