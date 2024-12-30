use {
    reqwasm::http::Request,
    shared::interface::{
        triple::FileHash,
        wire::{
            C2SReq,
            C2SReqTrait,
            FileGenerated,
            FileUrlQuery,
            Resp,
        },
    },
    wasm_bindgen::UnwrapThrowExt,
};

pub async fn req_post_json<T: C2SReqTrait>(base_url: &str, req: T) -> Result<T::Resp, String> {
    let req =
        Request::post(&format!("{}/api", base_url))
            .header("Content-type", "application/json")
            .body(serde_json::to_string(&C2SReq::from(req.into())).unwrap_throw());
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
    return serde_json::from_slice::<Resp<T::Resp>>(&body)
        .map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&body)),
        )?
        .into_std();
}

pub fn file_url(origin: &String, hash: &FileHash) -> String {
    return format!("{}/file/{}", origin, hash.to_string());
}

pub fn generated_file_url(origin: &String, hash: &FileHash, generation: &str, mime: &str) -> String {
    return format!(
        "{}/file/{}?{}",
        origin,
        hash.to_string(),
        serde_json::to_string(&FileUrlQuery { generated: Some(FileGenerated {
            name: generation.to_string(),
            mime_type: mime.to_string(),
        }) }).unwrap()
    );
}
