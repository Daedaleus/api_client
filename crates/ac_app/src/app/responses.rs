//! The last responses, byte for byte, in memory only: the UI shows a text
//! version of a body (shortened when large, nothing for binary data), so
//! saving a body to a file – or a response as an example – needs the
//! original.

use std::collections::VecDeque;

use ac_domain::exchange::Response;
use std::path::Path;
use std::sync::PoisonError;

use super::App;
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::ports::CollectionStore;

/// How many responses are kept (the oldest goes first).
pub(super) const KEPT_RESPONSES: usize = 20;

/// The kept responses, newest last.
#[derive(Debug, Default)]
pub(super) struct KeptResponses(VecDeque<(RunId, Response)>);

impl KeptResponses {
    pub(super) fn keep(&mut self, run_id: RunId, response: Response) {
        if self.0.len() == KEPT_RESPONSES {
            self.0.pop_front();
        }
        self.0.push_back((run_id, response));
    }

    pub(super) fn get(&self, run_id: RunId) -> Option<&Response> {
        self.0
            .iter()
            .find(|(id, _)| *id == run_id)
            .map(|(_, response)| response)
    }
}

impl<H, S: CollectionStore, V, B, R, A, Y, Z> App<H, S, V, B, R, A, Y, Z> {
    /// Remembers a run's response.
    pub(super) fn keep_response(&self, run_id: RunId, response: Response) {
        self.inner
            .responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(run_id, response);
    }

    /// A kept response (a copy).
    pub(super) fn kept_response(&self, run_id: RunId) -> Result<Response, AppError> {
        self.inner
            .responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(run_id)
            .cloned()
            .ok_or(AppError::ResponseGone)
    }

    pub(super) fn save_response_body(
        &self,
        run_id: RunId,
        file: &str,
    ) -> Result<CommandAck, AppError> {
        let responses = self
            .inner
            .responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let response = responses.get(run_id).ok_or(AppError::ResponseGone)?;
        self.inner
            .store
            .write_file(Path::new(file), &response.body)?;
        Ok(CommandAck::ResponseSaved)
    }
}

#[cfg(test)]
#[path = "../tests/app_responses.rs"]
mod tests;
