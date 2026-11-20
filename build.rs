//! Compiles the window of “Claude Profiles.app” (launcher/Launcher.swift) and puts the
//! binary into OUT_DIR; src/desktop.rs embeds it.
//!
//! Without macOS or without `swiftc` an empty file is created. `claude-profile launcher` then
//! creates the simple launcher with a picker dialog.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    const SOURCE: &str = "launcher/Launcher.swift";
    // Set for release builds: a macOS build without the launcher window fails instead of
    // falling back to the simple dialog.
    const REQUIRE: &str = "CLAUDE_PROFILE_REQUIRE_LAUNCHER_UI";
    println!("cargo:rerun-if-changed={SOURCE}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={REQUIRE}");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("launcher-ui");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default().as_str() {
        "aarch64" => "arm64",
        other => other,
    }
    .to_string();

    let built = os == "macos"
        && Command::new("swiftc")
            .args(["-O", "-swift-version", "5", "-parse-as-library"])
            .args(["-target", &format!("{arch}-apple-macos13.0")])
            // Inside OUT_DIR, so the build also works where the home directory is not
            // writable (Homebrew's build sandbox).
            .arg("-module-cache-path")
            .arg(out.with_file_name("swift-module-cache"))
            .arg(SOURCE)
            .arg("-o")
            .arg(&out)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if !built {
        if os == "macos" && std::env::var_os(REQUIRE).is_some() {
            panic!("swiftc is missing or failed, and {REQUIRE} is set");
        }
        if os == "macos" {
            println!(
                "cargo:warning=swiftc is missing or failed – “Claude Profiles.app” only gets the simple picker dialog"
            );
        }
        std::fs::write(&out, b"").expect("cannot write the placeholder for the launcher binary");
    }
}
