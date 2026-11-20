//! Sending requests: validated at once, sent in the background, the result
//! reported as [`AppEvent::RunFinished`].

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::App;
use crate::command::{CommandAck, RunId};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::ports::{CollectionStore, HttpSender};
use crate::resolve::resolve_request;
use crate::view::{ResponseView, RunOutcomeView};

impl<H: HttpSender, S: CollectionStore> App<H, S> {
    pub(super) fn send_request(&self, method: &str, url: &str) -> Result<CommandAck, AppError> {
        let request = resolve_request(method, url)?;
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
