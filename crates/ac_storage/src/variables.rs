//! Variable lists (`[[variables]]`) as they appear in collection, folder,
//! environment and request files – rows with name, value, `enabled` and
//! `secret`. A secret never has its value in these files (it is stored
//! encrypted, ADR S4): one written there by hand is ignored and dropped on
//! the next save.

use std::path::Path;

use ac_domain::environment::Variable;
use toml_edit::{Item, TableLike};

use crate::error::StorageError;
use crate::rows::{self, Row};

/// Key of the list in every file that has one.
pub(crate) const KEY: &str = "variables";

/// Reads the variables in `item` (missing = none); `field` names the list
/// in error messages.
pub(crate) fn read(
    item: Option<&Item>,
    path: &Path,
    field: &'static str,
) -> Result<Vec<Variable>, StorageError> {
    Ok(rows::read(item, path, field)?
        .into_iter()
        .map(|row| Variable {
            name: row.name,
            value: if row.secret { String::new() } else { row.value },
            enabled: row.enabled,
            secret: row.secret,
        })
        .collect())
}

/// Replaces the variables under [`KEY`] in `table` if they changed.
pub(crate) fn write(table: &mut dyn TableLike, before: &[Variable], after: &[Variable]) {
    rows::write(table, KEY, &to_rows(before), &to_rows(after));
}

fn to_rows(variables: &[Variable]) -> Vec<Row> {
    variables
        .iter()
        .map(|variable| Row {
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
