use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find workspace root");

    if std::env::var("CI").is_ok() {
        workspace_root = workspace_root
            .parent()
            .expect("Failed to find CI workspace root");
    }

    println!(
        "cargo:rustc-env=WORKSPACE_ROOT={}",
        workspace_root.display()
    );
}
