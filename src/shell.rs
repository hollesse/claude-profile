//! `claude-profile replace-claude-command` – shell function `claude` in the rc file.

use crate::error::usage;
use crate::{block, paths, prompt};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const FUNCTION: &str = r#"claude() { command claude-profile run -- "$@"; }"#;

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
}

fn detect() -> Result<Shell> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    match shell.rsplit('/').next() {
        Some("zsh") => Ok(Shell::Zsh),
        Some("bash") => Ok(Shell::Bash),
        _ => Err(usage(
            "shell not recognized ($SHELL) – please pass --shell zsh|bash",
        )),
    }
}

fn rc_file(shell: Shell) -> Result<PathBuf> {
    Ok(match shell {
        Shell::Zsh => match std::env::var_os("ZDOTDIR").filter(|z| !z.is_empty()) {
            Some(z) => PathBuf::from(z).join(".zshrc"),
            None => paths::home()?.join(".zshrc"),
        },
        Shell::Bash => paths::home()?.join(".bashrc"),
    })
}

pub fn run(shell: Option<Shell>, remove: bool, yes: bool) -> Result<()> {
    let shell = match shell {
        Some(s) => s,
        None => detect()?,
    };
    let rc = rc_file(shell)?;
    let shown = paths::tilde(&rc);
    let old = match fs::read_to_string(&rc) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("cannot read {shown}")),
    };
    let old_text = old.clone().unwrap_or_default();
    let new_text = if remove {
        block::remove(&old_text)
    } else {
        block::upsert(&old_text, FUNCTION)
    };
    if new_text == old_text {
        println!(
            "{shown}: {}",
            if remove {
                "no claude-profile block present – nothing to do."
            } else {
                "block is already up to date – nothing to do."
            }
        );
        return Ok(());
    }

    println!("Change to {shown}:");
    if let Some(body) = block::body(&old_text) {
        println!("- {}", block::BEGIN);
        for l in body.lines() {
            println!("- {l}");
        }
        println!("- {}", block::END);
    }
    if !remove {
        println!("+ {}", block::BEGIN);
        println!("+ {FUNCTION}");
        println!("+ {}", block::END);
    }
    if !prompt::confirm("Apply?", yes)? {
        println!("Aborted – nothing changed.");
        return Ok(());
    }

    if old.is_some() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = PathBuf::from(format!("{}.claude-profile-backup-{stamp}", rc.display()));
        fs::copy(&rc, &backup)
            .with_context(|| format!("cannot create backup {}", paths::tilde(&backup)))?;
        println!("Backup: {}", paths::tilde(&backup));
    }
    // fs::write follows symlinks – a linked rc file (dotfiles repo) stays a symlink.
    fs::write(&rc, new_text).with_context(|| format!("cannot write {shown}"))?;
    println!(
        "{shown} updated. Open a new shell or run `source {shown}`."
    );
    Ok(())
}
