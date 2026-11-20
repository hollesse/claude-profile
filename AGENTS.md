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

Releases are made only by the release workflow (`gh workflow run release.yml -f version=1.2.3`), described in the README. It sets the version in `Cargo.toml`, `Cargo.lock` and `docs/theme/version.js`, rolls the changelog, tags, updates `Formula/claude-profile.rb` and publishes the docs. Do not change those version numbers, the formula's `url`/`sha256`, or tags by hand, and do not start a release unless asked to.
