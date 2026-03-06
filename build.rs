use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap();

    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    println!("cargo:rustc-env=DXON_VERSION={}", version);
    println!("cargo:rustc-env=DXON_GIT_COMMIT={}", git_commit);
}
