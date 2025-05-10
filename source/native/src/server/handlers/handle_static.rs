use {
    http::Response,
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

pub async fn handle_static(path: &str) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    #[derive(RustEmbed)]
    #[folder = "$STATIC_DIR"]
    struct Static;

    let mut f = Static::get(path);
    if f.is_none() {
        f = Static::get("index.html");
    }
    match f {
        Some(f) => {
            return Ok(
                Response::builder()
                    .status(200)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-type", f.metadata.mimetype())
                    .header("Cross-Origin-Embedder-Policy", "require-corp")
                    .header("Cross-Origin-Opener-Policy", "same-origin")
                    .body(body_full(f.data.to_vec()))
                    .unwrap(),
            );
        },
        None => {
            return Ok(response_404());
        },
    }
}
