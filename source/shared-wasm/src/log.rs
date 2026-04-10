use {
    chrono::{
        DateTime,
        Utc,
    },
    gloo::storage::{
        LocalStorage,
        Storage,
        errors::StorageError,
    },
    js_sys::{
        JSON,
        Object,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
};

pub fn jsstr(v: JsValue) -> String {
    return match v.dyn_ref::<Object>() {
        Some(v) => v.to_string().as_string(),
        None => v.js_typeof().as_string(),
    }.unwrap();
}

pub trait Log {
    fn log(&self, x: &str);
    fn log_js(&self, x: &str, v: &JsValue);
    fn log_js2(&self, x: &str, v: &JsValue, v2: &JsValue);
}

pub struct VecLog {
    pub log: RefCell<Vec<(DateTime<Utc>, String)>>,
}

impl VecLog {
    pub fn new() -> VecLog {
        return VecLog { log: RefCell::new(match LocalStorage::get(PERSIST_LOGS) {
            Ok(l) => l,
            Err(_) => Default::default(),
        }) };
    }
}

fn trim_vec_log(log: &mut Vec<(DateTime<Utc>, String)>) {
    if log.len() > 250 {
        *log = log.split_off(log.len() - 200);
    }
}

const PERSIST_LOGS: &str = "logs";

impl Log for VecLog {
    fn log(&self, x: &str) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), x.to_string()));
        trim_vec_log(&mut log);
        web_sys::console::log_1(&JsValue::from(x));
        _ = LocalStorage::set(PERSIST_LOGS, &*log);
    }

    fn log_js(&self, x: &str, v: &JsValue) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), format!("{}: {}", x, JSON::stringify(v).unwrap())));
        trim_vec_log(&mut log);
        web_sys::console::log_2(&JsValue::from(x), v);
        _ = LocalStorage::set(PERSIST_LOGS, &*log);
    }

    fn log_js2(&self, x: &str, v: &JsValue, v2: &JsValue) {
        let mut log = self.log.borrow_mut();
        log.push((Utc::now(), format!("{}: {}, {}", x, JSON::stringify(v).unwrap(), JSON::stringify(v2).unwrap())));
        trim_vec_log(&mut log);
        web_sys::console::log_3(&JsValue::from(x), v, v2);
        _ = LocalStorage::set(PERSIST_LOGS, &*log);
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
