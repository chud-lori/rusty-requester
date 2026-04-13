#!/usr/bin/env bash
#
# Build a drag-to-Applications DMG from the .app bundle in target/bundle/.
# Uses only macOS built-ins (hdiutil + ln + cp) — no third-party deps.
#
# Usage:
#   ./scripts/make_dmg.sh
#
# Prereq: `make app` must have produced target/bundle/RustyRequester.app

set -euo pipefail

APP_NAME="RustyRequester"
VOLNAME="Rusty Requester"
APP_PATH="target/bundle/${APP_NAME}.app"
DMG_PATH="target/bundle/${APP_NAME}.dmg"
STAGE_DIR="target/bundle/dmg-staging"

if [[ ! -d "$APP_PATH" ]]; then
  echo "✗ $APP_PATH not found — run 'make app' first." >&2
  exit 1
fi

echo "→ staging $APP_NAME.app + /Applications symlink in $STAGE_DIR"
rm -rf "$STAGE_DIR" "$DMG_PATH"
mkdir -p "$STAGE_DIR"
cp -R "$APP_PATH" "$STAGE_DIR/"
ln -s /Applications "$STAGE_DIR/Applications"

echo "→ building $DMG_PATH (UDZO compressed)"
hdiutil create \
  -volname "$VOLNAME" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH" \
  >/dev/null

rm -rf "$STAGE_DIR"

SIZE=$(du -h "$DMG_PATH" | cut -f1)
echo ""
echo "✓ DMG ready: $DMG_PATH  (${SIZE})"
echo "  Open with: open $DMG_PATH"
