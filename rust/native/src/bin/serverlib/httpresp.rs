use http::Response;
use http_body_util::{
    combinators::BoxBody,
    BodyExt,
};
use hyper::body::Bytes;
use serde::Serialize;

pub fn body_empty() -> BoxBody<Bytes, std::io::Error> {
    return http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed();
}

pub fn body_full(data: Vec<u8>) -> BoxBody<Bytes, std::io::Error> {
    return http_body_util::Full::new(Bytes::from(data)).map_err(|_| std::io::Error::other("")).boxed();
}

pub fn body_json(data: impl Serialize) -> BoxBody<Bytes, std::io::Error> {
    return body_full(serde_json::to_vec(&data).unwrap());
}

pub fn response_400(message: impl ToString) -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(400).body(body_full(message.to_string().as_bytes().to_vec())).unwrap();
}

pub fn response_200() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(200).body(body_empty()).unwrap();
}

pub fn response_200_json(v: impl Serialize) -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(200).body(body_json(v)).unwrap();
}

pub fn response_404() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(404).body(body_empty()).unwrap();
}

pub fn response_401() -> Response<BoxBody<Bytes, std::io::Error>> {
    return Response::builder().status(401).body(body_empty()).unwrap();
}
