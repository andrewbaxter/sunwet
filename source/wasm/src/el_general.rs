use {
    futures::channel::oneshot::channel,
    gloo::events::EventListener,
    lunk::{
        link,
        HistPrim,
        ProcessingContext,
    },
    rooting::{
        el,
        El,
        WeakEl,
    },
    std::fmt::Display,
    wasm_bindgen::{
        JsValue,
        JsCast,
    },
    web_sys::{
        Event,
        EventTarget,
        HtmlElement,
    },
};

#[derive(Clone, Copy)]
pub struct CssIcon(pub &'static str);

pub static ICON_TRANSPORT_PLAY: CssIcon = CssIcon("\u{e037}");
pub static ICON_TRANSPORT_PAUSE: CssIcon = CssIcon("\u{e034}");
pub static ICON_TRANSPORT_NEXT: CssIcon = CssIcon("\u{e5cc}");
pub static ICON_TRANSPORT_PREVIOUS: CssIcon = CssIcon("\u{e5cb}");
pub static ICON_MENU: CssIcon = CssIcon("\u{e5d2}");
pub static ICON_NOMENU: CssIcon = CssIcon("\u{e9bd}");
pub static ICON_EDIT: CssIcon = CssIcon("\u{e3c9}");
pub static ICON_NOEDIT: CssIcon = CssIcon("\u{e8f4}");
pub static ICON_SAVE: CssIcon = CssIcon("\u{e161}");
pub static ICON_ADD: CssIcon = CssIcon("\u{e145}");
pub static ICON_REMOVE: CssIcon = CssIcon("\u{e15b}");
pub static ICON_FILL: CssIcon = CssIcon("\u{e877}");
pub static ICON_RESET: CssIcon = CssIcon("\u{e166}");
pub static ICON_SELECT_ALL: CssIcon = CssIcon("\u{e837}");
pub static ICON_SELECT_NONE: CssIcon = CssIcon("\u{e836}");
pub static ICON_VOLUME: CssIcon = CssIcon("\u{e050}");
pub static ICON_SHARE: CssIcon = CssIcon("\u{e80d}");
pub static ICON_NOSHARE: CssIcon = CssIcon("\u{f6cb}");
pub static ICON_CLOSE: CssIcon = CssIcon("\u{e5cd}");
pub static CSS_S_TITLE: &'static str = "s_title";
pub static CSS_S_BODY: &'static str = "s_body";
pub static CSS_S_ROOT: &'static str = "s_root";
pub static CSS_S_MENU: &'static str = "s_menu";
pub static CSS_S_VIEW: &'static str = "s_view";
pub static CSS_VBOX: &'static str = "g_vbox";
pub static CSS_HBOX: &'static str = "g_hbox";
pub static CSS_SPACER: &'static str = "g_space";
pub static CSS_HSCROLL: &'static str = "g_hscroll";
pub static CSS_GROUP: &'static str = "g_group";
pub static CSS_STACK: &'static str = "g_icon";
pub static CSS_ICON: &'static str = "g_stack";
pub static CSS_MODAL: &'static str = "g_modal";
pub static CSS_MODAL_BG: &'static str = "modal_bg";
pub static CSS_MODAL_CONTENT: &'static str = "modal_content";
pub static CSS_MODAL_TITLE: &'static str = "modal_title";
pub static CSS_MODAL_BODY: &'static str = "modal_body";
pub static CSS_STATE_GROW: &'static str = "grow";
pub static CSS_BUTTON: &'static str = "g_button";
pub static CSS_BUTTON_ICON: &'static str = "g_button_icon";
pub static CSS_BUTTON_ICON_TEXT: &'static str = "g_button_icon_text";
pub static CSS_BUTTON_TEXT: &'static str = "g_button_text";
pub static CSS_STATE_PRESSED: &'static str = "pressed";
pub static CSS_FORM_BUTTONBOX: &'static str = "g_form_buttonbox";
pub static CSS_ERROR: &'static str = "g_error";
pub static CSS_STATE_INVALID: &'static str = "invalid";
pub static CSS_STATE_DELETED: &'static str = "deleted";

pub fn el_err_span(text: impl AsRef<str>) -> El {
    return el("span").classes(&[CSS_ERROR]).text(text.as_ref());
}

pub fn el_err_block(text: impl AsRef<str>) -> El {
    return el("div").classes(&[CSS_ERROR]).text(text.as_ref());
}

pub fn el_hscroll(child: El) -> El {
    return el("div").classes(&[CSS_HSCROLL]).push(child);
}

pub fn el_group() -> El {
    return el("div").classes(&[CSS_GROUP]);
}

pub fn el_stack() -> El {
    return el("div").classes(&[CSS_STACK]);
}

pub fn el_icon(icon: CssIcon) -> El {
    return el("div").classes(&[CSS_ICON]).text(icon.0);
}

pub fn el_button_text(
    pc: &mut ProcessingContext,
    text: &str,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_TEXT]).text(text).on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| f(pc))
    });
}

pub fn el_button_icon_blank(
    pc: &mut ProcessingContext,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_ICON]).on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| f(pc))
    });
}

pub fn el_button_icon(
    pc: &mut ProcessingContext,
    icon: El,
    help: &str,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_ICON]).push(icon).attr("title", help).on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| f(pc))
    });
}

pub fn el_button_icon_toggle_auto(
    pc: &mut ProcessingContext,
    icon: CssIcon,
    help: &str,
    state: &HistPrim<bool>,
) -> El {
    return el("button")
        .classes(&[CSS_BUTTON, CSS_BUTTON_ICON])
        .push(el_icon(icon))
        .attr("title", help)
        .on("click", {
            let eg = pc.eg();
            let state = state.clone();
            move |ev| eg.event(|pc| {
                let new_state = !state.get();
                state.set(pc, new_state);
                ev
                    .target()
                    .unwrap()
                    .dyn_into::<HtmlElement>()
                    .unwrap()
                    .class_list()
                    .toggle_with_force(&CSS_STATE_PRESSED, new_state)
                    .unwrap();
            })
        });
}

pub fn el_button_icon_switch(
    pc: &mut ProcessingContext,
    off_icon: CssIcon,
    off_help: &str,
    on_icon: CssIcon,
    on_help: &str,
    state: &HistPrim<bool>,
) -> El {
    return el("button")
        .classes(&[CSS_BUTTON, CSS_BUTTON_ICON])
        .own(
            |e| link!(
                (_pc = pc),
                (state = state.clone()),
                (),
                (
                    e = e.weak(),
                    off_icon = off_icon,
                    off_help = off_help.to_string(),
                    on_icon = on_icon,
                    on_help = on_help.to_string()
                ) {
                    let e = e.upgrade()?;
                    if *state.borrow() {
                        e.ref_clear().ref_push(el_icon(*on_icon));
                        e.ref_attr("title", on_help);
                    } else {
                        e.ref_clear().ref_push(el_icon(*off_icon));
                        e.ref_attr("title", off_help);
                    }
                }
            ),
        );
}

pub fn el_button_icon_switch_auto(
    pc: &mut ProcessingContext,
    off_icon: CssIcon,
    off_help: &str,
    on_icon: CssIcon,
    on_help: &str,
    state: &HistPrim<bool>,
) -> El {
    return el_button_icon_switch(pc, off_icon, off_help, on_icon, on_help, state).on("click", {
        let eg = pc.eg();
        let state = state.clone();
        move |_| eg.event(|pc| {
            let new_value = !*state.borrow();
            state.set(pc, new_value);
        })
    });
}

pub fn el_button_icon_text(
    pc: &mut ProcessingContext,
    icon: CssIcon,
    text: &str,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button")
        .push(el_icon(icon))
        .push(el("span").text(text))
        .classes(&[CSS_BUTTON, CSS_BUTTON_ICON_TEXT])
        .on("click", {
            let eg = pc.eg();
            move |_| eg.event(|pc| f(pc))
        });
}

pub fn el_hbox() -> El {
    return el("div").classes(&[CSS_HBOX]);
}

pub fn el_vbox() -> El {
    return el("div").classes(&[CSS_VBOX]);
}

pub fn el_spacer() -> El {
    return el("div").classes(&[CSS_SPACER]);
}

pub fn log(x: impl Display) {
    web_sys::console::log_1(&JsValue::from_str(&x.to_string()));
}

pub fn log_js(x: impl Display, v: &JsValue) {
    web_sys::console::log_2(&JsValue::from_str(&x.to_string()), v);
}

pub fn log_js2(x: impl Display, v: &JsValue, v2: &JsValue) {
    web_sys::console::log_3(&JsValue::from_str(&x.to_string()), v, v2);
}

pub fn el_async() -> El {
    return el("video")
        .attr("autoplay", "true")
        .attr("loop", "true")
        .attr("playsinline", "true")
        .attr("src", "static/spinner.webm")
        .classes(&["g_async"]);
}

pub fn el_modal(
    pc: &mut ProcessingContext,
    title: &str,
    body: impl Fn(&mut ProcessingContext, WeakEl) -> Vec<El>,
) -> El {
    let root = el_stack().classes(&[CSS_MODAL]);
    root.ref_extend(vec![
        //. .
        el("div").classes(&[CSS_MODAL_BG]),
        el_vbox().classes(&[CSS_MODAL_CONTENT]).extend(vec![
            //. .
            el_hbox().classes(&[CSS_MODAL_TITLE]).extend(vec![
                //. .
                el("h1").text(title),
                el_button_icon(pc, el_icon(ICON_CLOSE), "Close", {
                    let out = root.weak();
                    move |_pc| {
                        let Some(out) = out.upgrade() else {
                            return;
                        };
                        out.ref_replace(vec![]);
                    }
                })
            ]),
            el_vbox().classes(&[CSS_MODAL_BODY]).extend(body(pc, root.weak()))
        ])
    ]);
    root
}

pub async fn async_event(e: &EventTarget, event: &str) -> Event {
    let (tx, rx) = channel();
    let _l = EventListener::once(e, event.to_string(), move |ev| {
        _ = tx.send(ev.clone());
    });
    return rx.await.unwrap();
}

pub fn el_video(src: &str) -> El {
    return el("video").attr("preload", "metadata").push(el("source").attr("src", src));
}

pub fn el_audio(src: &str) -> El {
    return el("audio").attr("preload", "metadata").attr("src", src);
}
