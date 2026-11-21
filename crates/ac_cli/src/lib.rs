//! Command line runner: runs the requests of a collection – all, a folder or
//! one request – with an environment and reports the results, for CI.
//!
//! ```text
//! api-client-cli run ./my-api --env Staging --only Auth --format junit > report.xml
//! ```
//!
//! Exit code 0 when every request passed, 1 when one failed (no response, or
//! a status of 400 or more unless `--allow-error-status`), 2 when the run
//! could not start (no collection, unknown environment …).
//!
//! - this file: the run – wiring the adapters, the exit code.
//! - `args`: the command line.
//! - `report`: text, JSON and `JUnit` output.
//! - `texts`: translated texts from `locales/app.json` (language from
//!   `LC_ALL`, `LC_MESSAGES` or `LANG`).
//!
//! Secrets are decrypted with the user's key, or in CI with the key in
//! `API_CLIENT_IDENTITY` (ADR S4).

mod args;
mod report;
mod texts;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use ac_app::{App, Command, CommandAck, Ports, RunPlan};
use ac_http::{DEFAULT_TIMEOUT, LoopbackLogin, ReqwestSender};
use ac_secrets::{AgeVault, KeyConfig};
use ac_storage::FileStore;
use clap::Parser;

pub use args::{Args, Format, RunArgs};
pub use texts::Texts;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every request passed.
    Passed,
    /// At least one request failed.
    Failed,
    /// The run could not start.
    Error,
}

impl From<Outcome> for ExitCode {
    fn from(outcome: Outcome) -> Self {
        Self::from(match outcome {
            Outcome::Passed => 0,
            Outcome::Failed => 1,
            Outcome::Error => 2,
        })
    }
}

/// The program: parses `arguments` and runs.
pub fn main(arguments: impl IntoIterator<Item = std::ffi::OsString>) -> ExitCode {
    let texts = Texts::from_environment();
    let args = match Args::try_parse_from(arguments) {
        Ok(args) => args,
        Err(error) => {
            // Help and version are not errors.
            let _ = error.print();
            return if error.use_stderr() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    let args::Command::Run(run_args) = args.command;
    let identity = config_dir().join(IDENTITY_FILE);
    let outcome = run(
        &run_args,
        &texts,
        KeyConfig::from_environment(identity),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    outcome.into()
}

/// The user's key file name in the app's config folder (as the desktop app).
const IDENTITY_FILE: &str = "identity.txt";
/// The desktop app's identifier, which names its config folder.
const APP_ID: &str = "io.github.daedaleus.apiclient";

/// The app's config folder, where the desktop app keeps the user's key.
fn config_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::home_dir().map(|home| home.join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::home_dir().map(|home| home.join(".config")))
    };
    base.unwrap_or_default().join(APP_ID)
}

/// Runs a collection as `args` says and writes the report to `out`, errors
/// to `err`.
pub fn run(
    args: &RunArgs,
    texts: &Texts,
    keys: KeyConfig,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Outcome {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = writeln!(err, "{error}");
            return Outcome::Error;
        }
    };
    let http = match ReqwestSender::new(DEFAULT_TIMEOUT) {
        Ok(http) => http,
        Err(error) => {
            let _ = writeln!(err, "{error}");
            return Outcome::Error;
        }
    };
    let app = App::new(
        Ports {
            http,
            store: FileStore::new(),
            vault: AgeVault::new(keys),
            login: LoopbackLogin::new(),
        },
        runtime.handle().clone(),
    );
    let fail = |error: ac_app::AppError, err: &mut dyn Write| {
        let _ = writeln!(err, "{}", texts.error(&error.user_text()));
        Outcome::Error
    };
    let collection = match app.execute(Command::OpenCollection {
        path: args.collection.display().to_string(),
    }) {
        Ok(CommandAck::CollectionChanged { collection, .. }) => collection,
        Ok(_) => return Outcome::Error,
        Err(error) => return fail(error, err),
    };
    let plan = RunPlan {
        environment: args.env.clone(),
        only: args.only.clone(),
    };
    let report = match runtime.block_on(app.run(&collection.id, &plan)) {
        Ok(report) => report,
        Err(error) => return fail(error, err),
    };
    let error_status_fails = !args.allow_error_status;
    let written = match args.format {
        Format::Text => report::text(
            &report,
            &collection.name,
            args.env.as_deref(),
            error_status_fails,
            texts,
        ),
        Format::Json => report::json(&report, error_status_fails),
        Format::Junit => report::junit(&report, &collection.name, error_status_fails, texts),
    };
    let _ = out.write_all(written.as_bytes());
    if report.failed(error_status_fails) == 0 {
        Outcome::Passed
    } else {
        Outcome::Failed
    }
}

#[cfg(test)]
#[path = "tests/run.rs"]
mod tests;
