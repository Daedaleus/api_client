//! Sending requests: validated at once, sent in the background, the result
//! reported as [`AppEvent::RunFinished`].

use std::sync::Arc;

use ac_domain::scope::Scope;
use std::sync::atomic::Ordering;

use super::App;
use super::secrets::explain;
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::resolve::resolve_request;
use crate::view::{
    BodyDraft, HeaderDraft, ResponseView, RunOutcomeView, SendContext, VariableDraft,
};

/// A request as the editor sends it.
pub(super) struct Outgoing<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [HeaderDraft],
    pub body: &'a BodyDraft,
    /// The request's own variables.
    pub variables: &'a [VariableDraft],
}

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    pub(super) fn send_request(
        &self,
        outgoing: &Outgoing<'_>,
        context: Option<&SendContext>,
    ) -> Result<CommandAck, AppError> {
        let context = self.context(context)?;
        let own = VariableDraft::to_plain_variables(outgoing.variables);
        let variables = match &context {
            Some(context) => context.scope(&own).values(),
            None => Scope::new().then(&own).values(),
        };
        let headers: Vec<_> = outgoing.headers.iter().map(HeaderDraft::to_entry).collect();
        let body = outgoing.body.to_body();
        let request = resolve_request(outgoing.method, outgoing.url, &headers, &body, &variables)
            .map_err(|error| {
            let unlocked = context
                .as_ref()
                .and_then(|context| context.unlocked.as_ref());
            explain(error, unlocked)
        })?;
        let run_id = RunId(self.inner.next_run.fetch_add(1, Ordering::Relaxed));
        let inner = Arc::clone(&self.inner);
        self.inner.runtime.spawn(async move {
            let outcome = match inner.http.send(request).await {
                Ok(response) => RunOutcomeView::Completed(ResponseView::from_response(&response)),
                Err(error) => RunOutcomeView::Failed {
                    error: error.user_text(),
                },
            };
            // Nobody listening is fine: then nobody wants the result.
            let _ = inner.events.send(AppEvent::RunFinished { run_id, outcome });
        });
        Ok(CommandAck::RunStarted { run_id })
    }
}

#[cfg(test)]
#[path = "../tests/app_runs.rs"]
mod tests;
