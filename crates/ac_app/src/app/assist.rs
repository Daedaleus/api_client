//! Checking and completing scripts in the editor, through the
//! [`ScriptAssist`] port. Async, so a UI can wait without blocking (the
//! checker needs about a second to start).

use super::App;
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HttpSender, ScriptAssist, ScriptRunner, SecretVault,
};
use crate::view::{AssistRequest, AssistResult, CompletionView, DiagnosticView};

impl<H, S, V, B, R, A> App<H, S, V, B, R, A>
where
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
{
    /// Answers a question of the script editor.
    ///
    /// # Errors
    /// [`AppError::Assist`] if the checker cannot answer.
    pub async fn assist(&self, request: AssistRequest) -> Result<AssistResult, AppError> {
        Ok(match request {
            AssistRequest::Check { source } => AssistResult::Diagnostics {
                items: self
                    .inner
                    .assist
                    .check(source)
                    .await?
                    .iter()
                    .map(DiagnosticView::from_diagnostic)
                    .collect(),
            },
            AssistRequest::Complete { source, offset } => AssistResult::Completions {
                items: self
                    .inner
                    .assist
                    .complete(source, offset)
                    .await?
                    .iter()
                    .map(CompletionView::from_completion)
                    .collect(),
            },
        })
    }
}

#[cfg(test)]
#[path = "../tests/app_assist.rs"]
mod tests;
