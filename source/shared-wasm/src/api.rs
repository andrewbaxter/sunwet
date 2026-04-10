use {
    crate::log::Log,
    gloo::timers::future::TimeoutFuture,
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
    std::{
        rc::Rc,
        sync::Mutex,
    },
    wasm_bindgen::UnwrapThrowExt,
};

pub static ON_401: Mutex<Option<fn() -> ()>> = Mutex::new(None);

async fn retry<T>(log: &Rc<dyn Log>, r: impl AsyncFn() -> Result<T, TempFinalErr>) -> Result<T, String> {
    loop {
        match r().await {
            Ok(v) => return Ok(v),
            Err(e) => match e {
                TempFinalErr::Temp(e) => {
                    log.log(&format!("Error making request: {}", e));
                    TimeoutFuture::new(1000).await;
                },
                TempFinalErr::Final(e) => {
                    return Err(e);
                },
            },
        }
    };
}

enum TempFinalErr {
    Temp(String),
    Final(String),
}

async fn read_resp(resp: Response) -> Result<Vec<u8>, TempFinalErr> {
    let status = resp.status();
    if status == 401 {
        //. && want_logged_in() {
        //. redirect_login(&state().env.base_url);
        if let Some(on) = &*ON_401.lock().unwrap() {
            (on)();
        }
        unreachable!();
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
        if status >= 500 {
            return Err(
                TempFinalErr::Temp(format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body))),
            );
        } else {
            return Err(
                TempFinalErr::Final(format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body))),
            );
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

pub async fn req_post_json<
    T: C2SReqTrait,
>(log: &Rc<dyn Log>, base_url: &String, req: T) -> Result<T::Resp, String> {
    let req = C2SReq::from(req.into());
    return retry(log, async || {
        let req =
            Request::post(&format!("{}api", base_url))
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

pub async fn file_post_json(
    log: &Rc<dyn Log>,
    base_url: &String,
    hash: &FileHash,
    chunk_start: u64,
    body: &[u8],
) -> Result<(), String> {
    return retry(log, async || {
        let req =
            Request::post(&format!("{}file/{}", base_url, hash.to_string()))
                .header(HEADER_OFFSET, &chunk_start.to_string())
                .body(Uint8Array::from(body));
        post(req).await?;
        return Ok(());
    }).await;
}

pub async fn req_file(log: &Rc<dyn Log>, url: &str) -> Result<Vec<u8>, String> {
    return retry(log, async || {
        let resp = match Request::get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Err(TempFinalErr::Temp(format!("Failed to send request: {}", e)));
            },
        };
        return read_resp(resp).await;
    }).await;
}
