//! Shell completions: commands, options and – read from the registry at completion time –
//! profile names.
//!
//! The shell does not get a static script. `claude-profile completions <shell>` prints a few
//! lines that register a function, and that function calls this binary again with
//! `CLAUDE_PROFILE_COMPLETE` set; `main` hands those calls to `clap_complete`. So the
//! completions always match the installed version.

use crate::registry::Registry;
use anyhow::Result;
use clap_complete::engine::{ArgValueCandidates, CompletionCandidate};
use clap_complete::env::{Bash, EnvCompleter, Fish, Zsh};
use std::io::Write;

/// Set by the registered shell function when it asks this binary for candidates.
pub const VAR: &str = "CLAUDE_PROFILE_COMPLETE";
/// Looked up in `PATH` on every completion, so the registration survives an upgrade.
pub const BIN: &str = "claude-profile";

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

/// Candidates for an argument that names an existing profile: the IDs, each with its directory.
pub fn profiles() -> ArgValueCandidates {
    ArgValueCandidates::new(|| {
        // Completion must never fail loudly: without a readable registry there are no candidates.
        let Ok(reg) = Registry::load() else {
            return Vec::new();
        };
        reg.profiles
            .iter()
            .map(|(id, profile)| {
                let help = match &profile.name {
                    Some(name) => format!("{name} – {}", profile.dir),
                    None => profile.dir.clone(),
                };
                CompletionCandidate::new(id).help(Some(help.into()))
            })
            .collect()
    })
}

/// `claude-profile completions <shell>`: prints what the shell has to load.
pub fn print(shell: Shell) -> Result<()> {
    let completer: &dyn EnvCompleter = match shell {
        Shell::Bash => &Bash,
        Shell::Zsh => &Zsh,
        Shell::Fish => &Fish,
    };
    let mut script = Vec::new();
    completer.write_registration(VAR, BIN, BIN, BIN, &mut script)?;
    match std::io::stdout().write_all(&script) {
        // The reader closed early (`… | head`): nothing to report.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        other => Ok(other?),
    }
}
