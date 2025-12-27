#!/usr/bin/env bash
set -euo pipefail

# Build a signed app bundle (unsigned here) and DMG for Apple Silicon macOS.
# Requirements (install via Homebrew if missing):
#   brew install librsvg   # for rsvg-convert (SVG -> PNG)
# macOS provides: sips, iconutil, hdiutil.

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO_ROOT"

APP_NAME="TrackPersonalInsights"
BUNDLE_ROOT="dist/macos/${APP_NAME}.app"
ICON_SRC="assets/trackinsights.svg"
ICONSET_DIR="dist/macos/icon.iconset"
ICNS_PATH="dist/macos/${APP_NAME}.icns"
DMG_PATH="dist/${APP_NAME}-macos-aarch64.dmg"
BUNDLE_BIN_DIR="$BUNDLE_ROOT/Contents/MacOS"
BUNDLE_RES_DIR="$BUNDLE_ROOT/Contents/Resources"
BUNDLE_REAL_BIN="$BUNDLE_BIN_DIR/${APP_NAME}.real"
BUNDLE_LAUNCHER="$BUNDLE_BIN_DIR/${APP_NAME}"

# Basic tool checks
for tool in cargo iconutil hdiutil sips; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing tool: $tool. Install it first." >&2
    exit 1
  fi
done
if ! command -v rsvg-convert >/dev/null 2>&1; then
  echo "Missing tool: rsvg-convert. Install with: brew install librsvg" >&2
  exit 1
fi

# Discover version from Cargo.toml
VERSION=$(sed -n 's/^version\s*=\s*"\(.*\)"/\1/p' Cargo.toml | head -n 1)
if [ -z "$VERSION" ]; then
  VERSION="0.0.0"
fi

# Build release binary for Apple Silicon
cargo build --release --target aarch64-apple-darwin
BIN_PATH="target/aarch64-apple-darwin/release/${APP_NAME}"
if [ ! -f "$BIN_PATH" ]; then
  echo "Expected binary not found at $BIN_PATH" >&2
  exit 1
fi

# Prepare dist folders
rm -rf "dist/macos"
mkdir -p "$ICONSET_DIR" "$(dirname "$BUNDLE_ROOT")"

# Generate icon set from SVG
rsvg-convert -w 1024 -h 1024 "$ICON_SRC" -o "$ICONSET_DIR/icon_512x512@2x.png"
for size in 16 32 64 128 256 512; do
  sips -z "$size" "$size" "$ICONSET_DIR/icon_512x512@2x.png" --out "$ICONSET_DIR/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$ICONSET_DIR/icon_512x512@2x.png" --out "$ICONSET_DIR/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET_DIR" -o "$ICNS_PATH"

# Build .app bundle structure
mkdir -p "$BUNDLE_BIN_DIR" "$BUNDLE_RES_DIR"
cat > "$BUNDLE_ROOT/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key><string>com.trackinsights.app</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleExecutable</key><string>${APP_NAME}</string>
    <key>CFBundleIconFile</key><string>${APP_NAME}</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
    <key>LSBackgroundOnly</key><false/>
</dict>
</plist>
EOF
cp "$ICNS_PATH" "$BUNDLE_RES_DIR/${APP_NAME}.icns"

# Place the real binary
cp "$BIN_PATH" "$BUNDLE_REAL_BIN"
chmod +x "$BUNDLE_REAL_BIN"

# Create a simple launcher that opens Terminal with the binary
cat > "$BUNDLE_LAUNCHER" <<'LAUNCH'
#!/usr/bin/env bash
APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
exec open -a Terminal "$APP_DIR/MacOS/TrackPersonalInsights.real"
LAUNCH
chmod +x "$BUNDLE_LAUNCHER"

# Create compressed DMG containing the app bundle
rm -f "$DMG_PATH"
hdiutil create -volname "$APP_NAME" -srcfolder "$(dirname "$BUNDLE_ROOT")" -ov -format UDZO "$DMG_PATH"

echo "Built installer: $DMG_PATH"
echo "Install by opening the DMG and dragging ${APP_NAME}.app to Applications."
