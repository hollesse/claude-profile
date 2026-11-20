# Instructions for coding agents

## Changelog first

**Every user-visible change gets an entry in [`CHANGELOG.md`](CHANGELOG.md) under `## [Unreleased]` – in the same change, before you report the work as done.** Do not wait to be asked, and do not leave it for release time: the release workflow publishes exactly that section as the release notes and refuses to run when it is empty.

- User-visible means: new or changed commands, options, output, prompts, defaults, file formats, environment variables, exit codes, the badge mod, the app bundles, installation.
- Use the Keep a Changelog groups: `### Added`, `### Changed`, `### Fixed`, `### Removed`. One line per change, written for someone who uses the tool, naming the command or option.
- No entry for pure refactorings, tests, CI, or documentation wording. If in doubt, add one.
- Never edit the sections of released versions, and never set versions by hand – see Releasing.

## Documentation with every change

**Whatever you implement, check the documentation and update it in the same change.** A feature, fix or changed message is not done while the docs still describe the old behaviour or do not mention the new one. Do this unasked, and say in your report which pages you changed – or that you checked and nothing needed changing.

- Search `docs/src` and `README.md` for every command, option, message, file and environment variable you touched; do not rely on memory of where it is mentioned.
- The usual places: `docs/src/reference/commands.md` (synopsis, options, behaviour), `reference/environment.md` (variables, exit codes, files), `reference/profiles-json.md` (registry fields), the guide of the feature under `docs/src/guides/`, `troubleshooting.md` (quoted warnings and errors), `getting-started.md` and `tutorial.md` (quoted output), and the feature list in `README.md`.
- Quoted program output in the docs must match what the program prints, character for character.
- A new feature needs its own description, not just a line in the command reference: what it is for, how to use it, what its limits are.
- A new page goes into `docs/src/SUMMARY.md`; a renamed heading changes its anchor, so fix the links to it.

## Compatibility with installed versions

**Once the first release is out, people run older versions and have files those versions wrote. Every change must keep working for them.** Before you change anything listed below, ask what happens to someone who upgrades with existing state – and what happens to things an older version created that the new binary now meets.

What older versions leave behind, and what therefore counts as an interface:

- `~/.config/claude-profiles/profiles.json` – every field name and its meaning.
- The marked blocks (`# BEGIN claude-profile` … `# END claude-profile`) in `~/.zshrc`, `~/.bashrc` and `.envrc`, and the line inside them: the shell function calls `claude-profile run -- "$@"`.
- Profile directories `~/.claude-<id>`, the symlinks `~/.claude` and `~/.claude.json`, and the backup file names `*.claude-profile-backup-<time>`.
- Starter apps and the launcher in `/Applications`: they are **not** recreated on upgrade. An old starter app keeps calling `claude-profile desktop <id> --exec`, and an old launcher window keeps calling `launcher-data --dir …` and `desktop-app <id> --dir …` and parsing their output. Their bundle IDs (`local.claude-profile.<id>`) are how they are found.
- Claude Desktop data folders `~/Library/Application Support/Claude-<id>`.
- Everything scripts may depend on: command and option names, exit codes, environment variables (`CLAUDE_PROFILE`, `CLAUDE_PROFILE_QUIET`, `CLAUDE_PROFILE_NO_BADGE`), and the output of `list --names`.

Rules:

- **Read old, write new.** New fields in `profiles.json` are optional with a default (`#[serde(default)]`), so a file from an older version still loads. Never repurpose or remove a field without a migration.
- **Migrate automatically and safely.** When a format has to change, the new version detects the old form and converts it on load – like `Registry::lowercase_ids` does for keys with capitals. A migration must be idempotent, must never delete user data, and gets a test that starts from a file as the old version wrote it.
- **Keep old entry points alive.** Do not rename or remove commands, options or hidden commands that rc blocks, starter apps or the launcher call. If one has to go, keep it as an alias for at least one minor version and say so in the changelog.
- **When recreating is unavoidable** (rc block, starter app, launcher), detect the outdated form and tell the user the exact command to run; do not fail silently.
- **Think about downgrades too.** A newer `profiles.json` may be read by an older binary; prefer additions that an older version can ignore.
- If a change cannot be made compatible, stop and ask before implementing it.

## Versioning

Versions follow [Semantic Versioning](https://semver.org/). The version number is chosen when the release is started, from what the Unreleased section of the changelog contains:

- **Patch** (`1.2.3` → `1.2.4`): only fixes; nothing new to learn, nothing to change for the user.
- **Minor** (`1.2.3` → `1.3.0`): new commands, options or behaviour; everything that worked before still works, with existing files and apps.
- **Major** (`1.2.3` → `2.0.0`): a breaking change – something a user, a script or an existing file/app relied on stops working without action on their side.
- **Before 1.0** a breaking change raises the minor version (`0.2.3` → `0.3.0`), everything else the patch version.

Mark a breaking change in the changelog as `**Breaking:**` at the start of its line and add what the user has to do. When you add changelog entries, say in your report which kind of version bump they imply. Suggest the next version number when asked to prepare a release – do not pick it silently.

## Keep in sync

- **Language.** Code, messages, comments and docs are English. `prd.md` is the only German file.
- **Logo.** `docs/src/assets/logo.svg` and `docs/theme/favicon.svg` are copies of `assets/launcher-icon.svg`; a test fails when they differ.
- **Badge palette.** The first six colors of `PALETTE` in `src/color.rs` must match `mod/profile-badge/hooks/register.tsx`.
- **No real names.** Examples use `personal`, `subscription`, `vertex` and `acme` – never the name of an employer or customer.

## Checks

```sh
cargo test
cargo clippy --all-targets
mdbook build docs    # when you touched docs/
```

The tests use a temporary `HOME`. Never run commands that write to the real `~/.claude`, `~/.config/claude-profiles` or `/Applications` to try something out; pass `--dir` and a temporary `HOME` instead.

## Releasing

Releases are made only by the release workflow (`gh workflow run release.yml -f version=1.2.3`), described in the README. It sets the version in `Cargo.toml`, `Cargo.lock` and `docs/theme/version.js`, rolls the changelog, builds the binaries for all platforms, generates `Formula/claude-profile.rb` (`scripts/formula.sh`), tags and publishes the docs. Do not change those version numbers, the formula, or tags by hand, and do not start a release unless asked to.
