//! One script run in a fresh `QuickJS` runtime with a time and a memory limit
//! (ADR S1): the input goes in as JSON, the prelude builds the API from it,
//! the script runs inside an async function, the result comes out as JSON.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ac_app::ports::{ScriptError, ScriptJob, ScriptOutcome, ScriptRequest, TestResult};
use rquickjs::context::EvalOptions;
use rquickjs::{Context, Ctx, Function, Promise, Runtime, Value};
use serde::Deserialize;
use serde_json::json;

use crate::strip::{Stripped, strip};

/// The API implementation, see `prelude.js`.
const PRELUDE: &str = include_str!("prelude.js");
/// The file name of the script in stack traces.
const SCRIPT_FILE: &str = "script.js";
/// The script runs inside an async function, so scripts can `await`; then
/// async tests settle.
const WRAP_START: &str = "(async () => {\n";
const WRAP_END: &str = "\n})().then(() => __settle())";
/// Lines in front of the script's first line.
const WRAP_LINES: u32 = 1;

/// What a script may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Wall-clock time per run.
    pub time: Duration,
    /// Memory of the engine per run, in bytes.
    pub memory: usize,
}

impl Default for Limits {
    /// 5 s and 64 MB: plenty for test scripts, little for a runaway one.
    fn default() -> Self {
        Self {
            time: Duration::from_secs(5),
            memory: 64 * 1024 * 1024,
        }
    }
}

/// The result as the prelude reports it.
#[derive(Debug, Deserialize)]
struct Output {
    request: OutputRequest,
    variables: Vec<(String, String)>,
    logs: Vec<String>,
    tests: Vec<OutputTest>,
}

#[derive(Debug, Deserialize)]
struct OutputTest {
    name: String,
    passed: bool,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OutputRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

/// Runs `job` to the end (blocking).
pub(crate) fn run(job: &ScriptJob, limits: Limits) -> Result<ScriptOutcome, ScriptError> {
    let stripped = strip(&job.source)?;
    let runtime = Runtime::new().map_err(|error| ScriptError::Failed(error.to_string()))?;
    runtime.set_memory_limit(limits.memory);
    let timed_out = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&timed_out);
    let start = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || {
        let over = start.elapsed() > limits.time;
        if over {
            flag.store(true, Ordering::Relaxed);
        }
        over
    })));
    let context =
        Context::full(&runtime).map_err(|error| ScriptError::Failed(error.to_string()))?;
    let result = context.with(|ctx| execute(&ctx, job, &stripped.code));
    result.map_err(|failure| match failure {
        _ if timed_out.load(Ordering::Relaxed) => ScriptError::TimedOut {
            seconds: limits.time.as_secs(),
        },
        Failure::Thrown { message, .. } if message.contains("out of memory") => {
            ScriptError::OutOfMemory {
                megabytes: limits.memory / (1024 * 1024),
            }
        }
        Failure::Thrown { message, stack } => {
            match stack.and_then(|stack| script_line(&stack, &stripped)) {
                Some(line) => ScriptError::Thrown { line, message },
                None => ScriptError::Failed(message),
            }
        }
    })
}

/// Why a run ended without a result (timeouts are told apart by the caller).
enum Failure {
    Thrown {
        message: String,
        /// The error's stack trace, if it has one.
        stack: Option<String>,
    },
}

impl Failure {
    fn message(message: impl Into<String>) -> Self {
        Self::Thrown {
            message: message.into(),
            stack: None,
        }
    }
}

/// The line of the script where the error happened: the innermost frame of
/// the script in `stack` (`at f (script.js:3:7)`), mapped back to the script
/// as written.
fn script_line(stack: &str, stripped: &Stripped) -> Option<usize> {
    let marker = format!("{SCRIPT_FILE}:");
    let start = stack.find(&marker)? + marker.len();
    let mut numbers = stack[start..]
        .split(|c: char| !c.is_ascii_digit())
        .take(2)
        .map(str::parse::<u32>);
    let line = numbers.next()?.ok()?;
    // Columns are 1-based where QuickJS gives one.
    let column = numbers.next().and_then(Result::ok).unwrap_or(1);
    stripped.source_line(line.checked_sub(1 + WRAP_LINES)?, column.saturating_sub(1))
}

fn execute(ctx: &Ctx<'_>, job: &ScriptJob, javascript: &str) -> Result<ScriptOutcome, Failure> {
    let input = json!({
        "request": {
            "method": job.request.method,
            "url": job.request.url,
            "headers": job.request.headers,
            "body": job.request.body,
        },
        "response": job.response.as_ref().map(|response| json!({
            "status": response.status,
            "timeMs": response.time_ms,
            "headers": response.headers,
            "body": response.body,
        })),
        "variables": job
            .variables
            .iter()
            .map(|(name, value)| (name.clone(), serde_json::Value::from(value.as_str())))
            .collect::<serde_json::Map<_, _>>(),
    });
    let thrown = |error: rquickjs::Error| describe(ctx, &error);
    ctx.globals()
        .set("__input", input.to_string())
        .map_err(thrown)?;
    ctx.eval::<(), _>(PRELUDE).map_err(thrown)?;
    let mut options = EvalOptions::default();
    options.filename = Some(SCRIPT_FILE.into());
    let promise: Promise = ctx
        .eval_with_options::<Value, _>(format!("{WRAP_START}{javascript}{WRAP_END}"), options)
        .map_err(thrown)?
        .into_promise()
        .ok_or_else(|| Failure::message("script did not start"))?;
    promise.finish::<Value>().map_err(thrown)?;
    let output: Function = ctx.globals().get("__output").map_err(thrown)?;
    let text: String = output.call(()).map_err(thrown)?;
    let output: Output =
        serde_json::from_str(&text).map_err(|error| Failure::message(error.to_string()))?;
    Ok(ScriptOutcome {
        request: ScriptRequest {
            method: output.request.method,
            url: output.request.url,
            headers: output.request.headers,
            body: output.request.body,
        },
        variables: output.variables,
        logs: output.logs,
        tests: output
            .tests
            .into_iter()
            .map(|test| TestResult {
                name: test.name,
                passed: test.passed,
                message: test.message,
            })
            .collect(),
    })
}

/// A JavaScript exception: its message (`Error: message` style) and stack.
fn describe(ctx: &Ctx<'_>, error: &rquickjs::Error) -> Failure {
    if !error.is_exception() {
        return Failure::message(error.to_string());
    }
    let caught = ctx.catch();
    if let Some(exception) = caught.as_exception() {
        let message = exception.message().unwrap_or_default();
        let message = match caught
            .as_object()
            .and_then(|object| object.get::<_, String>("name").ok())
        {
            Some(name) if !name.is_empty() => format!("{name}: {message}"),
            _ => message,
        };
        return Failure::Thrown {
            message,
            stack: exception.stack(),
        };
    }
    // Thrown values that are not errors (`throw 'text'`).
    Failure::message(
        caught
            .as_string()
            .and_then(|text| text.to_string().ok())
            .unwrap_or_else(|| format!("{caught:?}")),
    )
}

#[cfg(test)]
#[path = "tests/engine.rs"]
mod tests;
