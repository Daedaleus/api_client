//! The TypeScript compiler (`typescript.js`) and the library declarations
//! the language service needs – read from the `typescript` npm package or
//! from the compressed archive the desktop app embeds.
//!
//! Archive format (written by the desktop build, `apps/desktop/src-tauri/
//! build.rs`): raw DEFLATE of a sequence of entries, each a little-endian
//! `u32` name length, the name, a little-endian `u32` content length and the
//! content (UTF-8). The first entry is `typescript.js`.

use std::path::Path;

/// The compiler entry of an archive.
pub const COMPILER: &str = "typescript.js";

/// Whether a file of the package's `lib` folder is a declaration file the
/// service loads: the ECMAScript libraries up to ES2024 and decorators, no
/// DOM or web worker APIs (scripts cannot use them).
#[must_use]
pub fn is_library(name: &str) -> bool {
    let wanted = name.starts_with("lib.es") || name.starts_with("lib.decorators");
    wanted && name.ends_with(".d.ts") && !name.starts_with("lib.esnext")
}

/// The compiler and the library declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptFiles {
    /// `typescript.js`.
    pub compiler: String,
    /// Library declarations by file name, e.g. `lib.es5.d.ts`.
    pub libraries: Vec<(String, String)>,
}

impl TypeScriptFiles {
    /// From the `lib` folder of the `typescript` npm package.
    ///
    /// # Errors
    /// If a file cannot be read.
    pub fn from_dir(dir: &Path) -> std::io::Result<Self> {
        let compiler = std::fs::read_to_string(dir.join(COMPILER))?;
        let mut libraries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_library(&name) {
                libraries.push((name, std::fs::read_to_string(entry.path())?));
            }
        }
        libraries.sort();
        Ok(Self {
            compiler,
            libraries,
        })
    }

    /// From an archive (see the module documentation).
    ///
    /// # Errors
    /// If the archive is damaged.
    pub fn from_archive(compressed: &[u8]) -> Result<Self, String> {
        let bytes = miniz_oxide::inflate::decompress_to_vec(compressed)
            .map_err(|error| format!("cannot unpack TypeScript: {error:?}"))?;
        let mut rest = bytes.as_slice();
        let mut entries = Vec::new();
        while !rest.is_empty() {
            let name = take(&mut rest)?;
            let content = take(&mut rest)?;
            entries.push((name, content));
        }
        let mut entries = entries.into_iter();
        let Some((name, compiler)) = entries.next().filter(|(name, _)| name == COMPILER) else {
            return Err("archive does not start with typescript.js".into());
        };
        debug_assert_eq!(name, COMPILER);
        Ok(Self {
            compiler,
            libraries: entries.collect(),
        })
    }

    /// The archive of these files (the format the desktop build writes).
    #[must_use]
    pub fn to_archive(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let entries = std::iter::once((COMPILER, self.compiler.as_str())).chain(
            self.libraries
                .iter()
                .map(|(name, text)| (name.as_str(), text.as_str())),
        );
        for (name, content) in entries {
            for part in [name.as_bytes(), content.as_bytes()] {
                let length = u32::try_from(part.len()).unwrap_or(u32::MAX);
                bytes.extend_from_slice(&length.to_le_bytes());
                bytes.extend_from_slice(part);
            }
        }
        miniz_oxide::deflate::compress_to_vec(&bytes, 9)
    }
}

/// The next length-prefixed UTF-8 part of `rest`.
fn take(rest: &mut &[u8]) -> Result<String, String> {
    let damaged = || "damaged TypeScript archive".to_owned();
    let (length, tail) = rest.split_first_chunk::<4>().ok_or_else(damaged)?;
    let length = usize::try_from(u32::from_le_bytes(*length)).map_err(|_| damaged())?;
    if tail.len() < length {
        return Err(damaged());
    }
    let (part, tail) = tail.split_at(length);
    *rest = tail;
    String::from_utf8(part.to_vec()).map_err(|_| damaged())
}

#[cfg(test)]
#[path = "../tests/assist_files.rs"]
mod tests;
