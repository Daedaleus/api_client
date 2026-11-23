//! cURL commands.
//!
//! - `export`: a [`ResolvedRequest`] as a command for a POSIX shell (bash,
//!   zsh, Git Bash), one option per line.
//! - `import`: a pasted command as a [`Request`]; options without a
//!   counterpart are reported, not silently dropped.
//!
//! [`ResolvedRequest`]: ac_domain::exchange::ResolvedRequest
//! [`Request`]: ac_domain::collection::Request

mod export;
mod import;

pub use export::{DigestCredentials, to_curl};
pub use import::{CurlError, Imported, from_curl};
