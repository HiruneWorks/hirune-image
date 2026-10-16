use crate::{bootstrap::Application, http::v1::images::contracts::RecipeInput};
use application::{
    error::{ProcessingError as Error, Result},
    images::{JobInput, sha256},
};
use axum::{extract::Request, http::header};
use bytes::{Bytes, BytesMut};
use futures_util::{StreamExt, stream};
use tokio::time::{Duration, Instant, timeout_at};

pub struct Upload {
    pub job: JobInput,
    pub source: Bytes,
}
pub async fn read_upload(request: Request, app: &Application, deadline: Instant) -> Result<Upload> {
    let limits = &app.settings.limits;
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::InvalidRequest)?;
    let boundary = multer::parse_boundary(content_type).map_err(|_| Error::InvalidRequest)?;
    if boundary.is_empty() || boundary.len() > 70 {
        return Err(Error::InvalidRequest);
    }
    if let Some(length) = request.headers().get(header::CONTENT_LENGTH) {
        let length: u64 = length
            .to_str()
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::InvalidRequest)?;
        if length > limits.request_bytes {
            return Err(Error::RequestTooLarge);
        }
    }
    let mut stream = request.into_body().into_data_stream();
    let mut body = BytesMut::new();
    loop {
        let until = deadline.min(Instant::now() + Duration::from_millis(limits.idle_ms));
        let chunk = timeout_at(until, stream.next())
            .await
            .map_err(|_| Error::ProcessingTimeout)?;
        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|_| Error::InvalidRequest)?;
        if (body.len() as u64)
            .checked_add(chunk.len() as u64)
            .ok_or(Error::RequestTooLarge)?
            > limits.request_bytes
        {
            return Err(Error::RequestTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    let closing = format!("\r\n--{boundary}--");
    let position = body
        .windows(closing.len())
        .position(|w| w == closing.as_bytes())
        .ok_or(Error::InvalidRequest)?;
    let remainder = &body[position + closing.len()..];
    if !remainder.is_empty() && remainder != b"\r\n" {
        return Err(Error::InvalidRequest);
    }
    let constraints = multer::Constraints::new()
        .allowed_fields(vec!["file", "recipe"])
        .size_limit(
            multer::SizeLimit::new()
                .whole_stream(limits.request_bytes)
                .per_field(limits.source_bytes)
                .for_field("recipe", limits.recipe_bytes),
        );
    let body = body.freeze();
    let mut multipart = multer::Multipart::with_constraints(
        stream::once(async move { Ok::<Bytes, std::io::Error>(body) }),
        boundary,
        constraints,
    );
    let mut source = None;
    let mut recipe = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") => {
                if source.is_some() {
                    return Err(Error::InvalidRequest);
                }
                let bytes = field.bytes().await.map_err(multipart_error)?;
                if bytes.is_empty() {
                    return Err(Error::InvalidImage);
                }
                source = Some(bytes);
            }
            Some("recipe") => {
                if recipe.is_some()
                    || field
                        .content_type()
                        .is_none_or(|mime| mime.essence_str() != "application/json")
                {
                    return Err(Error::InvalidRequest);
                }
                let bytes = field.bytes().await.map_err(multipart_error)?;
                let input: RecipeInput =
                    serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRecipe)?;
                recipe = Some((input.into_recipe(limits)?, sha256(&bytes)));
            }
            _ => return Err(Error::InvalidRequest),
        }
    }
    let (recipe, recipe_sha256) = recipe.ok_or(Error::InvalidRequest)?;
    let source = source.ok_or(Error::InvalidRequest)?;
    Ok(Upload {
        job: JobInput {
            recipe,
            limits: limits.clone(),
            source_sha256: sha256(&source),
            recipe_sha256,
        },
        source,
    })
}
fn multipart_error(error: multer::Error) -> Error {
    match error {
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. } => {
            Error::RequestTooLarge
        }
        _ => Error::InvalidRequest,
    }
}
