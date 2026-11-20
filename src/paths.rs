use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub fn home() -> Result<PathBuf> {
    match std::env::var_os("HOME") {
        Some(h) if !h.is_empty() => Ok(PathBuf::from(h)),
        _ => Err(anyhow!("HOME is not set")),
    }
}

pub fn registry_path() -> Result<PathBuf> {
    Ok(home()?.join(".config/claude-profiles/profiles.json"))
}

/// Storage for files that can be regenerated at any time (icons for the launcher).
pub fn cache_dir() -> Result<PathBuf> {
    Ok(home()?.join(".config/claude-profiles/cache"))
}

/// `~/.claude` – symlink to a profile (or still a real directory).
pub fn claude_link() -> Result<PathBuf> {
    Ok(home()?.join(".claude"))
}

pub fn claude_json() -> Result<PathBuf> {
    Ok(home()?.join(".claude.json"))
}

/// Default directory of a profile; `id` is the lowercase ID.
pub fn profile_dir(id: &str) -> Result<PathBuf> {
    Ok(home()?.join(format!(".claude-{id}")))
}

/// Expands a leading `~`.
pub fn expand(s: &str) -> Result<PathBuf> {
    if s == "~" {
        home()
    } else if let Some(rest) = s.strip_prefix("~/") {
        Ok(home()?.join(rest))
    } else {
        Ok(PathBuf::from(s))
    }
}

/// Shortens `$HOME` to `~` (for display and the registry).
pub fn tilde(p: &Path) -> String {
    if let Ok(h) = home() {
        if let Ok(rest) = p.strip_prefix(&h) {
            return if rest.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    p.display().to_string()
}
