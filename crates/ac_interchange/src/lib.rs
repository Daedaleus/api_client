//! Import and export: converting between this program's requests and other
//! tools' formats. Pure conversions on domain types – no files, no network;
//! the application layer decides where text comes from and goes to.
//!
//! - [`curl`]: a resolved request as a cURL command, and a pasted cURL
//!   command as a request.
//! - [`postman`]: Postman collections (v2.1, v2.0) as collections.
//!
//! Bruno and `OpenAPI` imports follow (issue #18).

pub mod curl;
pub mod postman;
