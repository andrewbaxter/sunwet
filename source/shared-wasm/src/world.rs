use {
    crate::log::Log,
    flowcontrol::shed,
    gloo::utils::window,
    std::rc::Rc,
    wasm_bindgen::{
        JsValue,
        UnwrapThrowExt,
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
