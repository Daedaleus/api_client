//! Checking and completing scripts (ADR S2): the TypeScript language
//! service in a long-lived `QuickJS` runtime on a thread of its own.
//!
//! - this file: [`TsAssist`], the [`ScriptAssist`] adapter.
//! - `files`: the compiler and library declarations, and their archive.
//! - `service.js`: the language service set-up and the functions called.
//!
//! The service starts with the first question (about a second: loading the
//! compiler and parsing the libraries), not with the app. Questions wait in
//! order; each gets an answer through a channel, so callers never block a
//! UI thread.

mod files;

pub use files::{COMPILER, TypeScriptFiles, is_library};

use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use ac_app::ports::{
    AssistError, CompletionKind, ScriptAssist, ScriptCompletion, ScriptDiagnostic, Severity,
};
use rquickjs::{Context, Function, Object, Runtime, Value};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::API_DECLARATIONS;

/// The service set-up, see `service.js`.
const SERVICE: &str = include_str!("service.js");
/// Time for starting the service (loading the compiler, parsing libraries).
const START_LIMIT: Duration = Duration::from_secs(60);
/// Time for one check or completion.
const ANSWER_LIMIT: Duration = Duration::from_secs(10);
/// The compiler recurses deeply; the default stack of `QuickJS` is 256 KiB.
const STACK_SIZE: usize = 8 * 1024 * 1024;
/// The service thread's own stack: room for the `QuickJS` stack and the Rust
/// frames below it (debug builds need more).
const THREAD_STACK_SIZE: usize = 4 * STACK_SIZE;

/// Where the files come from, called once, on the service's thread.
type Loader = Box<dyn FnOnce() -> Result<TypeScriptFiles, String> + Send>;

/// [`ScriptAssist`] with the TypeScript language service.
#[derive(Debug)]
pub struct TsAssist {
    jobs: Mutex<mpsc::Sender<Job>>,
}

enum Job {
    Check(
        String,
        oneshot::Sender<Result<Vec<ScriptDiagnostic>, AssistError>>,
    ),
    Complete(
        String,
        usize,
        oneshot::Sender<Result<Vec<ScriptCompletion>, AssistError>>,
    ),
}

impl TsAssist {
    /// A checker that loads its files with `load` when first asked.
    ///
    /// # Panics
    /// If the operating system cannot start the service thread.
    #[must_use]
    pub fn new(load: impl FnOnce() -> Result<TypeScriptFiles, String> + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        let load: Loader = Box::new(load);
        std::thread::Builder::new()
            .name("script-assist".into())
            .stack_size(THREAD_STACK_SIZE)
            .spawn(move || serve(&receiver, load))
            .expect("the operating system can start a thread");
        Self {
            jobs: Mutex::new(sender),
        }
    }

    fn send(&self, job: Job) {
        // A send only fails when the service thread ended; the job's channel
        // is dropped then and the caller gets an error from it.
        let _ = self
            .jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(job);
    }
}

impl ScriptAssist for TsAssist {
    async fn check(&self, source: String) -> Result<Vec<ScriptDiagnostic>, AssistError> {
        let (answer, receiver) = oneshot::channel();
        self.send(Job::Check(source, answer));
        receiver.await.unwrap_or_else(|_| Err(stopped()))
    }

    async fn complete(
        &self,
        source: String,
        offset: usize,
    ) -> Result<Vec<ScriptCompletion>, AssistError> {
        let (answer, receiver) = oneshot::channel();
        self.send(Job::Complete(source, offset, answer));
        receiver.await.unwrap_or_else(|_| Err(stopped()))
    }
}

fn stopped() -> AssistError {
    AssistError("the checker stopped".into())
}

/// The service thread: starts the service with the first job, then answers.
fn serve(jobs: &mpsc::Receiver<Job>, load: Loader) {
    let mut load = Some(load);
    let mut service: Option<Result<Service, AssistError>> = None;
    for job in jobs {
        let started = service.get_or_insert_with(|| {
            let load = load.take().expect("loaded only once");
            Service::start(load)
        });
        match (job, started) {
            (Job::Check(source, answer), Ok(service)) => {
                let _ = answer.send(service.check(&source));
            }
            (Job::Complete(source, offset, answer), Ok(service)) => {
                let _ = answer.send(service.complete(&source, offset));
            }
            (Job::Check(_, answer), Err(error)) => {
                let _ = answer.send(Err(error.clone()));
            }
            (Job::Complete(_, _, answer), Err(error)) => {
                let _ = answer.send(Err(error.clone()));
            }
        }
    }
}

/// The running language service.
struct Service {
    // Kept alive with the context.
    _runtime: Runtime,
    context: Context,
    deadline: Arc<Mutex<Instant>>,
}

#[derive(Deserialize)]
struct RawDiagnostic {
    from: usize,
    to: usize,
    severity: String,
    message: String,
}

#[derive(Deserialize)]
struct RawCompletion {
    label: String,
    kind: String,
}

impl Service {
    fn start(load: Loader) -> Result<Self, AssistError> {
        let files = load().map_err(AssistError)?;
        let failed = |error: rquickjs::Error| AssistError(format!("cannot start: {error}"));
        let runtime = Runtime::new().map_err(failed)?;
        runtime.set_max_stack_size(STACK_SIZE);
        let deadline = Arc::new(Mutex::new(Instant::now() + START_LIMIT));
        let limit = Arc::clone(&deadline);
        runtime.set_interrupt_handler(Some(Box::new(move || {
            Instant::now() > *limit.lock().unwrap_or_else(PoisonError::into_inner)
        })));
        let context = Context::full(&runtime).map_err(failed)?;
        context.with(|ctx| -> Result<(), AssistError> {
            let thrown = |error: rquickjs::Error| {
                let exception = ctx.catch();
                AssistError(format!("cannot start: {error} {exception:?}"))
            };
            ctx.eval::<Value, _>(files.compiler).map_err(thrown)?;
            let declarations = Object::new(ctx.clone()).map_err(thrown)?;
            declarations
                .set("/api.d.ts", API_DECLARATIONS)
                .map_err(thrown)?;
            for (name, text) in files.libraries {
                declarations.set(format!("/{name}"), text).map_err(thrown)?;
            }
            ctx.globals().set("__files", declarations).map_err(thrown)?;
            ctx.eval::<Value, _>(SERVICE).map_err(thrown)?;
            Ok(())
        })?;
        Ok(Self {
            _runtime: runtime,
            context,
            deadline,
        })
    }

    /// Calls `function` of `service.js` and parses its JSON answer.
    fn call<T: for<'de> Deserialize<'de>>(
        &self,
        function: &str,
        source: &str,
        offset: Option<usize>,
    ) -> Result<T, AssistError> {
        *self.deadline.lock().unwrap_or_else(PoisonError::into_inner) =
            Instant::now() + ANSWER_LIMIT;
        let text = self.context.with(|ctx| -> rquickjs::Result<String> {
            let call: Function = ctx.globals().get(function)?;
            match offset {
                Some(offset) => call.call((source, offset)),
                None => call.call((source,)),
            }
        });
        let text = text.map_err(|error| AssistError(error.to_string()))?;
        serde_json::from_str(&text).map_err(|error| AssistError(error.to_string()))
    }

    fn check(&self, source: &str) -> Result<Vec<ScriptDiagnostic>, AssistError> {
        let raw: Vec<RawDiagnostic> = self.call("__check", source, None)?;
        Ok(raw
            .into_iter()
            .map(|diagnostic| ScriptDiagnostic {
                from: diagnostic.from,
                to: diagnostic.to,
                severity: match diagnostic.severity.as_str() {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Hint,
                },
                message: diagnostic.message,
            })
            .collect())
    }

    fn complete(&self, source: &str, offset: usize) -> Result<Vec<ScriptCompletion>, AssistError> {
        let raw: Vec<RawCompletion> = self.call("__complete", source, Some(offset))?;
        Ok(raw
            .into_iter()
            .map(|completion| ScriptCompletion {
                label: completion.label,
                kind: match completion.kind.as_str() {
                    "function" => CompletionKind::Function,
                    "property" => CompletionKind::Property,
                    "variable" => CompletionKind::Variable,
                    "type" => CompletionKind::Type,
                    "keyword" => CompletionKind::Keyword,
                    _ => CompletionKind::Other,
                },
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "../tests/assist.rs"]
mod tests;
