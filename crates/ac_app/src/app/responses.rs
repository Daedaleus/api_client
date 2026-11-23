//! The bodies of the last responses, byte for byte, in memory only: the UI
//! shows a text version (shortened when large, nothing for binary data), so
//! saving a body to a file needs the original bytes.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::PoisonError;

use super::App;
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::ports::CollectionStore;

/// How many responses are kept (the oldest goes first).
pub(super) const KEPT_RESPONSES: usize = 20;

/// The kept bodies, newest last.
#[derive(Debug, Default)]
pub(super) struct ResponseBodies(VecDeque<(RunId, Vec<u8>)>);

impl ResponseBodies {
    pub(super) fn keep(&mut self, run_id: RunId, body: Vec<u8>) {
        if self.0.len() == KEPT_RESPONSES {
            self.0.pop_front();
        }
        self.0.push_back((run_id, body));
    }

    fn get(&self, run_id: RunId) -> Option<&[u8]> {
        self.0
            .iter()
            .find(|(id, _)| *id == run_id)
            .map(|(_, body)| body.as_slice())
    }
}

impl<H, S: CollectionStore, V, B, R, A, Y, Z> App<H, S, V, B, R, A, Y, Z> {
    /// Remembers the body of a run's response.
    pub(super) fn keep_response_body(&self, run_id: RunId, body: Vec<u8>) {
        self.inner
            .responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(run_id, body);
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
        let body = responses.get(run_id).ok_or(AppError::ResponseGone)?;
        self.inner.store.write_file(Path::new(file), body)?;
        Ok(CommandAck::ResponseSaved)
    }
}

#[cfg(test)]
#[path = "../tests/app_responses.rs"]
mod tests;
