use std::{
    cell::RefCell,
    rc::Rc,
};
use futures::Future;
use tokio::sync::Semaphore;
use wasm_bindgen_futures::spawn_local;

pub static CSS_GROW: &'static str = "g_grow";

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
pub static ICON_ADD: CssIcon = CssIcon("\u{e145}");
pub static ICON_REMOVE: CssIcon = CssIcon("\u{e15b}");
pub static ICON_FILL: CssIcon = CssIcon("\u{e877}");
pub static ICON_RESET: CssIcon = CssIcon("\u{e166}");
pub static ICON_SELECT_ALL: CssIcon = CssIcon("\u{e837}");
pub static ICON_SELECT_NONE: CssIcon = CssIcon("\u{e836}");

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
