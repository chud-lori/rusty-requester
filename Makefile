# Rusty Requester — common build / dev tasks
APP_NAME       := RustyRequester
BUNDLE_ID      := com.rustyrequester.app
VERSION := 0.2.1
TARGET_DIR     := target
RELEASE_BIN    := $(TARGET_DIR)/release/rusty-requester
ICON_PNG       := assets/icon.png
BUNDLE_DIR     := $(TARGET_DIR)/bundle
APP_BUNDLE     := $(BUNDLE_DIR)/$(APP_NAME).app

DMG_PATH       := $(BUNDLE_DIR)/$(APP_NAME).dmg

.PHONY: help run release test fmt lint clean icon dmg-bg app app-install bundle-mac dmg

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
	@echo "  make dmg           Build a drag-to-Applications .dmg installer"
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
	  '  <key>CFBundleName</key><string>$(APP_NAME)</string>' \
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
