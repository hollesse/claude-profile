//! State and maintenance of the symlink `~/.claude`.

use crate::paths;
use crate::registry::Registry;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum LinkState {
    Missing,
    RealDir,
    /// A file or something else unexpected.
    Other,
    Symlink { target: PathBuf, exists: bool },
}

pub fn state() -> Result<LinkState> {
    let link = paths::claude_link()?;
    let meta = match fs::symlink_metadata(&link) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LinkState::Missing),
        Err(e) => return Err(e).context("cannot check ~/.claude"),
    };
    if meta.file_type().is_symlink() {
        let raw = fs::read_link(&link).context("cannot read symlink ~/.claude")?;
        let target = if raw.is_absolute() {
            raw
        } else {
            paths::home()?.join(raw)
        };
        let exists = target.is_dir();
        Ok(LinkState::Symlink { target, exists })
    } else if meta.is_dir() {
        Ok(LinkState::RealDir)
    } else {
        Ok(LinkState::Other)
    }
}

/// Name of the profile `~/.claude` points to (if registered).
pub fn linked_profile(reg: &Registry, state: &LinkState) -> Option<String> {
    let LinkState::Symlink { target, .. } = state else {
        return None;
    };
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let target = canon(target);
    reg.profiles
        .iter()
        .find(|(_, p)| p.dir_path().map(|d| canon(&d) == target).unwrap_or(false))
        .map(|(n, _)| n.clone())
}

/// Atomically creates `link` as a symlink to `target` (via a temporary sibling file).
fn replace_symlink(link: &Path, target: &Path) -> Result<()> {
    let tmp = link.with_file_name(format!(".claude-profile-tmp-{}", std::process::id()));
    if fs::symlink_metadata(&tmp).is_ok() {
        fs::remove_file(&tmp).ok();
    }
    std::os::unix::fs::symlink(target, &tmp).context("cannot create symlink")?;
    fs::rename(&tmp, link)
        .with_context(|| format!("cannot set symlink {}", paths::tilde(link)))?;
    Ok(())
}

/// Atomically points `~/.claude` at `target`. Only replaces a symlink, never a directory.
pub fn set(target: &Path) -> Result<()> {
    let link = paths::claude_link()?;
    match state()? {
        LinkState::Missing | LinkState::Symlink { .. } => {}
        _ => bail!("~/.claude is not a symlink – not replacing it"),
    }
    replace_symlink(&link, target)
}

/// What has to happen to `~/.claude.json` so that it points to the profile's `.claude.json`.
///
/// Without `CLAUDE_CONFIG_DIR`, Claude Code reads its state (onboarding, history, MCP) from
/// `~/.claude.json`, not from `~/.claude/`. So the symlink `~/.claude` alone is not enough.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonAction {
    Nothing,
    /// Create or repoint the symlink.
    Link,
    /// Move the real file into the profile (the profile has none yet), then symlink.
    MoveAndLink,
    /// Rename the real file (the profile already has one), then symlink.
    BackupAndLink,
}

/// `profile_has_json`: does the profile (after all previous steps) already have a `.claude.json`?
pub fn json_action(profile_dir: &Path, profile_has_json: bool) -> Result<JsonAction> {
    let json = paths::claude_json()?;
    let meta = match fs::symlink_metadata(&json) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(JsonAction::Link),
        Err(e) => return Err(e).context("cannot check ~/.claude.json"),
    };
    if meta.file_type().is_symlink() {
        let current = fs::read_link(&json).context("cannot read symlink ~/.claude.json")?;
        Ok(if current == profile_dir.join(".claude.json") {
            JsonAction::Nothing
        } else {
            JsonAction::Link
        })
    } else if meta.is_file() {
        Ok(if profile_has_json {
            JsonAction::BackupAndLink
        } else {
            JsonAction::MoveAndLink
        })
    } else {
        bail!("~/.claude.json is neither a file nor a symlink – please resolve manually")
    }
}

pub fn describe_json(action: &JsonAction, profile_dir: &Path) -> String {
    let target = paths::tilde(&profile_dir.join(".claude.json"));
    match action {
        JsonAction::Nothing => "~/.claude.json: unchanged".to_string(),
        JsonAction::Link => format!("symlink: ~/.claude.json → {target}"),
        JsonAction::MoveAndLink => {
            format!("move: ~/.claude.json → {target}, then symlink to it")
        }
        JsonAction::BackupAndLink => format!(
            "rename: ~/.claude.json → ~/.claude.json.claude-profile-backup-<time>, then symlink → {target}"
        ),
    }
}

/// Performs the action. Returns the path of a created backup. Never deletes anything.
pub fn apply_json(action: &JsonAction, profile_dir: &Path) -> Result<Option<PathBuf>> {
    let json = paths::claude_json()?;
    let target = profile_dir.join(".claude.json");
    let mut backup = None;
    match action {
        JsonAction::Nothing => return Ok(None),
        JsonAction::Link => {}
        JsonAction::MoveAndLink => {
            if fs::symlink_metadata(&target).is_ok() {
                bail!("{} already exists", paths::tilde(&target));
            }
            fs::rename(&json, &target)
                .with_context(|| format!("cannot move ~/.claude.json to {}", paths::tilde(&target)))?;
        }
        JsonAction::BackupAndLink => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let to = PathBuf::from(format!("{}.claude-profile-backup-{stamp}", json.display()));
            if fs::symlink_metadata(&to).is_ok() {
                bail!("{} already exists", paths::tilde(&to));
            }
            fs::rename(&json, &to).context("cannot rename ~/.claude.json")?;
            backup = Some(to);
        }
    }
    replace_symlink(&json, &target)?;
    Ok(backup)
}
