# Claude Desktop per profile

> **Status:** These commands have been tried with the real desktop app. macOS only. For the limitations in Vertex mode see [Profile badge](../how-it-works.md#profile-badge).

## Start an instance for a profile

```sh
claude-profile desktop subscription
```

This starts Claude Desktop as a separate instance with its own data folder. Without a name, the profile is resolved the same way as for `claude`: first `CLAUDE_PROFILE`, otherwise the picker.

How the instance starts depends on whether a [starter app](#starter-app-per-profile) is installed for the profile:

- **Installed** (in `/Applications` or `~/Applications`): The command opens the starter app. The instance then carries the profile icon in the Dock.
- **Not installed, in a terminal:** The command asks whether to create the starter app in `/Applications` and open Claude Desktop with it – or to start Claude Desktop without it, with the normal Claude icon.
- **Not installed, without a terminal** (for example in a script): The command starts Claude Desktop without a starter app and does not ask.

With `--no-app` you skip the starter app and the question:

```sh
claude-profile desktop subscription --no-app
```

To only look at the call without a starter app, without starting anything:

```sh
claude-profile desktop subscription --no-app --dry-run
```

```text
/usr/bin/open -n -a Claude --env CLAUDE_CONFIG_DIR=/Users/you/.claude-subscription --env CLAUDE_PROFILE=subscription --env CLAUDE_CODE_PLUGIN_DIRS=/Users/you/.config/claude-profiles/mods/profile-badge --args '--user-data-dir=/Users/you/Library/Application Support/Claude-subscription'
```

- `-n` forces a new instance, so several profiles can run at the same time.
- `--user-data-dir` separates login, cache and app data per profile.
- `CLAUDE_CONFIG_DIR` is meant to keep the Code tab from using the default config.
- `CLAUDE_PROFILE` and `CLAUDE_CODE_PLUGIN_DIRS` load the [profile badge](../how-it-works.md#profile-badge), which colors the window in Claude Desktop. With `CLAUDE_PROFILE_NO_BADGE`, `CLAUDE_CODE_PLUGIN_DIRS` is omitted.

Without `--no-app`, `--dry-run` shows `/usr/bin/open '/Applications/Claude (Subscription).app'` instead when a starter app is installed. `--dry-run` never asks and creates nothing.

You can override the data folder per profile in [profiles.json](../reference/profiles-json.md) with `desktop_user_data_dir`.

## Starter app per profile

```sh
claude-profile desktop-app vertex
```

creates `/Applications/Claude (Vertex).app`. The app can be found through Spotlight and calls `claude-profile desktop vertex --exec`.

The app's icon carries the colors of the [profile badge](../how-it-works.md#profile-badge) and the profile name. If you change the colors or the display name later, create the app again.

The instance runs as a process of the starter app and therefore carries its icon in the Dock. That lets you tell two running profiles apart by their icon. This only holds for a start through the starter app – directly, through the [launcher with profile picker](#launcher-with-profile-picker) or through `claude-profile desktop vertex`, which uses an installed starter app. Without a starter app or with `--no-app`, the instance shows the normal Claude icon.

For macOS the instance is still "Claude": the menu bar shows that name, and "Keep in Dock" on a running instance may pin the normal `Claude.app`. Drag the starter app from the Applications folder into the Dock instead.

## Launcher with profile picker

```sh
claude-profile launcher
```

creates `/Applications/Claude Profiles.app`. The app opens a window with all profiles, each with its icon:

- A click on a profile with an installed starter app opens that app – and with it Claude Desktop with the profile icon in the Dock.
- For a profile without a starter app, "Install & open" creates it in the launcher's folder and then opens it. "Install only" just creates it.
- While the window is open, the launcher sits in the Dock with its icon. After opening a profile it quits.

A starter app counts as installed when it is in the launcher's folder, in `/Applications` or in `~/Applications`. New profiles appear by themselves the next time you open the launcher.

## A different target folder

Both commands write to `/Applications` by default, the normal Applications folder. That works without `sudo` if your user is an administrator. With `--dir` you choose another location, for example the Applications folder of your user only:

```sh
claude-profile launcher --dir ~/Applications
claude-profile desktop-app vertex --dir ~/Applications
```

An app created earlier in another place stays there – delete it by hand, otherwise you have it twice.

## Notes

- The starter apps and the launcher contain the absolute path to the `claude-profile` binary. If it moves later – for example when you switch from a source build to Homebrew – create the apps again. Updates through Homebrew keep the path.
- The launcher's window is compiled with `swiftc` when `claude-profile` is built and needs macOS 13 or later. If `swiftc` is missing at build time, the launcher gets a simple selection dialog without icons instead.
- If Finder still shows an old icon after re-creating an app, `killall Finder Dock` helps.
- Existing apps of the same name are only overwritten if they were created by `claude-profile`.
- Don't start Claude Desktop directly from the Applications folder anymore if the separation matters: that instance uses the default config, that is, the profile `~/.claude` points to.
