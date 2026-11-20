# Installation

## Requirements

- macOS or Linux, bash or zsh
- [Claude Code](https://claude.com/claude-code) is installed and `claude` is on the `PATH`
- Rust toolchain with `cargo` (for example `brew install rust` or [rustup](https://rustup.rs))
- optional: [direnv](https://direnv.net), if you want to set a profile per directory

## Build and install

```sh
git clone git@github.com:hollesse/claude-profile.git
cd claude-profile
cargo install --path . --locked
```

The dot after `--path` is the current directory and is part of the command. The binary ends up in `~/.cargo/bin/claude-profile`.

## Check

```sh
claude-profile --version
```

If the command is not found, `~/.cargo/bin` is missing from the `PATH`. Add it to your `~/.zshrc` or `~/.bashrc`:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

## Update

```sh
cd claude-profile
git pull
cargo install --path . --locked
```

Profiles and settings stay unchanged.

## Uninstall

See [Undo everything](guides/undo.md).
