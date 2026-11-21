//! Import and export through `ac_interchange`: a request as a cURL command,
//! and a pasted cURL command as a new request.

use ac_interchange::curl::{from_curl, to_curl};

use super::App;
use crate::command::QueryResult;
use crate::error::AppError;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::view::{OutgoingRequest, RequestDraft};

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    /// The cURL command that sends exactly what sending `request` would:
    /// variables (also secrets) filled in, inherited auth applied.
    pub(super) fn query_curl(&self, request: &OutgoingRequest) -> Result<QueryResult, AppError> {
        let resolved = self.prepare(request)?;
        Ok(QueryResult::Curl {
            command: to_curl(&resolved),
        })
    }
}

/// A pasted cURL command as an unsaved request.
pub(super) fn parse_curl(text: &str) -> Result<QueryResult, AppError> {
    let imported = from_curl(text)?;
    let mut request = RequestDraft::from_request(&imported.request, false);
    // Not saved: the editor treats it like any new, unsaved request.
    request.id = String::new();
    Ok(QueryResult::ParsedCurl {
        request,
        ignored: imported.ignored,
    })
}

#[cfg(test)]
#[path = "../tests/app_interchange.rs"]
mod tests;
