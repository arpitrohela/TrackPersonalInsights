#!/usr/bin/env bash
# Install TrackPersonalInsights as a regular clickable app: copies the binary
# to ~/.local/bin, installs the icon into the XDG hicolor theme, and writes a
# .desktop launcher. Works with any spec-compliant app launcher (Omarchy/
# Hyprland launchers like walker/fuzzel/wofi, GNOME, KDE, etc.) since it uses
# Terminal=true instead of hardcoding a specific terminal emulator.
set -euo pipefail

APP_NAME="TrackPersonalInsights"
APP_ID="trackinsights"
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Prefer a freshly built binary; fall back to the committed release binary.
if [[ -x "$REPO_DIR/target/release/TrackPersonalInsights" ]]; then
  SRC_BIN="$REPO_DIR/target/release/TrackPersonalInsights"
elif [[ -x "$REPO_DIR/releases/TrackPersonalInsights-linux-x86_64" ]]; then
  SRC_BIN="$REPO_DIR/releases/TrackPersonalInsights-linux-x86_64"
else
  echo "Binary not found. Build it first with: cargo build --release" >&2
  exit 1
fi

BIN_DEST="$HOME/.local/bin/$APP_NAME"
ICON_SRC="$REPO_DIR/assets/trackinsights.svg"
ICON_DEST="$HOME/.local/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
DESKTOP_FILE="$HOME/.local/share/applications/${APP_ID}.desktop"

# Install the binary somewhere stable (independent of this repo's location).
mkdir -p "$(dirname "$BIN_DEST")"
cp "$SRC_BIN" "$BIN_DEST"
chmod +x "$BIN_DEST"

# Install the icon into the standard XDG icon theme location, referenced by
# name (not path) so any launcher that resolves icon themes can find it.
mkdir -p "$(dirname "$ICON_DEST")"
cp "$ICON_SRC" "$ICON_DEST"

# Create the desktop entry. Terminal=true + a bare Exec (no wrapping terminal
# command) is the portable choice: the launcher runs it inside whatever
# terminal emulator the user has configured (foot, alacritty, kitty, ghostty,
# gnome-terminal, ...) instead of assuming one is installed.
mkdir -p "$(dirname "$DESKTOP_FILE")"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=Terminal productivity app: notes, planner, journal, habits, finances, kanban, flashcards
Exec=$BIN_DEST
Terminal=true
Icon=$APP_ID
Categories=Office;
EOF
chmod +x "$DESKTOP_FILE"

# Refresh caches so the launcher picks up the new entry/icon immediately.
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "Installed binary:  $BIN_DEST"
echo "Installed icon:    $ICON_DEST"
echo "Installed launcher: $DESKTOP_FILE"
echo "Search for '$APP_NAME' in your app launcher to run it."
