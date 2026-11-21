//! Sending requests: validated at once, sent in the background, the result
//! reported as [`AppEvent::RunFinished`].

use std::sync::Arc;

use ac_domain::auth::effective;
use ac_domain::exchange::Response;
use ac_domain::scope::Scope;
use ac_domain::tls::checks_certificates;
use std::sync::atomic::Ordering;

use super::environments::Context;
use super::secrets::explain;
use super::{App, Inner};
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::oauth::authorize;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::resolve::{Entered, Prepared, resolve_request};
use crate::text::UserText;
use crate::view::{HeaderDraft, OutgoingRequest, ResponseView, RunOutcomeView, VariableDraft};

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    pub(super) fn send_request(&self, outgoing: &OutgoingRequest) -> Result<CommandAck, AppError> {
        let prepared = self.prepare(outgoing)?;
        let run_id = RunId(self.inner.next_run.fetch_add(1, Ordering::Relaxed));
        let inner = Arc::clone(&self.inner);
        self.inner.runtime.spawn(async move {
            let outcome = match inner.send(prepared).await {
                Ok(response) => RunOutcomeView::Completed(ResponseView::from_response(&response)),
                Err(error) => RunOutcomeView::Failed { error },
            };
            // Nobody listening is fine: then nobody wants the result.
            let _ = inner.events.send(AppEvent::RunFinished { run_id, outcome });
        });
        Ok(CommandAck::RunStarted { run_id })
    }

    /// The request as it will be sent: variables filled in from every level,
    /// inherited auth and certificate check applied, file paths absolute.
    pub(super) fn prepare(&self, outgoing: &OutgoingRequest) -> Result<Prepared, AppError> {
        let context = self.context(outgoing.context.as_ref())?;
        let own = VariableDraft::to_plain_variables(&outgoing.variables);
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
            method: &outgoing.method,
            url: &outgoing.url,
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
        resolve_request(&entered, &variables).map_err(|error| {
            let unlocked = context
                .as_ref()
                .and_then(|context| context.unlocked.as_ref());
            explain(error, unlocked)
        })
    }
}

impl<H: HttpSender, S, V> Inner<H, S, V> {
    /// Sends a prepared request, fetching its OAuth 2 token first if it
    /// needs one.
    pub(super) async fn send(&self, prepared: Prepared) -> Result<Response, UserText> {
        let mut request = prepared.request;
        if let Some(credentials) = &prepared.client_credentials {
            let token = self
                .tokens
                .token(&self.http, credentials, request.verify_tls)
                .await
                .map_err(|error| error.user_text())?;
            authorize(&mut request, &token);
        }
        self.http
            .send(request)
            .await
            .map_err(|error| error.user_text())
    }
}

#[cfg(test)]
#[path = "../tests/app_runs.rs"]
mod tests;
