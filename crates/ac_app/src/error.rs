//! Why a command or query cannot be carried out.

use ac_domain::protocol::InvalidMethod;

use crate::ports::StoreError;
use crate::text::{UserText, keys};

/// A command or query that cannot be carried out as given.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// The method is not a valid HTTP token.
    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),
    /// No URL entered.
    #[error("URL is empty")]
    EmptyUrl,
    /// The URL cannot be parsed.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    /// The URL uses a scheme other than http or https.
    #[error("unsupported URL scheme {0:?}")]
    UnsupportedScheme(String),
    /// A header name is not a token or its value contains a line break.
    #[error("invalid header {0:?}")]
    InvalidHeader(String),
    /// The collection is not open.
    #[error("collection is not open")]
    UnknownCollection,
    /// No folder or request with this id (any more).
    #[error("no such item")]
    UnknownItem,
    /// New items go into the collection or a folder, not a request.
    #[error("not a folder")]
    NotAFolder,
    /// A name is empty.
    #[error("name is empty")]
    EmptyName,
    /// Reading or writing a collection failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl AppError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        match self {
            Self::InvalidMethod(InvalidMethod(method)) => {
                UserText::new(keys::METHOD_INVALID).with("method", method.clone())
            }
            Self::EmptyUrl => UserText::new(keys::URL_EMPTY),
            Self::InvalidUrl(reason) => {
                UserText::new(keys::URL_INVALID).with("reason", reason.clone())
            }
            Self::UnsupportedScheme(scheme) => {
                UserText::new(keys::URL_SCHEME).with("scheme", scheme.clone())
            }
            Self::InvalidHeader(name) => {
                UserText::new(keys::HEADER_INVALID).with("name", name.clone())
            }
            Self::UnknownCollection => UserText::new(keys::UNKNOWN_COLLECTION),
            Self::UnknownItem => UserText::new(keys::UNKNOWN_ITEM),
            Self::NotAFolder => UserText::new(keys::NOT_A_FOLDER),
            Self::EmptyName => UserText::new(keys::NAME_EMPTY),
            Self::Store(error) => error.user_text(),
        }
    }
}
