use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn gather(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|s| s != "target") {
                gather(&path, files)
            }
        } else if path.extension().is_some_and(|s| s == "rs")
            || path.file_name().is_some_and(|s| s == "Cargo.toml")
        {
            files.push(path)
        }
    }
}
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = vec![root.join("Cargo.lock"), root.join("Cargo.toml")];
    gather(&root.join("src"), &mut files);
    gather(&root.join("crates"), &mut files);
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        digest.update(
            path.strip_prefix(&root)
                .expect("relative path")
                .to_string_lossy()
                .replace('\\', "/")
                .as_bytes(),
        );
        digest.update(fs::read(path).expect("source"));
    }
    println!("cargo:rerun-if-env-changed=HIRUNE_BUILD_ID");
    let fingerprint = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!("cargo:rustc-env=HIRUNE_SOURCE_FINGERPRINT={fingerprint}");
}
