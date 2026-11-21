//! Test double for running scripts.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::lock;
use crate::ports::{ScriptError, ScriptJob, ScriptOutcome, ScriptRunner};

/// A [`ScriptRunner`] that answers with queued results and records the jobs.
/// Without a queued result a script changes nothing and logs nothing.
#[derive(Debug, Default)]
pub struct FakeScriptRunner {
    results: Mutex<VecDeque<Result<ScriptOutcome, ScriptError>>>,
    jobs: Mutex<Vec<ScriptJob>>,
}

impl FakeScriptRunner {
    /// A runner without queued results.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the result of the next script.
    #[must_use]
    pub fn result(self, result: Result<ScriptOutcome, ScriptError>) -> Self {
        lock(&self.results).push_back(result);
        self
    }

    /// All jobs run so far.
    #[must_use]
    pub fn jobs(&self) -> Vec<ScriptJob> {
        lock(&self.jobs).clone()
    }
}

impl ScriptRunner for FakeScriptRunner {
    fn run(
        &self,
        job: ScriptJob,
    ) -> impl Future<Output = Result<ScriptOutcome, ScriptError>> + Send {
        let result = lock(&self.results).pop_front().unwrap_or_else(|| {
            Ok(ScriptOutcome {
                request: job.request.clone(),
                ..ScriptOutcome::default()
            })
        });
        lock(&self.jobs).push(job);
        std::future::ready(result)
    }
}
