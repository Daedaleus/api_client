//! Why a command or query cannot be carried out.

use ac_domain::environment::UnresolvedVariable;
use ac_domain::protocol::InvalidMethod;

use crate::ports::{SecretError, StoreError};
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
    /// A `{{name}}` without a value in the chosen environment (or none chosen).
    #[error(transparent)]
    UnresolvedVariable(#[from] UnresolvedVariable),
    /// A secret variable is used, but the secrets cannot be decrypted.
    #[error("secret {name:?} unavailable: {reason}")]
    SecretUnavailable {
        /// The variable.
        name: String,
        /// Why it cannot be decrypted.
        reason: SecretError,
    },
    /// Recipients not changed: the secrets of an environment cannot be
    /// decrypted for re-encrypting.
    #[error("secrets of {environment:?} not re-encrypted: {reason}")]
    NotReencrypted {
        /// The environment.
        environment: String,
        /// Why they cannot be decrypted.
        reason: SecretError,
    },
    /// Encrypting or decrypting secrets failed.
    #[error(transparent)]
    Secret(#[from] SecretError),
    /// A relative file path in a request that belongs to no collection
    /// (relative paths are relative to the collection folder).
    #[error("relative file path {0:?} without a collection")]
    RelativeFile(String),
    /// A pasted text is no usable cURL command.
    #[error(transparent)]
    Curl(#[from] ac_interchange::curl::CurlError),
    /// A file cannot be imported.
    #[error(transparent)]
    Import(#[from] ac_interchange::ImportError),
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
            Self::UnresolvedVariable(UnresolvedVariable(name)) => {
                UserText::new(keys::VARIABLE_UNKNOWN).with("name", name.clone())
            }
            Self::SecretUnavailable {
                name,
                reason: SecretError::NoKey,
            } => UserText::new(keys::SECRET_UNAVAILABLE).with("name", name.clone()),
            Self::NotReencrypted {
                environment,
                reason: SecretError::NoKey,
            } => {
                UserText::new(keys::SECRET_NOT_REENCRYPTED).with("environment", environment.clone())
            }
            Self::SecretUnavailable { reason, .. }
            | Self::NotReencrypted { reason, .. }
            | Self::Secret(reason) => reason.user_text(),
            Self::RelativeFile(path) => {
                UserText::new(keys::FILE_RELATIVE).with("path", path.clone())
            }
            Self::Curl(error) => {
                UserText::new(keys::CURL_INVALID).with("detail", error.to_string())
            }
            Self::Import(error) => {
                UserText::new(keys::IMPORT_INVALID).with("detail", error.to_string())
            }
            Self::UnknownCollection => UserText::new(keys::UNKNOWN_COLLECTION),
            Self::UnknownItem => UserText::new(keys::UNKNOWN_ITEM),
            Self::NotAFolder => UserText::new(keys::NOT_A_FOLDER),
            Self::EmptyName => UserText::new(keys::NAME_EMPTY),
            Self::Store(error) => error.user_text(),
        }
    }
}
