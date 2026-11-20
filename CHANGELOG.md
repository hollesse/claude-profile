# Changelog

All notable changes to `claude-profile` are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Homebrew installs a prebuilt binary instead of building from source: the installation takes seconds and no longer pulls in the Rust toolchain. Run `brew upgrade claude-profile` to switch; `brew autoremove` afterwards removes Rust if nothing else needs it.

### Added

- Prebuilt binaries for macOS (Apple Silicon, Intel) and Linux (x86_64, ARM) are attached to every GitHub release.

### Fixed

- The installation instructions for Homebrew include `brew trust hollesse/claude-profile`, which recent Homebrew versions require before they load a formula from this tap.

## [0.1.0] - 2026-11-20

First release.

### Added

- Profiles with their own config directory `~/.claude-<name>`: `add` (adopts an existing `~/.claude` without deleting anything), `list`, `rename`.
- `claude` as a shell function that never starts without a profile (`replace-claude-command`, `run`, `start`); the profile comes from `CLAUDE_PROFILE`, from a picker, or from the session when resuming with `--resume <id>`.
- A profile per directory through direnv (`set`).
- Symlinks `~/.claude` and `~/.claude.json` to a default profile for tools that use the default path (`default`), and `fix-paths` for hard-wired paths inside a profile.
- `forbid_env`: environment variables that are removed when a profile starts.
- Profile badge: a colored label above the prompt, and a tinted window in Claude Desktop (`badge`).
- Claude knows its profile: every session starts with a note naming its profile and config directory, and a bundled `claude-profile` skill explains how to find and change the configuration of a profile instead of using `~/.claude`.
- Claude Desktop per profile on macOS: `desktop`, starter apps with their own Dock icon (`desktop-app`) and a launcher app with a profile picker (`launcher`).
- Installation through Homebrew.
