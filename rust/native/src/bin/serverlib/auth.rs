use std::sync::Arc;
use http::{
    header::AUTHORIZATION,
    Response,
};
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use shared::model::FileHash;
use super::{
    httpresp::response_401,
    state::State,
};

pub fn check_auth(
    _state: &Arc<State>,
    parts: &http::request::Parts,
) -> Option<Response<BoxBody<Bytes, std::io::Error>>> {
    //. if parts.headers.get(AUTHORIZATION).is_none() {
    //.     return Some(response_401());
    //. }
    return None;
}

pub fn check_file_auth(
    state: &Arc<State>,
    parts: &http::request::Parts,
    file: &FileHash,
) -> Option<Response<BoxBody<Bytes, std::io::Error>>> {
    //. if state.link_public_files.contains(file) {
    //.     return None;
    //. }
    return check_auth(state, parts);
}
