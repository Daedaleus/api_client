//! Import and export through `ac_interchange`: a request as a cURL command,
//! and a pasted cURL command as a new request.

use std::path::Path;

use ac_interchange::bruno::{self, BrunoFile};
use ac_interchange::curl::{from_curl, to_curl};
use ac_interchange::{Imported, Skipped, SkippedKind, import_collection};

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::oauth::{TOKEN_PLACEHOLDER, authorize};
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, SecretVault};
use crate::text::{UserText, keys};
use crate::view::{CollectionView, OutgoingRequest, RequestDraft};

impl<H: HttpSender, S: CollectionStore, V: SecretVault, B: BrowserLogin> App<H, S, V, B> {
    /// Converts another tool's file into a new collection in `path` and
    /// opens it; what has no counterpart is listed for the user. A Bruno
    /// collection is a folder: it is chosen by its `bruno.json`.
    pub(super) fn import_collection(&self, file: &str, path: &str) -> Result<CommandAck, AppError> {
        let file = Path::new(file);
        let imported = if file.file_name().is_some_and(|name| name == bruno::MARKER) {
            self.read_bruno(file.parent().unwrap_or(file))?
        } else {
            import_collection(&self.inner.store.read_text(file)?)?
        };
        let opened = self.inner.store.import_collection(
            Path::new(path),
            &imported.collection,
            &imported.environments,
        )?;
        self.remember(&opened);
        Ok(CommandAck::CollectionImported {
            collection: Box::new(CollectionView::from_opened(&opened)),
            skipped: imported.skipped.iter().map(skipped_text).collect(),
        })
    }

    /// The Bruno collection in `dir`.
    fn read_bruno(&self, dir: &Path) -> Result<Imported, AppError> {
        let files: Vec<BrunoFile> = self
            .inner
            .store
            .read_tree(dir, &[bruno::EXTENSION, "json"])?
            .into_iter()
            .map(|file| BrunoFile {
                path: file.path,
                text: file.text,
            })
            .collect();
        Ok(bruno::from_files(dir, &files))
    }

    /// The cURL command that sends exactly what sending `request` would:
    /// variables (also secrets) filled in, inherited auth applied. An OAuth 2
    /// token is fetched only when sending, so a placeholder stands in.
    pub(super) fn query_curl(&self, request: &OutgoingRequest) -> Result<QueryResult, AppError> {
        let prepared = self.prepare(request)?;
        let mut resolved = prepared.request;
        if prepared.oauth.is_some() {
            authorize(&mut resolved, TOKEN_PLACEHOLDER);
        }
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
        SkippedKind::Headers => UserText::new(keys::IMPORT_SKIPPED_HEADERS),
        SkippedKind::Body(mode) => {
            UserText::new(keys::IMPORT_SKIPPED_BODY).with("mode", mode.clone())
        }
    };
    text.with("item", skipped.item.clone())
}

#[cfg(test)]
#[path = "../tests/app_interchange.rs"]
mod tests;
