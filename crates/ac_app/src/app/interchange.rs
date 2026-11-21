//! Import and export through `ac_interchange`: a request as a cURL command,
//! and a pasted cURL command as a new request.

use std::path::Path;

use ac_interchange::curl::{from_curl, to_curl};
use ac_interchange::postman::{Skipped, SkippedKind, from_postman};

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::text::{UserText, keys};
use crate::view::{CollectionView, OutgoingRequest, RequestDraft};

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    /// Converts a Postman export into a new collection in `path` and opens
    /// it; what has no counterpart is listed for the user.
    pub(super) fn import_postman(&self, file: &str, path: &str) -> Result<CommandAck, AppError> {
        let text = self.inner.store.read_text(Path::new(file))?;
        let imported = from_postman(&text)?;
        let opened = self
            .inner
            .store
            .import_collection(Path::new(path), &imported.collection)?;
        self.remember(&opened);
        Ok(CommandAck::CollectionImported {
            collection: Box::new(CollectionView::from_opened(&opened)),
            skipped: imported.skipped.iter().map(skipped_text).collect(),
        })
    }

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

/// What an import left out, as shown to the user.
fn skipped_text(skipped: &Skipped) -> UserText {
    let text = match &skipped.what {
        SkippedKind::Scripts => UserText::new(keys::IMPORT_SKIPPED_SCRIPTS),
        SkippedKind::Auth(kind) => {
            UserText::new(keys::IMPORT_SKIPPED_AUTH).with("kind", kind.clone())
        }
        SkippedKind::Body(mode) => {
            UserText::new(keys::IMPORT_SKIPPED_BODY).with("mode", mode.clone())
        }
    };
    text.with("item", skipped.item.clone())
}

#[cfg(test)]
#[path = "../tests/app_interchange.rs"]
mod tests;
