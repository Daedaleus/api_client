//! Path variables and switched-off query parameters as the editor shows
//! them (see `ac_domain::params`).

use ac_domain::params::{PathParam, QueryParam};
use serde::{Deserialize, Serialize};

/// A name and its value: a path variable, or a query parameter that is
/// switched off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ParamDraft {
    /// Name (path variables without the colon).
    pub name: String,
    /// Value; may contain `{{variables}}`.
    pub value: String,
}

impl ParamDraft {
    /// The editor's view of path variables.
    #[must_use]
    pub fn from_path(params: &[PathParam]) -> Vec<Self> {
        params
            .iter()
            .map(|param| Self {
                name: param.name.clone(),
                value: param.value.clone(),
            })
            .collect()
    }

    /// The editor's view of switched-off query parameters.
    #[must_use]
    pub fn from_query(params: &[QueryParam]) -> Vec<Self> {
        params
            .iter()
            .map(|param| Self {
                name: param.name.clone(),
                value: param.value.clone(),
            })
            .collect()
    }

    /// Path variables from editor rows (rows without a name are dropped).
    #[must_use]
    pub fn to_path(rows: &[Self]) -> Vec<PathParam> {
        rows.iter()
            .filter(|row| !row.name.trim().is_empty())
            .map(|row| PathParam {
                name: row.name.trim().to_owned(),
                value: row.value.clone(),
            })
            .collect()
    }

    /// Switched-off query parameters from editor rows (rows without a name
    /// are dropped).
    #[must_use]
    pub fn to_query(rows: &[Self]) -> Vec<QueryParam> {
        rows.iter()
            .filter(|row| !row.name.trim().is_empty())
            .map(|row| QueryParam {
                name: row.name.clone(),
                value: row.value.clone(),
            })
            .collect()
    }
}
