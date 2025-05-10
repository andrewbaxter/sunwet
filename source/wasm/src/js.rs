use {
    flowcontrol::shed,
    futures::channel::oneshot::channel,
    gloo::{
        events::EventListener,
        storage::errors::StorageError,
        utils::window,
    },
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
    },
    shared::interface::wire::{
        file_derivation_mime,
        FileUrlQuery,
        VttLang,
    },
    std::{
        fmt::Display,
        future::Future,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        UnwrapThrowExt,
    },
    web_sys::{
        console::{
            log_1,
            log_2,
        },
        Event,
        EventTarget,
    },
};

// Since bug detection isn't a thing, or rather I don't want to deal with that
#[derive(Clone, PartialEq, Eq, Copy)]
pub enum Engine {
    IosSafari,
    Chrome,
}

#[derive(Clone)]
pub struct Lang {
    // Lang as it comes from navigator
    pub nav_lang: String,
    // Lang name for vtt subtitle selection
    pub vtt_lang: VttLang,
}

#[derive(Clone)]
pub struct Env {
    // Ends with `/`
    pub base_url: String,
    pub engine: Option<Engine>,
    pub languages: Vec<Lang>,
}

pub fn scan_env() -> Env {
    return Env {
        base_url: shed!{
            let loc = window().location();
            break format!(
                "{}{}/",
                loc.origin().unwrap_throw(),
                loc.pathname().unwrap_throw().rsplit_once("/").unwrap_throw().0
            );
        },
        engine: shed!{
            'found _;
            shed!{
                let user_agent = match window().navigator().user_agent() {
                    Ok(a) => a,
                    Err(e) => {
                        log_js("Error getting user agent to enable ios workarounds", &e);
                        break;
                    },
                };
                if user_agent.contains("iPad") || user_agent.contains("iPhone") || user_agent.contains("iPod") {
                    log_1(&JsValue::from("Detected mobile ios, activating webkit workarounds."));
                    break 'found Some(Engine::IosSafari);
                }
            }
            if js_sys::Reflect::has(&window(), &JsValue::from("chrome")).unwrap() {
                log_1(&JsValue::from("Detected chrome(ish), activating chrome workarounds."));
                break 'found Some(Engine::Chrome);
            }
            break None;
        },
        languages: shed!{
            let mut out = vec![];
            for nav_lang in window().navigator().languages() {
                let nav_lang = nav_lang.as_string().unwrap();
                let short_lang = if let Some((l, _)) = nav_lang.split_once("-") {
                    l
                } else {
                    &nav_lang
                };
                let vtt_lang = match short_lang {
                    "en" => VttLang::Eng,
                    "jp" => VttLang::Jpn,
                    _ => {
                        log(format!("Unhandled subtitle translation for language {}", short_lang));
                        continue;
                    },
                };
                out.push(Lang {
                    nav_lang: nav_lang,
                    vtt_lang: vtt_lang,
                });
            }
            break out;
        },
    }
}

pub fn env_preferred_audio(env: &Env) -> FileUrlQuery {
    if env.engine == Some(Engine::IosSafari) {
        return file_derivation_mime("audio/aac".to_string());
    } else {
        return FileUrlQuery::default();
    }
}

pub fn env_preferred_video() -> FileUrlQuery {
    return file_derivation_mime("video/webm".to_string());
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

pub async fn async_event(e: &EventTarget, event: &str) -> Event {
    let (tx, rx) = channel();
    let _l = EventListener::once(e, event.to_string(), move |ev| {
        _ = tx.send(ev.clone());
    });
    return rx.await.unwrap();
}

pub fn get_dom_octothorpe() -> Option<String> {
    let hash = window().location().hash().unwrap();
    let Some(s) = hash.strip_prefix("#") else {
        return None;
    };
    let s = match urlencoding::decode(s) {
        Ok(s) => s,
        Err(e) => {
            log(format!("Unable to url-decode anchor state: {:?}\nAnchor: {}", e, s));
            return None;
        },
    };
    return Some(s.to_string());
}

pub mod style_export {
    use {
        gloo::utils::format::JsValueSerdeExt,
        shared::interface::config::view::{
            Direction,
            Orientation,
            TransAlign,
        },
        std::collections::HashMap,
        wasm_bindgen::{
            JsCast,
            JsValue,
        },
        web_sys::{
            console,
            Element,
            HtmlInputElement,
            HtmlSelectElement,
        },
    };

    pub trait JsExport {
        fn from_js(v: &JsValue) -> Self;
        fn to_js(&self) -> JsValue;
    }

    impl JsExport for JsValue {
        fn from_js(v: &JsValue) -> Self {
            return v.clone();
        }

        fn to_js(&self) -> JsValue {
            return self.clone();
        }
    }

    impl JsExport for js_sys::Promise {
        fn from_js(v: &JsValue) -> Self {
            return v.dyn_ref::<js_sys::Promise>().unwrap().clone();
        }

        fn to_js(&self) -> JsValue {
            return self.into();
        }
    }

    impl JsExport for usize {
        fn from_js(v: &JsValue) -> Self {
            return v.as_f64().unwrap() as Self;
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from_f64(*self as f64);
        }
    }

    impl JsExport for bool {
        fn from_js(v: &JsValue) -> Self {
            return v.as_bool().unwrap();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from_bool(*self);
        }
    }

    impl JsExport for String {
        fn from_js(v: &JsValue) -> Self {
            return v.as_string().unwrap();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(self);
        }
    }

    impl<'a> JsExport for &'a str {
        fn from_js(_v: &JsValue) -> Self {
            unimplemented!();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(*self);
        }
    }

    impl JsExport for Element {
        fn from_js(v: &JsValue) -> Self {
            return v.dyn_ref::<Self>().unwrap().clone();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(self);
        }
    }

    impl JsExport for HtmlInputElement {
        fn from_js(v: &JsValue) -> Self {
            return v.dyn_ref::<Self>().unwrap().clone();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(self);
        }
    }

    impl JsExport for HtmlSelectElement {
        fn from_js(v: &JsValue) -> Self {
            return v.dyn_ref::<Self>().unwrap().clone();
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(self);
        }
    }

    impl<T: JsExport> JsExport for Vec<T> {
        fn from_js(v: &JsValue) -> Self {
            let v = v.dyn_ref::<js_sys::Array>().unwrap();
            let mut out = vec![];
            for v in v.iter() {
                out.push(T::from_js(&v));
            }
            return out;
        }

        fn to_js(&self) -> JsValue {
            let v = js_sys::Array::new_with_length(self.len() as u32);
            for (i, e) in self.iter().enumerate() {
                v.set(i as u32, e.to_js());
            }
            return v.into();
        }
    }

    impl<T: JsExport> JsExport for Option<T> {
        fn from_js(v: &JsValue) -> Self {
            if v.is_undefined() || v.is_null() {
                return None;
            }
            return Some(T::from_js(v));
        }

        fn to_js(&self) -> JsValue {
            match self {
                Some(v) => {
                    return v.to_js();
                },
                None => {
                    return JsValue::undefined();
                },
            }
        }
    }

    impl JsExport for HashMap<String, String> {
        fn from_js(v: &JsValue) -> Self {
            let mut out = Self::new();
            for kv in js_sys::Object::entries(v.dyn_ref().unwrap()) {
                let mut kv = kv.dyn_into::<js_sys::Array>().unwrap().into_iter();
                let k = kv.next().unwrap();
                let v = kv.next().unwrap();
                out.insert(k.as_string().unwrap(), v.as_string().unwrap());
            }
            return out;
        }

        fn to_js(&self) -> JsValue {
            let out = js_sys::Object::new().into();
            for (k, v) in self {
                js_set(&out, k, v);
            }
            return out;
        }
    }

    impl JsExport for Direction {
        fn from_js(v: &JsValue) -> Self {
            return <JsValue as JsValueSerdeExt>::into_serde(v).unwrap();
        }

        fn to_js(&self) -> JsValue {
            return <JsValue as JsValueSerdeExt>::from_serde(self).unwrap();
        }
    }

    impl JsExport for Orientation {
        fn from_js(v: &JsValue) -> Self {
            return <JsValue as JsValueSerdeExt>::into_serde(v).unwrap();
        }

        fn to_js(&self) -> JsValue {
            return <JsValue as JsValueSerdeExt>::from_serde(self).unwrap();
        }
    }

    impl JsExport for TransAlign {
        fn from_js(v: &JsValue) -> Self {
            return <JsValue as JsValueSerdeExt>::into_serde(v).unwrap();
        }

        fn to_js(&self) -> JsValue {
            return <JsValue as JsValueSerdeExt>::from_serde(self).unwrap();
        }
    }

    pub fn js_get<T: JsExport>(o: &JsValue, p: &str) -> T {
        let v = match js_sys::Reflect::get(o, &JsValue::from(p)) {
            Ok(v) => v,
            Err(e) => {
                console::log_2(&JsValue::from(format!("js_get [{}] fail", p)), &e);
                panic!("");
            },
        };
        return T::from_js(&v);
    }

    fn js_set<T: JsExport>(o: &JsValue, p: &str, v: &T) {
        js_sys::Reflect::set(o, &JsValue::from(p), &v.to_js()).unwrap();
    }

    fn js_call(o: &JsValue, args: &js_sys::Object) -> JsValue {
        return o.dyn_ref::<js_sys::Function>().unwrap().call1(o, args).unwrap();
    }

    include!(concat!(env!("OUT_DIR"), "/style_export.rs"));
}

pub fn el_async<E: ToString, F: 'static + Future<Output = Result<El, E>>>(f: F) -> El {
    return el_async_(false, f);
}

pub fn el_async_<E: ToString, F: 'static + Future<Output = Result<El, E>>>(in_root: bool, f: F) -> El {
    let out =
        el_from_raw(
            style_export::leaf_async_block(style_export::LeafAsyncBlockArgs { in_root: in_root }).root.into(),
        );
    out.ref_own(|_| spawn_rooted({
        let out = out.weak();
        async move {
            let res = f.await;
            let Some(out) = out.upgrade() else {
                return;
            };
            let new_el;
            match res {
                Ok(v) => {
                    new_el = v;
                },
                Err(e) => {
                    new_el = el_from_raw(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                        in_root: in_root,
                        data: e.to_string(),
                    }).root.dyn_into().unwrap());
                },
            }
            out.raw().set_inner_html("");
            out.ref_push(new_el);
        }
    }));
    return out;
}

pub fn el_video(src: &str) -> El {
    return el("video").attr("preload", "metadata").push(el("source").attr("src", src));
}

pub fn el_audio(src: &str) -> El {
    return el("audio").attr("preload", "metadata").attr("src", src);
}

pub trait LogJsErr {
    fn log(self, msg: &str);
}

impl<T> LogJsErr for Result<T, JsValue> {
    fn log(self, msg: &str) {
        match self {
            Ok(_) => { },
            Err(e) => {
                log_2(&JsValue::from(format!("Warning: {}:", msg)), &e);
            },
        }
    }
}

impl<T> LogJsErr for Result<T, StorageError> {
    fn log(self, msg: &str) {
        match self {
            Ok(_) => { },
            Err(e) => {
                log_1(&JsValue::from(format!("Warning: {}: {}", msg, e)));
            },
        }
    }
}
