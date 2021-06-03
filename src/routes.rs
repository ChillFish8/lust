use base64::{decode, encode};
use log::{debug, error};

use gotham::handler::HandlerResult;
use gotham::hyper::http::StatusCode;
use gotham::hyper::{body, Body};
use gotham::state::{FromState, State};

use crate::cache::CACHE_STATE;
use crate::configure::StateConfig;
use crate::context::{FilesListPayload, FilterType, OrderBy};
use crate::image::{delete_image, get_image, process_new_image};
use crate::image::{ImageGet, ImageRemove, ImageUpload, ImageUploaded};
use crate::response::{empty_response, image_response, json_response};
use crate::storage::StorageBackend;
use crate::traits::ImageStore;
use crate::PathExtractor;

macro_rules! from_body {
    ( $e:expr ) => {{
        let res = body::to_bytes(Body::take_from(&mut $e)).await;
        let bod = match res {
            Ok(bod) => bod,
            Err(e) => {
                error!("failed to read data from body {:?}", &e);
                return Ok((
                    $e,
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Some(json!({
                            "message": format!("encountered exception: {:?}", e)
                        })),
                    ),
                ));
            }
        };

        match serde_json::from_slice(bod.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Ok((
                    $e,
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
        }
    }};
}

/// Gets a given image from the storage backend with the given
/// preset and format if it does not already exist in cache.
///
/// This endpoint can return any of the following status codes:
///
/// 404:
///     The image does not exist, NOTE: This endpoint will **always**
///     return a 404 if an unexpected error was encountered rather than
///     raising an error to the requester, instead it will be logged in
///     the console.
///
/// 200:
///     The image was successfully fetched and sent as the response.
///
/// TODO:
///     Likely performance issues could become apparent at higher
///     concurrency due to the Mutex on the LRU cache, although this
///     is probably insignificant compared to the time spent on IO.
pub async fn get_file(mut state: State) -> HandlerResult {
    let path_vars = PathExtractor::take_from(&mut state);
    let params = ImageGet::take_from(&mut state);
    let config = StateConfig::take_from(&mut state);

    let file_id = path_vars.file_id;
    let category = path_vars.category.unwrap_or_else(|| "default".to_string());

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

    let cache = CACHE_STATE.get().expect("not initialised");
    let img = if let Some(cached) = cache.get(file_id, preset.clone(), format) {
        debug!(
            "using cached version of image for file_id: {}, preset: {}, format: {:?}",
            file_id, &preset, format,
        );
        Some(cached)
    } else {
        debug!(
            "using backend version of image for file_id: {}, preset: {}, format: {:?}",
            file_id, &preset, format,
        );
        if let Some(data) = get_image(&mut state, file_id, preset.clone(), &category, format).await
        {
            cache.set(file_id, preset, format, data.clone());
            Some(data)
        } else {
            None
        }
    };

    match img {
        None => Ok((state, empty_response(StatusCode::NOT_FOUND))),
        Some(data) => {
            if params.encode.unwrap_or(false) {
                let encoded = encode(data.as_ref());
                return Ok((
                    state,
                    json_response(
                        StatusCode::OK,
                        Some(json!({
                                "image": encoded,
                        })),
                    ),
                ));
            }
            Ok((state, image_response(format, data)))
        }
    }
}

/// Handles a POST request for adding a image to the store.
///
/// The image payload must be in JSON format and be base64 encoded in
/// the standard specification.
///
/// E.g.
/// ```json
/// {
///     "format": "png",
///     "data": "...data ensues..."
/// }
/// ```
pub async fn add_file(mut state: State) -> HandlerResult {
    let upload: ImageUpload = from_body!(state);

    let format = upload.format;
    let data = match decode(upload.data) {
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

    let category = upload.category.unwrap_or_else(|| "default".to_string());

    let (file_id, formats) = match process_new_image(&mut state, &category, format, data).await {
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
            ));
        }
    };

    let resp = ImageUploaded {
        file_id,
        formats,
        category,
    };

    let resp = serde_json::to_value(resp).expect("failed to serialize uploaded stats");

    Ok((state, json_response(StatusCode::OK, Some(resp))))
}

/// Handles removing a image from the store.
///
/// This removes the image from both the database backend and
/// the cache if it exists in there.
///
/// This only requires the UUID of the image no other information
/// is needed.
///
/// Note on semantics:
///     This endpoint does not check if the image exists or not,
///     it simply tries to remove it if it exists otherwise ignores it.
///
///     For that reason this will always return 200 if no exceptions
///     happened at the time.
///
/// This endpoint can return any of the following responses:
///
/// 500:
///     The server could not complete the request due to a unexpected
///     exception, this is typically only possible via the transaction
///     on the database backend failing.
///
/// 200:
///     The image has been removed successfully.
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
                "message": "file deleted if exists",
                "file_id": params.file_id.to_string()
            })),
        ),
    ))
}

pub async fn list_files(mut state: State) -> HandlerResult {
    let payload: FilesListPayload = from_body!(state);
    let storage = StorageBackend::take_from(&mut state);

    let filter = payload.filter.unwrap_or_else(|| FilterType::All);
    let sort = payload.order.unwrap_or_else(|| OrderBy::CreationDate);
    let page = payload.page.unwrap_or_else(|| 1usize);

    let (status, payload) = match storage.list_entities(filter.clone(), sort, page).await {
        Ok(results) => (
            StatusCode::OK,
            Some(json!({
                "page": page,
                "filtered_by": filter,
                "ordered_by": sort,
                "results": results,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(json!({
                "message": format!("failed to fetch results for page due to error: {:?}", e)
            })),
        ),
    };

    Ok((state, json_response(status, payload)))
}
