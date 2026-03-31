#![forbid(unsafe_code)]

use camino::Utf8PathBuf;

pub fn repo(name: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/repos")
        .join(name)
}
