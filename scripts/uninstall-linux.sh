#!/usr/bin/env bash
# Remove the TrackPersonalInsights launcher, icon, and installed binary.
set -euo pipefail

APP_NAME="TrackPersonalInsights"
APP_ID="trackinsights"
BIN_FILE="$HOME/.local/bin/$APP_NAME"
ICON_FILE="$HOME/.local/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
DESKTOP_FILE="$HOME/.local/share/applications/${APP_ID}.desktop"

removed_any=false

if [[ -f "$DESKTOP_FILE" ]]; then
  rm "$DESKTOP_FILE"
  echo "Removed desktop entry: $DESKTOP_FILE"
  removed_any=true
fi

if [[ -f "$ICON_FILE" ]]; then
  rm "$ICON_FILE"
  echo "Removed icon: $ICON_FILE"
  removed_any=true
fi

if [[ -f "$BIN_FILE" ]]; then
  rm "$BIN_FILE"
  echo "Removed binary: $BIN_FILE"
  removed_any=true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

if [[ "$removed_any" == false ]]; then
  echo "Nothing to remove; launcher/icon/binary not found."
else
  echo "Uninstall complete."
fi
