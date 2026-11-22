use std::path::PathBuf;

use super::*;

/// The `typescript` package installed for the frontend (`mise run web:install`).
pub(crate) fn typescript_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/node_modules/typescript/lib")
}

#[test]
fn only_the_ecmascript_libraries_are_loaded() {
    assert!(is_library("lib.es5.d.ts"));
    assert!(is_library("lib.es2024.d.ts"));
    assert!(is_library("lib.es2015.promise.d.ts"));
    assert!(is_library("lib.decorators.legacy.d.ts"));
    assert!(!is_library("lib.dom.d.ts"));
    assert!(!is_library("lib.esnext.d.ts"));
    assert!(!is_library("lib.es5.js"));
}

#[test]
fn files_come_from_the_npm_package_and_survive_the_archive() {
    let files = TypeScriptFiles::from_dir(&typescript_dir()).unwrap();
    assert!(files.compiler.contains("createLanguageService"));
    let names: Vec<&str> = files
        .libraries
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(names.contains(&"lib.es5.d.ts") && names.contains(&"lib.es2024.d.ts"));
    assert!(!names.contains(&"lib.dom.d.ts"));

    let archive = files.to_archive();
    // Compressed to a fraction (ADR S2: about 2 MB).
    assert!(
        archive.len() < files.compiler.len() / 3,
        "{}",
        archive.len()
    );
    assert_eq!(TypeScriptFiles::from_archive(&archive).unwrap(), files);
}

#[test]
fn damaged_archives_are_reported() {
    assert!(TypeScriptFiles::from_archive(b"not deflate").is_err());
    let compress = |bytes: &[u8]| miniz_oxide::deflate::compress_to_vec(bytes, 1);
    assert_eq!(
        TypeScriptFiles::from_archive(&compress(&[5, 0, 0, 0, b'a'])).unwrap_err(),
        "damaged TypeScript archive"
    );
    assert_eq!(
        TypeScriptFiles::from_archive(&compress(&[1, 0])).unwrap_err(),
        "damaged TypeScript archive"
    );
    let other = TypeScriptFiles {
        compiler: String::new(),
        libraries: Vec::new(),
    };
    let mut bytes = Vec::new();
    for part in [b"x.js".as_slice(), b""] {
        bytes.extend_from_slice(&u32::try_from(part.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(part);
    }
    assert_eq!(
        TypeScriptFiles::from_archive(&compress(&bytes)).unwrap_err(),
        "archive does not start with typescript.js"
    );
    assert_eq!(
        TypeScriptFiles::from_archive(&other.to_archive()).unwrap(),
        other
    );
}
