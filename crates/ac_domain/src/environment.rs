//! Environments and `{{variable}}` placeholders.
//!
//! An environment is a named set of variables (`Local`, `Staging` …). Before
//! a request is sent, every `{{name}}` in its URL, headers and body is
//! replaced by the value of the variable `name` in the chosen environment.
//! Values are inserted as they are – a `{{…}}` inside a value is not resolved
//! again, so variables cannot refer to each other in circles.

use std::collections::HashMap;

use crate::collection::ItemId;

/// A named set of variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// Identity.
    pub id: ItemId,
    /// Display name, e.g. `Staging`.
    pub name: String,
    /// Variables in display order.
    pub variables: Vec<Variable>,
}

/// One variable of an [`Environment`].
#[derive(Clone, PartialEq, Eq)]
pub struct Variable {
    /// Name used in `{{name}}`.
    pub name: String,
    /// Value inserted for it. For a secret: the decrypted value, empty while
    /// it is not known (it is never stored in plain text).
    pub value: String,
    /// Disabled variables are kept but not used.
    pub enabled: bool,
    /// The value is a secret: stored encrypted, never shown or logged.
    pub secret: bool,
}

/// Shown in place of a secret value.
const MASK: &str = "***";

impl std::fmt::Debug for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = if self.secret { MASK } else { &self.value };
        f.debug_struct("Variable")
            .field("name", &self.name)
            .field("value", &value)
            .field("enabled", &self.enabled)
            .field("secret", &self.secret)
            .finish()
    }
}

impl Environment {
    /// A new environment without variables.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ItemId::new(),
            name: name.into(),
            variables: Vec::new(),
        }
    }

    /// Whether any variable is a secret.
    #[must_use]
    pub fn has_secrets(&self) -> bool {
        self.variables.iter().any(|variable| variable.secret)
    }

    /// The values of the enabled variables by name. If a name occurs twice,
    /// the later row wins (as in an assignment list). Secrets whose value is
    /// not known (not decrypted) are left out.
    #[must_use]
    pub fn values(&self) -> HashMap<&str, &str> {
        self.variables
            .iter()
            .filter(|variable| variable.enabled && !(variable.secret && variable.value.is_empty()))
            .map(|variable| (variable.name.trim(), variable.value.as_str()))
            .collect()
    }
}

/// A `{{name}}` without a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("variable {0:?} is not defined")]
pub struct UnresolvedVariable(pub String);

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

/// Replaces every `{{name}}` in `text` with `lookup(name)`. Spaces around the
/// name are ignored; an unclosed `{{` stays as it is.
///
/// # Errors
/// [`UnresolvedVariable`] for the first name `lookup` does not know.
pub fn interpolate<'a>(
    text: &str,
    lookup: impl Fn(&str) -> Option<&'a str>,
) -> Result<String, UnresolvedVariable> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            break;
        };
        let name = after_open[..end].trim();
        out.push_str(&rest[..start]);
        out.push_str(lookup(name).ok_or_else(|| UnresolvedVariable(name.to_owned()))?);
        rest = &after_open[end + CLOSE.len()..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
#[path = "tests/environment.rs"]
mod tests;
