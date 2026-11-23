//! Names and versions of the collection format (`docs/format/v1.md`).

/// `format` of `collection.toml`.
pub const COLLECTION_FORMAT: &str = "api-client/collection";
/// `format` of `folder.toml`.
pub const FOLDER_FORMAT: &str = "api-client/folder";
/// `format` of environment files.
pub const ENVIRONMENT_FORMAT: &str = "api-client/environment";
/// `format` of request files.
pub const REQUEST_FORMAT: &str = "api-client/request";
/// `format` of the response examples next to a request file.
pub const EXAMPLES_FORMAT: &str = "api-client/examples";
/// Added to a request's file stem for its examples file
/// (`login.toml` → `login.examples.toml`).
pub const EXAMPLES_SUFFIX: &str = ".examples";

/// The format version this program writes and fully understands.
pub const CURRENT_VERSION: i64 = 1;

/// Settings file in the collection root.
pub const COLLECTION_FILE: &str = "collection.toml";
/// Optional settings file in a folder of the collection.
pub const FOLDER_FILE: &str = "folder.toml";
/// Extension of collection files.
pub const EXTENSION: &str = "toml";

/// Folder names in the collection root that belong to later milestones
/// (environments and secrets, M3) and are not part of the request tree.
pub const RESERVED_ROOT_FOLDERS: [&str; 2] = [ENVIRONMENTS_FOLDER, SECRETS_FOLDER];

/// Folder in the collection root that holds the environment files.
pub const ENVIRONMENTS_FOLDER: &str = "environments";
/// Folder in the collection root with the encrypted secrets of each
/// environment (`secrets/dev.age` belongs to `environments/dev.toml`).
pub const SECRETS_FOLDER: &str = "secrets";
/// Extension of encrypted secrets files.
pub const SECRETS_EXTENSION: &str = "age";
/// File in the collection root listing who secrets are encrypted for.
pub const RECIPIENTS_FILE: &str = ".age-recipients";

/// Body types in request files.
pub mod body_type {
    /// JSON text.
    pub const JSON: &str = "json";
    /// Plain text.
    pub const TEXT: &str = "text";
    /// URL-encoded form fields.
    pub const FORM: &str = "form";
    /// Multipart form data.
    pub const MULTIPART: &str = "multipart";
    /// The contents of a file.
    pub const BINARY: &str = "binary";
}

/// Whether a directory entry is part of the request tree at all: hidden
/// entries (`.git`, `.age-recipients` …) are not.
#[must_use]
pub fn is_visible(name: &str) -> bool {
    !name.starts_with('.')
}
