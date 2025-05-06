use {
    super::{
        ministate::{
            Ministate,
            SESSIONSTORAGE_POST_REDIRECT,
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
    reqwasm::http::Request,
    shared::interface::wire::{
        C2SReq,
        C2SReqTrait,
    },
    wasm::js::LogJsErr,
    wasm_bindgen::UnwrapThrowExt,
    web_sys::Url,
};

pub const LOCALSTORAGE_LOGGED_IN: &str = "want_logged_in";

pub fn want_logged_in() -> bool {
    return LocalStorage::get::<bool>(LOCALSTORAGE_LOGGED_IN).is_ok();
}

pub fn set_want_logged_in() {
    LocalStorage::set(LOCALSTORAGE_LOGGED_IN, true).log("Error remembering logged in pref");
}

pub fn unset_want_logged_in() {
    LocalStorage::delete(LOCALSTORAGE_LOGGED_IN);
}

pub fn redirect_login(base_url: &str) -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT,
            &*state().ministate.borrow(),
        ).log("Error storing post-redirect ministate");
    }
    window()
        .location()
        .set_href(
            &Url::new_with_base(&format!("oidc?url={}", urlencoding::encode(&base_url)), base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

pub fn redirect_logout(base_url: &str) -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT,
            &*state().ministate.borrow(),
        ).log("Error storing post-redirect ministate");
    }
    window()
        .location()
        .set_href(
            &Url::new_with_base(&format!("logout?url={}", urlencoding::encode(&base_url)), base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

pub async fn req_post_json<T: C2SReqTrait>(base_url: &str, req: T) -> Result<T::Resp, String> {
    let req =
        Request::post(&format!("{}api", base_url))
            .header("Content-type", "application/json")
            .body(serde_json::to_string(&C2SReq::from(req.into())).unwrap_throw());
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    let status = resp.status();
    if status == 401 && want_logged_in() {
        redirect_login(base_url);
    }
    let body = match resp.binary().await {
        Err(e) => {
            return Err(format!("Got error response, got additional error trying to read body [{}]: {}", status, e));
        },
        Ok(r) => r,
    };
    if status >= 400 {
        return Err(format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body)));
    }
    return Ok(
        serde_json::from_slice::<T::Resp>(
            &body,
        ).map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&body)),
        )?,
    );
}
