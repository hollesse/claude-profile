# Troubleshooting

The first place to look is always:

```sh
claude-profile list
```

The command checks directories and symlinks and names the matching repair command for most problems.

## Claude asks for `/login` again

**Expected, once per profile.** On macOS the login lives in the keychain, and the entry is tied to the config directory. As soon as `claude-profile` passes the directory explicitly as `CLAUDE_CONFIG_DIR`, Claude Code looks under a different entry than before. Your old login is not deleted, it just isn't found anymore.

Fix: run `/login` once in a session of the profile.

## Claude shows the onboarding as if freshly installed

Claude Code was started without a profile and cannot find its state, because `~/.claude.json` is missing or is a real, empty file.

```sh
claude-profile list              # warns: ~/.claude.json is not a symlink …
claude-profile default <name>    # repairs it
```

A file displaced in the process is kept as `~/.claude.json.claude-profile-backup-<time>`. To keep `claude` from starting without a profile in the first place, add the shell function: `claude-profile replace-claude-command`.

## `claude` does not ask for the profile

The shell function is not active. Check:

```sh
type claude
```

The output should read `claude is a shell function` (zsh: `claude is a shell function from …`). If it shows a path:

- Run `claude-profile replace-claude-command` and open a **new** shell.
- An alias named `claude` in your rc file takes precedence over the function – remove it.

## A hook fails: "~/.claude/hooks/…: No such file or directory"

Or: a profile's plugins are suddenly missing. The profile contains paths that point to the fixed location `~/.claude/`, and the symlink now points to another profile.

```sh
claude-profile list               # warns: profile … contains N paths to ~/.claude/
claude-profile fix-paths <name>   # changes them to the profile directory
```

Restart running sessions of the profile afterwards.

## "no profile selected … no terminal" (exit code 2)

`claude` was started without a terminal, for example in a script, and `CLAUDE_PROFILE` is not set. That is intentional. Name the profile explicitly:

```sh
CLAUDE_PROFILE=vertex claude -p "…"
claude-profile start vertex -- -p "…"
```

## "unknown profile"

The name in `CLAUDE_PROFILE`, in an `.envrc` or on the command line is not registered. `claude-profile list` shows the known names and warns when an `.envrc` refers to an unknown profile.

## "claude binary not found"

The path `claude_bin` stored in `profiles.json` no longer exists and `claude` is not on the `PATH`. Correct `claude_bin` in `~/.config/claude-profiles/profiles.json` or remove the field so the `PATH` is searched.

## `list` warns: "~/.claude is a real directory"

A program has replaced the symlink with a directory, or you created a profile with `--new` next to an existing `~/.claude`. `default` does not touch a real directory. Adopt it as a profile of its own:

```sh
claude-profile add <new-name> --adopt
```

## `list` warns: "symlink ~/.claude is broken"

The target directory was moved or deleted. Restore it, or point to an existing profile with `claude-profile default <name>`.

## The `.envrc` has no effect

- Is direnv installed and its hook present in your rc file? Otherwise `claude-profile set` warns.
- After every change to `.envrc` you need to run `direnv allow`.
- `echo $CLAUDE_PROFILE` in the directory shows whether the variable arrives.

## Settings from the company setup tool end up in the wrong profile

The tool writes to `~/.claude/settings.json`, that is, into the profile the symlink points to. Check with `claude-profile list` which profile carries `← symlink`, and switch permanently if needed: `claude-profile default <company-profile>`.
