//! Rewrite hard-coded paths to `~/.claude/` in a profile to the profile directory.
//!
//! An adopted `~/.claude` contains references to itself: hook commands in
//! `settings.json` (`~/.claude/hooks/…`) and the plugins' install paths. As long as
//! `~/.claude` points to this profile, nobody notices. Once the symlink points to another
//! profile, these paths end up there – hooks fail, plugins are not found.

use crate::paths;
use crate::prompt;
use crate::registry::Registry;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Files in which Claude Code or the user stores such paths.
const FILES: &[&str] = &[
    "settings.json",
    "settings.local.json",
    "plugins/installed_plugins.json",
    "plugins/known_marketplaces.json",
];

pub struct Finding {
    pub file: &'static str,
    pub count: usize,
}

/// (old, new) for every spelling of `<home>/.claude/`.
fn replacements(profile_dir: &Path) -> Result<Vec<(String, String)>> {
    let home = paths::home()?;
    let abs = format!("{}/", profile_dir.display());
    let pairs = match profile_dir.strip_prefix(&home) {
        Ok(rest) => {
            let rest = rest.display();
            vec![
                (format!("{}/.claude/", home.display()), abs),
                ("~/.claude/".to_string(), format!("~/{rest}/")),
                ("$HOME/.claude/".to_string(), format!("$HOME/{rest}/")),
                ("${HOME}/.claude/".to_string(), format!("${{HOME}}/{rest}/")),
            ]
        }
        // Profile outside of $HOME: the absolute path everywhere.
        Err(_) => vec![
            (format!("{}/.claude/", home.display()), abs.clone()),
            ("~/.claude/".to_string(), abs.clone()),
            ("$HOME/.claude/".to_string(), abs.clone()),
            ("${HOME}/.claude/".to_string(), abs),
        ],
    };
    Ok(pairs)
}

fn rewrite_text(text: &str, pairs: &[(String, String)]) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0;
    for (from, to) in pairs {
        count += out.matches(from.as_str()).count();
        out = out.replace(from.as_str(), to);
    }
    (out, count)
}

/// Counts the occurrences. `root` is where the files currently live
/// (still `~/.claude` when adopting), `profile_dir` the future profile directory.
pub fn scan(root: &Path, profile_dir: &Path) -> Result<Vec<Finding>> {
    let pairs = replacements(profile_dir)?;
    Ok(FILES
        .iter()
        .filter_map(|file| {
            let text = fs::read_to_string(root.join(file)).ok()?;
            let (_, count) = rewrite_text(&text, &pairs);
            (count > 0).then_some(Finding { file, count })
        })
        .collect())
}

pub fn describe(findings: &[Finding], profile_dir: &Path) -> String {
    let files: Vec<String> = findings
        .iter()
        .map(|f| format!("{} ({})", f.file, f.count))
        .collect();
    format!(
        "adjust paths: ~/.claude/ → {}/ in {} – backups next to them",
        paths::tilde(profile_dir),
        files.join(", ")
    )
}

/// Rewrites the paths in the files under `profile_dir`. Before every change a
/// backup is created next to the file. Returns the number of changed occurrences.
pub fn apply(profile_dir: &Path) -> Result<usize> {
    let pairs = replacements(profile_dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut total = 0;
    for file in FILES {
        let path = profile_dir.join(file);
        let Ok(old) = fs::read_to_string(&path) else {
            continue;
        };
        let (new, count) = rewrite_text(&old, &pairs);
        if count == 0 {
            continue;
        }
        let was_json = serde_json::from_str::<serde_json::Value>(&old).is_ok();
        if was_json && serde_json::from_str::<serde_json::Value>(&new).is_err() {
            bail!(
                "{} would no longer be valid JSON after the change – not changed",
                paths::tilde(&path)
            );
        }
        let backup = PathBuf::from(format!("{}.claude-profile-backup-{stamp}", path.display()));
        fs::copy(&path, &backup)
            .with_context(|| format!("cannot create backup {}", paths::tilde(&backup)))?;
        fs::write(&path, new)
            .with_context(|| format!("cannot write {}", paths::tilde(&path)))?;
        total += count;
    }
    Ok(total)
}

/// `claude-profile fix-paths <name>`
pub fn command(name: &str, dry_run: bool, yes: bool) -> Result<()> {
    let reg = Registry::load()?;
    let name = &reg.id(name)?;
    let dir = reg.get(name)?.dir_path()?;
    if !dir.is_dir() {
        bail!("profile directory {} is missing", paths::tilde(&dir));
    }
    let findings = scan(&dir, &dir)?;
    if findings.is_empty() {
        println!("Profile `{name}` contains no paths to ~/.claude/ – nothing to do.");
        return Ok(());
    }
    println!("{}", describe(&findings, &dir));
    if dry_run {
        println!("(dry-run – nothing changed)");
        return Ok(());
    }
    if !prompt::confirm("Proceed?", yes)? {
        println!("Aborted – nothing changed.");
        return Ok(());
    }
    let total = apply(&dir)?;
    println!("Adjusted {total} occurrences. Restart running sessions of the profile.");
    Ok(())
}
