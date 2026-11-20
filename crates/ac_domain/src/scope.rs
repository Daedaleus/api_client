//! Which value a `{{name}}` gets when variables are defined in several
//! places. From weakest to strongest:
//!
//! 1. the collection,
//! 2. the chosen environment,
//! 3. the folders around the request, outer to inner,
//! 4. the request itself.
//!
//! The stronger place wins. Variables set by scripts at run time will rank
//! above all of them (milestone M4).

use std::collections::HashMap;

use crate::environment::{Variable, add_values};

/// The variable lists that apply to one request, weakest first.
#[derive(Debug, Clone, Default)]
pub struct Scope<'a> {
    layers: Vec<&'a [Variable]>,
}

impl<'a> Scope<'a> {
    /// No variables yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `variables` as the strongest list so far.
    #[must_use]
    pub fn then(mut self, variables: &'a [Variable]) -> Self {
        self.layers.push(variables);
        self
    }

    /// The value of every usable variable; stronger lists win.
    #[must_use]
    pub fn values(&self) -> HashMap<&'a str, &'a str> {
        let mut values = HashMap::new();
        for layer in &self.layers {
            add_values(&mut values, layer);
        }
        values
    }
}

#[cfg(test)]
#[path = "tests/scope.rs"]
mod tests;
