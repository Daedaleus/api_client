//! Import and export through `ac_interchange`: collections from files and
//! URLs, a request as a cURL command, and a pasted cURL command as a new
//! request.

use std::path::Path;

use ac_domain::exchange::{Header, ResolvedRequest};
use ac_domain::protocol::{HttpMethod, StatusClass, header};
use ac_interchange::bruno::{self, BrunoFile};
use ac_interchange::curl::{from_curl, to_curl};
use ac_interchange::snippet::to_snippet;
use ac_interchange::{
    ImportError, Imported, Skipped, SkippedKind, import_collection, import_collection_from,
};

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::oauth::{TOKEN_PLACEHOLDER, authorize};
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretVault};
use crate::ports::{HistoryStore, ScriptAssist, SettingsStore};
use crate::resolve::resolve_url;
use crate::text::{UserText, keys};
use crate::view::{OutgoingRequest, RequestDraft, SnippetLanguageView};

/// What an import from a URL asks for: the formats it reads, JSON first.
const IMPORT_ACCEPT: &str = "application/json, application/yaml;q=0.9, */*;q=0.5";

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
> App<H, S, V, B, R, A, Y, Z>
{
    /// Converts another tool's file into a new collection in `path` – or,
    /// without one, an unsaved collection in a temporary folder – and opens
    /// it; what has no counterpart is listed for the user. A Bruno
    /// collection is a folder: it is chosen by its `bruno.json`.
    pub(super) fn import_collection(
        &self,
        file: &str,
        path: Option<&str>,
    ) -> Result<CommandAck, AppError> {
        let file = Path::new(file);
        let imported = if file.file_name().is_some_and(|name| name == bruno::MARKER) {
            self.read_bruno(file.parent().unwrap_or(file))?
        } else {
            import_collection(&self.inner.store.read_text(file)?)?
        };
        self.store_import(&imported, path)
    }

    /// Downloads a Postman collection or an `OpenAPI` description from
    /// `url` and opens it as an unsaved collection. Relative servers of the
    /// description are resolved against `url`. Nothing is sent but a plain
    /// `GET` of the address the user entered.
    ///
    /// # Errors
    /// If the address is invalid, nothing usable arrives or the text is no
    /// known format.
    pub async fn import_from_url(&self, url: &str) -> Result<CommandAck, AppError> {
        let url = resolve_url(url)?;
        let request = ResolvedRequest {
            transport: ac_domain::transport::EffectiveTransport::default(),
            method: HttpMethod::Get,
            url: url.clone(),
            headers: vec![Header::new(header::ACCEPT, IMPORT_ACCEPT)],
            body: None,
            verify_tls: true,
        };
        let response = self
            .inner
            .http
            // An import is no request of a collection: no cookies.
            .send(request, None)
            .await
            .map_err(|error| AppError::Download(error.to_string()))?;
        if response.status.class() != StatusClass::Success {
            return Err(AppError::Download(format!(
                "status {}",
                response.status.as_u16()
            )));
        }
        let text = String::from_utf8_lossy(&response.body);
        let imported = import_collection_from(&text, Some(&url))?;
        self.store_import(&imported, None)
    }

    /// Writes an import into `path` – or, without one, an unsaved collection
    /// in a temporary folder – and opens it.
    fn store_import(
        &self,
        imported: &Imported,
        path: Option<&str>,
    ) -> Result<CommandAck, AppError> {
        let root = match path {
            Some(path) => Path::new(path).to_owned(),
            None => self.inner.store.temporary_root()?,
        };
        let opened = self.inner.store.import_collection(
            &root,
            &imported.collection,
            &imported.environments,
        )?;
        self.remember(&opened);
        if path.is_none() {
            self.temporary().insert(opened.collection.id);
        }
        Ok(CommandAck::CollectionImported {
            collection: Box::new(self.view(&opened)),
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
            .collect::<Vec<_>>();
        if !files.iter().any(|file| file.path == bruno::MARKER) {
            return Err(ImportError::NotBruno.into());
        }
        Ok(bruno::from_files(dir, &files))
    }

    /// The cURL command that sends exactly what sending `request` would:
    /// variables (also secrets) filled in, inherited auth applied. An OAuth 2
    /// token is fetched only when sending, so a placeholder stands in.
    pub(super) fn query_curl(&self, request: &OutgoingRequest) -> Result<QueryResult, AppError> {
        Ok(QueryResult::Curl {
            command: to_curl(&self.resolved_for_export(request)?),
        })
    }

    pub(super) fn query_snippet(
        &self,
        request: &OutgoingRequest,
        language: SnippetLanguageView,
    ) -> Result<QueryResult, AppError> {
        Ok(QueryResult::Snippet {
            code: to_snippet(&self.resolved_for_export(request)?, language.to_language()),
        })
    }

    /// The request as it would be sent, an OAuth 2 token as a placeholder
    /// (exports never fetch tokens).
    fn resolved_for_export(&self, request: &OutgoingRequest) -> Result<ResolvedRequest, AppError> {
        let prepared = self.prepare(request)?;
        let mut resolved = prepared.request;
        if prepared.oauth.is_some() {
            authorize(&mut resolved, TOKEN_PLACEHOLDER);
        }
        Ok(resolved)
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
        SkippedKind::Assertion(assertion) => {
            UserText::new(keys::IMPORT_SKIPPED_ASSERTION).with("assertion", assertion.clone())
        }
        SkippedKind::Auth(kind) => {
            UserText::new(keys::IMPORT_SKIPPED_AUTH).with("kind", kind.clone())
        }
        SkippedKind::Headers => UserText::new(keys::IMPORT_SKIPPED_HEADERS),
        SkippedKind::Body(mode) => {
            UserText::new(keys::IMPORT_SKIPPED_BODY).with("mode", mode.clone())
        }
        SkippedKind::Resources { kind, count } => UserText::new(keys::IMPORT_SKIPPED_RESOURCES)
            .with("kind", kind.clone())
            .with("count", count.to_string()),
        SkippedKind::RequestKind(kind) => {
            UserText::new(keys::IMPORT_SKIPPED_REQUEST_KIND).with("kind", kind.clone())
        }
        SkippedKind::TemplateTag(tag) => {
            UserText::new(keys::IMPORT_TEMPLATE_TAG).with("tag", tag.clone())
        }
        SkippedKind::RelativeServer(url) => {
            UserText::new(keys::IMPORT_RELATIVE_SERVER).with("url", url.clone())
        }
    };
    text.with("item", skipped.item.clone())
}

#[cfg(test)]
#[path = "../tests/app_interchange.rs"]
mod tests;
