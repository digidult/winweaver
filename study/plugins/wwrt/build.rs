//! The build of the runtime (`plugin::ABI`): Rust has no stable ABI, so a plugin works with the
//! runtime it was built against alone. The compiler, the target, the profile and the lock file of
//! the workspace name that build; another version of any crate changes the lock file.

use std::path::Path;
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let compiler = Command::new(rustc)
        .arg("-V")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let lock = Path::new(&dir).join("../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let bytes = std::fs::read(&lock).unwrap_or_default();
    // FNV-1a of the lock file without its carriage returns, so a checkout with CRLF is the same.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes.iter().filter(|b| **b != b'\r') {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!(
        "cargo:rustc-env=WWRT_ABI=wwrt {version}; {}; {target}; {profile}; lock {hash:016x}",
        compiler.trim()
    );
}
