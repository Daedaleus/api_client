//! The body of a saved request as the editor shows and returns it.

use ac_domain::collection::{Body, FormField, MultipartField};
use serde::{Deserialize, Serialize};

/// The body of a [`super::RequestDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BodyDraft {
    /// No body.
    #[default]
    None,
    /// JSON text.
    Json {
        /// The text.
        content: String,
    },
    /// Plain text.
    Text {
        /// The text.
        content: String,
    },
    /// URL-encoded form fields.
    Form {
        /// The fields in order.
        fields: Vec<FormFieldDraft>,
    },
    /// Multipart form data.
    Multipart {
        /// The fields in order.
        fields: Vec<MultipartFieldDraft>,
    },
    /// The contents of a file.
    Binary {
        /// Path of the file; relative = to the collection folder.
        file: String,
    },
}

/// A field of a URL-encoded form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FormFieldDraft {
    /// Name.
    pub name: String,
    /// Value.
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
}

/// A field of a multipart body: text, or a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MultipartFieldDraft {
    /// Name.
    pub name: String,
    /// The text, or the path of the file.
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
    /// `value` is a file path.
    #[serde(default)]
    pub file: bool,
}

impl BodyDraft {
    /// The editor's view of a body.
    #[must_use]
    pub fn from_body(body: &Body) -> Self {
        match body {
            Body::None => Self::None,
            Body::Json(content) => Self::Json {
                content: content.clone(),
            },
            Body::Text(content) => Self::Text {
                content: content.clone(),
            },
            Body::Form(fields) => Self::Form {
                fields: fields
                    .iter()
                    .map(|field| FormFieldDraft {
                        name: field.name.clone(),
                        value: field.value.clone(),
                        enabled: field.enabled,
                    })
                    .collect(),
            },
            Body::Multipart(fields) => Self::Multipart {
                fields: fields
                    .iter()
                    .map(|field| MultipartFieldDraft {
                        name: field.name.clone(),
                        value: field.value.clone(),
                        enabled: field.enabled,
                        file: field.file,
                    })
                    .collect(),
            },
            Body::Binary(file) => Self::Binary { file: file.clone() },
        }
    }

    /// The body this draft describes.
    #[must_use]
    pub fn to_body(&self) -> Body {
        match self {
            Self::None => Body::None,
            Self::Json { content } => Body::Json(content.clone()),
            Self::Text { content } => Body::Text(content.clone()),
            Self::Form { fields } => Body::Form(
                fields
                    .iter()
                    .map(|field| FormField {
                        name: field.name.clone(),
                        value: field.value.clone(),
                        enabled: field.enabled,
                    })
                    .collect(),
            ),
            Self::Multipart { fields } => Body::Multipart(
                fields
                    .iter()
                    .map(|field| MultipartField {
                        name: field.name.clone(),
                        value: field.value.clone(),
                        enabled: field.enabled,
                        file: field.file,
                    })
                    .collect(),
            ),
            Self::Binary { file } => Body::Binary(file.clone()),
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_request_body.rs"]
mod tests;
