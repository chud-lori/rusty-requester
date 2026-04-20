#!/usr/bin/env bash
# Rusty Requester — one-line installer for macOS + Linux.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash
#
# Specific version:
#   curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | VERSION=v0.3.0 bash
#
# What it does:
#   macOS — downloads the universal DMG, mounts it, copies
#     RustyRequester.app into /Applications (falls back to
#     ~/Applications), strips the Gatekeeper quarantine attribute,
#     re-registers with Launch Services so Dock / Spotlight pick up
#     the new bundle.
#   Linux — downloads the x86_64 tarball, extracts, runs the bundled
#     install-local.sh which puts the binary in ~/.local/bin and
#     registers a .desktop entry in ~/.local/share/applications.
#
# Env knobs:
#   VERSION=vX.Y.Z                pin a specific release tag
#   SKIP_QUARANTINE_STRIP=1       (macOS) keep Gatekeeper's attribute
#   RUSTY_REPO=owner/name         override repo (default chud-lori/rusty-requester)

set -euo pipefail

REPO="${RUSTY_REPO:-chud-lori/rusty-requester}"
TAG="${VERSION:-}"

red()   { printf "\033[31m%s\033[0m\n" "$*"; }
green() { printf "\033[32m%s\033[0m\n" "$*"; }
blue()  { printf "\033[34m%s\033[0m\n" "$*"; }
dim()   { printf "\033[2m%s\033[0m\n" "$*"; }
die()   { red "error: $*"; exit 1; }

require() {
    command -v "$1" >/dev/null 2>&1 || die "'$1' is required but not installed"
}

# --- Detect platform -----------------------------------------------------
OS=$(uname -s)
ARCH=$(uname -m)
case "$OS" in
    Darwin)  PLATFORM="macos"  ;;
    Linux)   PLATFORM="linux"  ;;
    *)       die "unsupported OS: $OS (only macOS + Linux are supported)" ;;
esac

if [ "$PLATFORM" = "linux" ] && [ "$ARCH" != "x86_64" ]; then
    die "unsupported Linux arch: $ARCH (only x86_64 is built today — build from source)"
fi

require curl

# --- Resolve release tag -------------------------------------------------
if [ -z "${TAG}" ]; then
    blue "→ Resolving latest release from github.com/${REPO}..."
    # Capture the full response first — piping curl directly into `awk '... exit}'`
    # triggers SIGPIPE (curl exit 23 "Failed writing body") under `set -o pipefail`
    # on Linux, because awk closes the pipe before curl finishes writing.
    RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest") \
        || die "couldn't reach GitHub API (rate-limited or offline?)"
    TAG=$(printf '%s' "$RELEASE_JSON" | awk -F'"' '/"tag_name"/{print $4; exit}')
fi
if [ -z "${TAG}" ]; then
    die "couldn't resolve a release tag (rate-limited or repo has no releases yet?)"
fi
green "  using ${TAG}"
VERSION_BARE="${TAG#v}"

# Portable mktemp: Linux requires the template to contain `XXXXXX`;
# macOS `mktemp -d -t foo` silently appends them. Use an explicit
# path template that works on both.
TMP=$(mktemp -d "${TMPDIR:-/tmp}/rusty-requester.XXXXXX")
MOUNT_DIR=""
cleanup() {
    if [ -n "$MOUNT_DIR" ] && [ -d "$MOUNT_DIR" ]; then
        hdiutil detach -quiet "$MOUNT_DIR" 2>/dev/null || true
    fi
    rm -rf "$TMP"
}
trap cleanup EXIT

# --- Platform-specific install ------------------------------------------
if [ "$PLATFORM" = "macos" ]; then
    require hdiutil
    ASSET="RustyRequester-${TAG}-macos-universal.dmg"
    DMG_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
    APP_NAME="RustyRequester.app"

    if [ -w "/Applications" ] || [ "$(id -u)" = "0" ]; then
        TARGET_DIR="/Applications"
    else
        TARGET_DIR="$HOME/Applications"
        mkdir -p "$TARGET_DIR"
        dim "  /Applications not writable — installing to $TARGET_DIR"
    fi

    blue "→ Downloading $ASSET..."
    curl -fL --progress-bar "$DMG_URL" -o "$TMP/r.dmg" \
        || die "download failed. Check that $ASSET exists on the release."

    MOUNT_DIR="$TMP/mount"
    mkdir -p "$MOUNT_DIR"
    blue "→ Mounting DMG..."
    hdiutil attach -quiet -nobrowse -readonly -mountpoint "$MOUNT_DIR" "$TMP/r.dmg"

    SRC_APP="$MOUNT_DIR/$APP_NAME"
    [ -d "$SRC_APP" ] || die "$APP_NAME not found inside the DMG"

    if [ -d "$TARGET_DIR/$APP_NAME" ]; then
        blue "→ Quitting any running instance..."
        osascript -e 'tell application "RustyRequester" to quit' 2>/dev/null || true
        pkill -x RustyRequester 2>/dev/null || true
        blue "→ Removing previous install..."
        rm -rf "$TARGET_DIR/$APP_NAME"
    fi

    blue "→ Copying to $TARGET_DIR/$APP_NAME..."
    cp -R "$SRC_APP" "$TARGET_DIR/"

    hdiutil detach -quiet "$MOUNT_DIR"
    MOUNT_DIR=""

    if [ "${SKIP_QUARANTINE_STRIP:-0}" != "1" ]; then
        if command -v xattr >/dev/null 2>&1; then
            xattr -dr com.apple.quarantine "$TARGET_DIR/$APP_NAME" 2>/dev/null || true
        fi
    fi

    LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"
    if [ -x "$LSREGISTER" ]; then
        "$LSREGISTER" -f "$TARGET_DIR/$APP_NAME" >/dev/null 2>&1 || true
    fi

    BIN="$TARGET_DIR/$APP_NAME/Contents/MacOS/RustyRequester"
    INSTALLED_SHA=""
    if [ -f "$BIN" ]; then
        INSTALLED_SHA=$(shasum -a 256 "$BIN" 2>/dev/null | awk '{print $1}')
    fi

    green "✓ Installed $APP_NAME to $TARGET_DIR ($TAG)"
    [ -n "$INSTALLED_SHA" ] && dim "  binary SHA256: $INSTALLED_SHA"
    echo
    dim "  Launch it from Spotlight, Launchpad, or:"
    dim "    open '$TARGET_DIR/$APP_NAME'"

elif [ "$PLATFORM" = "linux" ]; then
    require tar
    ASSET="RustyRequester-${TAG}-linux-x86_64.tar.gz"
    TARBALL_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

    blue "→ Downloading $ASSET..."
    curl -fL --progress-bar "$TARBALL_URL" -o "$TMP/r.tar.gz" \
        || die "download failed. Check that $ASSET exists on the release."

    blue "→ Extracting..."
    tar -xzf "$TMP/r.tar.gz" -C "$TMP"

    STAGE="$TMP/RustyRequester"
    [ -d "$STAGE" ] || die "unexpected tarball layout (expected RustyRequester/ at root)"

    blue "→ Running bundled installer (no sudo; installs to ~/.local)..."
    "$STAGE/install-local.sh"

    BIN="$HOME/.local/bin/rusty-requester"
    INSTALLED_SHA=""
    if [ -f "$BIN" ]; then
        INSTALLED_SHA=$(sha256sum "$BIN" 2>/dev/null | awk '{print $1}')
    fi

    green "✓ Installed rusty-requester ($TAG)"
    [ -n "$INSTALLED_SHA" ] && dim "  binary SHA256: $INSTALLED_SHA"
    echo
    case ":${PATH}:" in
        *":$HOME/.local/bin:"*) ;;
        *) dim "  note: $HOME/.local/bin is NOT on your PATH — add it to your shell rc:";
           dim "    export PATH=\"\$HOME/.local/bin:\$PATH\"";;
    esac
    dim "  Launch from your DE's app launcher or run: rusty-requester"
fi
