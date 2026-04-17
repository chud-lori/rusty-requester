#!/usr/bin/env bash
# Deploy script for Rusty Requester.
#
# Usage:
#   ./scripts/deploy.sh v0.2.0
#
# Steps:
#   1. Validate the tag format (vX.Y.Z) and preflight (clean tree, on
#      main, tests pass, not already tagged).
#   2. Bump the version in Cargo.toml and Makefile.
#   3. Rebuild so Cargo.lock picks up the new version.
#   4. Run the test suite.
#   5. Show the diff and ask for confirmation.
#   6. Commit the bump, create an annotated tag, push both.
#      Pushing the tag triggers .github/workflows/release.yml, which
#      builds the DMG and uploads it to a new GitHub Release.

set -euo pipefail

red()    { printf "\033[31m%s\033[0m\n" "$*" >&2; }
green()  { printf "\033[32m%s\033[0m\n" "$*"; }
blue()   { printf "\033[34m%s\033[0m\n" "$*"; }
yellow() { printf "\033[33m%s\033[0m\n" "$*"; }
dim()    { printf "\033[2m%s\033[0m\n" "$*"; }

die() { red "error: $*"; exit 1; }

# --- Parse arg -----------------------------------------------------------
if [ $# -lt 1 ]; then
    die "usage: $0 vX.Y.Z  (e.g. $0 v0.2.0)"
fi
TAG="$1"
if ! [[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    die "tag must look like vX.Y.Z (got: $TAG)"
fi
VERSION="${TAG#v}"   # strip the leading 'v' → 0.2.0

# --- cd to repo root -----------------------------------------------------
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || die "not inside a git repo"
cd "$REPO_ROOT"

# --- Preflight -----------------------------------------------------------
blue "→ Preflight checks"

BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
    die "must be on main (currently on: $BRANCH)"
fi

if ! git diff-index --quiet HEAD --; then
    red "error: working tree has uncommitted changes:"
    git status --short >&2
    exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists locally"
fi

if git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists on origin"
fi

# --- CHANGELOG sanity check ---------------------------------------------
# Require an `## Unreleased` block with real content at the top of
# CHANGELOG.md. Forces us to document every release instead of shipping
# empty tag messages. The block gets renamed to `## [X.Y.Z] — date`
# right before we commit so the committed CHANGELOG matches the tag.
if ! grep -q '^## Unreleased' CHANGELOG.md; then
    die "CHANGELOG.md has no '## Unreleased' section. Add one with this release's notes before deploying."
fi
# Count non-empty, non-header lines between `## Unreleased` and the
# next `##` — if zero, the block is empty and we refuse to release.
UNRELEASED_LINES=$(awk '
    /^## Unreleased/ { capture=1; next }
    capture && /^## / { exit }
    capture && NF > 0 { print }
' CHANGELOG.md | wc -l | tr -d ' ')
if [ "$UNRELEASED_LINES" -lt 1 ]; then
    die "CHANGELOG.md '## Unreleased' section is empty. Document this release before deploying."
fi

dim "  branch=main · tree clean · $TAG is new · changelog has $UNRELEASED_LINES line(s)"

# --- Bump Cargo.toml & Makefile -----------------------------------------
blue "→ Bumping version to $VERSION"

# Cargo.toml — only touch the [package] version line, not dep versions.
# The `-i ''` form works on macOS BSD sed; Linux GNU sed uses `-i` alone.
# Detect and branch:
sed_inplace() {
    if [ "$(uname -s)" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

# Use awk to target only the first `version = "..."` in the file
# (which lives under [package]). Safer than a greedy sed.
awk -v new="$VERSION" '
    /^\[package\]/ { in_pkg=1 }
    /^\[/ && !/\[package\]/ { in_pkg=0 }
    in_pkg && /^version *= *"/ && !done {
        sub(/"[^"]*"/, "\"" new "\"")
        done=1
    }
    { print }
' Cargo.toml > Cargo.toml.new && mv Cargo.toml.new Cargo.toml

# Promote CHANGELOG's `## Unreleased` heading to `## [X.Y.Z] — date`.
# Only the heading changes; the bullet list underneath is already the
# release notes as written during development.
TODAY=$(date +%Y-%m-%d)
awk -v ver="$VERSION" -v day="$TODAY" '
    /^## Unreleased/ && !done {
        print "## [" ver "] — " day
        done = 1
        next
    }
    { print }
' CHANGELOG.md > CHANGELOG.md.new && mv CHANGELOG.md.new CHANGELOG.md

# Makefile no longer needs bumping — it reads VERSION from Cargo.toml
# via `awk` at parse time. This avoids drift between the two. If you
# still see a hardcoded `VERSION := X.Y.Z` line (e.g. from an old
# checkout), update it so the Info.plist ends up correct:
if grep -qE '^VERSION[[:space:]]*:=[[:space:]]*[0-9]' Makefile; then
    yellow "  (Makefile has a legacy hardcoded VERSION — updating)"
    sed_inplace -E "s/^VERSION[[:space:]]*:=[[:space:]]*[0-9][^[:space:]]*/VERSION := $VERSION/" Makefile
fi

dim "  Cargo.toml + Makefile bumped"

# --- Format check --------------------------------------------------------
# Mirror what `ci.yml` enforces so we never tag a release that the CI
# rustfmt job will reject. Cheap (~1 s); fail fast before pushing.
blue "→ Checking formatting (cargo fmt --check)"
if ! cargo fmt --all -- --check >/dev/null 2>&1; then
    red "error: code is not rustfmt-clean. Run 'cargo fmt --all' and re-run deploy."
    cargo fmt --all -- --check 2>&1 | head -20
    exit 1
fi

# --- Clippy check --------------------------------------------------------
# Catches macOS-side clippy issues before the tag push. Note: only
# catches issues on THIS platform — Linux / Windows `#[cfg]`-gated
# code still needs CI to catch. CI (.github/workflows/ci.yml) runs
# clippy on ubuntu-latest for that reason.
blue "→ Running clippy (--all-targets -D warnings)"
if ! cargo clippy --all-targets -- -D warnings >/dev/null 2>&1; then
    red "error: clippy failed. Run 'cargo clippy --all-targets -- -D warnings' and fix before deploying."
    cargo clippy --all-targets -- -D warnings 2>&1 | tail -30
    exit 1
fi

# --- Rebuild so Cargo.lock picks up the new version ---------------------
blue "→ Refreshing Cargo.lock"
cargo build --release --quiet

# --- Tests ---------------------------------------------------------------
blue "→ Running tests"
cargo test --quiet

# --- Confirm -------------------------------------------------------------
green "✓ All checks passed. Diff to be committed:"
echo
git --no-pager diff --stat
echo
yellow "About to:"
yellow "  • commit: \"Release $TAG\""
yellow "  • tag:    $TAG (annotated)"
yellow "  • push:   origin main && origin $TAG"
echo
read -rp "Proceed? [y/N] " CONFIRM
case "$CONFIRM" in
    y|Y|yes|YES) ;;
    *) red "aborted."; exit 1 ;;
esac

# --- Commit + tag + push -------------------------------------------------
blue "→ Committing"
git add Cargo.toml Cargo.lock Makefile CHANGELOG.md
git commit -m "Release $TAG"

blue "→ Tagging"
git tag -a "$TAG" -m "Release $TAG"

blue "→ Pushing main"
git push origin main

blue "→ Pushing $TAG"
git push origin "$TAG"

green "✓ Done. CI will build the DMG and publish the release:"
dim "    https://github.com/chud-lori/rusty-requester/actions"
dim "    https://github.com/chud-lori/rusty-requester/releases/tag/$TAG"
