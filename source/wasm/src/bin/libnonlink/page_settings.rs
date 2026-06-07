use {
    crate::libnonlink::{
        ministate::{
            Ministate,
            ministate_octothorpe,
        },
        state::{
            set_page,
            state,
        },
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
    shared_wasm::world::Lang,
    shared_wasm::log::LogJsErr,
    std::collections::HashMap,
    wasm::{
        js::{
            on_thinking,
            style_export,
        },
        media::{
            AudioLangPref,
            SubtitleLangPref,
        },
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlElement,
        HtmlInputElement,
        HtmlSelectElement,
    },
};

pub const LOCALSTORAGE_OFFLINE_ENABLED: &str = "offline_enabled";
pub const LOCALSTORAGE_BOOK_FONT_SIZE: &str = "book_font_size";
pub const DEFAULT_BOOK_FONT_SIZE: &str = "14";
pub use wasm::media::{
    LOCALSTORAGE_DEFAULT_AUDIO_LANG,
    LOCALSTORAGE_DEFAULT_SUBTITLE_LANG,
    LOCALSTORAGE_SHOW_SUBS_IF_MATCHING_AUDIO,
};

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

    // Audio language dropdown
    let audio_lang_prefs: Vec<AudioLangPref> = [
        AudioLangPref::WorkOriginal,
        AudioLangPref::BrowserLanguage,
        AudioLangPref::FirstLanguage,
    ]
        .into_iter()
        .chain(Lang::all().iter().map(|l| AudioLangPref::Specific(*l)))
        .collect();
    let mut audio_lang_options = HashMap::new();
    for pref in &audio_lang_prefs {
        audio_lang_options.insert(
            serde_json::to_string(pref).unwrap(),
            pref.display_name().to_string(),
        );
    }
    let audio_lang_value =
        LocalStorage::get::<AudioLangPref>(LOCALSTORAGE_DEFAULT_AUDIO_LANG).unwrap_or(AudioLangPref::WorkOriginal);
    let audio_lang_value = serde_json::to_string(&audio_lang_value).unwrap();
    let audio_lang_pair = style_export::leaf_input_pair_enum(style_export::LeafInputPairEnumArgs {
        id: "default_audio_lang".to_string(),
        title: "Default audio language".to_string(),
        value: audio_lang_value,
        options: audio_lang_options,
    });

    // Subtitle language dropdown
    let sub_lang_prefs: Vec<SubtitleLangPref> = [
        SubtitleLangPref::None,
        SubtitleLangPref::WorkOriginal,
        SubtitleLangPref::BrowserLanguage,
        SubtitleLangPref::FirstLanguage,
    ]
        .into_iter()
        .chain(Lang::all().iter().map(|l| SubtitleLangPref::Specific(*l)))
        .collect();
    let mut sub_lang_options = HashMap::new();
    for pref in &sub_lang_prefs {
        sub_lang_options.insert(
            serde_json::to_string(pref).unwrap(),
            pref.display_name().to_string(),
        );
    }
    let sub_lang_value =
        LocalStorage::get::<SubtitleLangPref>(LOCALSTORAGE_DEFAULT_SUBTITLE_LANG).unwrap_or(SubtitleLangPref::None);
    let sub_lang_value = serde_json::to_string(&sub_lang_value).unwrap();
    let sub_lang_pair = style_export::leaf_input_pair_enum(style_export::LeafInputPairEnumArgs {
        id: "default_subtitle_lang".to_string(),
        title: "Default subtitle language".to_string(),
        value: sub_lang_value,
        options: sub_lang_options,
    });

    // Show subtitles if matching audio language
    let show_subs_if_matching =
        LocalStorage::get::<bool>(LOCALSTORAGE_SHOW_SUBS_IF_MATCHING_AUDIO).unwrap_or(true);
    let show_subs_pair = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
        id: "show_subs_if_matching_audio".to_string(),
        title: "Show subtitles if matching audio language".to_string(),
        value: show_subs_if_matching,
    });

    // Save button
    let save_button = style_export::leaf_button_big_commit().root;
    on_thinking(&save_button, {
        let input = offline_pair.input.weak();
        let font_size_input = font_size_pair.input.weak();
        let audio_lang_input = audio_lang_pair.input.weak();
        let sub_lang_input = sub_lang_pair.input.weak();
        let show_subs_input = show_subs_pair.input.weak();
        move || {
            let input = input.clone();
            let font_size_input = font_size_input.clone();
            let audio_lang_input = audio_lang_input.clone();
            let sub_lang_input = sub_lang_input.clone();
            let show_subs_input = show_subs_input.clone();
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
                if let Some(audio_lang_input) = audio_lang_input.upgrade() {
                    let val = audio_lang_input.raw().dyn_into::<HtmlSelectElement>().unwrap().value();
                    let pref: AudioLangPref = serde_json::from_str(&val).unwrap();
                    LocalStorage::set(
                        LOCALSTORAGE_DEFAULT_AUDIO_LANG,
                        pref,
                    ).log(&state().log, "Error storing default_audio_lang setting");
                }
                if let Some(sub_lang_input) = sub_lang_input.upgrade() {
                    let val = sub_lang_input.raw().dyn_into::<HtmlSelectElement>().unwrap().value();
                    let pref: SubtitleLangPref = serde_json::from_str(&val).unwrap();
                    LocalStorage::set(
                        LOCALSTORAGE_DEFAULT_SUBTITLE_LANG,
                        pref,
                    ).log(&state().log, "Error storing default_subtitle_lang setting");
                }
                if let Some(show_subs_input) = show_subs_input.upgrade() {
                    let checked = show_subs_input.raw().dyn_into::<HtmlInputElement>().unwrap().checked();
                    LocalStorage::set(
                        LOCALSTORAGE_SHOW_SUBS_IF_MATCHING_AUDIO,
                        checked,
                    ).log(&state().log, "Error storing show_subs_if_matching_audio setting");
                }
                window().location().reload().log(&state().log, "Error reloading page");
            }
        }
    });

    // Storage and Logs links
    let opfs_link = style_export::leaf_form_link(style_export::LeafFormLinkArgs {
        title: "Storage".to_string(),
        href: ministate_octothorpe(&Ministate::Opfs),
    }).root;
    let logs_link = style_export::leaf_form_link(style_export::LeafFormLinkArgs {
        title: "Logs".to_string(),
        href: ministate_octothorpe(&Ministate::Logs),
    }).root;

    let page = style_export::cont_page_form(style_export::ContPageFormArgs {
        bar_children: vec![save_button],
        entries: vec![
            offline_pair.root,
            font_size_pair.root,
            audio_lang_pair.root,
            sub_lang_pair.root,
            show_subs_pair.root,
            opfs_link,
            logs_link,
        ],
    });
    set_page(pc, "Settings", page.root);
}
