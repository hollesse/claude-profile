//! Mod “profile-badge”: shows above the prompt in Claude Code which profile is running.
//!
//! The mod's files are embedded in the binary. On start they are written to
//! `~/.config/claude-profiles/mods/profile-badge` and the folder is passed to
//! Claude Code via `CLAUDE_CODE_PLUGIN_DIRS`.

use crate::paths;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

/// Turns the badge off when set.
pub const DISABLE_ENV: &str = "CLAUDE_PROFILE_NO_BADGE";
pub const PLUGIN_DIRS_ENV: &str = "CLAUDE_CODE_PLUGIN_DIRS";

const FILES: &[(&str, &str)] = &[
    // Run at session start: tells the model which profile it runs with.
    (
        "hooks/hint.sh",
        include_str!("../mod/profile-badge/hooks/hint.sh"),
    ),
    // Tells the model how to find the configuration of a profile.
    (
        "skills/claude-profile/SKILL.md",
        include_str!("../mod/profile-badge/skills/claude-profile/SKILL.md"),
    ),
    (
        ".claude-plugin/plugin.json",
        include_str!("../mod/profile-badge/.claude-plugin/plugin.json"),
    ),
    (
        "hooks/hooks.json",
        include_str!("../mod/profile-badge/hooks/hooks.json"),
    ),
    (
        "hooks/register.tsx",
        include_str!("../mod/profile-badge/hooks/register.tsx"),
    ),
    (
        "hooks/look.ts",
        include_str!("../mod/profile-badge/hooks/look.ts"),
    ),
    // Shipped with Claude Code and copied unchanged: builds the desktop app's color scheme
    // from a few colors. Copy again for a new Claude Code build.
    (
        "hooks/make-colors.js",
        include_str!("../mod/profile-badge/hooks/make-colors.js"),
    ),
    (
        "hooks/make-colors.d.ts",
        include_str!("../mod/profile-badge/hooks/make-colors.d.ts"),
    ),
];

pub fn enabled() -> bool {
    std::env::var_os(DISABLE_ENV).is_none()
}

pub fn dir() -> Result<PathBuf> {
    Ok(paths::home()?.join(".config/claude-profiles/mods/profile-badge"))
}

/// Writes the mod files if they are missing or outdated.
pub fn ensure() -> Result<PathBuf> {
    let dir = dir()?;
    for (rel, content) in FILES {
        let path = dir.join(rel);
        if fs::read_to_string(&path).map(|c| c == *content).unwrap_or(false) {
            continue;
        }
        let parent = path.parent().expect("mod file has a parent directory");
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", paths::tilde(parent)))?;
        fs::write(&path, content)
            .with_context(|| format!("cannot write {}", paths::tilde(&path)))?;
    }
    Ok(dir)
}

/// Value for `CLAUDE_CODE_PLUGIN_DIRS`: the mod folder, followed by folders already set.
pub fn plugin_dirs(dir: &PathBuf) -> OsString {
    let existing = std::env::var_os(PLUGIN_DIRS_ENV).unwrap_or_default();
    let mut all = vec![dir.clone()];
    all.extend(std::env::split_paths(&existing).filter(|p| !p.as_os_str().is_empty() && p != dir));
    std::env::join_paths(all).unwrap_or_else(|_| dir.clone().into_os_string())
}
