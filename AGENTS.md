# Instructions for coding agents

## Changelog first

**Every user-visible change gets an entry in [`CHANGELOG.md`](CHANGELOG.md) under `## [Unreleased]` – in the same change, before you report the work as done.** Do not wait to be asked, and do not leave it for release time: the release workflow publishes exactly that section as the release notes and refuses to run when it is empty.

- User-visible means: new or changed commands, options, output, prompts, defaults, file formats, environment variables, exit codes, the badge mod, the app bundles, installation.
- Use the Keep a Changelog groups: `### Added`, `### Changed`, `### Fixed`, `### Removed`. One line per change, written for someone who uses the tool, naming the command or option.
- No entry for pure refactorings, tests, CI, or documentation wording. If in doubt, add one.
- Never edit the sections of released versions, and never set versions by hand – see Releasing.

## Keep in sync

- **Docs.** `docs/src` describes the behaviour and quotes program output. When you change a command, option or message, update the pages that mention it (start with `docs/src/reference/commands.md`).
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
