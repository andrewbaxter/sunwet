use reqwasm::http::Request;
use serde::de::DeserializeOwned;
use shared::model::{
    C2SReq,
    FileGenerated,
    FileHash,
    FileUrlQuery,
};
use wasm_bindgen::UnwrapThrowExt;

pub async fn send_req(req: Request) -> Result<Vec<u8>, String> {
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    let status = resp.status();
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

pub async fn req_post(origin: &str, req: C2SReq) -> Result<Vec<u8>, String> {
    let res =
        send_req(
            Request::post(&format!("{}/api", origin)).body(serde_json::to_string(&req).unwrap_throw()),
        ).await?;
    return Ok(res);
}

pub async fn req_post_json<R: DeserializeOwned>(origin: &str, req: C2SReq) -> Result<R, String> {
    let res =
        send_req(
            Request::post(&format!("{}/api", origin))
                .header("Content-type", "application/json")
                .body(serde_json::to_string(&req).unwrap_throw()),
        ).await?;
    return Ok(
        serde_json::from_slice::<R>(
            &res,
        ).map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&res)),
        )?,
    );
}

pub fn file_url(origin: &String, hash: &FileHash) -> String {
    return format!("{}/file/{}", origin, hash.to_string());
}

pub fn generated_file_url(origin: &String, hash: &FileHash, generation: &str, mime: &str) -> String {
    return format!(
        "{}/file/{}?{}",
        origin,
        hash.to_string(),
        serde_urlencoded::to_string(&FileUrlQuery { generated: Some(FileGenerated {
            name: generation.to_string(),
            mime: mime.to_string(),
        }) }).unwrap()
    );
}
