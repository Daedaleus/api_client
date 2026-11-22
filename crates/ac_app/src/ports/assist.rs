//! Checking and completing scripts while they are edited (ADR S2: the
//! TypeScript language service in `QuickJS`, in `ac_scripting`).
//!
//! Positions are offsets in the script as JavaScript counts them (UTF-16
//! code units), the unit of the editors that show the results.

use std::future::Future;
use std::sync::Arc;

use crate::text::{UserText, keys};

/// Checks scripts and suggests completions.
pub trait ScriptAssist: Send + Sync + 'static {
    /// Problems in `source`.
    fn check(
        &self,
        source: String,
    ) -> impl Future<Output = Result<Vec<ScriptDiagnostic>, AssistError>> + Send;

    /// What may be written at `offset` in `source`.
    fn complete(
        &self,
        source: String,
        offset: usize,
    ) -> impl Future<Output = Result<Vec<ScriptCompletion>, AssistError>> + Send;
}

impl<T: ScriptAssist> ScriptAssist for Arc<T> {
    fn check(
        &self,
        source: String,
    ) -> impl Future<Output = Result<Vec<ScriptDiagnostic>, AssistError>> + Send {
        T::check(self, source)
    }

    fn complete(
        &self,
        source: String,
        offset: usize,
    ) -> impl Future<Output = Result<Vec<ScriptCompletion>, AssistError>> + Send {
        T::complete(self, source, offset)
    }
}

/// A problem in a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDiagnostic {
    /// Start offset.
    pub from: usize,
    /// End offset (exclusive).
    pub to: usize,
    /// How bad it is.
    pub severity: Severity,
    /// What is wrong (technical, from the checker).
    pub message: String,
}

/// How bad a problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The script will not work.
    Error,
    /// The script may not work as meant.
    Warning,
    /// A suggestion.
    Hint,
}

/// A completion suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCompletion {
    /// What is inserted.
    pub label: String,
    /// What it is.
    pub kind: CompletionKind,
}

/// What a suggestion is, for its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// A method or function.
    Function,
    /// A property.
    Property,
    /// A variable or constant.
    Variable,
    /// A class, interface or type.
    Type,
    /// A language keyword.
    Keyword,
    /// Anything else.
    Other,
}

/// Why checking or completing failed (e.g. the checker did not start).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct AssistError(pub String);

impl AssistError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        UserText::new(keys::SCRIPT_ASSIST).with("detail", self.0.clone())
    }
}

/// No checker: every script is fine and nothing is suggested (for programs
/// without an editor, like the command line runner).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoAssist;

impl ScriptAssist for NoAssist {
    fn check(
        &self,
        _source: String,
    ) -> impl Future<Output = Result<Vec<ScriptDiagnostic>, AssistError>> + Send {
        std::future::ready(Ok(Vec::new()))
    }

    fn complete(
        &self,
        _source: String,
        _offset: usize,
    ) -> impl Future<Output = Result<Vec<ScriptCompletion>, AssistError>> + Send {
        std::future::ready(Ok(Vec::new()))
    }
}
