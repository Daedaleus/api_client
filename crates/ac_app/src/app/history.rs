//! The request history: every sent request is recorded (masked, see
//! [`crate::history`]); the UI lists, opens and clears it.

use std::time::Duration;

use ac_domain::collection::{Body, HeaderEntry, Request};
use ac_domain::exchange::{Header, ResolvedRequest, Response};
use ac_domain::protocol::{HttpMethod, StatusCode, header};

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::history::{HistoryEntry, HistoryOutcome, Masking};
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretVault, SettingsStore,
};
use crate::text::{UserText, keys};
use crate::view::{HistoryItemView, OutgoingRequest, RequestDraft, ResponseView, RunOutcomeView};

impl<H, S, V, B, R, A, Y, Z> App<H, S, V, B, R, A, Y, Z>
where
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
{
    /// Records a sent request and tells subscribers; a failed write is
    /// reported in the event, never swallowed.
    pub(super) fn record_history(
        &self,
        outgoing: &OutgoingRequest,
        sent: &ResolvedRequest,
        response: &Result<Response, UserText>,
        masking: &Masking,
    ) {
        let collection = outgoing
            .context
            .as_ref()
            .and_then(|context| self.current(&context.collection_id).ok())
            .map(|opened| opened.collection.name);
        let entry = HistoryEntry::new(collection, sent, response, masking);
        let error = self
            .inner
            .history
            .record(entry)
            .err()
            .map(|e| e.user_text());
        // Nobody listening is fine.
        let _ = self.inner.events.send(AppEvent::HistoryChanged { error });
    }

    pub(super) fn query_history(&self) -> Result<QueryResult, AppError> {
        let entries = self.inner.history.entries()?;
        Ok(QueryResult::History {
            entries: entries.iter().map(HistoryItemView::from_entry).collect(),
        })
    }

    pub(super) fn query_history_entry(&self, id: &str) -> Result<QueryResult, AppError> {
        let entries = self.inner.history.entries()?;
        let entry = entries
            .iter()
            .find(|entry| entry.id.0 == id)
            .ok_or(AppError::UnknownItem)?;
        Ok(QueryResult::HistoryEntry {
            request: Box::new(request_draft(entry)),
            outcome: Box::new(outcome_view(entry)),
        })
    }

    pub(super) fn clear_history(&self) -> Result<CommandAck, AppError> {
        self.inner.history.clear()?;
        let _ = self
            .inner
            .events
            .send(AppEvent::HistoryChanged { error: None });
        Ok(CommandAck::HistoryCleared)
    }
}

/// The entry's request, to open as a new, unsaved tab.
fn request_draft(entry: &HistoryEntry) -> RequestDraft {
    let sent = &entry.request;
    let mut request = Request::new(&sent.url);
    request.method = sent.method.parse().unwrap_or(HttpMethod::Get);
    request.url.clone_from(&sent.url);
    request.headers = sent
        .headers
        .iter()
        .map(|(name, value)| HeaderEntry {
            name: name.clone(),
            value: value.clone(),
            enabled: true,
        })
        .collect();
    let is_json = sent.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case(header::CONTENT_TYPE) && value.contains("json")
    });
    request.body = match &sent.body {
        None => Body::None,
        Some(text) if is_json => Body::Json(text.clone()),
        Some(text) => Body::Text(text.clone()),
    };
    let mut draft = RequestDraft::from_request(&request, false);
    // Not a saved request: the tab gets no id.
    draft.id = String::new();
    draft
}

/// The entry's response or error as the response viewer shows it.
fn outcome_view(entry: &HistoryEntry) -> RunOutcomeView {
    match &entry.outcome {
        HistoryOutcome::Response {
            status,
            elapsed_ms,
            headers,
            body,
            size_bytes,
        } => {
            // Stored from a real response, so the status is valid.
            let Ok(status) = StatusCode::new(*status) else {
                return RunOutcomeView::Failed {
                    error: UserText::new(keys::REQUEST).with("detail", "invalid status"),
                    logs: Vec::new(),
                    tests: Vec::new(),
                };
            };
            let response = Response {
                status,
                headers: headers
                    .iter()
                    .map(|(name, value)| Header {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect(),
                body: body.clone().unwrap_or_default().into_bytes(),
                elapsed: Duration::from_millis(*elapsed_ms),
            };
            let mut view = ResponseView::from_response(&response);
            view.size_bytes = *size_bytes;
            RunOutcomeView::Completed(view)
        }
        HistoryOutcome::Failed { .. } => RunOutcomeView::Failed {
            error: entry
                .error_text()
                .unwrap_or_else(|| UserText::new(keys::REQUEST)),
            logs: Vec::new(),
            tests: Vec::new(),
        },
    }
}

#[cfg(test)]
#[path = "../tests/app_history.rs"]
mod tests;
