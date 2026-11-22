//! Tauri build step (app manifest, icons, capabilities) and the TypeScript
//! compiler for the script editor's checker (ADR S2), packed from the
//! frontend's `typescript` npm package into `OUT_DIR/typescript.deflate`.
//!
//! The archive format is that of `ac_scripting::assist::TypeScriptFiles`
//! (`to_archive` / `from_archive`): raw DEFLATE of entries, each a
//! little-endian `u32` name length, the name, a little-endian `u32` content
//! length and the content; `typescript.js` first.

use std::path::{Path, PathBuf};

/// The package's files, installed with the frontend (`mise run web:install`).
const TYPESCRIPT_LIB: &str = "../node_modules/typescript/lib";
const COMPILER: &str = "typescript.js";

fn main() {
    tauri_build::build();
    pack_typescript();
}

fn pack_typescript() {
    let dir = Path::new(TYPESCRIPT_LIB);
    let compiler = dir.join(COMPILER);
    println!("cargo:rerun-if-changed={}", compiler.display());
    let read = |path: &Path| {
        std::fs::read(path).unwrap_or_else(|error| {
            panic!(
                "cannot read {} ({error}); install the frontend packages first: mise run web:install",
                path.display()
            )
        })
    };
    let mut libraries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot list {} ({error})", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_library)
        })
        .collect();
    libraries.sort();

    let mut bytes = Vec::new();
    let mut add = |name: &str, content: &[u8]| {
        for part in [name.as_bytes(), content] {
            let length = u32::try_from(part.len()).expect("files below 4 GB");
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(part);
        }
    };
    add(COMPILER, &read(&compiler));
    for path in &libraries {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        add(name, &read(path));
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("set by cargo"));
    std::fs::write(
        out.join("typescript.deflate"),
        miniz_oxide::deflate::compress_to_vec(&bytes, 9),
    )
    .expect("OUT_DIR is writable");
}

/// The same choice as `ac_scripting::assist::is_library`: the ECMAScript
/// libraries up to ES2024 and decorators, no DOM.
fn is_library(name: &str) -> bool {
    let wanted = name.starts_with("lib.es") || name.starts_with("lib.decorators");
    wanted && name.ends_with(".d.ts") && !name.starts_with("lib.esnext")
}
