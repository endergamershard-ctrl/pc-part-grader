#!/usr/bin/env bash
# Install the latest PC Part Grader release on Linux (x86_64).
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/install.sh | bash
set -euo pipefail

REPO="${PC_PART_GRADER_REPO:-endergamershard-ctrl/pc-part-grader}"
ASSET_NAME="pc-part-grader-linux-x86_64.tar.gz"
PREFIX="${PC_PART_GRADER_PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
ICON_DIR="$PREFIX/share/icons/hicolor/128x128/apps"
APP_DIR="$PREFIX/share/applications"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "error: this installer supports Linux only" >&2
  exit 1
fi

arch="$(uname -m)"
if [[ "$arch" != "x86_64" && "$arch" != "amd64" ]]; then
  echo "error: unsupported architecture: $arch (need x86_64)" >&2
  exit 1
fi

for cmd in curl tar; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: missing required command: $cmd" >&2
    exit 1
  fi
done

api="https://api.github.com/repos/${REPO}/releases/latest"
echo "Fetching latest release from ${REPO}..."
json="$(curl -fsSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: pc-part-grader-installer' "$api")"
asset_url="$(printf '%s\n' "$json" | grep -oE "https://[^\"]+/${ASSET_NAME}" | head -n1)"
tag="$(printf '%s\n' "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"

if [[ -z "$asset_url" ]]; then
  echo "error: could not find release asset ${ASSET_NAME}" >&2
  echo "Check https://github.com/${REPO}/releases" >&2
  exit 1
fi

echo "Downloading ${tag} (${ASSET_NAME})..."
curl -fL --progress-bar -o "$TMP_DIR/$ASSET_NAME" "$asset_url"
tar -xzf "$TMP_DIR/$ASSET_NAME" -C "$TMP_DIR"

if [[ ! -f "$TMP_DIR/pc-part-grader" ]]; then
  echo "error: archive did not contain pc-part-grader binary" >&2
  exit 1
fi

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APP_DIR"
install -m 755 "$TMP_DIR/pc-part-grader" "$BIN_DIR/pc-part-grader"

if [[ -f "$TMP_DIR/pc-part-grader.png" ]]; then
  install -m 644 "$TMP_DIR/pc-part-grader.png" "$ICON_DIR/pc-part-grader.png"
fi

if [[ -f "$TMP_DIR/pc-part-grader.desktop" ]]; then
  install -m 644 "$TMP_DIR/pc-part-grader.desktop" "$APP_DIR/pc-part-grader.desktop"
else
  cat > "$APP_DIR/pc-part-grader.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=PC Part Grader
Comment=Grade your PC parts with transparent benchmark scores
Exec=pc-part-grader
Icon=pc-part-grader
Terminal=false
Categories=Utility;System;
Keywords=benchmark;hardware;score;cpu;gpu;
StartupNotify=true
EOF
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo
    echo "Note: $BIN_DIR is not on your PATH."
    echo "Add this to your shell profile, then open a new terminal:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac

echo
echo "Installed PC Part Grader ${tag}"
echo "  Binary:  $BIN_DIR/pc-part-grader"
echo "  Desktop: $APP_DIR/pc-part-grader.desktop"
echo
echo "Launch from your app menu (Super+Space) or run: pc-part-grader"
echo "Uninstall: curl -fsSL https://raw.githubusercontent.com/${REPO}/master/scripts/uninstall.sh | bash"
