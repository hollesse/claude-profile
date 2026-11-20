<p align="center">
  <img src="assets/launcher-icon.svg" width="160" height="160" alt="claude-profile logo: a fan of three profile cards">
</p>

<h1 align="center">claude-profile</h1>

<p align="center">
  Separate Claude Code profiles on one machine –<br>
  each with its own settings, history and login.
</p>

<p align="center">
  <a href="https://hollesse.github.io/claude-profile/"><b>Documentation</b></a> ·
  <a href="https://hollesse.github.io/claude-profile/getting-started.html">Getting started</a> ·
  <a href="https://hollesse.github.io/claude-profile/tutorial.html">Tutorial</a> ·
  <a href="https://hollesse.github.io/claude-profile/reference/commands.html">Commands</a>
</p>

---

Claude Code has no notion of profiles. `claude-profile` adds them – for example one profile for your personal Claude subscription and one for Vertex AI through your company – without a virtual machine.

- **One directory per profile.** `~/.claude-<name>`, passed to Claude Code as `CLAUDE_CONFIG_DIR`.
- **No start without a profile.** `claude` becomes a shell function that always resolves a profile first: from `CLAUDE_PROFILE`, per directory via direnv, or by asking you.
- **A symlink for everything else.** `~/.claude` points to a profile of your choice, so tools with the default path hard-wired land in exactly that profile.
- **No leaking environment.** Each profile lists variables that are removed on start, such as `OTEL_*` or `ANTHROPIC_VERTEX_*`.
- **You see where you are.** A colored badge above the prompt names the profile of every session.
- **Claude Desktop too (macOS).** One instance per profile, each with its own Dock icon, plus a launcher app to pick one.

## Install

```sh
brew tap hollesse/claude-profile https://github.com/hollesse/claude-profile
brew install claude-profile
```

Or build it from source with `cargo install --path . --locked` – see [Installation](https://hollesse.github.io/claude-profile/installation.html).

## Quick start

```sh
claude-profile add personal --adopt --dry-run   # only shows the plan
claude-profile add personal                     # adopt the existing ~/.claude
claude-profile replace-claude-command           # `claude` now runs through a profile
```

```text
$ claude-profile list
NAME          DIRECTORY               STATUS
subscription  ~/.claude-subscription  ok
vertex        ~/.claude-vertex        ok       ← symlink ← active ← direnv
```

Nothing is ever deleted: your existing `~/.claude` is renamed, and a symlink takes its place.

## Documentation

The full guide is at **[hollesse.github.io/claude-profile](https://hollesse.github.io/claude-profile/)**:

| | |
|---|---|
| [Installation](https://hollesse.github.io/claude-profile/installation.html) | Prerequisites, building, updating |
| [Getting started](https://hollesse.github.io/claude-profile/getting-started.html) | Your first profile in five minutes |
| [Tutorial: two profiles](https://hollesse.github.io/claude-profile/tutorial.html) | Subscription and Vertex side by side |
| [Claude Desktop per profile](https://hollesse.github.io/claude-profile/guides/desktop.html) | Starter apps and the launcher |
| [Commands](https://hollesse.github.io/claude-profile/reference/commands.html) | Every command and option |
| [How it works](https://hollesse.github.io/claude-profile/how-it-works.html) | What is isolated and what is not |
| [Troubleshooting](https://hollesse.github.io/claude-profile/troubleshooting.html) | Common problems and their fixes |
| [Changelog](https://hollesse.github.io/claude-profile/changelog.html) | What changed in each release |

Requirements and open questions are in the [PRD](prd.md) (German).

## Development

```sh
cargo test
cargo clippy --all-targets
```

The tests run against a temporary `HOME` and never touch the real `~/.claude`.

The documentation sources live in [`docs/src`](docs/src/SUMMARY.md). Preview them locally with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
brew install mdbook
mdbook serve docs --open
```

## Releasing

1. While you work, describe every user-visible change under `## [Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).
2. When `main` is ready, start the release with the new version number:

   ```sh
   gh workflow run release.yml -f version=1.2.3
   ```

   The same button is in the Actions tab under “Release” → “Run workflow”.
3. Run `git pull` afterwards – the workflow adds two commits to `main`.

The [release workflow](.github/workflows/release.yml) then does the rest:

- sets the version in `Cargo.toml`, `Cargo.lock` and the docs, and turns the Unreleased section into the section of the new version ([`scripts/release.sh`](scripts/release.sh)),
- runs the tests, commits, tags `v1.2.3` and pushes,
- points the [Homebrew formula](Formula/claude-profile.rb) at the new tag,
- creates the GitHub release with the changelog section as notes,
- publishes the docs of that version to GitHub Pages.

The published docs change only with a release. To republish them without one – for a typo fix, say – run `gh workflow run docs.yml -f ref=main`.
