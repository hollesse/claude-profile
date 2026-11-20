# How it works

## Two ways to Claude Code

```text
                 ┌──────────────────────────────┐
 claude ────────▶│ shell function               │
                 │  claude-profile run          │
                 │   1. resolve the profile     │
                 │   2. set CLAUDE_CONFIG_DIR   │──▶ ~/.claude-<profile>/
                 │   3. remove forbid_env       │
                 │   4. start the real claude   │
                 └──────────────────────────────┘

 IDE plugin,     ┌──────────────────────────────┐
 setup tool, ───▶│ default paths                │
 command claude  │  ~/.claude       (symlink)   │──▶ ~/.claude-<default>/
                 │  ~/.claude.json  (symlink)   │
                 └──────────────────────────────┘
```

**The controlled way** is the shell function. It never starts without a resolved profile and passes the directory explicitly as `CLAUDE_CONFIG_DIR`.

**The uncontrolled way** is every program that uses the default path. Those programs cannot choose a profile. So that they still end up in a defined profile, `~/.claude` and `~/.claude.json` are symlinks.

## Why two symlinks?

Claude Code spreads its state over two places:

| | without `CLAUDE_CONFIG_DIR` | with `CLAUDE_CONFIG_DIR=<dir>` |
|---|---|---|
| Settings, history, plugins | `~/.claude/` | `<dir>/` |
| State (onboarding, MCP servers, project list) | `~/.claude.json` | `<dir>/.claude.json` |

In the default case the state file therefore sits **next to** the directory, not inside it. With a symlink only for `~/.claude`, a start through the default path would see the profile's settings but an empty state – Claude Code would behave as if freshly installed. That is why `claude-profile` maintains both symlinks together. Claude Code writes through the symlink and preserves it (checked with version 2.1.329).

## Profile badge

So you can tell which profile a session belongs to, `claude-profile` loads a small mod into Claude Code at every start. It draws a colored badge with the profile name above the input field. You set the colors when you create the profile or later with [`claude-profile badge`](reference/commands.md#badge); a profile without stored colors always gets the same one, derived from its name.

```text
 acme
```

In Claude Desktop the mod colors the session's window instead: the surfaces are tinted in the badge color, and the background fades out from that color at the top. There is no badge above the input field there. Two limitations:

- The color only appears with the first message, because no Claude Code session is running before that.
- In Vertex mode, Claude Desktop starts Claude Code with its own environment and its own config directory. The mod is not loaded there, and the window stays uncolored.

The mod lives in `~/.config/claude-profiles/mods/profile-badge` and is passed via `CLAUDE_CODE_PLUGIN_DIRS`, to Claude Desktop as well. It reads `CLAUDE_PROFILE`, `CLAUDE_CONFIG_DIR` and the colors and display name from `profiles.json`, and changes nothing. If both variables are missing, it follows the `~/.claude` symlink and shows the profile with the suffix "via ~/.claude" – that is how you recognize a session that was started past `claude-profile` and for which `forbid_env` therefore does not apply. Set `CLAUDE_PROFILE_NO_BADGE=1` to turn it off.

The mod's source is in the repository under `mod/profile-badge`.

## What is separated

- Settings (`settings.json`), `CLAUDE.md`, plugins, skills, hooks
- Session history and project data
- State and MCP configuration (`.claude.json`)
- Login: each profile signs in once on its own
- Environment: `forbid_env` removes variables of the other context

## What is not separated

`claude-profile` is not a sandbox and does not fully replace a virtual machine:

- **The default path belongs to exactly one profile.** Everything that does not start through `claude-profile` uses the profile the symlinks point to – even while you are working in the other context.
- **The shell function only applies in interactive shells** that load your rc file. Scripts that call `claude` directly and IDE terminals without an rc file bypass it.
- **The file system is shared.** A session can read files of the other context if you lead it there.
- **Background services run globally.** A telemetry daemon of the company context runs independently of the profile. Sessions of the other profile send nothing to it only as long as the relevant variables (such as `OTEL_*`) are not in their environment – that is what `forbid_env` is for.
- **Claude Desktop is only separated when you start it through `claude-profile`.** An instance started directly from the Applications folder uses the profile the symlinks point to, see [Claude Desktop per profile](guides/desktop.md).

## Principles

- **No guessing.** An unknown profile, an invalid name or a missing choice leads to an abort, never to a substitute profile.
- **No data loss.** Things are renamed and moved, never deleted. A backup is created before rc files are changed.
- **No credentials.** `claude-profile` does not read or copy credentials and sends nothing over the network.
