//! Names and versions of the collection format (`docs/format/v1.md`).

/// `format` of `collection.toml`.
pub const COLLECTION_FORMAT: &str = "api-client/collection";
/// `format` of `folder.toml`.
pub const FOLDER_FORMAT: &str = "api-client/folder";
/// `format` of request files.
pub const REQUEST_FORMAT: &str = "api-client/request";

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
pub const RESERVED_ROOT_FOLDERS: [&str; 2] = ["environments", "secrets"];

/// Body types in request files.
pub mod body_type {
    /// JSON text.
    pub const JSON: &str = "json";
    /// Plain text.
    pub const TEXT: &str = "text";
}

/// Whether a directory entry is part of the request tree at all: hidden
/// entries (`.git`, `.age-recipients` …) are not.
#[must_use]
pub fn is_visible(name: &str) -> bool {
    !name.starts_with('.')
}
