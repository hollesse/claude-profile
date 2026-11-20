//! `claude-profile set` – profile per folder via direnv (`.envrc`).

use crate::error::usage;
use crate::registry::Registry;
use crate::{block, paths, prompt};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXPORT: &str = "export CLAUDE_PROFILE=";

/// Searches upwards from `start` for the nearest `.envrc` with our block.
pub fn lookup(start: &Path) -> Option<(String, PathBuf)> {
    start.ancestors().find_map(|dir| {
        let file = dir.join(".envrc");
        let body = block::body(&fs::read_to_string(&file).ok()?)?;
        let value = body
            .lines()
            .find_map(|l| l.trim().strip_prefix(EXPORT))?
            .trim()
            .to_string();
        Some((value, file))
    })
}

fn in_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

/// Heuristic: does `direnv` appear in an rc file?
fn direnv_hooked() -> bool {
    let Ok(home) = paths::home() else { return false };
    [".zshrc", ".bashrc", ".bash_profile", ".zprofile", ".config/fish/config.fish"]
        .iter()
        .any(|f| {
            fs::read_to_string(home.join(f))
                .map(|s| s.contains("direnv"))
                .unwrap_or(false)
        })
}

fn git_root(start: &Path) -> Option<&Path> {
    start.ancestors().find(|d| d.join(".git").exists())
}

pub fn set(name: Option<&str>, unset: bool, yes: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let file = cwd.join(".envrc");
    let old = match fs::read_to_string(&file) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).context("cannot read .envrc"),
    };

    if unset {
        let Some(old) = old.filter(|o| block::body(o).is_some()) else {
            println!("No claude-profile block in .envrc – nothing to do.");
            return Ok(());
        };
        fs::write(&file, block::remove(&old)).context("cannot write .envrc")?;
        println!("Removed the claude-profile block from {}.", paths::tilde(&file));
    } else {
        let name = name.ok_or_else(|| usage("profile name is missing (or --unset)"))?;
        // The .envrc gets the lowercase ID.
        let name = &Registry::load()?.id(name)?;
        let old = old.unwrap_or_default();
        let new = block::upsert(&old, &format!("{EXPORT}{name}"));
        if new == old {
            println!("{} already sets CLAUDE_PROFILE={name}.", paths::tilde(&file));
        } else {
            fs::write(&file, new).context("cannot write .envrc")?;
            println!("{}: CLAUDE_PROFILE={name}", paths::tilde(&file));
        }
        if git_root(&cwd).is_some() {
            println!(
                "Note: do not commit .envrc – e.g. `echo .envrc >> .git/info/exclude`."
            );
        }
    }

    if !in_path("direnv") {
        eprintln!(
            "Warning: direnv is not installed – without direnv, .envrc has no effect (e.g. `brew install direnv`)."
        );
        return Ok(());
    }
    if !direnv_hooked() {
        eprintln!(
            "Warning: no direnv hook found in the rc files – e.g. `eval \"$(direnv hook zsh)\"` in ~/.zshrc."
        );
    }
    // direnv blocks a changed .envrc until it is allowed again.
    if yes || prompt::interactive() {
        if prompt::confirm("Run `direnv allow` for this folder?", yes)? {
            let status = Command::new("direnv")
                .arg("allow")
                .current_dir(&cwd)
                .status()
                .context("cannot start direnv")?;
            if !status.success() {
                eprintln!("Warning: `direnv allow` failed.");
            }
        }
    } else {
        println!("Next: run `direnv allow` in this folder.");
    }
    Ok(())
}
