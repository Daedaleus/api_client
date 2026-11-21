//! Running request scripts (ADR S1: `QuickJS` with TypeScript stripped by
//! oxc, in `ac_scripting`).

use std::future::Future;
use std::sync::Arc;

use ac_domain::script::ScriptPhase;

use crate::text::{UserText, keys};

/// Runs the scripts of requests.
pub trait ScriptRunner: Send + Sync + 'static {
    /// Runs one script with a fresh, limited engine.
    fn run(
        &self,
        job: ScriptJob,
    ) -> impl Future<Output = Result<ScriptOutcome, ScriptError>> + Send;
}

impl<T: ScriptRunner> ScriptRunner for Arc<T> {
    fn run(
        &self,
        job: ScriptJob,
    ) -> impl Future<Output = Result<ScriptOutcome, ScriptError>> + Send {
        T::run(self, job)
    }
}

/// One script run: the script and what it sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptJob {
    /// Before sending or after the response.
    pub phase: ScriptPhase,
    /// TypeScript or JavaScript.
    pub source: String,
    /// Before sending: the request as entered (with `{{variables}}`), which
    /// the script may change. After the response: the request as sent.
    pub request: ScriptRequest,
    /// After the response only.
    pub response: Option<ScriptResponse>,
    /// What `env.get` returns: every variable that applies, by name.
    pub variables: Vec<(String, String)>,
}

/// The request as a script sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptRequest {
    /// Method, e.g. `POST`.
    pub method: String,
    /// URL.
    pub url: String,
    /// Headers that are sent, in order.
    pub headers: Vec<(String, String)>,
    /// The body if it is text (JSON or plain text); `None` for other bodies,
    /// which scripts can neither read nor replace.
    pub body: Option<String>,
}

/// The response as a script sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptResponse {
    /// Status code.
    pub status: u16,
    /// Headers as received.
    pub headers: Vec<(String, String)>,
    /// Body as text (invalid UTF-8 replaced).
    pub body: String,
    /// Time until the whole response was read.
    pub time_ms: u64,
}

/// What a script did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptOutcome {
    /// The request after the script (only used before sending).
    pub request: ScriptRequest,
    /// Variables the script set with `env.set`, in order.
    pub variables: Vec<(String, String)>,
    /// Lines the script logged.
    pub logs: Vec<String>,
}

/// Why a script did not finish.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScriptError {
    /// The script is not valid TypeScript.
    #[error("syntax error in line {line}: {message}")]
    Syntax {
        /// 1-based line in the script.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// The script threw an error.
    #[error("{0}")]
    Failed(String),
    /// The script ran longer than allowed.
    #[error("stopped after {seconds} s")]
    TimedOut {
        /// The time limit.
        seconds: u64,
    },
    /// The script used more memory than allowed.
    #[error("used more than {megabytes} MB of memory")]
    OutOfMemory {
        /// The memory limit.
        megabytes: usize,
    },
}

impl ScriptError {
    /// The text to show the user, naming the phase.
    #[must_use]
    pub fn user_text(&self, phase: ScriptPhase) -> UserText {
        let key = match phase {
            ScriptPhase::PreRequest => keys::SCRIPT_PRE_REQUEST,
            ScriptPhase::PostResponse => keys::SCRIPT_POST_RESPONSE,
        };
        UserText::new(key).with("detail", self.to_string())
    }
}
