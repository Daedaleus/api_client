//! Environments as the editor shows and returns them.

use ac_domain::environment::{Environment, Variable};
use serde::{Deserialize, Serialize};

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

/// A variable of an [`EnvironmentDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct VariableDraft {
    /// Name used in `{{name}}`.
    pub name: String,
    /// Value.
    pub value: String,
    /// Used when sending.
    pub enabled: bool,
}

/// The environment to resolve `{{variables}}` with when sending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentRef {
    /// The collection the environment belongs to.
    pub collection_id: String,
    /// The environment.
    pub environment_id: String,
}

impl EnvironmentDraft {
    /// The editor's view of `environment`.
    #[must_use]
    pub fn from_environment(environment: &Environment, read_only: bool) -> Self {
        Self {
            id: environment.id.to_string(),
            name: environment.name.clone(),
            variables: environment
                .variables
                .iter()
                .map(|variable| VariableDraft {
                    name: variable.name.clone(),
                    value: variable.value.clone(),
                    enabled: variable.enabled,
                })
                .collect(),
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
            variables: self
                .variables
                .iter()
                .map(|variable| Variable {
                    name: variable.name.clone(),
                    value: variable.value.clone(),
                    enabled: variable.enabled,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
#[path = "../tests/view_environment.rs"]
mod tests;
