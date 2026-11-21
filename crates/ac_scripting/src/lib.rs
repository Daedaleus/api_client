//! Script adapter: implements the [`ScriptRunner`] port with `QuickJS`
//! (`rquickjs`, quickjs-ng); TypeScript is stripped with oxc first
//! ([ADR S1](../../../docs/adr/s1-script-runtime.md)).
//!
//! - this file: [`QuickJsRunner`] and the API declarations.
//! - `strip`: TypeScript → JavaScript.
//! - `engine`: one run in a fresh, limited runtime.
//! - `prelude.js`: the script API, built from the run's input in JavaScript.
//!
//! Scripts reach nothing but the API in [`API_DECLARATIONS`]: `QuickJS` has no
//! file, network or timer functions, and the host adds none.

mod engine;
mod strip;

pub use engine::Limits;

use ac_app::ports::{ScriptError, ScriptJob, ScriptOutcome, ScriptRunner};

/// The script API as TypeScript declarations – for the checker, completion
/// and documentation.
pub const API_DECLARATIONS: &str = include_str!("../api/script-api.d.ts");

/// Runs every script in a fresh `QuickJS` runtime on Tokio's blocking pool.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuickJsRunner {
    limits: Limits,
}

impl QuickJsRunner {
    /// A runner with these limits per run.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self { limits }
    }
}

impl ScriptRunner for QuickJsRunner {
    async fn run(&self, job: ScriptJob) -> Result<ScriptOutcome, ScriptError> {
        let limits = self.limits;
        tokio::task::spawn_blocking(move || engine::run(&job, limits))
            .await
            .unwrap_or_else(|error| Err(ScriptError::Failed(error.to_string())))
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
