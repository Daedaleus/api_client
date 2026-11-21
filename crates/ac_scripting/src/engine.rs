//! One script run in a fresh `QuickJS` runtime with a time and a memory limit
//! (ADR S1): the input goes in as JSON, the prelude builds the API from it,
//! the script runs inside an async function, the result comes out as JSON.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ac_app::ports::{ScriptError, ScriptJob, ScriptOutcome, ScriptRequest};
use rquickjs::{Context, Ctx, Function, Promise, Runtime, Value};
use serde::Deserialize;
use serde_json::json;

use crate::strip::strip;

/// The API implementation, see `prelude.js`.
const PRELUDE: &str = include_str!("prelude.js");

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
    let javascript = strip(&job.source)?;
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
    let result = context.with(|ctx| execute(&ctx, job, &javascript));
    result.map_err(|failure| match failure {
        _ if timed_out.load(Ordering::Relaxed) => ScriptError::TimedOut {
            seconds: limits.time.as_secs(),
        },
        Failure::Thrown(message) if message.contains("out of memory") => ScriptError::OutOfMemory {
            megabytes: limits.memory / (1024 * 1024),
        },
        Failure::Thrown(message) => ScriptError::Failed(message),
    })
}

/// Why a run ended without a result (timeouts are told apart by the caller).
enum Failure {
    Thrown(String),
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
    let thrown = |error: rquickjs::Error| Failure::Thrown(describe(ctx, &error));
    ctx.globals()
        .set("__input", input.to_string())
        .map_err(thrown)?;
    ctx.eval::<(), _>(PRELUDE).map_err(thrown)?;
    // An async function, so scripts can `await` (later API calls return
    // promises); its jobs run until it settles.
    let wrapped = format!("(async () => {{\n{javascript}\n}})()");
    let promise: Promise = ctx
        .eval::<Value, _>(wrapped)
        .map_err(thrown)?
        .into_promise()
        .ok_or_else(|| Failure::Thrown("script did not start".into()))?;
    promise.finish::<Value>().map_err(thrown)?;
    let output: Function = ctx.globals().get("__output").map_err(thrown)?;
    let text: String = output.call(()).map_err(thrown)?;
    let output: Output =
        serde_json::from_str(&text).map_err(|error| Failure::Thrown(error.to_string()))?;
    Ok(ScriptOutcome {
        request: ScriptRequest {
            method: output.request.method,
            url: output.request.url,
            headers: output.request.headers,
            body: output.request.body,
        },
        variables: output.variables,
        logs: output.logs,
    })
}

/// The message of a JavaScript exception (`Error: message` style).
fn describe(ctx: &Ctx<'_>, error: &rquickjs::Error) -> String {
    if !error.is_exception() {
        return error.to_string();
    }
    let caught = ctx.catch();
    if let Some(exception) = caught.as_exception() {
        let message = exception.message().unwrap_or_default();
        return match caught
            .as_object()
            .and_then(|object| object.get::<_, String>("name").ok())
        {
            Some(name) if !name.is_empty() => format!("{name}: {message}"),
            _ => message,
        };
    }
    // Thrown values that are not errors (`throw 'text'`).
    caught
        .as_string()
        .and_then(|text| text.to_string().ok())
        .unwrap_or_else(|| format!("{caught:?}"))
}

#[cfg(test)]
#[path = "tests/engine.rs"]
mod tests;
