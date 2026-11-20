//! Marked block in text files (`.envrc`, `~/.zshrc`, `~/.bashrc`).

pub const BEGIN: &str = "# BEGIN claude-profile";
pub const END: &str = "# END claude-profile";

/// Line range `[begin, end]` of the block (including the markers).
fn find(lines: &[&str]) -> Option<(usize, usize)> {
    let begin = lines.iter().position(|l| l.trim() == BEGIN)?;
    let end = lines[begin..].iter().position(|l| l.trim() == END)? + begin;
    Some((begin, end))
}

/// Content of the block without the markers.
pub fn body(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let (b, e) = find(&lines)?;
    Some(lines[b + 1..e].concat())
}

/// Replaces the block or appends it. `body` without markers; lines without a trailing `\n` are allowed.
pub fn upsert(content: &str, body: &str) -> String {
    let mut block = format!("{BEGIN}\n{body}");
    if !block.ends_with('\n') {
        block.push('\n');
    }
    block.push_str(END);
    block.push('\n');

    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    match find(&lines) {
        Some((b, e)) => {
            let mut out = lines[..b].concat();
            out.push_str(&block);
            out.push_str(&lines[e + 1..].concat());
            out
        }
        None => {
            let mut out = content.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&block);
            out
        }
    }
}

pub fn remove(content: &str) -> String {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    match find(&lines) {
        Some((b, e)) => {
            let mut out = lines[..b].concat();
            out.push_str(&lines[e + 1..].concat());
            out
        }
        None => content.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_is_idempotent_and_removable() {
        let orig = "export A=1\n";
        let once = upsert(orig, "export CLAUDE_PROFILE=vertex");
        assert_eq!(
            once,
            "export A=1\n# BEGIN claude-profile\nexport CLAUDE_PROFILE=vertex\n# END claude-profile\n"
        );
        assert_eq!(upsert(&once, "export CLAUDE_PROFILE=vertex"), once);
        assert_eq!(remove(&once), orig);
    }

    #[test]
    fn replaces_in_place() {
        let c = "a\n# BEGIN claude-profile\nold\n# END claude-profile\nb\n";
        let n = upsert(c, "new");
        assert_eq!(n, "a\n# BEGIN claude-profile\nnew\n# END claude-profile\nb\n");
        assert_eq!(body(&n).as_deref(), Some("new\n"));
        assert_eq!(remove(&n), "a\nb\n");
    }

    #[test]
    fn no_trailing_newline_and_empty() {
        assert_eq!(upsert("x", "y"), "x\n# BEGIN claude-profile\ny\n# END claude-profile\n");
        assert_eq!(upsert("", "y"), "# BEGIN claude-profile\ny\n# END claude-profile\n");
        assert_eq!(remove("nothing\n"), "nothing\n");
        assert_eq!(body("nothing\n"), None);
    }
}
