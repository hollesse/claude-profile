//! `claude --resume <id>` via the wrapper: use the profile the session ran under.
//!
//! Claude Code stores sessions per config directory:
//! `<profile>/projects/<folder>/<session-id>.jsonl`. With the wrong profile,
//! `--resume <id>` does not find the session.

use crate::registry::Registry;
use anyhow::Result;
use std::fs;
use std::time::SystemTime;

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// The session ID from `--resume <id>`, `--resume=<id>`, `-r <id>` or `-r=<id>`.
/// Without a value, or with a search term or session name instead of an ID, returns `None`.
pub fn session_id(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let value = match arg.as_str() {
            "--resume" | "-r" => it.next().map(String::as_str),
            a => a.strip_prefix("--resume=").or_else(|| a.strip_prefix("-r=")),
        };
        if let Some(v) = value.filter(|v| is_uuid(v)) {
            return Some(v.to_ascii_lowercase());
        }
    }
    None
}

/// Profiles that contain the session, each with the session's modification time.
fn find(reg: &Registry, id: &str) -> Vec<(String, SystemTime)> {
    let file = format!("{id}.jsonl");
    let mut found = Vec::new();
    for name in reg.names() {
        let Ok(dir) = reg.profiles[&name].dir_path() else {
            continue;
        };
        let Ok(projects) = fs::read_dir(dir.join("projects")) else {
            continue;
        };
        let newest = projects
            .filter_map(|p| p.ok())
            .filter_map(|p| fs::metadata(p.path().join(&file)).ok())
            .filter(|m| m.is_file())
            .filter_map(|m| m.modified().ok())
            .max();
        if let Some(t) = newest {
            found.push((name, t));
        }
    }
    found
}

/// The session's profile from `args`, if they contain `--resume <id>` and the session lives in a
/// profile. If it lives in several, `CLAUDE_PROFILE` wins, otherwise the most recently modified.
pub fn session_profile(reg: &Registry, args: &[String]) -> Result<Option<String>> {
    let Some(id) = session_id(args) else {
        return Ok(None);
    };
    let mut found = find(reg, &id);
    if found.is_empty() {
        return Ok(None);
    }
    let env = std::env::var("CLAUDE_PROFILE").unwrap_or_default();
    found.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    let chosen = found
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(&env))
        .unwrap_or(&found[0])
        .0
        .clone();
    if !env.is_empty() && !env.eq_ignore_ascii_case(&chosen) && std::env::var_os("CLAUDE_PROFILE_QUIET").is_none() {
        eprintln!(
            "claude-profile: session {id} ran under `{chosen}` – ignoring CLAUDE_PROFILE={env} for it"
        );
    }
    Ok(Some(chosen))
}

#[cfg(test)]
mod tests {
    use super::session_id;

    const ID: &str = "8d2182af-6c52-4815-a30c-e9e15c75738e";

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn finds_id_in_all_spellings() {
        let eq = vec![format!("--resume={ID}")];
        for args in [
            a(&["--resume", ID]),
            a(&["-r", ID]),
            eq,
            a(&["--model", "x", "-r", ID, "--fork-session"]),
        ] {
            assert_eq!(session_id(&args).as_deref(), Some(ID), "{args:?}");
        }
    }

    #[test]
    fn ignores_everything_else() {
        assert_eq!(session_id(&a(&[])), None);
        assert_eq!(session_id(&a(&["--resume"])), None);
        assert_eq!(session_id(&a(&["--resume", "my-title"])), None);
        assert_eq!(session_id(&a(&["--continue"])), None);
        assert_eq!(session_id(&a(&["-p", ID])), None);
    }
}
