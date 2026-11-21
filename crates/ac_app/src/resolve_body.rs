//! The body of a request as it will be sent: `{{variables}}` filled in,
//! form fields encoded, files named by absolute path (they are read by the
//! HTTP adapter when sending).

use std::path::{Path, PathBuf};

use ac_domain::collection::Body;
use ac_domain::environment::UnresolvedVariable;
use ac_domain::exchange::{Header, Part, PartContent, RequestBody};
use ac_domain::protocol::{header, media_type};

use crate::error::AppError;

/// Resolves `body`; adds a matching `Content-Type` to `headers` unless one
/// is set (multipart gets its own, with the boundary, from the adapter).
/// Relative file paths are taken relative to `files_base`.
pub(crate) fn resolve_body(
    body: &Body,
    headers: &mut Vec<Header>,
    files_base: Option<&Path>,
    fill: &impl Fn(&str) -> Result<String, UnresolvedVariable>,
) -> Result<Option<RequestBody>, AppError> {
    let path_of = |text: &str| file_path(&fill(text)?, files_base);
    let (body, content_type) = match body {
        Body::None => return Ok(None),
        Body::Json(content) => (
            RequestBody::Bytes(fill(content)?.into_bytes()),
            Some(media_type::JSON),
        ),
        Body::Text(content) => (
            RequestBody::Bytes(fill(content)?.into_bytes()),
            Some(media_type::TEXT),
        ),
        Body::Form(fields) => {
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            for field in fields
                .iter()
                .filter(|field| field.enabled && !field.name.trim().is_empty())
            {
                form.append_pair(fill(&field.name)?.trim(), &fill(&field.value)?);
            }
            (
                RequestBody::Bytes(form.finish().into_bytes()),
                Some(media_type::FORM),
            )
        }
        Body::Multipart(fields) => {
            let parts = fields
                .iter()
                .filter(|field| field.enabled && !field.name.trim().is_empty())
                .map(|field| {
                    let content = if field.file {
                        PartContent::File(path_of(&field.value)?)
                    } else {
                        PartContent::Text(fill(&field.value)?)
                    };
                    Ok(Part {
                        name: fill(&field.name)?.trim().to_owned(),
                        content,
                    })
                })
                .collect::<Result<_, AppError>>()?;
            (RequestBody::Multipart(parts), None)
        }
        Body::Binary(path) => (
            RequestBody::File(path_of(path)?),
            Some(media_type::OCTET_STREAM),
        ),
    };
    if let Some(content_type) = content_type {
        let has_type = headers
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case(header::CONTENT_TYPE));
        if !has_type {
            headers.push(Header::new(header::CONTENT_TYPE, content_type));
        }
    }
    Ok(Some(body))
}

/// An absolute path for `text`: relative ones start at `base`.
fn file_path(text: &str, base: Option<&Path>) -> Result<PathBuf, AppError> {
    let path = Path::new(text.trim());
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    base.map(|base| base.join(path))
        .ok_or_else(|| AppError::RelativeFile(text.trim().to_owned()))
}

#[cfg(test)]
#[path = "tests/resolve_body.rs"]
mod tests;
