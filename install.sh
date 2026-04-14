#!/usr/bin/env bash
# Rusty Requester — one-line installer for macOS.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash
#
# Or with a specific version (defaults to the latest release):
#   curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | VERSION=v0.2.0 bash
#
# What it does:
#   1. Resolves the latest (or requested) release from the GitHub API.
#   2. Downloads RustyRequester.dmg into a temp dir.
#   3. Mounts the DMG, copies RustyRequester.app into /Applications
#      (or ~/Applications if /Applications isn't writable).
#   4. Strips the quarantine attribute so Gatekeeper doesn't block
#      first launch — because the app isn't notarised (no paid Apple
#      developer account). If you want Gatekeeper to check it, drop
#      `SKIP_QUARANTINE_STRIP=1` in front of the curl.
#   5. Detaches the DMG and cleans up.

set -euo pipefail

REPO="${RUSTY_REPO:-chud-lori/rusty-requester}"
ASSET_NAME="RustyRequester.dmg"
APP_NAME="RustyRequester.app"
TAG="${VERSION:-}"

red()   { printf "\033[31m%s\033[0m\n" "$*"; }
green() { printf "\033[32m%s\033[0m\n" "$*"; }
blue()  { printf "\033[34m%s\033[0m\n" "$*"; }
dim()   { printf "\033[2m%s\033[0m\n" "$*"; }

require() {
    command -v "$1" >/dev/null 2>&1 || { red "error: '$1' is required but not installed"; exit 1; }
}

# --- Platform check ------------------------------------------------------
case "$(uname -s)" in
    Darwin) ;;
    *) red "error: this installer currently supports macOS only. Build from source on Linux/Windows — see the README."; exit 1 ;;
esac

require curl
require hdiutil
require grep
require awk

# --- Resolve release tag -------------------------------------------------
if [ -z "${TAG}" ]; then
    blue "→ Resolving latest release from github.com/${REPO}..."
    TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
          | awk -F'"' '/"tag_name"/{print $4; exit}')
fi
if [ -z "${TAG}" ]; then
    red "error: couldn't resolve a release tag (rate-limited or repo has no releases yet?)"
    exit 1
fi
green "  using ${TAG}"

DMG_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET_NAME}"

# --- Pick install target -------------------------------------------------
if [ -w "/Applications" ] || [ "$(id -u)" = "0" ]; then
    TARGET_DIR="/Applications"
else
    TARGET_DIR="$HOME/Applications"
    mkdir -p "$TARGET_DIR"
    dim "  /Applications not writable — installing to $TARGET_DIR"
fi

# --- Download ------------------------------------------------------------
TMP_DIR=$(mktemp -d -t rusty-requester)
trap 'hdiutil detach -quiet "${MOUNT_DIR:-/nonexistent}" 2>/dev/null || true; rm -rf "$TMP_DIR"' EXIT

DMG_PATH="$TMP_DIR/$ASSET_NAME"
blue "→ Downloading $ASSET_NAME..."
curl -fL --progress-bar "$DMG_URL" -o "$DMG_PATH"

# --- Mount, copy, unmount ------------------------------------------------
MOUNT_DIR="$TMP_DIR/mount"
mkdir -p "$MOUNT_DIR"
blue "→ Mounting DMG..."
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$MOUNT_DIR" "$DMG_PATH"

SRC_APP="$MOUNT_DIR/$APP_NAME"
if [ ! -d "$SRC_APP" ]; then
    red "error: $APP_NAME not found inside the DMG"
    exit 1
fi

if [ -d "$TARGET_DIR/$APP_NAME" ]; then
    blue "→ Removing previous install at $TARGET_DIR/$APP_NAME..."
    rm -rf "$TARGET_DIR/$APP_NAME"
fi

blue "→ Copying to $TARGET_DIR/$APP_NAME..."
cp -R "$SRC_APP" "$TARGET_DIR/"

hdiutil detach -quiet "$MOUNT_DIR"
MOUNT_DIR=""

# --- Strip quarantine so Gatekeeper doesn't block first launch -----------
if [ "${SKIP_QUARANTINE_STRIP:-0}" != "1" ]; then
    if command -v xattr >/dev/null 2>&1; then
        xattr -dr com.apple.quarantine "$TARGET_DIR/$APP_NAME" 2>/dev/null || true
    fi
fi

green "✓ Installed $APP_NAME to $TARGET_DIR"
echo
dim "  Launch it from Spotlight, Launchpad, or:"
dim "    open '$TARGET_DIR/$APP_NAME'"
