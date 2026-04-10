use {
    crate::libnonlink::{
        ministate::{
            Ministate,
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
        },
        state::state,
    },
    gloo::{
        storage::{
            LocalStorage,
            SessionStorage,
            Storage,
        },
        utils::window,
    },
    shared::interface::{
        triple::FileHash,
        wire::C2SReqTrait,
    },
    shared_wasm::{
        api::{
            self,
            ON_401,
        },
        log::LogJsErr,
    },
    web_sys::Url,
};

pub const LOCALSTORAGE_LOGGED_IN: &str = "want_logged_in";

pub fn want_logged_in() -> bool {
    return LocalStorage::get::<bool>(LOCALSTORAGE_LOGGED_IN).is_ok();
}

pub fn set_want_logged_in() {
    LocalStorage::set(LOCALSTORAGE_LOGGED_IN, true).log(&state().log, "Error remembering logged in pref");
}

pub fn unset_want_logged_in() {
    LocalStorage::delete(LOCALSTORAGE_LOGGED_IN);
}

pub fn redirect_login(base_url: &str) -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            &*state().ministate.borrow(),
        ).log(&state().log, "Error storing post-redirect ministate");
    }
    window()
        .location()
        .set_href(
            &Url::new_with_base(&format!("oidc?url={}", urlencoding::encode(&base_url)), base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

pub fn api2_init() {
    *ON_401.lock().unwrap() = Some(|| {
        if want_logged_in() {
            redirect_login(&state().env.base_url);
        }
    });
}

pub fn redirect_logout(base_url: &str) -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            &*state().ministate.borrow(),
        ).log(&state().log, "Error storing post-redirect ministate");
    }
    window()
        .location()
        .set_href(
            &Url::new_with_base(&format!("logout?url={}", urlencoding::encode(&base_url)), base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

pub async fn req_post_json<T: C2SReqTrait>(req: T) -> Result<T::Resp, String> {
    return api::req_post_json(&state().log, &state().env.base_url, req).await;
}

pub async fn file_post_json(hash: &FileHash, chunk_start: u64, body: &[u8]) -> Result<(), String> {
    return api::file_post_json(&state().log, &state().env.base_url, hash, chunk_start, body).await;
}

pub async fn req_file(url: &str) -> Result<Vec<u8>, String> {
    return api::req_file(&state().log, url).await;
}
