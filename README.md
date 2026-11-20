# claude-profile

Separate Claude Code profiles on one machine – for example one for the Claude subscription and one for Vertex AI. Each profile has its own config directory, and `claude` never starts without an explicitly chosen profile.

## Quick start

```sh
cargo install --path . --locked
claude-profile add personal --adopt --dry-run   # only shows the plan
claude-profile add personal                     # adopt the existing ~/.claude
claude-profile replace-claude-command           # `claude` now runs through a profile
```

## Documentation

The guide lives in [`docs/src`](docs/src/SUMMARY.md) and can be built as an HTML site with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
brew install mdbook
mdbook serve docs --open
```

- [Installation](docs/src/installation.md)
- [Getting started](docs/src/getting-started.md)
- [Tutorial: two profiles](docs/src/tutorial.md)
- [Commands](docs/src/reference/commands.md)
- [Troubleshooting](docs/src/troubleshooting.md)

Requirements and open questions are in the [PRD](prd.md).

## Development

```sh
cargo test
cargo clippy --all-targets
```

The tests run against a temporary `HOME` and never touch the real `~/.claude`.
