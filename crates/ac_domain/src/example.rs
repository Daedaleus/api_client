//! Examples of responses kept with a request: a typical answer as
//! documentation, or to compare later answers with.

use crate::exchange::Header;
use crate::protocol::StatusCode;

/// A response kept as an example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseExample {
    /// Its name, e.g. `OK` or `Not found`.
    pub name: String,
    /// The status code.
    pub status: StatusCode,
    /// The headers as received.
    pub headers: Vec<Header>,
    /// The body as text (examples keep text bodies only).
    pub body: String,
}
