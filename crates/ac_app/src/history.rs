//! The request history: every request that was sent, with what came back,
//! kept on the user's machine only (see `ports::history`).
//!
//! Entries are stored as sent, with secrets masked: values of secret
//! variables and credentials (authorization headers, API keys) never reach
//! the history file. Bodies are kept up to [`MAX_BODY`] bytes.

use std::collections::BTreeMap;

use ac_domain::collection::ItemId;
use ac_domain::exchange::{ResolvedRequest, Response};
use ac_domain::protocol::header;
use serde::{Deserialize, Serialize};

use crate::text::{UserText, keys};

/// Entries kept; older ones are dropped.
pub const MAX_ENTRIES: usize = 200;
/// Bytes of a request or response body kept per entry.
pub const MAX_BODY: usize = 64 * 1024;
/// What masked values are replaced with.
pub const MASK: &str = "•••";
/// Headers whose values are credentials, masked whatever they contain.
const CREDENTIAL_HEADERS: [&str; 3] = [header::AUTHORIZATION, "proxy-authorization", "cookie"];

/// Identity of a history entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HistoryId(pub String);

impl HistoryId {
    /// A new, unique id (a time-ordered UUID like item ids, but its own type
    /// so the two cannot be mixed up).
    #[must_use]
    pub fn new() -> Self {
        Self(ItemId::new().to_string())
    }
}

impl Default for HistoryId {
    fn default() -> Self {
        Self::new()
    }
}

/// One request that was sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// Identity.
    pub id: HistoryId,
    /// When it was sent, in milliseconds since 1970 (set by the store).
    pub at_ms: u64,
    /// Name of the collection it belongs to, if any.
    pub collection: Option<String>,
    /// The request as sent (masked).
    pub request: SentRequest,
    /// What came back.
    pub outcome: HistoryOutcome,
}

/// A request as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentRequest {
    /// Method, e.g. `GET`.
    pub method: String,
    /// Absolute URL.
    pub url: String,
    /// Headers in the order sent.
    pub headers: Vec<(String, String)>,
    /// Text body (cut to [`MAX_BODY`]); none for files and binary bodies.
    pub body: Option<String>,
}

/// What came back for a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HistoryOutcome {
    /// A response.
    Response {
        /// Status code.
        status: u16,
        /// Time until the whole body arrived.
        elapsed_ms: u64,
        /// Headers in received order.
        headers: Vec<(String, String)>,
        /// Text body (cut to [`MAX_BODY`]); none for binary bodies.
        body: Option<String>,
        /// Size of the whole body in bytes.
        size_bytes: u64,
    },
    /// No response (e.g. the server could not be reached).
    Failed {
        /// Translation key of the error.
        key: String,
        /// Values of its placeholders.
        args: BTreeMap<String, String>,
    },
}

impl HistoryEntry {
    /// The entry for a sent request, with what `masking` names hidden.
    #[must_use]
    pub fn new(
        collection: Option<String>,
        sent: &ResolvedRequest,
        outcome: &Result<Response, UserText>,
        masking: &Masking,
    ) -> Self {
        let url = masking.url(&sent.url);
        let request = SentRequest {
            method: sent.method.as_str().to_owned(),
            url,
            headers: masking.headers(sent.headers.iter().map(|h| (&h.name, &h.value))),
            body: sent
                .body_bytes()
                .and_then(text_body)
                .map(|body| masking.text(&body)),
        };
        let outcome = match outcome {
            Ok(response) => HistoryOutcome::Response {
                status: response.status.as_u16(),
                elapsed_ms: u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX),
                headers: masking.headers(response.headers.iter().map(|h| (&h.name, &h.value))),
                body: text_body(&response.body).map(|body| masking.text(&body)),
                size_bytes: u64::try_from(response.size()).unwrap_or(u64::MAX),
            },
            Err(error) => HistoryOutcome::Failed {
                key: error.key.to_owned(),
                args: error
                    .args
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), masking.text(value)))
                    .collect(),
            },
        };
        Self {
            id: HistoryId::new(),
            at_ms: 0,
            collection,
            request,
            outcome,
        }
    }

    /// The error of a failed entry as a display text; `None` for responses.
    /// Keys the app no longer knows become a general request error.
    #[must_use]
    pub fn error_text(&self) -> Option<UserText> {
        let HistoryOutcome::Failed { key, args } = &self.outcome else {
            return None;
        };
        let known = keys::ALL.iter().find(|(known, _)| known == key);
        Some(match known {
            Some((key, names)) => names.iter().fold(UserText::new(key), |text, name| {
                text.with(name, args.get(*name).cloned().unwrap_or_default())
            }),
            None => UserText::new(keys::REQUEST).with("detail", key.clone()),
        })
    }
}

/// What must not be stored.
#[derive(Debug, Clone, Default)]
pub struct Masking {
    /// Secret values (longest first, so a secret containing another is
    /// masked whole).
    secrets: Vec<String>,
    /// Header or query parameter carrying an API key.
    api_key: Option<String>,
}

impl Masking {
    /// Masks `secrets` everywhere and the header or query parameter
    /// `api_key`.
    #[must_use]
    pub fn new(secrets: impl IntoIterator<Item = String>, api_key: Option<String>) -> Self {
        let mut secrets: Vec<String> = secrets
            .into_iter()
            .filter(|secret| !secret.is_empty())
            .collect();
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        secrets.dedup();
        Self { secrets, api_key }
    }

    /// `text` with every secret value replaced.
    #[must_use]
    pub fn text(&self, text: &str) -> String {
        self.secrets.iter().fold(text.to_owned(), |text, secret| {
            text.replace(secret.as_str(), MASK)
        })
    }

    fn is_credential(&self, name: &str) -> bool {
        CREDENTIAL_HEADERS
            .iter()
            .any(|credential| name.eq_ignore_ascii_case(credential))
            || self
                .api_key
                .as_deref()
                .is_some_and(|key| name.eq_ignore_ascii_case(key))
    }

    fn headers<'a>(
        &self,
        headers: impl Iterator<Item = (&'a String, &'a String)>,
    ) -> Vec<(String, String)> {
        headers
            .map(|(name, value)| {
                let value = if self.is_credential(name) {
                    MASK.to_owned()
                } else {
                    self.text(value)
                };
                (name.clone(), value)
            })
            .collect()
    }

    fn url(&self, url: &url::Url) -> String {
        let mut url = url.clone();
        if let Some(key) = &self.api_key
            && url.query_pairs().any(|(name, _)| name == key.as_str())
        {
            let pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|(name, value)| {
                    let value = if name == key.as_str() {
                        MASK.to_owned()
                    } else {
                        value.into_owned()
                    };
                    (name.into_owned(), value)
                })
                .collect();
            url.query_pairs_mut().clear().extend_pairs(pairs);
        }
        self.text(url.as_str())
    }
}

/// A body as text, cut to [`MAX_BODY`] at a character boundary; `None` if
/// it is not UTF-8.
fn text_body(bytes: &[u8]) -> Option<String> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        // Valid up to a character cut off at the end of a long body.
        Err(error) if bytes.len() > MAX_BODY && error.valid_up_to() >= MAX_BODY => {
            std::str::from_utf8(&bytes[..error.valid_up_to()]).ok()?
        }
        Err(_) => return None,
    };
    let mut end = text.len().min(MAX_BODY);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Some(text[..end].to_owned())
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
