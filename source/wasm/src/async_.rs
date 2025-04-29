use {
    futures::Future,
    std::{
        sync::{
            Arc,
            Mutex,
        },
    },
    tokio::sync::Semaphore,
    wasm_bindgen_futures::spawn_local,
};

struct BgVal_<T> {
    value: Mutex<Option<T>>,
    notify: Semaphore,
}

#[derive(Clone)]
pub struct BgVal<T>(Arc<BgVal_<T>>);

impl<T: Clone> BgVal<T> {
    pub async fn get(&self) -> T {
        if let Some(v) = &*self.0.value.lock().unwrap() {
            return v.clone();
        }
        _ = self.0.notify.acquire().await.unwrap();
        return self.0.value.lock().unwrap().as_ref().unwrap().clone();
    }
}

pub fn bg_val<T: 'static + Clone>(f: impl 'static + Future<Output = T>) -> BgVal<T> {
    let v = BgVal(Arc::new(BgVal_ {
        value: Mutex::new(None),
        notify: Semaphore::new(0),
    }));
    spawn_local({
        let v = v.clone();
        async move {
            *v.0.value.lock().unwrap() = Some(f.await);
            v.0.notify.add_permits(1);
        }
    });
    return v;
}

struct WaitVal_<T> {
    value: Mutex<Option<T>>,
    notify: Semaphore,
}

#[derive(Clone)]
pub struct WaitVal<T>(Arc<WaitVal_<T>>);

impl<T: Clone> WaitVal<T> {
    pub fn new() -> Self {
        return Self(Arc::new(WaitVal_ {
            value: Mutex::new(None),
            notify: Semaphore::new(0),
        }));
    }

    pub fn set(&self, v: Option<T>) {
        match v {
            Some(v) => {
                if self.0.value.lock().unwrap().is_some() {
                    *self.0.value.lock().unwrap() = Some(v);
                } else {
                    *self.0.value.lock().unwrap() = Some(v);
                    self.0.notify.add_permits(1);
                }
            },
            None => {
                if self.0.value.lock().unwrap().is_some() {
                    self.0.notify.try_acquire().unwrap().forget();
                    *self.0.value.lock().unwrap() = None;
                } else {
                    // nop
                }
            },
        }
    }

    pub async fn get(&self) -> T {
        if let Some(v) = &*self.0.value.lock().unwrap() {
            return v.clone();
        }
        _ = self.0.notify.acquire().await.unwrap();
        return self.0.value.lock().unwrap().as_ref().unwrap().clone();
    }
}
