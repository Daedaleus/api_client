//! Import and export: converting between this program's requests and other
//! tools' formats. Pure conversions on domain types – no files, no network;
//! the application layer decides where text comes from and goes to.
//!
//! - [`curl`]: a resolved request as a cURL command, and a pasted cURL
//!   command as a request.
//! - [`import`]: importing a collection, the format detected from the
//!   file – `postman` (Postman collections v2.1, v2.0), `insomnia`
//!   (Insomnia exports, format 4), `har` (recorded browser requests) or
//!   `openapi` (`OpenAPI` 3.0 and 3.1 descriptions, JSON or YAML).
//!
//! - [`bruno`]: Bruno collections (a folder of `.bru` files) with their
//!   environments.
//! - [`export`]: a collection in another tool's format (Postman v2.1).
//! - [`snippet`]: a resolved request as code (cURL, JavaScript, Python, Go,
//!   `HTTPie`).

pub mod bruno;
pub mod curl;
pub mod export;
mod har;
pub mod import;
mod insomnia;
mod openapi;
mod postman;
pub mod snippet;

pub use import::{
    ImportError, Imported, ItemSetting, Skipped, SkippedKind, import_collection,
    import_collection_from,
};
