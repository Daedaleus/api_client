//! Display texts as translation keys with arguments.
//!
//! The application layer never produces translated text: it hands out a key
//! and named arguments, the UI looks the key up in `locales/app.json`. So a
//! language switch also changes messages that are already on screen.

use std::collections::BTreeMap;

use serde::Serialize;

/// A display text: translation key plus named arguments for its placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UserText {
    /// Key in `locales/app.json`, e.g. `error.connect`.
    pub key: &'static str,
    /// Values for the `{name}` placeholders of the text.
    pub args: BTreeMap<&'static str, String>,
}

impl UserText {
    /// A text without arguments.
    #[must_use]
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            args: BTreeMap::new(),
        }
    }

    /// Adds the value for placeholder `{name}`.
    #[must_use]
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.args.insert(name, value.into());
        self
    }
}

/// Every translation key the application layer uses.
pub mod keys {
    /// The URL field is empty.
    pub const URL_EMPTY: &str = "error.url_empty";
    /// The URL cannot be parsed. Argument: `reason`.
    pub const URL_INVALID: &str = "error.url_invalid";
    /// The URL scheme is not http or https. Argument: `scheme`.
    pub const URL_SCHEME: &str = "error.url_scheme";
    /// The method is not a valid token. Argument: `method`.
    pub const METHOD_INVALID: &str = "error.method_invalid";
    /// A header cannot be sent. Argument: `name`.
    pub const HEADER_INVALID: &str = "error.header_invalid";
    /// No answer in time.
    pub const TIMEOUT: &str = "error.timeout";
    /// No connection. Argument: `detail`.
    pub const CONNECT: &str = "error.connect";
    /// Any other transport failure. Argument: `detail`.
    pub const REQUEST: &str = "error.request";

    /// A file or folder cannot be read or written. Arguments: `path`, `detail`.
    pub const STORE_IO: &str = "error.store_io";
    /// A file is damaged. Arguments: `path`, `detail`.
    pub const STORE_DAMAGED: &str = "error.store_damaged";
    /// A file comes from a newer version and is not written. Argument: `path`.
    pub const STORE_READ_ONLY: &str = "error.store_read_only";
    /// The folder is no collection. Argument: `path`.
    pub const NOT_A_COLLECTION: &str = "error.not_a_collection";
    /// A collection already exists there. Argument: `path`.
    pub const COLLECTION_EXISTS: &str = "error.collection_exists";
    /// The collection is not open (any more).
    pub const UNKNOWN_COLLECTION: &str = "error.unknown_collection";
    /// The item does not exist (any more), e.g. changed outside the app.
    pub const UNKNOWN_ITEM: &str = "error.unknown_item";
    /// New items can only be created in the collection or a folder.
    pub const NOT_A_FOLDER: &str = "error.not_a_folder";
    /// Names must not be empty.
    pub const NAME_EMPTY: &str = "error.name_empty";

    /// All keys with the names of their placeholders, checked against the
    /// translation file by a test.
    pub const ALL: [(&str, &[&str]); 17] = [
        (URL_EMPTY, &[]),
        (URL_INVALID, &["reason"]),
        (URL_SCHEME, &["scheme"]),
        (METHOD_INVALID, &["method"]),
        (HEADER_INVALID, &["name"]),
        (TIMEOUT, &[]),
        (CONNECT, &["detail"]),
        (REQUEST, &["detail"]),
        (STORE_IO, &["path", "detail"]),
        (STORE_DAMAGED, &["path", "detail"]),
        (STORE_READ_ONLY, &["path"]),
        (NOT_A_COLLECTION, &["path"]),
        (COLLECTION_EXISTS, &["path"]),
        (UNKNOWN_COLLECTION, &[]),
        (UNKNOWN_ITEM, &[]),
        (NOT_A_FOLDER, &[]),
        (NAME_EMPTY, &[]),
    ];
}

#[cfg(test)]
#[path = "tests/text.rs"]
mod tests;
