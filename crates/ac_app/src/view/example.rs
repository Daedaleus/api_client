//! Response examples as the request editor shows them.

use std::time::Duration;

use ac_domain::example::ResponseExample;
use ac_domain::exchange::Response;
use serde::Serialize;

use super::ResponseView;

/// One example: its name and the response, shown like a received one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExampleView {
    /// Its name.
    pub name: String,
    /// Status, headers and body (no time: examples are not timed).
    pub response: ResponseView,
}

impl ExampleView {
    /// The view of `example`.
    #[must_use]
    pub fn from_example(example: &ResponseExample) -> Self {
        let response = Response {
            status: example.status,
            headers: example.headers.clone(),
            body: example.body.as_bytes().to_vec(),
            elapsed: Duration::ZERO,
        };
        Self {
            name: example.name.clone(),
            response: ResponseView::from_response(&response),
        }
    }

    /// The views of `examples`, in order.
    #[must_use]
    pub fn list(examples: &[ResponseExample]) -> Vec<Self> {
        examples.iter().map(Self::from_example).collect()
    }
}
