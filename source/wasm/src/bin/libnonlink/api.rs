use {
    super::{
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
    js_sys::Uint8Array,
    reqwasm::http::{
        Request,
        Response,
    },
    shared::interface::{
        triple::FileHash,
        wire::{
            C2SReq,
            C2SReqTrait,
            HEADER_OFFSET,
        },
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
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
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
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
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

async fn read_resp(base_url: &str, resp: Response) -> Result<Vec<u8>, String> {
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
    return Ok(body);
}

async fn post(base_url: &str, req: Request) -> Result<Vec<u8>, String> {
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    return read_resp(base_url, resp).await;
}

pub async fn req_post_json<T: C2SReqTrait>(base_url: &str, req: T) -> Result<T::Resp, String> {
    let req =
        Request::post(&format!("{}api", base_url))
            .header("Content-type", "application/json")
            .body(serde_json::to_string(&C2SReq::from(req.into())).unwrap_throw());
    let body = post(base_url, req).await?;
    return Ok(
        serde_json::from_slice::<T::Resp>(
            &body,
        ).map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&body)),
        )?,
    );
}

pub async fn file_post_json(base_url: &str, hash: &FileHash, chunk_start: u64, body: &[u8]) -> Result<(), String> {
    let req =
        Request::post(&format!("{}file/{}", base_url, hash.to_string()))
            .header(HEADER_OFFSET, &chunk_start.to_string())
            .body(Uint8Array::from(body));
    post(base_url, req).await?;
    return Ok(());
}

pub async fn req_file(base_url: &str, url: &str) -> Result<Vec<u8>, String> {
    let resp = match Request::get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    return read_resp(base_url, resp).await;
}
