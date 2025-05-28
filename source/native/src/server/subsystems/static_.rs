use {
    http::{
        header::{
            ETAG,
            IF_MATCH,
        },
        HeaderMap,
        Response,
    },
    http_body_util::combinators::BoxBody,
    htwrap::htserve::{
        responses::{
            body_full,
            response_404,
        },
        viserr::VisErr,
    },
    hyper::body::Bytes,
    rust_embed::RustEmbed,
};

pub async fn handle_static(
    headers: &HeaderMap,
    path: &str,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    #[derive(RustEmbed)]
    #[folder = "$STATIC_DIR"]
    struct Static;

    let mut f = Static::get(path);
    if f.is_none() {
        f = Static::get("index.html");
    }
    match f {
        Some(f) => {
            let etag = format!("\"{}\"", hex::encode(f.metadata.sha256_hash()));
            if let Some(h) = headers.get(IF_MATCH) {
                if h == etag.as_bytes() {
                    return Ok(Response::builder().status(304).body(body_full(vec![])).unwrap());
                }
            }
            return Ok(
                Response::builder()
                    .status(200)
                    .header("Content-type", f.metadata.mimetype())
                    .header("Cross-Origin-Embedder-Policy", "require-corp")
                    .header("Cross-Origin-Opener-Policy", "same-origin")
                    .header(ETAG, etag)
                    .body(body_full(f.data.to_vec()))
                    .unwrap(),
            );
        },
        None => {
            return Ok(response_404());
        },
    }
}
