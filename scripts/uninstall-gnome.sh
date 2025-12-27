#!/usr/bin/env bash
# Remove GNOME launcher/icon for TrackPersonalInsights.
set -euo pipefail

APP_ID="trackinsights"
DESKTOP_FILE="$HOME/.local/share/applications/${APP_ID}.desktop"
ICON_FILE="$HOME/.local/share/icons/trackinsights.svg"

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

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" || true
fi

if [[ "$removed_any" == false ]]; then
  echo "Nothing to remove; launcher/icon not found."
else
  echo "Uninstall complete."
fi
