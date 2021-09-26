use bytes::BytesMut;
use gotham::hyper::http::{header, Response, StatusCode};
use gotham::hyper::Body;
use headers::{ContentType, HeaderMapExt};
use serde_json::Value;

use crate::image::ImageFormat;

/// A standard JSON response with the content type set to application/json
pub fn json_response(status: StatusCode, data: Option<Value>) -> Response<Body> {
    let payload = json!({
        "status": status.as_u16(),
        "data": data,
    });

    let mut resp = Response::builder()
        .status(status)
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    resp.headers_mut().typed_insert(ContentType::json());

    resp
}

pub fn image_response(format: ImageFormat, data: BytesMut) -> Response<Body> {
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(data.to_vec()))
        .unwrap();

    let content_type = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Gif => "image/gif",
        ImageFormat::WebP => "image/webp",
    };

    resp.headers_mut()
        .insert(header::CONTENT_TYPE, content_type.parse().unwrap());

    resp
}

pub fn empty_response(status: StatusCode) -> Response<Body> {
    let mut resp = Response::builder()
        .status(status)
        .body(Body::from(Vec::new()))
        .unwrap();

    resp.headers_mut().typed_insert(ContentType::text_utf8());

    resp
}
