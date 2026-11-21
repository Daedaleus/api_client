//! Sending requests: validated at once, sent in the background, the result
//! reported as [`AppEvent::RunFinished`].

use std::sync::Arc;

use ac_domain::auth::effective;
use ac_domain::scope::Scope;
use ac_domain::tls::checks_certificates;
use std::sync::atomic::Ordering;

use super::App;
use super::environments::Context;
use super::secrets::explain;
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::resolve::{Entered, resolve_request};
use crate::view::{
    AuthDraft, BodyDraft, CertificateCheckView, HeaderDraft, ResponseView, RunOutcomeView,
    SendContext, VariableDraft,
};

/// A request as the editor sends it.
pub(super) struct Outgoing<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [HeaderDraft],
    pub body: &'a BodyDraft,
    /// The request's own variables.
    pub variables: &'a [VariableDraft],
    /// The request's own authentication.
    pub auth: &'a AuthDraft,
    /// The request's own certificate check.
    pub certificate_check: CertificateCheckView,
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
        let own_auth = outgoing.auth.to_auth();
        let auth = match &context {
            Some(context) => context.auth(&own_auth),
            None => effective([&own_auth]).clone(),
        };
        let entered = Entered {
            method: outgoing.method,
            url: outgoing.url,
            headers: &headers,
            body: &body,
            auth: &auth,
            files_base: context.as_ref().map(Context::root),
            verify_tls: {
                let own = outgoing.certificate_check.to_check();
                match &context {
                    Some(context) => context.checks_certificates(own),
                    None => checks_certificates([own]),
                }
            },
        };
        let request = resolve_request(&entered, &variables).map_err(|error| {
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
