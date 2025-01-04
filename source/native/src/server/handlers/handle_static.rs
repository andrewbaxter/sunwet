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
    std::str::Split,
};

pub async fn handle_static(
    path_iter: Split<'_, char>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, VisErr<loga::Error>> {
    #[derive(RustEmbed)]
    #[folder = "$STATIC_DIR"]
    struct Static;

    let mut path = path_iter.collect::<Vec<&str>>();
    eprintln!("static path {:?}", path);
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
