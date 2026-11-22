//! Checking and completing scripts in the editor: what the editor asks and
//! what it gets back.

use serde::{Deserialize, Serialize};

use crate::ports::{CompletionKind, ScriptCompletion, ScriptDiagnostic, Severity};

/// A question of the script editor.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssistRequest {
    /// Problems in the script.
    Check {
        /// The script.
        source: String,
    },
    /// Suggestions at a position.
    Complete {
        /// The script.
        source: String,
        /// Where the cursor is (JavaScript string offset).
        offset: usize,
    },
}

/// The answer to an [`AssistRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssistResult {
    /// Problems found.
    Diagnostics {
        /// In the order of the script.
        items: Vec<DiagnosticView>,
        /// Only syntax errors: the checker is still starting, ask again
        /// soon.
        partial: bool,
    },
    /// Suggestions.
    Completions {
        /// Best first.
        items: Vec<CompletionView>,
    },
}

/// A problem as the editor marks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticView {
    /// Start offset.
    pub from: usize,
    /// End offset (exclusive).
    pub to: usize,
    /// How bad it is.
    pub severity: SeverityView,
    /// What is wrong.
    pub message: String,
}

/// How bad a problem is (the editor's names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SeverityView {
    /// Breaks the script.
    Error,
    /// May not work as meant.
    Warning,
    /// A suggestion.
    Info,
}

/// A suggestion as the editor lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CompletionView {
    /// What is inserted.
    pub label: String,
    /// What it is (the editor's icon names).
    pub kind: CompletionKindView,
}

/// What a suggestion is (`CodeMirror`'s completion types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CompletionKindView {
    /// A method or function.
    Function,
    /// A property.
    Property,
    /// A variable or constant.
    Variable,
    /// A class, interface or type.
    Type,
    /// A keyword.
    Keyword,
    /// Anything else.
    Text,
}

impl DiagnosticView {
    /// The editor's view of a problem.
    #[must_use]
    pub fn from_diagnostic(diagnostic: &ScriptDiagnostic) -> Self {
        Self {
            from: diagnostic.from,
            to: diagnostic.to,
            severity: match diagnostic.severity {
                Severity::Error => SeverityView::Error,
                Severity::Warning => SeverityView::Warning,
                Severity::Hint => SeverityView::Info,
            },
            message: diagnostic.message.clone(),
        }
    }
}

impl CompletionView {
    /// The editor's view of a suggestion.
    #[must_use]
    pub fn from_completion(completion: &ScriptCompletion) -> Self {
        Self {
            label: completion.label.clone(),
            kind: match completion.kind {
                CompletionKind::Function => CompletionKindView::Function,
                CompletionKind::Property => CompletionKindView::Property,
                CompletionKind::Variable => CompletionKindView::Variable,
                CompletionKind::Type => CompletionKindView::Type,
                CompletionKind::Keyword => CompletionKindView::Keyword,
                CompletionKind::Other => CompletionKindView::Text,
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_assist.rs"]
mod tests;
