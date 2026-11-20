//! The facade every user interface talks to.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::runtime::Handle;
use tokio::sync::broadcast;

use crate::command::{AppError, Command, CommandAck, RunId};
use crate::event::AppEvent;
use crate::ports::HttpSender;
use crate::resolve::resolve_request;
use crate::view::{ResponseView, RunOutcomeView};

/// Events kept for slow subscribers before they start missing some.
const EVENT_CAPACITY: usize = 64;

/// The adapters the application works with.
pub struct Ports<H> {
    /// Sends HTTP requests.
    pub http: H,
}

/// The application: send [`Command`]s, receive [`AppEvent`]s.
///
/// Cheap to clone; clones share state.
pub struct App<H> {
    inner: Arc<Inner<H>>,
}

struct Inner<H> {
    http: H,
    /// Background work runs here, whichever thread calls `execute` – UIs
    /// may call from threads without a runtime (Tauri: the main thread).
    runtime: Handle,
    events: broadcast::Sender<AppEvent>,
    next_run: AtomicU64,
}

impl<H> Clone for App<H> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<H: HttpSender> App<H> {
    /// Creates the application with its adapters; background work runs on
    /// `runtime`.
    #[must_use]
    pub fn new(ports: Ports<H>, runtime: Handle) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                http: ports.http,
                runtime,
                events,
                next_run: AtomicU64::new(1),
            }),
        }
    }

    /// Receives all events from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.inner.events.subscribe()
    }

    /// Carries out a command. Long work continues in the background and
    /// reports through [`App::subscribe`]. May be called from any thread.
    ///
    /// # Errors
    /// [`AppError`] if the command cannot be carried out as given.
    pub fn execute(&self, command: Command) -> Result<CommandAck, AppError> {
        match command {
            Command::SendRequest { method, url } => self.send_request(&method, &url),
        }
    }

    fn send_request(&self, method: &str, url: &str) -> Result<CommandAck, AppError> {
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
#[path = "tests/app.rs"]
mod tests;
