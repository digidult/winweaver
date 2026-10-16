//! Names the commit a build is made from (`WINWEAVER_COMMIT`): the variable of that name when CI sets
//! it (the head of a pull request, not the merge commit it tests), else the HEAD of the checkout as
//! git gives it, else "unknown". The first 12 hexadecimal digits are kept.

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=WINWEAVER_COMMIT");
    let commit = std::env::var("WINWEAVER_COMMIT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(from_git);
    let short: String = commit
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(12)
        .collect();
    let short = if short.is_empty() {
        "unknown".to_owned()
    } else {
        short.to_ascii_lowercase()
    };
    println!("cargo:rustc-env=WINWEAVER_COMMIT={short}");
}

/// The HEAD of the repository the crate is in; the files git changes with a new commit rerun this.
fn from_git() -> Option<String> {
    let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR").ok()?).join("../..");
    let git = root.join(".git");
    let head = git.join("HEAD");
    if head.is_file() {
        println!("cargo:rerun-if-changed={}", head.display());
        if let Ok(text) = std::fs::read_to_string(&head)
            && let Some(name) = text.trim().strip_prefix("ref: ")
        {
            let branch = git.join(name);
            if branch.is_file() {
                println!("cargo:rerun-if-changed={}", branch.display());
            }
        }
    }
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
