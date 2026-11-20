# Tutorial: two profiles

This tutorial sets up two profiles:

| Profile | Backend | Special |
|---|---|---|
| `subscription` | Claude subscription (`/login`) | Vertex and telemetry variables are removed at start |
| `vertex` | Vertex AI through a company setup tool | `~/.claude` permanently points here |

Starting point: `claude-profile` is [installed](installation.md), your current `~/.claude` holds your subscription usage, and no Claude Code session is running.

## 1. Adopt the existing data as `subscription`

```sh
claude-profile add subscription --adopt --forbid-env-template subscription
```

The plan is shown and, after you confirm, executed. Afterwards:

- `~/.claude-subscription` contains your existing data.
- `~/.claude` and `~/.claude.json` are symlinks to it.
- When this profile starts, `CLAUDE_CODE_USE_VERTEX`, `ANTHROPIC_VERTEX_*`, `ANTHROPIC_BASE_URL`, `GOOGLE_CLOUD_*` and `OTEL_*` are removed from the environment.

## 2. Create an empty `vertex` profile and repoint the symlink

```sh
claude-profile add vertex --link-target vertex
```

`--link-target vertex` makes `~/.claude` and `~/.claude.json` point to the Vertex profile from now on. That is intentional: the company's setup tool writes to the fixed path `~/.claude/settings.json` and thereby ends up in the right profile.

```sh
claude-profile list
```

```text
NAME          DIRECTORY               STATUS
subscription  ~/.claude-subscription  ok
vertex        ~/.claude-vertex        ok       ← symlink
```

## 3. Set up Vertex

Now run the company's setup tool. Then check that it landed in the Vertex profile and left the other profile untouched:

```sh
ls -l ~/.claude-vertex/settings.json
ls -l ~/.claude-subscription/settings.json
```

Leave the symlink pointing to `vertex` permanently. The setup tool may write again later (for example on re-authentication), so repointing the symlink only briefly for the setup would not be safe.

## 4. Add the shell function

```sh
claude-profile replace-claude-command
```

Open a new shell. `claude` now asks for the profile:

```text
? Claude profile ›
❯  subscription   (~/.claude-subscription)
   vertex         (~/.claude-vertex)
```

Each name is shown in the colors of its [profile badge](how-it-works.md#profile-badge).

## 5. Try both profiles

```sh
CLAUDE_PROFILE=subscription claude
```

Run `/login` once (see [Getting started](getting-started.md#3-start-with-the-profile)). If your shell has Vertex variables set, you see at start that they were removed – only the names are printed, never the values:

```text
claude-profile: warning: removed variables forbidden for profile `subscription`: CLAUDE_CODE_USE_VERTEX
claude-profile: subscription (~/.claude-subscription)
```

Then the other profile:

```sh
CLAUDE_PROFILE=vertex claude
```

## 6. Set a profile per project directory

So you don't have to choose every time in company projects (requires [direnv](https://direnv.net)):

```sh
cd ~/projects/company/some-repo
claude-profile set vertex
```

In this directory and all its subdirectories, `claude` now starts with `vertex` without asking. Everywhere else the picker remains. Details: [Set a profile per directory](guides/per-directory-profile.md).

## Result

```sh
claude-profile list
```

shows in the company directory:

```text
NAME          DIRECTORY               STATUS
subscription  ~/.claude-subscription  ok
vertex        ~/.claude-vertex        ok       ← symlink ← active ← direnv
```

- `← symlink`: `~/.claude` and `~/.claude.json` point here.
- `← active`: the value of `CLAUDE_PROFILE` in this shell.
- `← direnv`: the value from the nearest `.envrc`, searching upwards from the current directory.
