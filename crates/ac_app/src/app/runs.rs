//! Sending requests: validated at once, sent in the background (with the
//! request's scripts, see `script_runs`), the result reported as
//! [`AppEvent::RunFinished`].

use ac_domain::auth::Auth;
use ac_domain::auth::effective;
use ac_domain::environment::Variable;
use ac_domain::exchange::{Header, ResolvedRequest, Response};
use ac_domain::protocol::{StatusCode, header};
use ac_domain::scope::Scope;
use ac_domain::script::ScriptPhase;
use ac_domain::tls::checks_certificates;
use std::sync::atomic::Ordering;

use super::environments::Context;
use super::secrets::explain;
use super::{App, Inner};
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::history::Masking;
use crate::oauth::authorize;
use crate::ports::SharedCookieJar;
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretVault};
use crate::ports::{HistoryStore, ScriptAssist, SettingsStore};
use crate::resolve::{Entered, Prepared, resolve_request};
use crate::signing::{Signer, aws, digest};
use crate::text::UserText;
use crate::view::{HeaderDraft, OutgoingRequest, ParamDraft, VariableDraft};

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
    /// Starts a run: checked at once unless a pre-request script may still
    /// change the request, then scripts and sending in the background.
    pub(super) fn send_request(&self, outgoing: &OutgoingRequest) -> Result<CommandAck, AppError> {
        let steps = self.script_steps(outgoing)?;
        let prepared = if steps
            .iter()
            .any(|step| step.scripts.get(ScriptPhase::PreRequest).is_some())
        {
            None
        } else {
            Some(self.prepare(outgoing)?)
        };
        let run_id = RunId(self.inner.next_run.fetch_add(1, Ordering::Relaxed));
        let app = self.clone();
        let outgoing = outgoing.clone();
        self.inner.runtime.spawn(async move {
            let outcome = app
                .run_with_steps(&outgoing, &steps, prepared)
                .await
                .into_view();
            // Nobody listening is fine: then nobody wants the result.
            let _ = app
                .inner
                .events
                .send(AppEvent::RunFinished { run_id, outcome });
        });
        Ok(CommandAck::RunStarted { run_id })
    }

    /// The request as it will be sent: variables filled in from every level,
    /// inherited auth and certificate check applied, file paths absolute.
    pub(super) fn prepare(&self, outgoing: &OutgoingRequest) -> Result<Prepared, AppError> {
        let context = self.context(outgoing.context.as_ref())?;
        let own = VariableDraft::to_plain_variables(&outgoing.variables);
        let runtime = self.runtime_variables(outgoing);
        let variables = scope(context.as_ref(), &own, &runtime).values();
        let headers: Vec<_> = outgoing.headers.iter().map(HeaderDraft::to_entry).collect();
        let path_params = ParamDraft::to_path(&outgoing.path_params);
        let body = outgoing.body.to_body();
        let own_auth = outgoing.auth.to_auth();
        let auth = match &context {
            Some(context) => context.auth(&own_auth),
            None => effective([&own_auth]).clone(),
        };
        let entered = Entered {
            method: &outgoing.method,
            url: &outgoing.url,
            path_params: &path_params,
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
            transport: {
                let own = outgoing.transport.to_settings();
                match &context {
                    Some(context) => context.transport(&own),
                    None => ac_domain::transport::effective([&own]),
                }
            },
        };
        let unlocked = context
            .as_ref()
            .and_then(|context| context.unlocked.as_ref());
        let mut prepared =
            resolve_request(&entered, &variables).map_err(|error| explain(error, unlocked))?;
        let secrets = unlocked.into_iter().flat_map(|unlocked| {
            unlocked
                .environment
                .variables
                .iter()
                .filter(|variable| variable.secret && variable.enabled)
                .map(|variable| variable.value.clone())
        });
        let api_key = match &auth {
            Auth::ApiKey { name, .. } => Some(name.clone()),
            _ => None,
        };
        prepared.masking = Masking::new(secrets, api_key);
        if prepared.request.transport.cookies {
            let collection = outgoing.context.as_ref().map(|c| c.collection_id.clone());
            prepared.cookies = Some(self.cookie_jar(collection));
        }
        Ok(prepared)
    }
}

/// The variables of a request, weakest first: collection, environment,
/// folders, the request's own, and the ones scripts set (strongest).
pub(super) fn scope<'a>(
    context: Option<&'a Context>,
    own: &'a [Variable],
    runtime: &'a [Variable],
) -> Scope<'a> {
    match context {
        Some(context) => context.scope(own),
        None => Scope::new().then(own),
    }
    .then(runtime)
}

impl<H: HttpSender, S, V, B: BrowserLogin, R, A, Y, Z> Inner<H, S, V, B, R, A, Y, Z> {
    /// Sends a prepared request, fetching its OAuth 2 token first if it
    /// needs one.
    pub(super) async fn send(&self, prepared: Prepared) -> Result<Response, UserText> {
        let mut request = prepared.request;
        if let Some(client) = &prepared.oauth {
            let token = self
                .tokens
                .token(&self.http, &self.login, client, request.verify_tls)
                .await
                .map_err(|error| error.user_text())?;
            authorize(&mut request, &token);
        }
        let cookies = prepared.cookies;
        match prepared.signer {
            Some(Signer::Aws(credentials)) => {
                aws::sign(&mut request, &credentials, crate::signing::unix_now());
                self.send_once(request, cookies).await
            }
            Some(Signer::Digest { username, password }) => {
                self.send_digest(request, cookies, &username, &password)
                    .await
            }
            None => self.send_once(request, cookies).await,
        }
    }

    async fn send_once(
        &self,
        request: ResolvedRequest,
        cookies: Option<SharedCookieJar>,
    ) -> Result<Response, UserText> {
        self.http
            .send(request, cookies)
            .await
            .map_err(|error| error.user_text())
    }

    /// HTTP Digest: sends without credentials; if the server answers 401
    /// with a Digest challenge, sends again with the answer. Without a
    /// challenge it can answer, the first response is the result.
    async fn send_digest(
        &self,
        mut request: ResolvedRequest,
        cookies: Option<SharedCookieJar>,
        username: &str,
        password: &str,
    ) -> Result<Response, UserText> {
        let first = self.send_once(request.clone(), cookies.clone()).await?;
        if first.status != StatusCode::UNAUTHORIZED {
            return Ok(first);
        }
        let challenges = first
            .headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case(header::WWW_AUTHENTICATE))
            .map(|h| h.value.as_str());
        let Some(challenge) = digest::best_challenge(challenges) else {
            return Ok(first);
        };
        let uri = match request.url.query() {
            Some(query) => format!("{}?{query}", request.url.path()),
            None => request.url.path().to_owned(),
        };
        let answer = digest::authorization(
            &challenge,
            &digest::Answer {
                username,
                password,
                method: request.method.as_str(),
                uri: &uri,
                cnonce: &digest::new_cnonce(),
                nonce_count: 1,
            },
        );
        request
            .headers
            .retain(|h| !h.name.eq_ignore_ascii_case(header::AUTHORIZATION));
        request.headers.push(Header::new("Authorization", answer));
        self.send_once(request, cookies).await
    }
}

#[cfg(test)]
#[path = "../tests/app_runs.rs"]
mod tests;
