//! Scripts of a request: TypeScript (or plain JavaScript) that runs before
//! the request is sent and after its response arrived. They see and change
//! the request, read the response and set variables for later requests.

/// The scripts of a request; empty text means no script.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scripts {
    /// Runs before variables are filled in; may change the request and set
    /// variables.
    pub pre_request: String,
    /// Runs after the response arrived; may read it and set variables.
    pub post_response: String,
}

impl Scripts {
    /// The script of `phase`, if there is one (blank text counts as none).
    #[must_use]
    pub fn get(&self, phase: ScriptPhase) -> Option<&str> {
        let text = match phase {
            ScriptPhase::PreRequest => &self.pre_request,
            ScriptPhase::PostResponse => &self.post_response,
        };
        (!text.trim().is_empty()).then_some(text.as_str())
    }

    /// Whether neither script has any text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.get(ScriptPhase::PreRequest).is_none() && self.get(ScriptPhase::PostResponse).is_none()
    }
}

/// When a script runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptPhase {
    /// Before the request is sent.
    PreRequest,
    /// After the response arrived.
    PostResponse,
}

#[cfg(test)]
#[path = "tests/script.rs"]
mod tests;
