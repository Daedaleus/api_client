//! Translated texts for the command line, from the same `locales/app.json`
//! as the desktop app.

use std::collections::BTreeMap;

use ac_app::UserText;
use serde_json::Value;

/// The translation file, built in.
const MESSAGES: &str = include_str!("../../../locales/app.json");

/// Languages with texts; the first is the fallback.
const LANGUAGES: [&str; 2] = ["en", "de"];

/// Keys of the command line's own texts.
pub(crate) mod keys {
    /// `{collection}` heading. Argument: `collection`.
    pub const HEADING: &str = "cli.heading";
    /// Heading with environment. Arguments: `collection`, `environment`.
    pub const HEADING_ENVIRONMENT: &str = "cli.heading_environment";
    /// Summary. Arguments: `total`, `passed`, `failed`.
    pub const SUMMARY: &str = "cli.summary";
    /// An error before the run. Argument: `message`.
    pub const ERROR: &str = "cli.error";

    /// All of them with their placeholders (checked by a test).
    #[cfg(test)]
    pub const ALL: [(&str, &[&str]); 4] = [
        (HEADING, &["collection"]),
        (HEADING_ENVIRONMENT, &["collection", "environment"]),
        (SUMMARY, &["total", "passed", "failed"]),
        (ERROR, &["message"]),
    ];
}

/// Texts in one language.
#[derive(Debug, Clone)]
pub struct Texts {
    language: &'static str,
    messages: BTreeMap<String, Value>,
}

impl Texts {
    /// Texts in `language` (`de`, `en`; others fall back to English).
    #[must_use]
    pub fn new(language: &str) -> Self {
        let language = LANGUAGES
            .iter()
            .find(|known| language.starts_with(*known))
            .copied()
            .unwrap_or(LANGUAGES[0]);
        let messages = serde_json::from_str(MESSAGES).unwrap_or_default();
        Self { language, messages }
    }

    /// Texts in the language of `LC_ALL`, `LC_MESSAGES` or `LANG`.
    #[must_use]
    pub fn from_environment() -> Self {
        let language = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
            .unwrap_or_default();
        Self::new(&language)
    }

    /// The text for `key` with `{name}` placeholders filled in; the key
    /// itself if it is unknown.
    #[must_use]
    pub fn get(&self, key: &str, args: &[(&str, &str)]) -> String {
        let template = self
            .messages
            .get(key)
            .and_then(|entry| entry.get(self.language).or_else(|| entry.get(LANGUAGES[0])))
            .and_then(Value::as_str)
            .unwrap_or(key);
        let mut text = template.to_owned();
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// A text of the application layer.
    #[must_use]
    pub fn text(&self, text: &UserText) -> String {
        let args: Vec<(&str, &str)> = text
            .args
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        self.get(text.key, &args)
    }

    /// An error before the run, as a line.
    #[must_use]
    pub fn error(&self, text: &UserText) -> String {
        self.get(keys::ERROR, &[("message", &self.text(text))])
    }
}

#[cfg(test)]
#[path = "tests/texts.rs"]
mod tests;
