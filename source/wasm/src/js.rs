use {
    crate::world::{
        file_url,
        generated_file_url,
    },
    chrono::{
        DateTime,
        Utc,
    },
    flowcontrol::shed,
    futures::channel::oneshot::channel,
    gloo::{
        events::EventListener,
        storage::errors::StorageError,
        timers::future::TimeoutFuture,
        utils::{
            document,
            window,
        },
    },
    js_sys::{
        Array,
        JSON,
        Object,
    },
    rooting::{
        El,
        ScopeValue,
        scope_any,
        spawn_rooted,
    },
    shared::interface::{
        triple::FileHash,
        wire::{
            GENTYPE_VTT,
            gentype_transcode,
            gentype_vtt_subpath,
        },
    },
    std::{
        cell::RefCell,
        future::Future,
        rc::Rc,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        UnwrapThrowExt,
        prelude::Closure,
    },
    wasm_bindgen_futures::{
        JsFuture,
        spawn_local,
    },
    web_sys::{
        Blob,
        BlobPropertyBag,
        ClipboardItem,
        Element,
        Event,
        EventTarget,
        HtmlElement,
        IntersectionObserver,
        IntersectionObserverEntry,
        IntersectionObserverInit,
        Url,
    },
};

// Since bug detection isn't a thing, or rather I don't want to deal with that
#[derive(Clone, PartialEq, Eq)]
pub enum Engine {
    IosSafari,
    Chrome,
}

#[derive(Clone)]
pub struct Lang {
    // Lang as it comes from navigator
    pub nav_lang: String,
}

#[derive(Clone)]
pub struct Env {
    // Ends with `/`
    pub base_url: String,
    pub engine: Option<Engine>,
    // Languages as they come from the navigator
    pub languages: Vec<String>,
    pub pwa: bool,
}

pub fn scan_env(log: &Rc<dyn Log>) -> Env {
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
                        log.log_js("Error getting user agent to enable ios workarounds", &e);
                        break;
                    },
                };
                if user_agent.contains("iPad") || user_agent.contains("iPhone") || user_agent.contains("iPod") {
                    log.log("Detected mobile ios, activating webkit workarounds.");
                    break 'found Some(Engine::IosSafari);
                }
            }
            if js_sys::Reflect::has(&window(), &JsValue::from("chrome")).unwrap() {
                log.log("Detected chrome(ish), activating chrome workarounds.");
                break 'found Some(Engine::Chrome);
            }
            break None;
        },
        languages: shed!{
            let mut out = vec![];
            for nav_lang in window().navigator().languages() {
                let nav_lang = nav_lang.as_string().unwrap();
                out.push(nav_lang);
            }
            break out;
        },
        pwa: {
            // Needs to match manifest
            let pwa = match window().match_media("(display-mode: standalone)") {
                Ok(v) => if let Some(v) = v {
                    v.matches()
                } else {
                    false
                },
                Err(e) => {
                    log.log_js("Error running media query to determine if PWA", &e);
                    false
                },
            };
            log.log(&format!("Detected pwa, activating (safari?) pwa workarounds: {}", pwa));
            pwa
        },
    }
}

pub fn env_preferred_audio_gentype(env: &Env) -> Option<String> {
    if env.engine == Some(Engine::IosSafari) {
        return Some(gentype_transcode("audio/aac"));
    } else {
        return None;
    }
}

pub fn env_preferred_audio_url(env: &Env, hash: &FileHash) -> String {
    if let Some(gentype) = env_preferred_audio_gentype(env) {
        return generated_file_url(env, hash, &gentype, "");
    } else {
        return file_url(env, hash);
    }
}

pub fn env_preferred_video_gentype() -> String {
    return gentype_transcode("video/webm");
}

pub fn env_preferred_video_url(env: &Env, hash: &FileHash) -> String {
    return generated_file_url(env, hash, &env_preferred_video_gentype(), "");
}

pub fn gen_video_subtitle_subpath(nav_lang: &str) -> String {
    let short_lang = if let Some((l, _)) = nav_lang.split_once("-") {
        l
    } else {
        nav_lang
    };
    let vtt_lang = match short_lang {
        "en" => "eng",
        "jp" => "jpn",
        x => x,
    };
    return gentype_vtt_subpath(vtt_lang);
}

pub fn env_video_subtitle_url(env: &Env, nav_lang: &str, hash: &FileHash) -> String {
    return generated_file_url(env, hash, GENTYPE_VTT, &gen_video_subtitle_subpath(nav_lang));
}

pub trait Log {
    fn log(&self, x: &str);
    fn log_js(&self, x: &str, v: &JsValue);
    fn log_js2(&self, x: &str, v: &JsValue, v2: &JsValue);
}

pub struct VecLog {
    pub log: RefCell<Vec<(DateTime<Utc>, String)>>,
}

fn trim_vec_log(log: &mut Vec<(DateTime<Utc>, String)>) {
    if log.len() > 250 {
        *log = log.split_off(log.len() - 200);
    }
}

impl Log for VecLog {
    fn log(&self, x: &str) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), x.to_string()));
        trim_vec_log(&mut log);
        web_sys::console::log_1(&JsValue::from(x));
    }

    fn log_js(&self, x: &str, v: &JsValue) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), format!("{}: {}", x, JSON::stringify(v).unwrap())));
        trim_vec_log(&mut log);
        web_sys::console::log_2(&JsValue::from(x), v);
    }

    fn log_js2(&self, x: &str, v: &JsValue, v2: &JsValue) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), format!("{}: {}, {}", x, JSON::stringify(v).unwrap(), JSON::stringify(v2).unwrap())));
        trim_vec_log(&mut log);
        web_sys::console::log_3(&JsValue::from(x), v, v2);
    }
}

pub struct ConsoleLog {}

impl Log for ConsoleLog {
    fn log(&self, x: &str) {
        web_sys::console::log_1(&JsValue::from(x));
    }

    fn log_js(&self, x: &str, v: &JsValue) {
        web_sys::console::log_2(&JsValue::from(x), v);
    }

    fn log_js2(&self, x: &str, v: &JsValue, v2: &JsValue) {
        web_sys::console::log_3(&JsValue::from(x), v, v2);
    }
}

struct MyIntersectionObserver_ {
    _root_cb: ScopeValue,
    o: IntersectionObserver,
}

impl Drop for MyIntersectionObserver_ {
    fn drop(&mut self) {
        self.o.disconnect();
    }
}

pub struct MyIntersectionObserver(Rc<MyIntersectionObserver_>);

impl MyIntersectionObserver {
    pub fn new(threshold: f64, mut cb: impl 'static + FnMut(Vec<IntersectionObserverEntry>)) -> Self {
        let scroll_observer_cb = Closure::new(move |entries: Array| {
            let entries =
                entries
                    .into_iter()
                    .map(|x| x.dyn_into::<IntersectionObserverEntry>().unwrap())
                    .collect::<Vec<_>>();
            cb(entries);
        });
        let scroll_observer = IntersectionObserver::new_with_options(scroll_observer_cb.as_ref().unchecked_ref(), &{
            let o = IntersectionObserverInit::new();
            o.set_threshold(&JsValue::from(threshold));
            o
        }).unwrap();
        return Self(Rc::new(MyIntersectionObserver_ {
            _root_cb: scope_any(scroll_observer_cb),
            o: scroll_observer,
        }));
    }
}

impl MyIntersectionObserver {
    pub fn observe(&self, e: &Element) {
        self.0.o.observe(e);
    }
}

pub async fn async_event(e: &EventTarget, event: &str) -> Event {
    let (tx, rx) = channel();
    let _l = EventListener::once(e, event.to_string(), move |ev| {
        _ = tx.send(ev.clone());
    });
    return rx.await.unwrap();
}

pub fn get_dom_octothorpe(log: &Rc<dyn Log>) -> Option<String> {
    let hash = window().location().hash().unwrap();
    let Some(s) = hash.strip_prefix("#") else {
        return None;
    };
    let s = match urlencoding::decode(s) {
        Ok(s) => s,
        Err(e) => {
            log.log(&format!("Unable to url-decode anchor state: {:?}\nAnchor: {}", e, s));
            return None;
        },
    };
    return Some(s.to_string());
}

pub mod style_export {
    use {
        gloo::utils::format::JsValueSerdeExt,
        rooting::{
            El,
            el_from_raw,
        },
        serde::{
            Deserialize,
            Serialize,
        },
        shared::interface::config::view::{
            Direction,
            Orientation,
            TextSizeMode,
            TransAlign,
        },
        std::collections::HashMap,
        wasm_bindgen::{
            JsCast,
            JsValue,
        },
        web_sys::{
            Element,
            HtmlInputElement,
            HtmlSelectElement,
            console,
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

    impl JsExport for El {
        fn from_js(v: &JsValue) -> Self {
            return el_from_raw(v.dyn_ref::<Element>().unwrap().clone());
        }

        fn to_js(&self) -> JsValue {
            return JsValue::from(self.raw());
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

    // More html non-orthogonality
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum OrientationType {
        Grid,
        Flex,
    }

    impl JsExport for OrientationType {
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

    impl JsExport for TextSizeMode {
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

pub fn el_async<E: ToString, F: 'static + Future<Output = Result<Vec<El>, E>>>(f: F) -> El {
    return el_async_(false, f);
}

pub fn el_async_<E: ToString, F: 'static + Future<Output = Result<Vec<El>, E>>>(in_root: bool, f: F) -> El {
    let out = style_export::leaf_async_block(style_export::LeafAsyncBlockArgs { in_root: in_root }).root;
    out.ref_own(|_| spawn_rooted({
        let out = out.weak();
        async move {
            // To ensure this doesn't happen synchronously with caller, so the caller can root
            // the element before it gets replaced (i.e. view, with non-query data)
            TimeoutFuture::new(0).await;
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
                    new_el = vec![style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                        in_root: in_root,
                        data: e.to_string(),
                    }).root];
                },
            }
            out.ref_replace(new_el);
        }
    }));
    return out;
}

pub fn lazy_el_async<E: ToString, F: 'static + AsyncFnOnce() -> Result<Vec<El>, E>>(f: F) -> El {
    let out = style_export::leaf_async_block(style_export::LeafAsyncBlockArgs { in_root: false }).root;
    let data = Rc::new(RefCell::new(scope_any(())));
    let o = MyIntersectionObserver::new(0., {
        let data = Rc::downgrade(&data);
        let out = out.weak();
        let mut f = Some(f);
        move |entries| {
            let Some(data) = data.upgrade() else {
                return;
            };
            for e in entries {
                if e.intersection_ratio() >= 0.5 {
                    *data.borrow_mut() = spawn_rooted({
                        let Some(f) = f.take() else {
                            return;
                        };
                        let out = out.clone();
                        async move {
                            // To ensure this doesn't happen synchronously with caller, so the caller can root
                            // the element before it gets replaced (i.e. view, with non-query data)
                            TimeoutFuture::new(0).await;
                            let res = f().await;
                            let Some(out) = out.upgrade() else {
                                return;
                            };
                            let new_el;
                            match res {
                                Ok(v) => {
                                    new_el = v;
                                },
                                Err(e) => {
                                    new_el = vec![style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                        in_root: false,
                                        data: e.to_string(),
                                    }).root];
                                },
                            }
                            out.ref_replace(new_el);
                        }
                    });
                }
            }
        }
    });
    o.observe(&out.raw());
    *data.borrow_mut() = scope_any(o);
    out.ref_own(|_| data);
    return out;
}

pub trait LogJsErr {
    fn log(self, log: &Rc<dyn Log>, msg: &str);
}

impl<T> LogJsErr for Result<T, JsValue> {
    fn log(self, log: &Rc<dyn Log>, msg: &str) {
        match self {
            Ok(_) => { },
            Err(e) => {
                log.log_js(&format!("Warning: {}:", msg), &e);
            },
        }
    }
}

impl<T> LogJsErr for Result<T, StorageError> {
    fn log(self, log: &Rc<dyn Log>, msg: &str) {
        match self {
            Ok(_) => { },
            Err(e) => {
                log.log(&format!("Warning: {}: {}", msg, e));
            },
        }
    }
}

pub trait ElExt {
    fn html(&self) -> HtmlElement;
}

impl ElExt for El {
    fn html(&self) -> HtmlElement {
        return self.raw().dyn_into::<HtmlElement>().unwrap();
    }
}

static CLIPBOARD_MIME: &str = "text/plain";

pub fn as_blob(data: impl serde::Serialize) -> Blob {
    return Blob::new_with_str_sequence_and_options(&JsValue::from(vec![
        //. .
        JsValue::from(serde_json::to_string_pretty(&data).unwrap())
    ]), &{
        let p = BlobPropertyBag::new();
        p.set_type(CLIPBOARD_MIME);
        p
    }).unwrap();
}

pub fn copy(log: &Rc<dyn Log>, data: impl serde::Serialize) {
    let a = window().navigator().clipboard().write(&JsValue::from(vec![
        //. .
        ClipboardItem::new_with_record_from_str_to_str_promise(&Object::from_entries(&JsValue::from(vec![
            //. .
            JsValue::from(vec![
                //. .
                JsValue::from(CLIPBOARD_MIME),
                JsValue::from(as_blob(data))
            ])
        ])).unwrap()).unwrap()
    ]));
    spawn_local({
        let log = log.clone();
        async move {
            JsFuture::from(a).await.log(&log, "Error copying");
        }
    });
}

pub fn download(filename: String, data: impl serde::Serialize) {
    let document = document();
    let body = document.body().unwrap();
    let a = document.create_element("a").unwrap().dyn_into::<HtmlElement>().unwrap();
    let url = Url::create_object_url_with_blob(&as_blob(data)).unwrap();
    a.set_attribute("href", &url).unwrap();
    a.set_attribute("download", &filename).unwrap();
    body.append_child(&a).unwrap();
    a.click();
    body.remove_child(&a).unwrap();
    Url::revoke_object_url(&url).unwrap();
}

pub fn on_thinking(e: &El, f: impl 'static + Clone + AsyncFn() -> ()) {
    e.ref_on("click", {
        let e = e.weak();
        let bg = Rc::new(RefCell::new(None));
        move |_| {
            if bg.borrow().is_some() {
                return;
            }
            if let Some(e) = e.upgrade() {
                e.ref_classes(&[&style_export::class_state_thinking().value]);
            }
            *bg.borrow_mut() = Some(spawn_rooted({
                let f = f.clone();
                let e = e.clone();
                let bg = bg.clone();
                async move {
                    f().await;
                    if let Some(e) = e.upgrade() {
                        e.ref_remove_classes(&[&style_export::class_state_thinking().value]);
                    }
                    *bg.borrow_mut() = None;
                }
            }));
        }
    });
}

pub fn jsstr(v: JsValue) -> String {
    return match v.dyn_ref::<Object>() {
        Some(v) => v.to_string().as_string(),
        None => v.js_typeof().as_string(),
    }.unwrap();
}
