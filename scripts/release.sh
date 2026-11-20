#!/usr/bin/env bash
# Prepares the working tree for a release:
#   - sets the version in Cargo.toml, Cargo.lock and docs/theme/version.js
#   - turns the "Unreleased" section of CHANGELOG.md into the section of this version
#   - writes that section to a file (the notes of the GitHub release)
#
# It only edits files. Committing, tagging and publishing is the job of the release workflow
# (.github/workflows/release.yml). To try it locally: run it, look at `git diff`, then undo
# with `git checkout -- Cargo.toml Cargo.lock CHANGELOG.md docs/theme/version.js`.
#
# Usage: scripts/release.sh <version> [--notes <file>] [--date <yyyy-mm-dd>]

set -euo pipefail

usage() {
    echo "usage: scripts/release.sh <version> [--notes <file>] [--date <yyyy-mm-dd>]" >&2
    exit 2
}

version=""
notes=""
date="$(date -u +%Y-%m-%d)"
while [ $# -gt 0 ]; do
    case "$1" in
        --notes) [ $# -ge 2 ] || usage; notes="$2"; shift 2 ;;
        --date) [ $# -ge 2 ] || usage; date="$2"; shift 2 ;;
        -*) usage ;;
        *) [ -z "$version" ] || usage; version="$1"; shift ;;
    esac
done
[ -n "$version" ] || usage

if ! printf '%s' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "release: \`$version\` is not a version like 1.2.3 (no leading v)" >&2
    exit 2
fi
if ! printf '%s' "$date" | grep -Eq '^[0-9]{4}-[0-9]{2}-[0-9]{2}$'; then
    echo "release: \`$date\` is not a date like 2026-01-31" >&2
    exit 2
fi

cd "$(dirname "$0")/.."

if grep -Fq "## [$version]" CHANGELOG.md; then
    echo "release: CHANGELOG.md already has a section for $version" >&2
    exit 1
fi

# The text between "## [Unreleased]" and the next "## [" heading.
unreleased="$(awk '
    /^## \[Unreleased\]/ { inside = 1; next }
    /^## \[/ { inside = 0 }
    inside { print }
' CHANGELOG.md)"
if [ -z "$(printf '%s' "$unreleased" | tr -d '[:space:]')" ]; then
    echo "release: the Unreleased section of CHANGELOG.md is empty – nothing to release" >&2
    exit 1
fi

export VERSION="$version" RELEASE_DATE="$date"

# Cargo.toml: the version of the package (the first `version = ` line).
perl -0pi -e 's/^version = "[^"]*"/version = "$ENV{VERSION}"/m or die "release: no version line in Cargo.toml\n"' Cargo.toml
# Cargo.lock: the entry of this package.
perl -0pi -e 's/(name = "claude-profile"\nversion = ")[^"]*"/$1$ENV{VERSION}"/ or die "release: claude-profile not found in Cargo.lock\n"' Cargo.lock
# The version tag shown in the docs.
perl -0pi -e 's/(const CLAUDE_PROFILE_VERSION = ")[^"]*"/$1$ENV{VERSION}"/ or die "release: no version line in docs\/theme\/version.js\n"' docs/theme/version.js
# CHANGELOG.md: a fresh, empty Unreleased section above the section of this version.
perl -0pi -e 's/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$ENV{VERSION}] - $ENV{RELEASE_DATE}\n/m or die "release: no Unreleased section in CHANGELOG.md\n"' CHANGELOG.md

if [ -n "$notes" ]; then
    # Without leading and trailing blank lines.
    printf '%s\n' "$unreleased" | sed -e '/./,$!d' | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' > "$notes"
fi

echo "release: prepared $version ($date)"
