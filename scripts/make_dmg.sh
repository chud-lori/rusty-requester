#!/usr/bin/env bash
#
# Build a polished, drag-to-Applications DMG.
#
# Pipeline (the standard "macOS installer" recipe):
#   1. stage  : copy .app + Applications symlink + .background/background.png
#   2. create : a temporary read-write DMG from the staging dir
#   3. mount  : that temp DMG so Finder can see it as a Volume
#   4. apply  : window bounds / icon size / icon positions / background via
#               AppleScript so the layout is baked into .DS_Store
#   5. detach : unmount the read-write DMG
#   6. convert: to a final UDZO-compressed read-only DMG
#
# Uses only macOS built-ins (hdiutil + osascript + ln + cp). No homebrew.
#
# Usage:
#   ./scripts/make_dmg.sh
#
# Prereqs:
#   `make app`                      → target/bundle/RustyRequester.app
#   `python3 scripts/generate_dmg_bg.py` → assets/dmg_background.png
#
set -euo pipefail

APP_NAME="RustyRequester"
VOLNAME="Rusty Requester"
APP_PATH="target/bundle/${APP_NAME}.app"
DMG_PATH="target/bundle/${APP_NAME}.dmg"
TEMP_DMG="target/bundle/${APP_NAME}-temp.dmg"
STAGE_DIR="target/bundle/dmg-staging"
BG_IMAGE="assets/dmg_background.png"

# Window bounds (must match the background image dimensions).
WIN_X=400
WIN_Y=120
WIN_W=600
WIN_H=400
ICON_SIZE=128
APP_X=175
APP_Y=200
LINK_X=425
LINK_Y=200

if [[ ! -d "$APP_PATH" ]]; then
  echo "✗ $APP_PATH not found — run 'make app' first." >&2
  exit 1
fi
if [[ ! -f "$BG_IMAGE" ]]; then
  echo "✗ $BG_IMAGE not found — run 'python3 scripts/generate_dmg_bg.py' first." >&2
  exit 1
fi

# Force-unmount any stale "/Volumes/$VOLNAME" left behind by a previous
# interrupted run — otherwise a new mount returns the old cached volume
# and the AppleScript ends up editing the wrong .DS_Store.
if [ -d "/Volumes/${VOLNAME}" ]; then
  echo "→ unmounting stale /Volumes/${VOLNAME}"
  hdiutil detach "/Volumes/${VOLNAME}" -force >/dev/null 2>&1 || true
fi

echo "→ staging in $STAGE_DIR"
rm -rf "$STAGE_DIR" "$TEMP_DMG" "$DMG_PATH"
mkdir -p "$STAGE_DIR/.background"
cp -R "$APP_PATH" "$STAGE_DIR/"
ln -s /Applications "$STAGE_DIR/Applications"
cp "$BG_IMAGE" "$STAGE_DIR/.background/background.png"

echo "→ creating temporary read-write DMG"
hdiutil create \
  -srcfolder "$STAGE_DIR" \
  -volname "$VOLNAME" \
  -fs HFS+ \
  -format UDRW \
  -ov \
  "$TEMP_DMG" \
  >/dev/null

echo "→ mounting temporary DMG"
DEVICE=$(hdiutil attach -readwrite -noverify -noautoopen "$TEMP_DMG" \
  | grep -E '^/dev/' | head -n 1 | awk '{print $1}')
echo "  mounted: $DEVICE"

# Wait for Finder to fully register the volume + index its contents
# before scripting it. On newer macOS (Sonoma+) the first osascript call
# often fails if this is too short.
sleep 4

echo "→ applying Finder window layout via AppleScript"
# Each fragile call is wrapped in `try` so a single failure (e.g. macOS
# rejecting the background-picture write) doesn't abort the rest of the
# layout. The script as a whole is also `|| true`-tolerant for headless CI
# runners — the DMG ships either way, just possibly without the polish.
BG_POSIX="/Volumes/${VOLNAME}/.background/background.png"
osascript <<EOF || echo "  ⚠ AppleScript layout did not apply cleanly — DMG will still build."
tell application "Finder"
    tell disk "$VOLNAME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        try
            set sidebar width of container window to 0
        end try
        set the bounds of container window to {$WIN_X, $WIN_Y, $((WIN_X + WIN_W)), $((WIN_Y + WIN_H))}
        set viewOptions to the icon view options of container window
        set arrangement of viewOptions to not arranged
        set icon size of viewOptions to $ICON_SIZE
        try
            set text size of viewOptions to 13
        end try
        try
            set shows item info of viewOptions to false
        end try
        -- Background picture: try several syntaxes, stop at the first that
        -- works. Different macOS versions accept different forms of file
        -- reference for this attribute.
        set bgApplied to false
        try
            set background picture of viewOptions to file ".background:background.png"
            set bgApplied to true
        end try
        if not bgApplied then
            try
                set background picture of viewOptions to POSIX file "$BG_POSIX"
                set bgApplied to true
            end try
        end if
        if not bgApplied then
            try
                set bgAlias to (POSIX file "$BG_POSIX") as alias
                set background picture of viewOptions to bgAlias
                set bgApplied to true
            end try
        end if
        if not bgApplied then
            log "background picture could not be applied in any form"
        end if
        try
            set position of item "${APP_NAME}.app" of container window to {$APP_X, $APP_Y}
        end try
        try
            set position of item "Applications" of container window to {$LINK_X, $LINK_Y}
        end try
        update without registering applications
        delay 1
        close
    end tell
end tell
EOF

# Make sure .DS_Store + the background symlink make it to disk before unmount.
sync
sleep 1

echo "→ detaching"
hdiutil detach "$DEVICE" -force >/dev/null

echo "→ converting to compressed read-only DMG"
hdiutil convert "$TEMP_DMG" \
  -format UDZO \
  -imagekey zlib-level=9 \
  -o "$DMG_PATH" \
  >/dev/null
rm -f "$TEMP_DMG"
rm -rf "$STAGE_DIR"

SIZE=$(du -h "$DMG_PATH" | cut -f1)
echo ""
echo "✓ DMG ready: $DMG_PATH  (${SIZE})"
echo "  Open with: open $DMG_PATH"
