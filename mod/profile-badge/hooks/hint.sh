#!/bin/sh
# Runs when a session starts (SessionStart hook in hooks.json). What it prints is added to
# the session's context: which claude-profile profile the session runs with and where its
# configuration lives. Without it, a session asked to change "the Claude settings" goes to
# ~/.claude – which may be another profile.
#
# Prints nothing when no profile can be recognized.

dir="${CLAUDE_CONFIG_DIR:-}"
profile="${CLAUDE_PROFILE:-}"

# Profile ID from a config directory …/.claude-<id>.
id_of() {
    case "$1" in
        */.claude-?*) printf '%s' "${1##*/.claude-}" | tr '[:upper:]' '[:lower:]' ;;
    esac
}

intro='Claude Code is split into several profiles on this machine by the tool `claude-profile`. Each profile has its own config directory with its own settings, history, plugins and login, and the profiles must stay separate.'
outro='Before you read or change Claude Code configuration, or work with another profile, load the `claude-profile` skill. `claude-profile list` shows all profiles and their directories.'

if [ -n "$dir" ]; then
    dir="${dir%/}"
    [ -n "$profile" ] || profile="$(id_of "$dir")"
    [ -n "$profile" ] || exit 0
    profile="$(printf '%s' "$profile" | tr '[:upper:]' '[:lower:]')"
    cat <<EOT
$intro
This session runs with the profile \`$profile\`. Its config directory is \`$dir\` (also in \`\$CLAUDE_CONFIG_DIR\`): the user-level settings.json, CLAUDE.md, skills, agents, commands, hooks, plugins, history and \`.claude.json\` of this session live there.
Do not read or change \`~/.claude\` or \`~/.claude.json\` for this session: they are symlinks to the default profile, which may be a different one. Where a guide or instruction says \`~/.claude/...\`, use \`$dir/...\`; for \`~/.claude.json\` use \`$dir/.claude.json\`. Project-level files inside a repository (\`.claude/\`, \`CLAUDE.md\`, \`.mcp.json\`) are not affected.
$outro
EOT
else
    # Started without claude-profile: the default config ~/.claude applies.
    target="$(readlink "$HOME/.claude" 2>/dev/null)" || exit 0
    target="${target%/}"
    profile="$(id_of "$target")"
    [ -n "$profile" ] || exit 0
    cat <<EOT
$intro
This session was started without \`claude-profile\` and uses the default config \`~/.claude\`, a symlink that currently points to the profile \`$profile\` (\`$target\`). \`~/.claude.json\` is a symlink into the same profile. Variables that this profile forbids (\`forbid_env\`) were not removed from the environment.
$outro
EOT
fi
