use base64::{decode, encode};
use flate2::read::GzDecoder;
use std::io::Read;

use gotham::handler::HandlerResult;
use gotham::hyper::http::{HeaderMap, StatusCode};
use gotham::hyper::{body, Body};
use gotham::state::{FromState, State};

use crate::configure::StateConfig;
use crate::context::{CompressionMode, ImageGet, ImageRemove, ImageUpload, ImageUploaded};
use crate::image::{delete_image, get_image, process_new_image};
use crate::response::{empty_response, image_response, json_response};
use crate::PathExtractor;

const COMMA: u8 = 44;
static GZIP: &[u8] = b"gzip";

macro_rules! byte_or_return {
    ( $v:expr, $cmp:expr ) => {{
        if let Some(item) = $v {
            if item != $cmp {
                return false;
            }
        } else {
            return false;
        }
    }};
}

fn maybe_gzip(headers: &HeaderMap) -> bool {
    if let Some(v) = headers.get("Accept-Encoding") {
        let buff = v.as_bytes();
        let mut options = buff.split(|v| v == &COMMA); // analyser ignore
        while let Some(v) = options.next() {
            let mut v = v.into_iter();
            while let Some(x) = v.next() {
                if x != &GZIP[0] {
                    continue;
                };
                byte_or_return!(v.next(), &GZIP[1]);
                byte_or_return!(v.next(), &GZIP[2]);
                byte_or_return!(v.next(), &GZIP[3]);
                return true;
            }
        }
    }
    false
}

pub async fn get_file(mut state: State) -> HandlerResult {
    let path_vars = PathExtractor::take_from(&mut state);
    let params = ImageGet::take_from(&mut state);
    let config = StateConfig::take_from(&mut state);
    let headers = HeaderMap::take_from(&mut state);

    let file_id = path_vars.file_id;
    let format = params
        .format
        .unwrap_or_else(|| config.0.default_serving_format.clone());

    let mut preset = params
        .preset
        .unwrap_or_else(|| config.0.default_serving_preset.clone());

    if preset != "original" {
        // We dont want to necessarily error if you give an invalid
        // preset, but we dont want to attempt something that doesnt
        // exist.
        if !config.0.size_presets.contains_key(&preset) {
            preset = "original".into();
        }
    }

    let compression_mode = config.0.serve_compression_mode.clone();
    let compress = match compression_mode {
        CompressionMode::Always => true,
        CompressionMode::Never => false,
        CompressionMode::Auto => {
            if let Some(compress) = params.compress {
                compress
            } else {
                maybe_gzip(&headers)
            }
        }
    };

    let img = get_image(&mut state, file_id, preset, format, compress).await;

    match img {
        None => Ok((state, empty_response(StatusCode::NOT_FOUND))),
        Some(data) => {
            if params.encode.unwrap_or(false) {
                let encoded = encode(data.as_ref());
                return Ok((state, json_response(
                    StatusCode::OK,
                    Some(json!({
                            "compressed": compress,
                            "image": encoded,
                    }))
                )))
            }
            Ok((state, image_response(format, data, compress)))
        },
    }
}

pub async fn add_file(mut state: State) -> HandlerResult {
    let res = body::to_bytes(Body::take_from(&mut state)).await;
    let bod = match res {
        Ok(bod) => bod,
        Err(e) => {
            return Ok((
                state,
                json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(json!({
                        "message": format!("encountered exception: {:?}", e)
                    })),
                ),
            ))
        }
    };

    let upload: ImageUpload = match serde_json::from_slice(bod.as_ref()) {
        Ok(v) => v,
        Err(e) => {
            return Ok((
                state,
                json_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Some(json!({
                        "message":
                            format!(
                                "failed to deserialize POST body due to the following error: {:?}",
                                e
                            )
                    })),
                ),
            ))
        }
    };

    let format = upload.format;
    let compressed = upload.compressed.unwrap_or_else(|| false);
    let mut data = match decode(upload.data) {
        Ok(d) => d,
        Err(_) => {
            return Ok((
                state,
                json_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Some(json!({
                        "message": "data is not encoded in base64 format correctly",
                    })),
                ),
            ))
        }
    };

    if compressed {
        let mut decoder = GzDecoder::new(&data[..]);
        let mut out = Vec::with_capacity(data.len());
        if let Err(e) = decoder.read_to_end(&mut out) {
            return Ok((
                state,
                json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(json!({
                        "message": format!("failed to decompress data: {:?}", e),
                    })),
                ),
            ));
        }

        data = out;
    }

    let (file_id, formats) = match process_new_image(&mut state, format, data).await {
        Ok(v) => v,
        Err(e) => {
            return Ok((
                state,
                json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(json!({
                        "message": format!("failed to process image: {:?}", e),
                    })),
                ),
            ))
        }
    };

    let resp = ImageUploaded { file_id, formats };

    let resp = serde_json::to_value(resp).expect("failed to serialize uploaded stats");

    Ok((state, json_response(StatusCode::OK, Some(resp))))
}

pub async fn remove_file(mut state: State) -> HandlerResult {
    let params = ImageRemove::take_from(&mut state);

    if let Err(e) = delete_image(&mut state, params.file_id).await {
        return Ok((
            state,
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(json!({
                    "message": format!(
                        "failed to delete image with id: {} due to the following exception: {:?}",
                        params.file_id,
                        e
                    )
                })),
            ),
        ));
    };

    Ok((
        state,
        json_response(
            StatusCode::OK,
            Some(json!({
                "message": "yes"
            })),
        ),
    ))
}
