# Getting started

By the end of this page you will have adopted your existing Claude Code as a profile, and `claude` will only start with a profile.

> **Before you begin:** Quit all running Claude Code sessions. Step 1 renames `~/.claude`.

## 1. Adopt the existing `~/.claude` as a profile

First look at what would happen – `--dry-run` changes nothing:

```sh
claude-profile add personal --adopt --dry-run
```

```text
Plan for profile `personal`:
  1. rename: ~/.claude → ~/.claude-personal
  2. symlink: ~/.claude → ~/.claude-personal (profile personal)
  3. move: ~/.claude.json → ~/.claude-personal/.claude.json, then symlink to it
  4. register in profiles.json (forbid_env: empty; badge: text #000000 on #4c9aff (contrast 7.4:1))
(dry-run – nothing changed)
```

If your `~/.claude` contains hooks or plugins, the plan also shows an "adjust paths" step: references to `~/.claude/` are changed to the profile directory, so they stay correct even when the symlink later points to another profile.

If the plan looks right, run it interactively:

```sh
claude-profile add personal
```

`add` asks, one after the other:

1. whether to adopt the existing `~/.claude` or create a new empty profile,
2. which environment variables to remove when this profile starts (the "Subscription" template or none – see [profiles.json](reference/profiles-json.md)),
3. which color the [profile badge](how-it-works.md#profile-badge) gets – suggestions with a preview, or your own values,
4. whether to run the plan.

You can choose the name freely; `a-z`, `A-Z`, `0-9` and `-` are allowed. It is displayed the way you write it, for example `ACME`. Internally – in the directory name, in `profiles.json` and in `CLAUDE_PROFILE` – it is lowercase, and case doesn't matter when you type it. Nothing is deleted: your directory is renamed, and a symlink takes its place.

## 2. Check

```sh
claude-profile list
```

```text
NAME      DIRECTORY           STATUS
personal  ~/.claude-personal  ok       ← symlink
```

## 3. Start with the profile

```sh
claude-profile start personal
```

```text
claude-profile: personal (~/.claude-personal)
```

Claude Code now asks for `/login` once. That is expected: the login is tied to the config directory, and from Claude Code's point of view the profile is a new one. More on this under [Troubleshooting](troubleshooting.md#claude-asks-for-login-again).

## 4. Replace `claude` with the profile variant

```sh
claude-profile replace-claude-command
```

The command shows the change to your `~/.zshrc` or `~/.bashrc`, creates a backup and, after you confirm, adds this block:

```sh
# BEGIN claude-profile
claude() { command claude-profile run -- "$@"; }
# END claude-profile
```

Open a new shell. From now on, `claude` behaves like this:

- If `CLAUDE_PROFILE` is set, that profile starts.
- Otherwise a picker with your profiles appears.
- Without a terminal (in scripts) and without `CLAUDE_PROFILE`, `claude` exits with code 2 instead of guessing.

## What next?

- Create a second profile: [Tutorial](tutorial.md)
- Preselect the profile per project directory: [Set a profile per directory](guides/per-directory-profile.md)
