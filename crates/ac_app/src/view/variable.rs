//! Variables as the editors show and return them – for environments,
//! collections, folders and requests – and where to take them from when
//! sending.

use ac_domain::environment::Variable;
use serde::{Deserialize, Serialize};

/// A variable row in an editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct VariableDraft {
    /// Name used in `{{name}}`.
    pub name: String,
    /// Value. For a secret always empty towards the UI (never shown); from
    /// the UI a new value, or empty to keep the stored one.
    pub value: String,
    /// Used when sending.
    pub enabled: bool,
    /// Stored encrypted (ADR S4); only environments have secrets.
    #[serde(default)]
    pub secret: bool,
}

impl VariableDraft {
    /// The editor's rows for `variables`; secret values are left out.
    #[must_use]
    pub fn list(variables: &[Variable]) -> Vec<Self> {
        variables
            .iter()
            .map(|variable| Self {
                name: variable.name.clone(),
                value: if variable.secret {
                    String::new()
                } else {
                    variable.value.clone()
                },
                enabled: variable.enabled,
                secret: variable.secret,
            })
            .collect()
    }

    /// The variables of an environment editor (secrets allowed).
    #[must_use]
    pub fn to_variables(drafts: &[Self]) -> Vec<Variable> {
        drafts.iter().map(Self::to_variable).collect()
    }

    /// The variables of a collection, folder or request: these have no
    /// secrets, so a row marked secret is taken as a plain one.
    #[must_use]
    pub fn to_plain_variables(drafts: &[Self]) -> Vec<Variable> {
        drafts
            .iter()
            .map(|draft| Variable {
                secret: false,
                ..draft.to_variable()
            })
            .collect()
    }

    fn to_variable(&self) -> Variable {
        Variable {
            name: self.name.clone(),
            value: self.value.clone(),
            enabled: self.enabled,
            secret: self.secret,
        }
    }
}

/// Where the `{{variables}}` of a request come from when it is sent: its
/// collection, the folders around it and the chosen environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SendContext {
    /// The collection the request belongs to.
    pub collection_id: String,
    /// The saved request (for its folders), if it is one.
    #[serde(default)]
    pub request_id: Option<String>,
    /// The chosen environment, if any.
    #[serde(default)]
    pub environment_id: Option<String>,
}
