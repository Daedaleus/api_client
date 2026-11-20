//! What can go wrong when reading or writing collection files.

use std::path::PathBuf;

/// An error with one file or folder of a collection.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The file system refused (missing file, no permission …).
    #[error("{path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Cause.
        source: std::io::Error,
    },
    /// The file is not valid TOML.
    #[error("{path}: not valid TOML: {message}")]
    Toml {
        /// Affected file.
        path: PathBuf,
        /// Parser message with line and column.
        message: String,
    },
    /// The file has another `format` than its place requires.
    #[error("{path}: expected format {expected:?}, found {found:?}")]
    WrongFormat {
        /// Affected file.
        path: PathBuf,
        /// Format the place requires.
        expected: &'static str,
        /// Format in the file, if any.
        found: Option<String>,
    },
    /// A value is missing or has the wrong type.
    #[error("{path}: {field}: {message}")]
    Invalid {
        /// Affected file.
        path: PathBuf,
        /// Key, e.g. `request.method`.
        field: &'static str,
        /// What is wrong.
        message: String,
    },
    /// The file comes from a newer program version and is not written.
    #[error("{path}: written by a newer version (format version {version}); opened read-only")]
    ReadOnly {
        /// Affected file.
        path: PathBuf,
        /// Version in the file.
        version: i64,
    },
    /// The folder has no `collection.toml`.
    #[error("{path}: not a collection (collection.toml missing)")]
    NotACollection {
        /// The folder.
        path: PathBuf,
    },
    /// Creating a collection where one already exists.
    #[error("{path}: a collection already exists here")]
    AlreadyExists {
        /// The folder.
        path: PathBuf,
    },
}

impl StorageError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn invalid(
        path: impl Into<PathBuf>,
        field: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::Invalid {
            path: path.into(),
            field,
            message: message.into(),
        }
    }
}

/// A file or folder that could not be loaded; the rest of the collection
/// still opens. The file is left untouched.
#[derive(Debug)]
pub struct LoadProblem {
    /// What went wrong (contains the path).
    pub error: StorageError,
}
