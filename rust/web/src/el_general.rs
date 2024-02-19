use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    panic,
    rc::Rc,
    str::FromStr,
};
use gloo::{
    console::{
        log,
        warn,
    },
    utils::{
        document,
        window,
    },
};
use js_sys::Function;
use lunk::{
    link,
    EventGraph,
    HistPrim,
    Prim,
    ProcessingContext,
};
use reqwasm::http::Request;
use rooting::{
    el,
    set_root,
    spawn_rooted,
    El,
};
use rooting_forms::{
    BigString,
    Form,
};
use serde::de::DeserializeOwned;
use shared::{
    model::{
        C2SReq,
        FileHash,
        Node,
        Query,
    },
    unenum,
};
use wasm_bindgen::{
    closure::Closure,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use web_sys::{
    console::log_1,
    HtmlAudioElement,
    HtmlMediaElement,
    MediaMetadata,
    MediaSession,
};
use crate::util::CssIcon;

pub static CSS_BUTTON: &'static str = "g_button";
pub static CSS_BUTTON_ICON: &'static str = "g_button_icon";
pub static CSS_BUTTON_ICON_TEXT: &'static str = "g_button_icon_text";
pub static CSS_BUTTON_TEXT: &'static str = "g_button_text";

pub fn el_group() -> El {
    return el("div").classes(&["g_group"]);
}

pub fn el_stack() -> El {
    return el("div").classes(&["g_stack"]);
}

pub fn el_icon(icon: CssIcon, help: &str) -> El {
    return el("div").classes(&["g_icon"]).attr("title", help).text(icon.0);
}

pub fn el_button_text(
    pc: &mut ProcessingContext,
    text: &str,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_TEXT]).on("click", {
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
    icon: CssIcon,
    help: &str,
    mut f: impl 'static + FnMut(&mut ProcessingContext) -> (),
) -> El {
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_ICON]).push(el_icon(icon, help)).on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| f(pc))
    });
}

pub fn el_button_icon_switch(
    pc: &mut ProcessingContext,
    off_icon: CssIcon,
    off_help: &str,
    on_icon: CssIcon,
    on_help: &str,
    state: &Prim<bool>,
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
                        e.ref_clear().ref_push(el_icon(*on_icon, on_help));
                    } else {
                        e.ref_clear().ref_push(el_icon(*off_icon, off_help));
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
    state: &Prim<bool>,
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
    return el("button").classes(&[CSS_BUTTON, CSS_BUTTON_ICON_TEXT]).on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| f(pc))
    });
}

pub fn el_hbox() -> El {
    return el("div").classes(&["g_hbox"]);
}

pub fn el_vbox() -> El {
    return el("div").classes(&["g_vbox"]);
}

pub fn log(x: impl Display) {
    log_1(&JsValue::from_str(&x.to_string()));
}
