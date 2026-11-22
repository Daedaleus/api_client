//! Running the requests of a collection one after another, as a command
//! line runner in CI does: in tree order, with an environment, each request
//! sent exactly as the editor would send it.

use ac_domain::collection::{Item, Request};

use super::App;
use crate::error::AppError;
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretVault};
use crate::ports::{HistoryStore, ScriptAssist};
use crate::view::{
    OutgoingRequest, RequestDraft, RunReport, RunResult, RunResultOutcome, SendContext,
    TestResultView,
};

/// Which requests to run, and with what.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunPlan {
    /// Name of the environment to use, if any.
    pub environment: Option<String>,
    /// Only the folder or request at this path of names (`Auth/Login`);
    /// all requests if `None`.
    pub only: Option<String>,
}

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
> App<H, S, V, B, R, A, Y>
{
    /// Runs the requests of an open collection one after another and
    /// reports what happened. Requests that cannot be sent (an unknown
    /// variable, say) are reported as failed; the run goes on.
    ///
    /// # Errors
    /// If the collection is not open, the environment does not exist or
    /// nothing matches `plan.only`.
    pub async fn run(&self, collection_id: &str, plan: &RunPlan) -> Result<RunReport, AppError> {
        let opened = self.current(collection_id)?;
        let environment_id = match &plan.environment {
            None => None,
            Some(name) => Some(
                opened
                    .environments
                    .iter()
                    .find(|environment| environment.name == *name)
                    .ok_or_else(|| AppError::UnknownEnvironment(name.clone()))?
                    .id
                    .to_string(),
            ),
        };
        let mut requests = Vec::new();
        collect(&opened.collection.items, &mut Vec::new(), &mut requests);
        if let Some(only) = &plan.only {
            let wanted: Vec<&str> = only.split('/').filter(|name| !name.is_empty()).collect();
            requests.retain(|(path, _)| {
                path.iter()
                    .map(String::as_str)
                    .take(wanted.len())
                    .eq(wanted.iter().copied())
            });
            if requests.is_empty() {
                return Err(AppError::NothingToRun(only.clone()));
            }
        }
        let mut results = Vec::with_capacity(requests.len());
        for (path, request) in requests {
            let draft = RequestDraft::from_request(request, false);
            let outgoing = OutgoingRequest {
                method: draft.method.clone(),
                url: draft.url,
                headers: draft.headers,
                body: draft.body,
                variables: draft.variables,
                auth: draft.auth,
                certificate_check: draft.certificate_check,
                scripts: draft.scripts,
                path_params: draft.path_params,
                context: Some(SendContext {
                    collection_id: collection_id.to_owned(),
                    request_id: Some(request.id.to_string()),
                    environment_id: environment_id.clone(),
                }),
            };
            // A failing post-response script fails the request, as a
            // failing test would.
            let execution = self.run_with_scripts(&outgoing).await;
            let outcome = match (execution.response, execution.script_error) {
                (Err(error), _) | (Ok(_), Some(error)) => RunResultOutcome::Failed { error },
                (Ok(response), None) => RunResultOutcome::Response {
                    status: response.status.as_u16(),
                    elapsed_ms: u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX),
                },
            };
            results.push(RunResult {
                path,
                method: draft.method,
                outcome,
                tests: execution
                    .tests
                    .iter()
                    .map(TestResultView::from_result)
                    .collect(),
            });
        }
        Ok(RunReport { results })
    }
}

/// The requests of `items` in tree order, each with its path of names.
fn collect<'a>(
    items: &'a [Item],
    path: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, &'a Request)>,
) {
    for item in items {
        path.push(item.name().to_owned());
        match item {
            Item::Request(request) => out.push((path.clone(), request)),
            Item::Folder(folder) => collect(&folder.items, path, out),
        }
        path.pop();
    }
}

#[cfg(test)]
#[path = "../tests/app_runner.rs"]
mod tests;
