# Switch the default profile

`~/.claude` and `~/.claude.json` are symlinks to a profile. This is the profile that every program gets that is **not** started through `claude-profile`: IDE plugins, the company's setup tool, a `command claude` past the shell function.

For `claude` through the shell function the symlink plays no role – there the resolved profile always decides.

## Switch

```sh
claude-profile default vertex
```

The command shows what changes and asks for confirmation. With `--yes` it doesn't ask.

## Repair

`claude-profile list` warns when one of the two symlinks is missing, broken or has been replaced by a real file. The same command repairs that:

```sh
claude-profile default <name>
```

If a real file sits in the place of `~/.claude.json`, it is not deleted:

- If the profile has no `.claude.json` yet, the file is moved there.
- If it already has one, the file is renamed to `~/.claude.json.claude-profile-backup-<time>`.

## Limits

`default` does not touch a **real directory** `~/.claude` and aborts. Adopt it as a profile instead:

```sh
claude-profile add <name> --adopt
```

## Which profile should be the default?

The one that hard-wired tools need to write into. If a company setup tool writes to `~/.claude/settings.json`, the symlink should permanently point to the company profile.
