# Undo everything

`claude-profile` never deletes data and deliberately offers no command for deleting profiles. You undo the setup by hand. Quit all Claude Code sessions first.

## 1. Remove the shell function

```sh
claude-profile replace-claude-command --remove
```

Open a new shell. `claude` is the original again.

## 2. Turn a profile back into `~/.claude`

In this example the profile `personal` becomes the normal config again. First check that both paths really are symlinks:

```sh
ls -ld ~/.claude ~/.claude.json
```

Both lines must start with `l` and show an arrow `->`. Only then:

```sh
rm ~/.claude ~/.claude.json
mv ~/.claude-personal ~/.claude
mv ~/.claude/.claude.json ~/.claude.json
```

`rm` only removes the symlinks here, not the data behind them. If there is no symlink, stop – you would be deleting real data.

Claude Code then asks for `/login` once more, because the login is tied to the config directory.

## 3. Change paths back

When the profile was adopted (`add --adopt`) or with `fix-paths`, paths in hook commands and plugin entries were changed from `~/.claude/` to `~/.claude-personal/`. That directory no longer exists – without this step hooks fail and plugins are not found.

Check whether such paths exist:

```sh
cd ~/.claude
grep -n '\.claude-personal/' settings.json settings.local.json plugins/installed_plugins.json plugins/known_marketplaces.json
```

If there are matches, change them back:

```sh
perl -pi -e 's{/\.claude-personal/}{/.claude/}g' settings.json settings.local.json plugins/installed_plugins.json plugins/known_marketplaces.json
```

You can ignore messages about files you don't have. The state from before the change is also kept as a backup next to every changed file (`<file>.claude-profile-backup-<time>`); it does not contain changes you have made since, though.

## 4. Other profiles

Other profile directories (`~/.claude-<name>`) stay where they are. Delete them only if you no longer need the data in them.

## 5. Remove the registry and the binary

```sh
rm -r ~/.config/claude-profiles
brew uninstall claude-profile    # installed with Homebrew
cargo uninstall claude-profile   # installed from source
```

Run the line that matches how you installed it. With Homebrew, `brew untap hollesse/claude-profile` also removes the tap.

## 6. Leftovers

- Backups of the rc file: `~/.zshrc.claude-profile-backup-*` or `~/.bashrc.claude-profile-backup-*`
- Backups of a displaced state file: `~/.claude.json.claude-profile-backup-*`
- Backups of changed paths in the config directories: `settings.json.claude-profile-backup-*`, `settings.local.json.claude-profile-backup-*` and `plugins/*.json.claude-profile-backup-*`
- Claude Desktop data folders per profile: `~/Library/Application Support/Claude-<name>` (contains the login and app data of that instance)
- Starter apps: `/Applications/Claude (…).app`, `/Applications/Claude Profiles.app` (or the folder chosen with `--dir`)
- Blocks in `.envrc` files: `claude-profile set --unset` per directory (before uninstalling) or by hand
