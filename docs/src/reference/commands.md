# Commands

```text
claude-profile <COMMAND> [OPTIONS]
```

`claude-profile --help` and `claude-profile <COMMAND> --help` show the short help.

Common rules:

- Commands that change existing files first show the change and ask for confirmation. `--yes` (`-y`) answers the question with yes.
- Without a terminal and without `--yes`, these commands exit with code 2 without changing anything.
- No command deletes user data.

## Profile names

A profile has an **ID** and a **display name**:

- The ID is the name in lowercase, for example `acme`. It is used in `profiles.json`, in `CLAUDE_PROFILE`, in `.envrc` and in the directory name `~/.claude-acme`.
- The display name is the name the way you wrote it when creating the profile, for example `ACME`. It appears everywhere the profile is shown: in the [profile badge](../how-it-works.md#profile-badge), in the picker, in the status line, in `list`, and in the name and icon of the starter app.

Case doesn't matter when you type a name: `claude-profile start acme`, `CLAUDE_PROFILE=Acme` and `claude-profile badge ACME` all mean the same profile. So there cannot be two profiles that differ only in case. You change the display name of an existing profile with [rename](#rename).

## add

```text
claude-profile add [NAME] [--adopt | --new] [--link-target PROFILE]
                   [--forbid-env-template subscription|none]
                   [--badge-bg COLOR [--badge-fg COLOR]] [--dry-run] [--yes]
```

Creates a profile with the directory `~/.claude-<id>`. Without `NAME` you are asked for it. `a-z`, `A-Z`, `0-9` and `-` are allowed. For the display name see [Profile names](#profile-names).

| Option | Meaning |
|---|---|
| `--adopt` | Adopt the existing real directory `~/.claude` as this profile (rename it). |
| `--new` | Create a new empty profile, don't touch `~/.claude`. |
| `--link-target PROFILE` | Profile that `~/.claude` and `~/.claude.json` should point to. |
| `--forbid-env-template` | Template for `forbid_env`: `subscription` or `none`. |
| `--badge-bg COLOR` | Background color of the profile badge, `#rrggbb` or `#rgb`. |
| `--badge-fg COLOR` | Text color of the badge. If omitted, black or white, depending on contrast. |
| `--dry-run` | Only show the plan. |
| `--yes`, `-y` | Run the plan without asking. |

Behavior:

- Without `--badge-bg`, `add` suggests badge colors in a terminal (see [badge](#badge)); without a terminal or with `--yes` it takes the first free suggestion.
- If `~/.claude` is a real directory, you have to decide between adopting it and a new profile – interactively or with `--adopt`/`--new`.
- When adopting, paths with which the directory refers to itself are changed to the profile directory (see [fix-paths](#fix-paths)). The plan names the files and the number of places.
- When adopting, the symlink afterwards points to the adopted profile, unless `--link-target` says otherwise.
- If `~/.claude` is missing, the symlink is created. If there are several profiles, `--link-target` or the interactive choice is required.
- With `--new` next to a real directory `~/.claude`, no symlink is created.
- An existing target directory is never overwritten by `--adopt`.
- On the first `add`, the path of the real `claude` binary is determined and stored.
- Running it again for a finished profile changes nothing.

## badge

```text
claude-profile badge NAME [--bg COLOR [--fg COLOR]]
```

Sets the colors of the [profile badge](../how-it-works.md#profile-badge). The change applies to newly started sessions.

Without options, a picker with a preview appears:

```text
? Badge color ›
❯  vertex   Blue: text #000000 on #4c9aff (contrast 7.4:1)
   vertex   Orange: text #000000 on #f5a623 (contrast 10.4:1)
   …
  Enter custom values …
```

- Only colors that no other profile uses are suggested. If all are taken, all of them appear.
- Each suggestion combines the background color with black or white – whichever is more readable.
- For custom values, the higher-contrast of the two is suggested as the text color.

With `--bg` (and optionally `--fg`) you set the values directly:

```sh
claude-profile badge vertex --bg '#e5484d' --fg '#ffffff'
```

The command warns when the contrast is below 4.5:1 (the WCAG AA guideline) or another profile has the same background color, but stores the values anyway. Put color values in quotes in the shell, because `#` otherwise starts a comment.

## rename

```text
claude-profile rename NAME NEW
```

Changes the display name of a profile, for example from `acme` to `ACME`. `NEW` must be the same name in different case; the command rejects any other name.

```sh
claude-profile rename acme ACME
```

- The ID `acme` stays – and with it the directory, `.envrc` files, `CLAUDE_PROFILE` and the Claude Desktop data folder.
- The new display name applies to newly started sessions. A starter app gets it when you re-create it with `desktop-app NAME`; the existing one is renamed in the process.

## fix-paths

```text
claude-profile fix-paths NAME [--dry-run] [--yes]
```

Changes hard-wired paths to `~/.claude/` in a profile to the profile's own directory. Such paths typically appear in hook commands (`~/.claude/hooks/…`) and in the plugins' installation paths. They only work as long as `~/.claude` points to exactly this profile.

The files checked are `settings.json`, `settings.local.json`, `plugins/installed_plugins.json` and `plugins/known_marketplaces.json`. The spellings `~/.claude/`, `$HOME/.claude/`, `${HOME}/.claude/` and the absolute path are recognized. Before every change a backup `<file>.claude-profile-backup-<time>` is created next to the file.

`add --adopt` does this itself when adopting. You need the command for profiles that were adopted with an older version, or when new paths of this kind have appeared later – `list` points this out.

## list

```text
claude-profile list [--names]
```

Shows all profiles with directory and status (`ok` / `missing`) and the marks `← symlink`, `← active` (`CLAUDE_PROFILE`) and `← direnv` (nearest `.envrc` upwards). Warnings appear on stderr: missing directories, paths to `~/.claude/` in a profile, a broken or replaced symlink, a mismatched `~/.claude.json`, unknown profile names.

`--names` prints only the IDs, one per line.

## start

```text
claude-profile start NAME [-- CLAUDE-ARGS…]
```

Starts Claude Code with the named profile and ignores `CLAUDE_PROFILE`. Everything after `--` is passed to `claude` unchanged:

```sh
claude-profile start vertex -- --continue
```

## run

```text
claude-profile run [-- CLAUDE-ARGS…]
```

Resolves the profile and starts Claude Code. This is the command the `claude` shell function uses. Order of resolution:

0. With `--resume <id>` (also `-r <id>` and `--resume=<id>`), the profile the session ran under. For this, `run` looks for `<id>.jsonl` under `projects/` in the directories of all profiles. This takes precedence over `CLAUDE_PROFILE`; if the two differ, a note is printed on stderr. If the session exists in several profiles, `CLAUDE_PROFILE` decides, otherwise the profile with the most recently modified session. If the session is found nowhere, resolution continues with the following steps.
1. `CLAUDE_PROFILE`, if set. An unknown value is an error, not a fallback to another profile.
2. Interactive picker, if a terminal is available.
3. Otherwise exit with code 2.

`--resume` without an ID, with a session name or a search term, and `--continue` only know the sessions of the chosen profile. For these, the normal resolution from step 1 applies.

At start (applies to `run` and `start`):

- `CLAUDE_CONFIG_DIR` is set to the profile directory, `CLAUDE_PROFILE` to the profile ID.
- Variables that match the profile's `forbid_env` are removed and named in a warning (names only).
- A status line `claude-profile: <name> (<directory>)` appears on stderr.
- The mod with the [profile badge](../how-it-works.md#profile-badge) and the [note and skill for Claude](../how-it-works.md#what-claude-knows-about-its-profile) is loaded via `CLAUDE_CODE_PLUGIN_DIRS`.
- The process is replaced by the real `claude`.

## default

```text
claude-profile default NAME [--yes]
```

Makes `~/.claude` and `~/.claude.json` point to the profile and repairs the two symlinks. Aborts if `~/.claude` is a real directory. See [Switch the default profile](../guides/default-profile.md).

## set

```text
claude-profile set NAME [--yes]
claude-profile set --unset
```

Writes or removes the block with `export CLAUDE_PROFILE=NAME` in `.envrc` in the current directory. `--yes` runs `direnv allow` without asking. See [Set a profile per directory](../guides/per-directory-profile.md).

## replace-claude-command

```text
claude-profile replace-claude-command [--shell zsh|bash] [--remove] [--yes]
```

Adds the `claude` shell function as a marked block to the rc file, or removes it with `--remove`.

- Without `--shell`, the shell is detected from `$SHELL`.
- zsh: `~/.zshrc` (or `$ZDOTDIR/.zshrc`), bash: `~/.bashrc`.
- Before every change a backup `<rc-file>.claude-profile-backup-<time>` is created.
- Running it several times results in exactly one block.

## desktop

```text
claude-profile desktop [NAME | --dialog] [--exec | --no-app] [--dry-run]
```

Starts Claude Desktop as a separate instance for a profile (macOS). Without `NAME`, the profile is resolved as for `run`. `--dry-run` only prints the call.

If a starter app for the profile is installed in `/Applications` or `~/Applications` (see [desktop-app](#desktop-app)), the command opens it; the instance then carries the profile icon in the Dock. If it is missing, the command asks in a terminal whether to create it in `/Applications` or to start Claude Desktop without it. Without a terminal and with `--dialog` the question is skipped; if the starter app is missing, the command then starts without it. `--no-app` never uses a starter app and does not ask.

With `--dialog`, a macOS dialog asks for the profile; that is the path of a launcher built without `swiftc`. With `--exec`, the command does not start Claude Desktop through `open` but becomes Claude Desktop itself; that is how the starter apps call it, so the instance carries their icon in the Dock.

## desktop-app

```text
claude-profile desktop-app NAME [--dir DIR]
```

Creates `Claude (<Name>).app`, which calls `desktop <id>`. The parentheses contain the profile's display name; without one of its own, the name with a capital first letter. The target folder is `/Applications`; with `--dir` you choose another one, such as `~/Applications`.

## launcher

```text
claude-profile launcher [--dir DIR]
```

Creates `Claude Profiles.app`: a window with all profiles and their icons that installs missing starter apps and opens the chosen profile. Target folder as for `desktop-app`; that is also where the launcher creates the starter apps. The app has an icon of its own: a fan of profile cards. See [Launcher with profile picker](../guides/desktop.md#launcher-with-profile-picker).

For the three desktop commands see [Claude Desktop per profile](../guides/desktop.md).
