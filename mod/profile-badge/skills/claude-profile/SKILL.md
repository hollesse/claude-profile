---
name: claude-profile
description: Use before reading or changing Claude Code's own configuration on this machine – user settings (settings.json), the user-level CLAUDE.md, skills, plugins, hooks, MCP servers, keybindings, memory, or anything a guide places under ~/.claude or in ~/.claude.json – and for questions about profiles. Claude Code is split into profiles by claude-profile here, so ~/.claude is usually NOT this session's config directory. Explains how to find the right directory and how to inspect or switch profiles.
---

# claude-profile: where Claude Code's configuration lives

This machine runs `claude-profile`. It splits Claude Code into several **profiles** – for example one for a personal subscription and one for a company backend. Each profile has its own config directory with its own settings, history, plugins and login. The profiles exist to keep these contexts apart, so writing to the wrong one is a real mistake, not a cosmetic one.

## The rule

**User-level configuration of this session lives in `$CLAUDE_CONFIG_DIR`, not in `~/.claude`.**

`~/.claude` and `~/.claude.json` still exist, but they are symlinks to the *default profile* – the one that tools with a hard-wired path get. That may be a different profile than the one this session runs with. Whenever documentation, a guide, a skill or your own habit says `~/.claude/...`, use `$CLAUDE_CONFIG_DIR/...` instead.

| A guide says | Use in this session |
|---|---|
| `~/.claude/settings.json` | `$CLAUDE_CONFIG_DIR/settings.json` |
| `~/.claude/CLAUDE.md` | `$CLAUDE_CONFIG_DIR/CLAUDE.md` |
| `~/.claude/skills/`, `agents/`, `commands/`, `hooks/`, `plugins/` | the same folder under `$CLAUDE_CONFIG_DIR/` |
| `~/.claude/projects/` (history, memory) | `$CLAUDE_CONFIG_DIR/projects/` |
| `~/.claude.json` (MCP servers, state) | `$CLAUDE_CONFIG_DIR/.claude.json` |

Project-level configuration is **not** affected: `.claude/settings.json`, `.claude/settings.local.json`, `CLAUDE.md` and `.mcp.json` inside a repository stay where they are and apply to every profile.

Inside files of a profile, write paths to the profile directory itself (`~/.claude-<id>/hooks/…`), never `~/.claude/…` – such a path breaks as soon as the default profile changes.

## Find out where you are

```sh
echo "$CLAUDE_PROFILE"       # ID of this session's profile, e.g. vertex
echo "$CLAUDE_CONFIG_DIR"    # its config directory, e.g. /Users/you/.claude-vertex
claude-profile list          # all profiles, their directories and status
```

`claude-profile list` prints a table. The marks at the end of a row mean:

- `← symlink` – `~/.claude` and `~/.claude.json` point to this profile (the default profile).
- `← active` – the profile named by `CLAUDE_PROFILE` in the current environment.
- `← direnv` – the profile the nearest `.envrc` sets for this directory.

Warnings on stderr name problems and the command that repairs them.

If `CLAUDE_CONFIG_DIR` is not set, this session was started without `claude-profile` and uses the default config `~/.claude`. `readlink ~/.claude` shows which profile that is.

`claude-profile list --names` prints only the IDs, one per line. The registry itself is `~/.config/claude-profiles/profiles.json`; each profile's `dir` field is its config directory.

## Which profile does the user mean?

- "My settings", "my config", "add a hook", "install this skill for me": this session's profile, `$CLAUDE_CONFIG_DIR`.
- A named profile ("in my vertex profile"): look up its directory with `claude-profile list` and edit there.
- "For all profiles", or unclear which one: ask. Do not copy configuration between profiles on your own – they are separate on purpose, and settings such as backend variables or credentials helpers must not leak from one into the other.

A change to another profile takes effect when a session of that profile starts; a change to this session's settings may need a restart of this session.

## Commands you may run

| Command | What it does |
|---|---|
| `claude-profile list` | Show profiles, directories, status and warnings. Read-only. |
| `claude-profile fix-paths <name> --dry-run` | Show hard-wired `~/.claude/` paths inside a profile. Read-only. |
| `claude-profile add <name> --dry-run …` | Show what creating a profile would do. Read-only. |
| `claude-profile desktop <name> --dry-run` | Show how Claude Desktop would be started for a profile. Read-only. |

These change things – run them only when the user asked for exactly that:

| Command | What it changes |
|---|---|
| `claude-profile add <name> --new --yes` | Creates an empty profile. Without flags it asks questions and needs a terminal. |
| `claude-profile set <name>` / `set --unset` | Writes or removes the profile for the current directory in `.envrc` (direnv). |
| `claude-profile badge <name> --bg '#rrggbb'` | Sets the badge color of a profile. |
| `claude-profile rename <name> <NEW>` | Changes the capitalization a profile is displayed with. |
| `claude-profile fix-paths <name> --yes` | Rewrites `~/.claude/` paths inside a profile to its own directory, with backups. |
| `claude-profile default <name> --yes` | Points `~/.claude` and `~/.claude.json` to another profile. Affects every tool that uses the default path – always confirm with the user first. |
| `claude-profile replace-claude-command` | Adds or removes the `claude` shell function in the rc file. |

`claude-profile <command> --help` shows all options.

## Do not

- Do not write user-level configuration to `~/.claude/…` or `~/.claude.json`.
- Do not replace, remove or recreate the symlinks `~/.claude` and `~/.claude.json` by hand, and do not create a real directory at `~/.claude`.
- Do not edit `~/.config/claude-profiles/profiles.json` or the folder `~/.config/claude-profiles/mods/` by hand unless the user asks; the mod folder is rewritten on every start.
- Do not delete a profile directory. `claude-profile` has no delete command on purpose.
- Do not start a nested `claude` to "switch" profiles. To work with another profile, the user starts a session with it: `claude-profile start <name>`, or `CLAUDE_PROFILE=<name> claude`.
- Do not set variables that a profile forbids (`forbid_env` in `profiles.json`) to work around the separation.
