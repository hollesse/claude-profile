# Installation

## Requirements

- macOS or Linux, bash or zsh
- [Claude Code](https://claude.com/claude-code) is installed and `claude` is on the `PATH`
- optional: [direnv](https://direnv.net), if you want to set a profile per directory

## With Homebrew

```sh
brew tap hollesse/claude-profile https://github.com/hollesse/claude-profile
brew install claude-profile
```

Homebrew builds `claude-profile` from the source of the latest release; the Rust toolchain it needs for that is installed along the way. The first line tells Homebrew where the formula lives and is only needed once.

### Update

```sh
brew update
brew upgrade claude-profile
```

Profiles and settings stay unchanged. What changed between versions is in the [changelog](changelog.md).

## From source

You need a Rust toolchain with `cargo` (for example `brew install rust` or [rustup](https://rustup.rs)).

```sh
git clone https://github.com/hollesse/claude-profile.git
cd claude-profile
cargo install --path . --locked
```

The dot after `--path` is the current directory and is part of the command. The binary ends up in `~/.cargo/bin/claude-profile`. If the command is not found afterwards, `~/.cargo/bin` is missing from the `PATH`. Add it to your `~/.zshrc` or `~/.bashrc`:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

To update, run `git pull` in the cloned folder and install again.

On macOS the build compiles the window of the [launcher app](guides/desktop.md#launcher-with-profile-picker) with `swiftc`, which comes with the Xcode Command Line Tools. Without it, the launcher falls back to a simple selection dialog.

## Check

```sh
claude-profile --version
```

## Uninstall

See [Undo everything](guides/undo.md).
