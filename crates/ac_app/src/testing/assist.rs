//! Test double for checking and completing scripts.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::lock;
use crate::ports::{AssistError, ScriptAssist, ScriptCheck, ScriptCompletion, ScriptDiagnostic};

/// A [`ScriptAssist`] with queued answers; records what it was asked.
/// Without a queued answer it finds nothing and suggests nothing.
#[derive(Debug, Default)]
pub struct FakeScriptAssist {
    checks: Mutex<VecDeque<Result<ScriptCheck, AssistError>>>,
    completions: Mutex<VecDeque<Result<Vec<ScriptCompletion>, AssistError>>>,
    asked: Mutex<Vec<(String, Option<usize>)>>,
}

impl FakeScriptAssist {
    /// An assist without answers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the answer to the next check (a full check).
    #[must_use]
    pub fn diagnostics(self, answer: Result<Vec<ScriptDiagnostic>, AssistError>) -> Self {
        self.check_answer(answer.map(ScriptCheck::complete))
    }

    /// Queues the answer to the next check.
    #[must_use]
    pub fn check_answer(self, answer: Result<ScriptCheck, AssistError>) -> Self {
        lock(&self.checks).push_back(answer);
        self
    }

    /// Queues the answer to the next completion.
    #[must_use]
    pub fn completions(self, answer: Result<Vec<ScriptCompletion>, AssistError>) -> Self {
        lock(&self.completions).push_back(answer);
        self
    }

    /// Scripts asked about, with the offset for completions.
    #[must_use]
    pub fn asked(&self) -> Vec<(String, Option<usize>)> {
        lock(&self.asked).clone()
    }
}

impl ScriptAssist for FakeScriptAssist {
    fn check(
        &self,
        source: String,
    ) -> impl Future<Output = Result<ScriptCheck, AssistError>> + Send {
        lock(&self.asked).push((source, None));
        std::future::ready(
            lock(&self.checks)
                .pop_front()
                .unwrap_or(Ok(ScriptCheck::default())),
        )
    }

    fn complete(
        &self,
        source: String,
        offset: usize,
    ) -> impl Future<Output = Result<Vec<ScriptCompletion>, AssistError>> + Send {
        lock(&self.asked).push((source, Some(offset)));
        std::future::ready(
            lock(&self.completions)
                .pop_front()
                .unwrap_or(Ok(Vec::new())),
        )
    }
}
