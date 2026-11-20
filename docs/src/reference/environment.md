# Environment variables and exit codes

## Variables claude-profile reads

| Variable | Effect |
|---|---|
| `CLAUDE_PROFILE` | Profile for `run` (that is, for `claude`) and for `desktop` without a name. An unknown value is an error. `start` ignores the variable. |
| `CLAUDE_PROFILE_QUIET` | If set, the status line at start is omitted. Warnings about removed variables still appear. |
| `CLAUDE_PROFILE_NO_BADGE` | If set, the [profile badge](../how-it-works.md#profile-badge) is not loaded. |
| `NO_COLOR` | If set, profile names appear without badge colors in the picker and in the status line. |
| `HOME` | Base for all paths (`~/.claude`, `~/.claude-<name>`, `~/.config/claude-profiles`). |
| `PATH` | Search for the real `claude` and for `direnv`. |
| `SHELL` | Shell detection in `replace-claude-command`. |
| `ZDOTDIR` | Location of `.zshrc`, if set. |

## Variables claude-profile sets for Claude Code

| Variable | Value |
|---|---|
| `CLAUDE_CONFIG_DIR` | Directory of the chosen profile. Set on every start, also for the profile the symlink points to. |
| `CLAUDE_PROFILE` | ID of the chosen profile, that is, the name in lowercase. |
| `CLAUDE_CODE_PLUGIN_DIRS` | Folder of the profile badge; folders set before are kept after it. Omitted with `CLAUDE_PROFILE_NO_BADGE`. |

Variables that match the profile's `forbid_env` are removed. See [profiles.json](profiles-json.md#forbid_env).

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success. For `run` and `start`, the exit code of Claude Code itself applies afterwards. |
| `1` | Error during execution, for example a missing profile directory or a target that is not overwritten. |
| `2` | Wrong input or missing decision: unknown profile, invalid name, no profile selected and no terminal, confirmation needed without `--yes`. |

## Files

| Path | Purpose |
|---|---|
| `~/.config/claude-profiles/profiles.json` | Registry |
| `~/.config/claude-profiles/mods/profile-badge/` | Mod for the profile badge; kept up to date on every start |
| `~/.claude-<name>/` | Config directory of a profile |
| `~/.claude` | Symlink to a profile directory |
| `~/.claude.json` | Symlink to the `.claude.json` of the same profile |
| `.envrc` | Profile choice per directory (block between `# BEGIN claude-profile` and `# END claude-profile`) |
| `~/.zshrc`, `~/.bashrc` | Shell function `claude` (also as a marked block) |
