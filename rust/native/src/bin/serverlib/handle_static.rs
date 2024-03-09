use std::str::Split;
use http::Response;
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use rust_embed::RustEmbed;
use crate::serverlib::httpresp::{
    body_full,
    response_404,
};

pub async fn handle_static(
    path_iter: Split<'_, char>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    #[derive(RustEmbed)]
    #[folder= "$CARGO_MANIFEST_DIR/../../stage/static"]
    struct Static;

    let mut path = path_iter.collect::<Vec<&str>>();
    let mut f = Static::get(&path.join("/"));
    if f.is_none() {
        path.push("index.html");
        f = Static::get(&path.join("/"));
    }
    match f {
        Some(f) => {
            return Ok(
                Response::builder()
                    .status(200)
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
