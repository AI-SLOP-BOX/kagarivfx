#!/usr/bin/env bash
set -euo pipefail

echo "🦀 Building AEVFX Studio release binary..."
cargo build --release --features gui --bin aftereffects-oss

echo "📦 Packaging macOS App Bundle..."
BUNDLE_DIR="target/bundle/AEVFX Studio.app/Contents"
mkdir -p "${BUNDLE_DIR}/MacOS"
mkdir -p "${BUNDLE_DIR}/Resources"

cp "target/release/aftereffects-oss" "${BUNDLE_DIR}/MacOS/AEVFX Studio"
chmod +x "${BUNDLE_DIR}/MacOS/AEVFX Studio"

cat << 'PLIST' > "${BUNDLE_DIR}/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>AEVFX Studio</string>
    <key>CFBundleIdentifier</key>
    <string>org.aevfx.studio</string>
    <key>CFBundleName</key>
    <string>AEVFX Studio</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "💿 Creating macOS DMG disk image..."
hdiutil create -volname "AEVFX Studio" -srcfolder "target/bundle/AEVFX Studio.app" -ov -format UDZO "AEVFX-Studio-macOS.dmg"

echo "✅ DMG build complete: AEVFX-Studio-macOS.dmg"
