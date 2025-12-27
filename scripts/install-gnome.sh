#!/usr/bin/env bash
# Install a GNOME launcher for TrackPersonalInsights that opens the terminal app with the provided icon.
set -euo pipefail

APP_NAME="TrackPersonalInsights"
APP_ID="trackinsights"
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
# Prefer release binary if present; fallback to built target
if [[ -x "$REPO_DIR/releases/v1.11.0/TrackPersonalInsights-linux-x86_64" ]]; then
  BIN_PATH="$REPO_DIR/releases/v1.11.0/TrackPersonalInsights-linux-x86_64"
elif [[ -x "$REPO_DIR/releases/TrackPersonalInsights-linux-x86_64" ]]; then
  BIN_PATH="$REPO_DIR/releases/TrackPersonalInsights-linux-x86_64"
else
  BIN_PATH="$REPO_DIR/target/release/TrackPersonalInsights"
fi
ICON_SRC="$REPO_DIR/assets/trackinsights.svg"
ICON_DEST="$HOME/.local/share/icons/trackinsights.svg"
DESKTOP_FILE="$HOME/.local/share/applications/${APP_ID}.desktop"

# Ensure the binary exists
if [[ ! -x "$BIN_PATH" ]]; then
  echo "Binary not found. Build it with: cargo build --release" >&2
  echo "Or place the release binary at: releases/v1.11.0/TrackPersonalInsights-linux-x86_64" >&2
  exit 1
fi

# Install icon
mkdir -p "$(dirname "$ICON_DEST")"
cp "$ICON_SRC" "$ICON_DEST"

# Create desktop entry
mkdir -p "$(dirname "$DESKTOP_FILE")"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=Track insights with notes, tasks, habits, and more
Exec=gnome-terminal -- $BIN_PATH
Terminal=true
Icon=$ICON_DEST
Categories=Office;Utility;
EOF

# Update desktop database (if available)
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" || true
fi

echo "Installed launcher: $DESKTOP_FILE"
echo "You can now search 'TrackPersonalInsights' in GNOME and launch it."
