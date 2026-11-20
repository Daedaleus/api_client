//! Display texts as translation keys with arguments.
//!
//! The application layer never produces translated text: it hands out a key
//! and named arguments, the UI looks the key up in `locales/app.json`. So a
//! language switch also changes messages that are already on screen.

use std::collections::BTreeMap;

use serde::Serialize;

/// A display text: translation key plus named arguments for its placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// No answer in time.
    pub const TIMEOUT: &str = "error.timeout";
    /// No connection. Argument: `detail`.
    pub const CONNECT: &str = "error.connect";
    /// Any other transport failure. Argument: `detail`.
    pub const REQUEST: &str = "error.request";

    /// All keys with the names of their placeholders, checked against the
    /// translation file by a test.
    pub const ALL: [(&str, &[&str]); 7] = [
        (URL_EMPTY, &[]),
        (URL_INVALID, &["reason"]),
        (URL_SCHEME, &["scheme"]),
        (METHOD_INVALID, &["method"]),
        (TIMEOUT, &[]),
        (CONNECT, &["detail"]),
        (REQUEST, &["detail"]),
    ];
}

#[cfg(test)]
#[path = "tests/text.rs"]
mod tests;
