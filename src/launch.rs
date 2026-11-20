//! Profile resolution (PRD section 5) and starting Claude Code (section 6).

use crate::badge;
use crate::color;
use crate::error::usage;
use crate::paths;
use crate::prompt;
use crate::registry::Registry;
use anyhow::{anyhow, bail, Result};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

/// Returns the lowercase ID of the profile.
/// Order: explicit argument → `CLAUDE_PROFILE` → interactive picker → abort.
/// There is no default profile.
pub fn resolve(reg: &Registry, explicit: Option<&str>, use_env: bool) -> Result<String> {
    if let Some(name) = explicit {
        return reg.id(name);
    }
    if use_env {
        if let Ok(name) = std::env::var("CLAUDE_PROFILE") {
            if !name.is_empty() {
                return reg
                    .id(&name)
                    .map_err(|e| usage(format!("CLAUDE_PROFILE: {e}")));
            }
        }
    }
    if reg.profiles.is_empty() {
        return Err(usage(
            "no profiles yet – run `claude-profile add` first",
        ));
    }
    if !prompt::interactive() {
        return Err(usage(format!(
            "no profile selected: CLAUDE_PROFILE is not set and there is no terminal to choose one (profiles: {})",
            reg.names().join(", ")
        )));
    }
    let names = reg.names();
    // Each profile in its badge colors – the same ones shown above the prompt later.
    let width = names
        .iter()
        .map(|n| reg.shown(n).chars().count())
        .max()
        .unwrap_or(0);
    let items: Vec<String> = names
        .iter()
        .map(|n| {
            let p = &reg.profiles[n];
            format!("{}  ({})", color::label(n, p, width), p.dir)
        })
        .collect();
    // No echo of the choice: start and desktop report the profile themselves in their status line.
    let idx = prompt::select_silent("Claude profile", &items, 0)?;
    Ok(names[idx].clone())
}

/// Glob with `*` as the only wildcard.
pub fn glob_match(pattern: &str, s: &str) -> bool {
    let (p, s) = (pattern.as_bytes(), s.as_bytes());
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if pi < p.len() && p[pi] == s[si] {
            pi += 1;
            si += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

fn is_executable(p: &std::path::Path) -> bool {
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Looks for the real `claude` binary in PATH (the shell function does not live there).
pub fn find_claude_in_path() -> Option<PathBuf> {
    let me = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok());
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join("claude"))
        .find(|c| is_executable(c) && c.canonicalize().ok() != me)
}

fn claude_bin(reg: &Registry) -> Result<PathBuf> {
    if let Some(bin) = &reg.claude_bin {
        let p = paths::expand(bin)?;
        if is_executable(&p) {
            return Ok(p);
        }
    }
    find_claude_in_path().ok_or_else(|| {
        anyhow!("claude binary not found (neither `claude_bin` in profiles.json nor in PATH)")
    })
}

/// Replaces the process with Claude Code using the chosen profile. Only returns on error.
pub fn launch(reg: &Registry, name: &str, args: &[String]) -> Result<()> {
    let profile = reg.get(name)?;
    let dir = profile.dir_path()?;
    if !dir.is_dir() {
        bail!(
            "profile directory {} for `{name}` is missing",
            paths::tilde(&dir)
        );
    }
    let bin = claude_bin(reg)?;

    let mut cmd = Command::new(&bin);
    cmd.args(args);

    let mut removed = Vec::new();
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        if profile.forbid_env.iter().any(|p| glob_match(p, key)) {
            cmd.env_remove(key);
            removed.push(key.to_string());
        }
    }
    cmd.env("CLAUDE_CONFIG_DIR", &dir);
    cmd.env("CLAUDE_PROFILE", name);
    if badge::enabled() {
        // The badge is a convenience: an error here must not prevent the start.
        match badge::ensure() {
            Ok(mod_dir) => {
                cmd.env(badge::PLUGIN_DIRS_ENV, badge::plugin_dirs(&mod_dir));
            }
            Err(e) => eprintln!("claude-profile: warning: profile badge not available: {e:#}"),
        }
    }

    if !removed.is_empty() {
        removed.sort();
        eprintln!(
            "claude-profile: warning: removed variables forbidden for profile `{name}`: {}",
            removed.join(", ")
        );
    }
    if std::env::var_os("CLAUDE_PROFILE_QUIET").is_none() {
        eprintln!(
            "claude-profile: {} ({})",
            color::label(name, profile, 0),
            paths::tilde(&dir)
        );
    }

    let err = cmd.exec();
    Err(anyhow!(err).context(format!("cannot start {}", bin.display())))
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn glob() {
        assert!(glob_match("OTEL_*", "OTEL_EXPORTER_OTLP_ENDPOINT"));
        assert!(glob_match("OTEL_*", "OTEL_"));
        assert!(!glob_match("OTEL_*", "MY_OTEL_X"));
        assert!(glob_match("ANTHROPIC_BASE_URL", "ANTHROPIC_BASE_URL"));
        assert!(!glob_match("ANTHROPIC_BASE_URL", "ANTHROPIC_BASE_URL_2"));
        assert!(glob_match("*_VERTEX_*", "ANTHROPIC_VERTEX_PROJECT_ID"));
        assert!(glob_match("A*B*C", "AxxBxxBC"));
        assert!(!glob_match("A*B*C", "AxxBxx"));
        assert!(!glob_match("", "X"));
    }
}
