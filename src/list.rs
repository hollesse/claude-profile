//! `claude-profile list`, `claude-profile default` and `claude-profile rename`.

use crate::error::usage;
use crate::link::{self, JsonAction, LinkState};
use crate::registry::{profile_id, spelling, validate_name, Registry};
use crate::{envrc, paths, prompt, rewrite};
use anyhow::{bail, Result};

pub fn list(names_only: bool) -> Result<()> {
    let reg = Registry::load()?;
    if names_only {
        for n in reg.names() {
            println!("{n}");
        }
        return Ok(());
    }
    if reg.profiles.is_empty() {
        println!("No profiles yet. Start with `claude-profile add`");
        return Ok(());
    }

    let state = link::state()?;
    let linked = link::linked_profile(&reg, &state);
    let active = std::env::var("CLAUDE_PROFILE").ok().filter(|s| !s.is_empty());
    let direnv = envrc::lookup(&std::env::current_dir()?);
    let mut warnings = Vec::new();

    // The table shows the display name; `--names` prints the IDs.
    let w_name = reg
        .profiles
        .iter()
        .map(|(id, p)| p.shown(id).chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let w_dir = reg
        .profiles
        .values()
        .map(|p| p.dir.chars().count())
        .max()
        .unwrap_or(9)
        .max(9);
    println!("{:w_name$}  {:w_dir$}  STATUS", "NAME", "DIRECTORY");
    for (name, profile) in &reg.profiles {
        let exists = profile.dir_path()?.is_dir();
        if !exists {
            warnings.push(format!("directory {} of `{name}` is missing", profile.dir));
        } else {
            let dir = profile.dir_path()?;
            let stale: usize = rewrite::scan(&dir, &dir)?.iter().map(|f| f.count).sum();
            if stale > 0 {
                warnings.push(format!(
                    "profile `{name}` contains {stale} paths to ~/.claude/ – they point into the symlink's profile, not into this one (repair: `claude-profile fix-paths {name}`)"
                ));
            }
        }
        let mut marks = Vec::new();
        if linked.as_ref() == Some(name) {
            marks.push("← symlink");
        }
        if active.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(name)) {
            marks.push("← active");
        }
        if direnv.as_ref().is_some_and(|(n, _)| n.eq_ignore_ascii_case(name)) {
            marks.push("← direnv");
        }
        let line = format!(
            "{:w_name$}  {:w_dir$}  {:7}  {}",
            profile.shown(name),
            profile.dir,
            if exists { "ok" } else { "missing" },
            marks.join(" ")
        );
        println!("{}", line.trim_end());
    }

    match &state {
        LinkState::Missing => warnings.push(
            "~/.claude is missing – `claude-profile default <name>` creates the symlink".to_string(),
        ),
        LinkState::RealDir => warnings.push(
            "~/.claude is a real directory, not a symlink – programs that use ~/.claude directly end up in no profile".to_string(),
        ),
        LinkState::Other => warnings.push("~/.claude is neither a directory nor a symlink".to_string()),
        LinkState::Symlink { target, exists } => {
            if !exists {
                warnings.push(format!(
                    "symlink ~/.claude is broken: target {} is missing",
                    paths::tilde(target)
                ));
            } else if linked.is_none() {
                warnings.push(format!(
                    "symlink ~/.claude points to {} – that is not a registered profile",
                    paths::tilde(target)
                ));
            }
        }
    }
    if let Some(l) = &linked {
        let dir = reg.profiles[l].dir_path()?;
        let action = link::json_action(&dir, dir.join(".claude.json").exists());
        if !matches!(action, Ok(JsonAction::Nothing)) {
            warnings.push(format!(
                "~/.claude.json is not a symlink to the .claude.json of `{l}` – a `claude` without a profile then sees a different state (repair: `claude-profile default {l}`)"
            ));
        }
    }
    if let Some(a) = &active {
        if !reg.contains(a) {
            warnings.push(format!("CLAUDE_PROFILE={a} is not a known profile"));
        }
    }
    if let Some((d, file)) = &direnv {
        if !reg.contains(d) {
            warnings.push(format!(
                "{} sets CLAUDE_PROFILE={d} – not a known profile",
                paths::tilde(file)
            ));
        }
    }
    for w in warnings {
        eprintln!("Warning: {w}");
    }
    Ok(())
}

pub fn default(name: &str, yes: bool) -> Result<()> {
    let mut reg = Registry::load()?;
    let name = &reg.id(name)?;
    let dir = reg.get(name)?.dir_path()?;
    if !dir.is_dir() {
        bail!("profile directory {} is missing", paths::tilde(&dir));
    }
    let state = link::state()?;
    match &state {
        LinkState::RealDir | LinkState::Other => {
            return Err(usage(
                "~/.claude is not a symlink – leaving it untouched (adopt it: `claude-profile add --adopt`)",
            ))
        }
        LinkState::Missing | LinkState::Symlink { .. } => {}
    }
    let current = link::linked_profile(&reg, &state);
    let relink = current.as_deref() != Some(name);
    let json = link::json_action(&dir, dir.join(".claude.json").exists())?;
    if !relink && json == JsonAction::Nothing {
        println!("~/.claude and ~/.claude.json already point to `{name}`.");
    } else {
        if relink {
            let from = match (&state, &current) {
                (_, Some(c)) => format!("was: {c}"),
                (LinkState::Symlink { target, .. }, None) => {
                    format!("was: {}", paths::tilde(target))
                }
                _ => "was: missing".to_string(),
            };
            println!("symlink: ~/.claude → {} ({from})", paths::tilde(&dir));
        }
        if json != JsonAction::Nothing {
            println!("{}", link::describe_json(&json, &dir));
        }
        if !prompt::confirm("Proceed?", yes)? {
            println!("Aborted – nothing changed.");
            return Ok(());
        }
        if relink {
            link::set(&dir)?;
        }
        if let Some(backup) = link::apply_json(&json, &dir)? {
            println!("Backup: {}", paths::tilde(&backup));
        }
        println!("The default config now points to profile `{name}`.");
    }
    if reg.link_target.as_deref() != Some(name) {
        reg.link_target = Some(name.to_string());
        reg.save()?;
    }
    Ok(())
}

/// `claude-profile rename <name> <new>` – only changes how the profile name is shown.
/// The lowercase ID and everything tied to it stays.
pub fn rename(name: &str, new: &str) -> Result<()> {
    let mut reg = Registry::load()?;
    let id = reg.id(name)?;
    let old = reg.shown(&id).to_string();
    validate_name(new)?;
    if profile_id(new) != id {
        return Err(usage(format!(
            "`{new}` is a different name than `{old}` – `rename` only changes capitalization"
        )));
    }
    if old == new {
        println!("Profile is already called `{old}` – nothing to do.");
        return Ok(());
    }
    reg.profiles.get_mut(&id).expect("profile was just found").name = spelling(new);
    reg.save()?;
    println!("Profile `{old}` is now shown as `{new}`. This applies to newly started sessions.");
    println!("A starter app gets the new name with `claude-profile desktop-app {id}`.");
    Ok(())
}
