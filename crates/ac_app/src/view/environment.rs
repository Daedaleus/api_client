//! Environments as the editor shows and returns them.

use ac_domain::environment::Environment;
use serde::{Deserialize, Serialize};

use super::VariableDraft;
use super::collection::non_empty;
use crate::error::AppError;

/// An environment with its variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDraft {
    /// Environment id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Variables in display order.
    pub variables: Vec<VariableDraft>,
    /// The file is never written (set by the app, ignored when saving).
    #[serde(default)]
    pub read_only: bool,
}

impl EnvironmentDraft {
    /// The editor's view of `environment`.
    #[must_use]
    pub fn from_environment(environment: &Environment, read_only: bool) -> Self {
        Self {
            id: environment.id.to_string(),
            name: environment.name.clone(),
            variables: VariableDraft::list(&environment.variables),
            read_only,
        }
    }

    /// The environment the editor describes.
    ///
    /// # Errors
    /// If the id or name is not valid.
    pub fn to_environment(&self) -> Result<Environment, AppError> {
        Ok(Environment {
            id: self.id.parse().map_err(|_| AppError::UnknownItem)?,
            name: non_empty(&self.name)?.to_owned(),
            variables: VariableDraft::to_variables(&self.variables),
        })
    }
}

#[cfg(test)]
#[path = "../tests/view_environment.rs"]
mod tests;
