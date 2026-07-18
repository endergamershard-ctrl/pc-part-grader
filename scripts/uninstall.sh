#!/usr/bin/env bash
# Remove a user-local PC Part Grader install created by scripts/install.sh.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/uninstall.sh | bash
set -euo pipefail

PREFIX="${PC_PART_GRADER_PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin/pc-part-grader"
ICON="$PREFIX/share/icons/hicolor/128x128/apps/pc-part-grader.png"
DESKTOP="$PREFIX/share/applications/pc-part-grader.desktop"

removed=0
for path in "$BIN" "$ICON" "$DESKTOP"; do
  if [[ -e "$path" ]]; then
    rm -f "$path"
    echo "Removed $path"
    removed=1
  fi
done

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
fi

if [[ "$removed" -eq 0 ]]; then
  echo "No PC Part Grader install found under $PREFIX"
else
  echo "PC Part Grader uninstalled."
fi
