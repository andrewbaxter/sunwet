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
    std::time::Duration,
    tokio::time::sleep,
    wasm::js::{
        LogJsErr,
    },
    wasm_bindgen::UnwrapThrowExt,
    web_sys::Url,
};

pub async fn retry<T>(r: impl AsyncFn() -> Result<T, TempFinalErr>) -> Result<T, String> {
    loop {
        match r().await {
            Ok(v) => return Ok(v),
            Err(e) => match e {
                TempFinalErr::Temp(e) => {
                    state().log.log(&format!("Error making request: {}", e));
                    sleep(Duration::from_secs(1)).await;
                },
                TempFinalErr::Final(e) => {
                    return Err(e);
                },
            },
        }
    };
}

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

enum TempFinalErr {
    Temp(String),
    Final(String),
}

async fn read_resp(resp: Response) -> Result<Vec<u8>, TempFinalErr> {
    let status = resp.status();
    if status == 401 && want_logged_in() {
        redirect_login(&state().env.base_url);
    }
    let body = match resp.binary().await {
        Err(e) => {
            return Err(
                TempFinalErr::Temp(
                    format!("Got error response, got additional error trying to read body [{}]: {}", status, e),
                ),
            );
        },
        Ok(r) => r,
    };
    if status >= 400 {
        if status >= 400 {
            if status >= 500 {
                return Err(
                    TempFinalErr::Temp(
                        format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body)),
                    ),
                );
            } else {
                return Err(
                    TempFinalErr::Final(
                        format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body)),
                    ),
                );
            }
        }
    }
    return Ok(body);
}

async fn post(req: Request) -> Result<Vec<u8>, TempFinalErr> {
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(TempFinalErr::Temp(format!("Failed to send request: {}", e)));
        },
    };
    return read_resp(resp).await;
}

pub async fn req_post_json<T: C2SReqTrait>(req: T) -> Result<T::Resp, String> {
    let req = C2SReq::from(req.into());
    return retry(async || {
        let req =
            Request::post(&format!("{}api", state().env.base_url))
                .header("Content-type", "application/json")
                .body(serde_json::to_string(&req).unwrap_throw());
        let body = post(req).await?;
        return Ok(
            serde_json::from_slice::<T::Resp>(
                &body,
            ).map_err(
                |e| TempFinalErr::Temp(
                    format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&body)),
                ),
            )?,
        );
    }).await;
}

pub async fn file_post_json(hash: &FileHash, chunk_start: u64, body: &[u8]) -> Result<(), String> {
    return retry(async || {
        let req =
            Request::post(&format!("{}file/{}", state().env.base_url, hash.to_string()))
                .header(HEADER_OFFSET, &chunk_start.to_string())
                .body(Uint8Array::from(body));
        post(req).await?;
        return Ok(());
    }).await;
}

pub async fn req_file(url: &str) -> Result<Vec<u8>, String> {
    return retry(async || {
        let resp = match Request::get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Err(TempFinalErr::Temp(format!("Failed to send request: {}", e)));
            },
        };
        return read_resp(resp).await;
    }).await;
}
