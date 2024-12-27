use {

std::{
    cell::RefCell,
    rc::Rc,
},
futures::Future,
tokio::sync::Semaphore,
wasm_bindgen_futures::spawn_local,
};

struct BgVal_<T> {
    value: RefCell<Option<T>>,
    notify: Semaphore,
}

#[derive(Clone)]
pub struct BgVal<T>(Rc<BgVal_<T>>);

impl<T: Clone> BgVal<T> {
    pub async fn get(&self) -> T {
        if let Some(v) = &*self.0.value.borrow() {
            return v.clone();
        }
        _ = self.0.notify.acquire().await.unwrap();
        return self.0.value.borrow().as_ref().unwrap().clone();
    }
}

pub fn bg_val<T: 'static + Clone>(f: impl 'static + Future<Output = T>) -> BgVal<T> {
    let v = BgVal(Rc::new(BgVal_ {
        value: RefCell::new(None),
        notify: Semaphore::new(0),
    }));
    spawn_local({
        let v = v.clone();
        async move {
            *v.0.value.borrow_mut() = Some(f.await);
            v.0.notify.add_permits(1);
        }
    });
    return v;
}

struct WaitVal_<T> {
    value: RefCell<Option<T>>,
    notify: Semaphore,
}

#[derive(Clone)]
pub struct WaitVal<T>(Rc<WaitVal_<T>>);

impl<T: Clone> WaitVal<T> {
    pub fn new() -> Self {
        return Self(Rc::new(WaitVal_ {
            value: RefCell::new(None),
            notify: Semaphore::new(0),
        }));
    }

    pub fn set(&self, v: Option<T>) {
        match v {
            Some(v) => {
                if self.0.value.borrow().is_some() {
                    *self.0.value.borrow_mut() = Some(v);
                } else {
                    *self.0.value.borrow_mut() = Some(v);
                    self.0.notify.add_permits(1);
                }
            },
            None => {
                if self.0.value.borrow().is_some() {
                    self.0.notify.try_acquire().unwrap().forget();
                    *self.0.value.borrow_mut() = None;
                } else {
                    // nop
                }
            },
        }
    }

    pub async fn get(&self) -> T {
        if let Some(v) = &*self.0.value.borrow() {
            return v.clone();
        }
        _ = self.0.notify.acquire().await.unwrap();
        return self.0.value.borrow().as_ref().unwrap().clone();
    }
}
