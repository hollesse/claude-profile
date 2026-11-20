# Set a profile per directory

`claude-profile set` defines which profile `claude` uses in a directory. Technically, [direnv](https://direnv.net) sets the `CLAUDE_PROFILE` variable when you enter the directory.

## Requirement: direnv

```sh
brew install direnv
```

and the hook in your rc file, for zsh:

```sh
eval "$(direnv hook zsh)"
```

`claude-profile set` warns if direnv is missing or no hook can be found, but writes the `.envrc` anyway.

## Set the profile

```sh
cd ~/projects/company/repo
claude-profile set vertex
```

This writes a marked block to `.envrc` and asks whether to run `direnv allow` (needed after every change to `.envrc`):

```sh
# BEGIN claude-profile
export CLAUDE_PROFILE=vertex
# END claude-profile
```

Other lines in an existing `.envrc` stay unchanged. Running `set` again only replaces the block.

## Change or remove the profile

```sh
claude-profile set subscription   # another profile
claude-profile set --unset        # remove the block
```

## Don't commit `.envrc`

The profile choice is your personal setting. Exclude the file locally without changing the project's `.gitignore`:

```sh
echo .envrc >> .git/info/exclude
```

## Deviate for a single session

```sh
claude-profile start subscription
```

`start` ignores `CLAUDE_PROFILE` and therefore the `.envrc` too.
