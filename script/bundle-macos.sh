#!/usr/bin/env bash
# Baut ein universelles Fettsack.app (Apple Silicon + Intel) und packt es als Zip.
# Aufruf: script/bundle-macos.sh <version> <zieldatei.zip>
set -euo pipefail

version="$1"
out="$2"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin

app="target/bundle/Fettsack.app"
rm -rf target/bundle
mkdir -p "$app/Contents/MacOS"
lipo -create -output "$app/Contents/MacOS/fettsack" \
  target/aarch64-apple-darwin/release/fettsack \
  target/x86_64-apple-darwin/release/fettsack

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Fettsack</string>
  <key>CFBundleDisplayName</key><string>Fettsack</string>
  <key>CFBundleIdentifier</key><string>de.fettsack.app</string>
  <key>CFBundleExecutable</key><string>fettsack</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc-Signatur: ohne sie startet die App auf Apple Silicon gar nicht.
codesign --force --deep --sign - "$app"
ditto -c -k --keepParent "$app" "$out"
