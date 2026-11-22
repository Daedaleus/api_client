//! A run with the request's scripts: the pre-request script may change the
//! request and set variables, then the request is sent, then the
//! post-response script reads the response and may set variables.
//!
//! The scripts of the collection and the folders around the request run
//! too: before sending from the outside in (collection, outer folders,
//! inner folders, request), after the response from the inside out. Each
//! sees what the ones before changed and set.
//!
//! Variables set by scripts live in memory only, per collection, and are
//! the strongest of all (`ac_domain::scope`); they are never written to
//! disk, so a token a script stores cannot end up in git.

use std::collections::BTreeMap;
use std::sync::PoisonError;

use ac_domain::environment::Variable;
use ac_domain::exchange::{ResolvedRequest, Response};
use ac_domain::script::{ScriptPhase, Scripts};

use super::App;
use super::collections::item_id_from;
use super::runs::scope;
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptJob,
    ScriptRequest, ScriptResponse, ScriptRunner, SecretVault, TestResult,
};
use crate::resolve::Prepared;
use crate::text::UserText;
use crate::view::{
    BodyDraft, HeaderDraft, OutgoingRequest, ResponseView, RunOutcomeView, TestResultView,
    VariableDraft,
};

/// Variables set by scripts, per open collection (`None`: requests outside
/// any collection).
pub(super) type RuntimeVariables = BTreeMap<Option<String>, BTreeMap<String, String>>;

/// The scripts of one owner in a run.
#[derive(Debug, Clone)]
pub(super) struct ScriptStep {
    /// The folder's or collection's name; `None` for the request's own.
    pub owner: Option<String>,
    /// Its scripts.
    pub scripts: Scripts,
}

/// What a run produced.
#[derive(Debug)]
pub(super) struct Execution {
    /// The response, or why there is none.
    pub response: Result<Response, UserText>,
    /// Lines the scripts logged.
    pub logs: Vec<String>,
    /// Why the post-response script failed (the response is still shown).
    pub script_error: Option<UserText>,
    /// Tests of both scripts, in order.
    pub tests: Vec<TestResult>,
}

impl Execution {
    fn failed(error: UserText, logs: Vec<String>, tests: Vec<TestResult>) -> Self {
        Self {
            response: Err(error),
            logs,
            script_error: None,
            tests,
        }
    }

    /// The run as the UI shows it.
    pub(super) fn into_view(self) -> RunOutcomeView {
        match self.response {
            Ok(response) => {
                let mut view = ResponseView::from_response(&response);
                view.logs = self.logs;
                view.script_error = self.script_error;
                view.tests = self.tests.iter().map(TestResultView::from_result).collect();
                RunOutcomeView::Completed(view)
            }
            Err(error) => RunOutcomeView::Failed {
                error,
                logs: self.logs,
                tests: self.tests.iter().map(TestResultView::from_result).collect(),
            },
        }
    }
}

impl<H, S, V, B, R, A, Y> App<H, S, V, B, R, A, Y>
where
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
{
    /// Runs `outgoing` with its scripts and those around it.
    pub(super) async fn run_with_scripts(&self, outgoing: &OutgoingRequest) -> Execution {
        match self.script_steps(outgoing) {
            Ok(steps) => self.run_with_steps(outgoing, &steps, None).await,
            Err(error) => Execution::failed(error.user_text(), Vec::new(), Vec::new()),
        }
    }

    /// The scripts that run for `outgoing`, outermost first: the
    /// collection's, its folders' (see `Collection::scripts_around`), then
    /// the request's own. Owners without scripts are left out.
    pub(super) fn script_steps(
        &self,
        outgoing: &OutgoingRequest,
    ) -> Result<Vec<ScriptStep>, AppError> {
        let mut steps = Vec::new();
        if let Some(context) = &outgoing.context {
            let opened = self.current(&context.collection_id)?;
            let collection = &opened.collection;
            let around = match context.request_id.as_deref() {
                Some(id) => collection.scripts_around(item_id_from(id)?),
                None => std::iter::once((collection.name.as_str(), &collection.scripts))
                    .filter(|(_, scripts)| !scripts.is_empty())
                    .collect(),
            };
            steps.extend(around.into_iter().map(|(owner, scripts)| ScriptStep {
                owner: Some(owner.to_owned()),
                scripts: scripts.clone(),
            }));
        }
        let own = outgoing.scripts.to_scripts();
        if !own.is_empty() {
            steps.push(ScriptStep {
                owner: None,
                scripts: own,
            });
        }
        Ok(steps)
    }

    /// Runs `outgoing` with the scripts of `steps`. `prepared` is the
    /// request already checked when no step has a pre-request script.
    pub(super) async fn run_with_steps(
        &self,
        outgoing: &OutgoingRequest,
        steps: &[ScriptStep],
        prepared: Option<Prepared>,
    ) -> Execution {
        let mut logs = Vec::new();
        let mut tests = Vec::new();
        let mut outgoing = outgoing.clone();
        for step in steps {
            let Some(source) = step.scripts.get(ScriptPhase::PreRequest) else {
                continue;
            };
            let job = match self.job(&outgoing, ScriptPhase::PreRequest, source) {
                Ok(job) => job,
                Err(error) => return Execution::failed(error.user_text(), logs, tests),
            };
            match self.inner.scripts.run(job).await {
                Ok(outcome) => {
                    apply(&mut outgoing, outcome.request);
                    self.set_runtime_variables(&outgoing, outcome.variables);
                    logs.extend(outcome.logs);
                    tests.extend(outcome.tests);
                }
                Err(error) => {
                    let text = error.user_text_of(ScriptPhase::PreRequest, step.owner.as_deref());
                    return Execution::failed(text, logs, tests);
                }
            }
        }
        let prepared = match prepared.map_or_else(|| self.prepare(&outgoing), Ok) {
            Ok(prepared) => prepared,
            Err(error) => return Execution::failed(error.user_text(), logs, tests),
        };
        let sent = prepared.request.clone();
        let masking = prepared.masking.clone();
        let response = self.inner.send(prepared).await;
        self.record_history(&outgoing, &sent, &response, &masking);
        let mut script_error = None;
        if let Ok(received) = &response {
            for step in steps.iter().rev() {
                let Some(source) = step.scripts.get(ScriptPhase::PostResponse) else {
                    continue;
                };
                let job = self
                    .job(&outgoing, ScriptPhase::PostResponse, source)
                    .map(|mut job| {
                        job.request = sent_request(&sent);
                        job.response = Some(script_response(received));
                        job
                    });
                let job = match job {
                    Ok(job) => job,
                    Err(error) => {
                        script_error = Some(error.user_text());
                        break;
                    }
                };
                match self.inner.scripts.run(job).await {
                    Ok(outcome) => {
                        self.set_runtime_variables(&outgoing, outcome.variables);
                        logs.extend(outcome.logs);
                        tests.extend(outcome.tests);
                    }
                    Err(error) => {
                        let owner = step.owner.as_deref();
                        script_error = Some(error.user_text_of(ScriptPhase::PostResponse, owner));
                        break;
                    }
                }
            }
        }
        Execution {
            response,
            logs,
            script_error,
            tests,
        }
    }

    /// A script job for `outgoing`: the request as entered and every
    /// variable that applies.
    fn job(
        &self,
        outgoing: &OutgoingRequest,
        phase: ScriptPhase,
        source: &str,
    ) -> Result<ScriptJob, AppError> {
        let context = self.context(outgoing.context.as_ref())?;
        let own = VariableDraft::to_plain_variables(&outgoing.variables);
        let runtime = self.runtime_variables(outgoing);
        let mut variables: Vec<(String, String)> = scope(context.as_ref(), &own, &runtime)
            .values()
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect();
        variables.sort();
        Ok(ScriptJob {
            phase,
            source: source.to_owned(),
            request: entered_request(outgoing),
            response: None,
            variables,
        })
    }

    /// The variables scripts set for the collection of `outgoing`.
    pub(super) fn runtime_variables(&self, outgoing: &OutgoingRequest) -> Vec<Variable> {
        let all = self
            .inner
            .runtime_variables
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        all.get(&collection_of(outgoing)).map_or_default(|values| {
            values
                .iter()
                .map(|(name, value)| Variable::plain(name, value))
                .collect()
        })
    }

    fn set_runtime_variables(&self, outgoing: &OutgoingRequest, set: Vec<(String, String)>) {
        if set.is_empty() {
            return;
        }
        let mut all = self
            .inner
            .runtime_variables
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        all.entry(collection_of(outgoing)).or_default().extend(set);
    }

    /// Forgets what scripts set for a collection (when it is closed).
    pub(super) fn forget_runtime_variables(&self, collection_id: &str) {
        self.inner
            .runtime_variables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&Some(collection_id.to_owned()));
    }
}

fn collection_of(outgoing: &OutgoingRequest) -> Option<String> {
    outgoing
        .context
        .as_ref()
        .map(|context| context.collection_id.clone())
}

/// The request as entered, for the pre-request script.
fn entered_request(outgoing: &OutgoingRequest) -> ScriptRequest {
    ScriptRequest {
        method: outgoing.method.clone(),
        url: outgoing.url.clone(),
        headers: outgoing
            .headers
            .iter()
            .filter(|header| header.enabled && !header.name.trim().is_empty())
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect(),
        body: match &outgoing.body {
            BodyDraft::Json { content } | BodyDraft::Text { content } => Some(content.clone()),
            _ => None,
        },
    }
}

/// Takes over what the pre-request script changed.
fn apply(outgoing: &mut OutgoingRequest, request: ScriptRequest) {
    outgoing.method = request.method;
    outgoing.url = request.url;
    outgoing.headers = request
        .headers
        .into_iter()
        .map(|(name, value)| HeaderDraft {
            name,
            value,
            enabled: true,
        })
        .collect();
    if let (Some(text), BodyDraft::Json { content } | BodyDraft::Text { content }) =
        (request.body, &mut outgoing.body)
    {
        *content = text;
    }
}

/// The request as sent, for the post-response script.
fn sent_request(request: &ResolvedRequest) -> ScriptRequest {
    ScriptRequest {
        method: request.method.as_str().to_owned(),
        url: request.url.to_string(),
        headers: request
            .headers
            .iter()
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect(),
        body: request
            .body_bytes()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
    }
}

fn script_response(response: &Response) -> ScriptResponse {
    ScriptResponse {
        status: response.status.as_u16(),
        headers: response
            .headers
            .iter()
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect(),
        body: String::from_utf8_lossy(&response.body).into_owned(),
        time_ms: u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
#[path = "../tests/app_scripts.rs"]
mod tests;
