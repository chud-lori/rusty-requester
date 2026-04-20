# Rusty Requester — common build / dev tasks
APP_NAME       := RustyRequester
BUNDLE_ID      := com.rustyrequester.app
# Single source of truth: read the version straight from Cargo.toml's
# `[package]` block so Info.plist's CFBundleShortVersionString and the
# embedded CARGO_PKG_VERSION can never drift. deploy.sh only needs to
# update Cargo.toml now (the `VERSION := ...` bump is still performed
# for backwards compat).
VERSION := $(shell awk '/^\[package\]/{p=1; next} /^\[/{p=0} p && /^version/{split($$0, a, "\""); print a[2]; exit}' Cargo.toml)
TARGET_DIR     := target
RELEASE_BIN    := $(TARGET_DIR)/release/rusty-requester
ICON_PNG       := assets/icon.png
BUNDLE_DIR     := $(TARGET_DIR)/bundle
APP_BUNDLE     := $(BUNDLE_DIR)/$(APP_NAME).app

DMG_PATH       := $(BUNDLE_DIR)/$(APP_NAME).dmg
# Release-asset filenames. Versioned so CDN caches can't serve stale
# bytes under the same URL.
DMG_ASSET      := $(BUNDLE_DIR)/$(APP_NAME)-v$(VERSION)-macos-universal.dmg
LINUX_TARBALL  := $(BUNDLE_DIR)/$(APP_NAME)-v$(VERSION)-linux-x86_64.tar.gz

.PHONY: help run release test fmt lint clean icon dmg-bg app app-install bundle-mac dmg dmg-universal tarball-linux

help:
	@echo "Targets:"
	@echo "  make run           Run the app in debug mode"
	@echo "  make release       Build optimized release binary"
	@echo "  make test          Run all unit tests"
	@echo "  make fmt           Format with cargo fmt"
	@echo "  make lint          Run cargo clippy"
	@echo "  make icon          Regenerate assets/icon.png from Python"
	@echo "  make dmg-bg        Regenerate assets/dmg_background.png from Python"
	@echo "  make app           Build a macOS .app bundle (uses release binary + ICNS icon)"
	@echo "  make app-install   Build the bundle and copy to /Applications"
	@echo "  make dmg           Build a drag-to-Applications .dmg installer (current arch only)"
	@echo "  make dmg-universal Build a universal (arm64 + x86_64) .dmg — used by CI"
	@echo "  make tarball-linux Build a Linux x86_64 .tar.gz (release asset) — used by CI"
	@echo "  make clean         cargo clean + remove bundle"

run:
	cargo run

release:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

icon:
	python3 scripts/generate_icon.py

dmg-bg:
	python3 scripts/generate_dmg_bg.py

clean:
	cargo clean
	rm -rf $(BUNDLE_DIR)

# ----- macOS app bundle -----
app: bundle-mac
	@echo "Bundle ready: $(APP_BUNDLE)"
	@echo "Open with: open $(APP_BUNDLE)"

app-install: bundle-mac
	rm -rf /Applications/$(APP_NAME).app
	cp -R $(APP_BUNDLE) /Applications/
	@echo "Installed to /Applications/$(APP_NAME).app"

bundle-mac: $(RELEASE_BIN) $(ICON_PNG)
	@command -v iconutil >/dev/null 2>&1 || { echo "iconutil not found (needs macOS)"; exit 1; }
	@command -v sips >/dev/null 2>&1 || { echo "sips not found (needs macOS)"; exit 1; }
	rm -rf $(APP_BUNDLE)
	mkdir -p $(APP_BUNDLE)/Contents/MacOS
	mkdir -p $(APP_BUNDLE)/Contents/Resources
	cp $(RELEASE_BIN) $(APP_BUNDLE)/Contents/MacOS/$(APP_NAME)
	chmod +x $(APP_BUNDLE)/Contents/MacOS/$(APP_NAME)
	# Build .icns from PNG via iconutil
	rm -rf $(BUNDLE_DIR)/icon.iconset
	mkdir -p $(BUNDLE_DIR)/icon.iconset
	sips -z 16   16   $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_16x16.png      >/dev/null
	sips -z 32   32   $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_16x16@2x.png   >/dev/null
	sips -z 32   32   $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_32x32.png      >/dev/null
	sips -z 64   64   $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_32x32@2x.png   >/dev/null
	sips -z 128  128  $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_128x128.png    >/dev/null
	sips -z 256  256  $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_128x128@2x.png >/dev/null
	sips -z 256  256  $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_256x256.png    >/dev/null
	sips -z 512  512  $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_256x256@2x.png >/dev/null
	sips -z 512  512  $(ICON_PNG) --out $(BUNDLE_DIR)/icon.iconset/icon_512x512.png    >/dev/null
	cp $(ICON_PNG) $(BUNDLE_DIR)/icon.iconset/icon_512x512@2x.png
	iconutil -c icns $(BUNDLE_DIR)/icon.iconset -o $(APP_BUNDLE)/Contents/Resources/AppIcon.icns
	# Write Info.plist
	@printf '%s\n' \
	  '<?xml version="1.0" encoding="UTF-8"?>' \
	  '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' \
	  '<plist version="1.0">' \
	  '<dict>' \
	  '  <key>CFBundleName</key><string>Rusty Requester</string>' \
	  '  <key>CFBundleDisplayName</key><string>Rusty Requester</string>' \
	  '  <key>CFBundleIdentifier</key><string>$(BUNDLE_ID)</string>' \
	  '  <key>CFBundleVersion</key><string>$(VERSION)</string>' \
	  '  <key>CFBundleShortVersionString</key><string>$(VERSION)</string>' \
	  '  <key>CFBundlePackageType</key><string>APPL</string>' \
	  '  <key>CFBundleExecutable</key><string>$(APP_NAME)</string>' \
	  '  <key>CFBundleIconFile</key><string>AppIcon</string>' \
	  '  <key>NSHighResolutionCapable</key><true/>' \
	  '  <key>LSMinimumSystemVersion</key><string>10.13</string>' \
	  '</dict>' \
	  '</plist>' > $(APP_BUNDLE)/Contents/Info.plist

$(RELEASE_BIN):
	$(MAKE) release

# ----- DMG installer -----
dmg: bundle-mac
	./scripts/make_dmg.sh
	@echo "DMG ready: $(DMG_PATH)"

# Universal macOS DMG — combines arm64 + x86_64 release binaries with
# `lipo -create` before bundling. Requires both targets installed:
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
dmg-universal:
	@command -v lipo >/dev/null 2>&1 || { echo "lipo not found (needs macOS)"; exit 1; }
	rustup target add aarch64-apple-darwin x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin
	cargo build --release --target x86_64-apple-darwin
	mkdir -p $(TARGET_DIR)/release
	lipo -create \
	  $(TARGET_DIR)/aarch64-apple-darwin/release/rusty-requester \
	  $(TARGET_DIR)/x86_64-apple-darwin/release/rusty-requester \
	  -output $(RELEASE_BIN)
	@echo "Universal binary:"
	@file $(RELEASE_BIN)
	$(MAKE) bundle-mac
	./scripts/make_dmg.sh
	cp $(DMG_PATH) $(DMG_ASSET)
	@echo "Universal DMG ready: $(DMG_ASSET)"

# Linux tarball — single x86_64 binary + icon + .desktop file, gzipped.
# Extracted by the installer into ~/.local/share/rusty-requester and
# symlinked into ~/.local/bin. Runs on most glibc-based distros from
# ~2018 onward; static linking against musl would give broader reach
# but pulls in extra reqwest/openssl complexity — defer if needed.
tarball-linux:
	cargo build --release --target x86_64-unknown-linux-gnu
	rm -rf $(BUNDLE_DIR)/linux-stage
	mkdir -p $(BUNDLE_DIR)/linux-stage/$(APP_NAME)
	cp $(TARGET_DIR)/x86_64-unknown-linux-gnu/release/rusty-requester \
	   $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/rusty-requester
	chmod +x $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/rusty-requester
	cp $(ICON_PNG) $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/icon.png
	@# NOTE: the Icon= line is a placeholder — install-local.sh
	@# rewrites it to an ABSOLUTE path at install time (`$HOME` is
	@# user-dependent so we can't hardcode it here). Absolute paths
	@# bypass freedesktop icon-theme lookup entirely, avoiding the
	@# "icon not shown until icon cache refreshed / logout" trap on
	@# GNOME/Ubuntu. Also drop into pixmaps/ as a legacy fallback
	@# for DEs that ignore hicolor 512x512-only themes.
	@# StartupWMClass must match the app's Wayland `app_id` /
	@# X11 `WM_CLASS` (see `with_app_id` in main.rs). Without this,
	@# GNOME on Wayland can't associate the running window with
	@# the launcher and shows a generic cog as the dock icon
	@# (issue #18), regardless of `_NET_WM_ICON`.
	@printf '%s\n' \
	  '[Desktop Entry]' \
	  'Type=Application' \
	  'Name=Rusty Requester' \
	  'Comment=Native, offline, lightweight API client' \
	  'Exec=rusty-requester' \
	  'Icon=rusty-requester' \
	  'StartupWMClass=rusty-requester' \
	  'Categories=Development;Network;' \
	  'Terminal=false' \
	  > $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/rusty-requester.desktop
	@# install-local.sh: installs the binary DIRECTLY into
	@# ~/.local/bin (previously it sat in ~/.local/share/rusty-requester/
	@# next to data.json, so uninstalling was footgun-y — `rm -rf` the
	@# dir wiped user collections). Binary path is now purely
	@# executable; ~/.local/share/rusty-requester/ is reserved for
	@# user data (data.json). No symlink indirection needed.
	@printf '%s\n' \
	  '#!/bin/sh' \
	  '# Installer invoked by install.sh. Idempotent.' \
	  'set -e' \
	  'STAGE=$$(cd "$$(dirname "$$0")" && pwd)' \
	  'BIN="$$HOME/.local/bin"' \
	  'DESKTOP="$$HOME/.local/share/applications"' \
	  'ICONS_HICOLOR="$$HOME/.local/share/icons/hicolor/512x512/apps"' \
	  'PIXMAPS="$$HOME/.local/share/pixmaps"' \
	  'mkdir -p "$$BIN" "$$DESKTOP" "$$ICONS_HICOLOR" "$$PIXMAPS"' \
	  '# Migrate from pre-v0.16.9 layout (binary lived in ~/.local/share/rusty-requester/' \
	  '# alongside data.json). Remove the old binary + symlink; data.json stays put.' \
	  'OLD_BIN="$$HOME/.local/share/rusty-requester/rusty-requester"' \
	  '[ -f "$$OLD_BIN" ] && rm -f "$$OLD_BIN" || true' \
	  '[ -L "$$BIN/rusty-requester" ] && rm -f "$$BIN/rusty-requester" || true' \
	  'install -m 755 "$$STAGE/rusty-requester" "$$BIN/rusty-requester"' \
	  'install -m 644 "$$STAGE/icon.png" "$$ICONS_HICOLOR/rusty-requester.png"' \
	  'install -m 644 "$$STAGE/icon.png" "$$PIXMAPS/rusty-requester.png"' \
	  '# Rewrite Icon= to an absolute path — bypasses icon-theme lookup + caching.' \
	  'sed "s|^Icon=.*|Icon=$$PIXMAPS/rusty-requester.png|" "$$STAGE/rusty-requester.desktop" > "$$DESKTOP/rusty-requester.desktop"' \
	  'chmod 644 "$$DESKTOP/rusty-requester.desktop"' \
	  'command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$$DESKTOP" >/dev/null 2>&1 || true' \
	  'command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true' \
	  'echo "Installed. If $$BIN is on your PATH, run: rusty-requester"' \
	  'echo "To uninstall later: $$STAGE/uninstall-local.sh   (or curl | UNINSTALL=1 bash)"' \
	  > $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/install-local.sh
	chmod +x $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/install-local.sh
	@# uninstall-local.sh: removes everything install-local.sh drops,
	@# but preserves user data at ~/.local/share/rusty-requester/
	@# (collections, history, OAuth tokens). Pass `--purge` to wipe
	@# user data too.
	@printf '%s\n' \
	  '#!/bin/sh' \
	  '# Uninstaller for Rusty Requester. Safe by default: preserves' \
	  '# ~/.local/share/rusty-requester/data.json. Pass --purge to wipe it.' \
	  'set -e' \
	  'PURGE=0' \
	  '[ "$$1" = "--purge" ] && PURGE=1' \
	  'BIN="$$HOME/.local/bin/rusty-requester"' \
	  'DESKTOP="$$HOME/.local/share/applications/rusty-requester.desktop"' \
	  'ICON_HICOLOR="$$HOME/.local/share/icons/hicolor/512x512/apps/rusty-requester.png"' \
	  'ICON_PIXMAP="$$HOME/.local/share/pixmaps/rusty-requester.png"' \
	  'DATA_DIR="$$HOME/.local/share/rusty-requester"' \
	  'rm -f "$$BIN" "$$DESKTOP" "$$ICON_HICOLOR" "$$ICON_PIXMAP"' \
	  '# Legacy binary location (pre-v0.16.9) — clean up if present.' \
	  'rm -f "$$DATA_DIR/rusty-requester"' \
	  'if [ "$$PURGE" = "1" ]; then' \
	  '  rm -rf "$$DATA_DIR"' \
	  '  echo "Purged user data at $$DATA_DIR"' \
	  'else' \
	  '  [ -d "$$DATA_DIR" ] && echo "Kept user data at $$DATA_DIR (use --purge to remove)" || true' \
	  'fi' \
	  'command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$$HOME/.local/share/applications" >/dev/null 2>&1 || true' \
	  'command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true' \
	  'echo "Uninstalled rusty-requester."' \
	  > $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/uninstall-local.sh
	chmod +x $(BUNDLE_DIR)/linux-stage/$(APP_NAME)/uninstall-local.sh
	tar -czf $(LINUX_TARBALL) -C $(BUNDLE_DIR)/linux-stage $(APP_NAME)
	@echo "Linux tarball ready: $(LINUX_TARBALL)"
