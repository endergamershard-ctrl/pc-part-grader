#!/usr/bin/env bash
# Package the Linux release binary + desktop entry + icon into a portable tarball.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="${1:-$ROOT/src-tauri/target/release/pc-part-grader}"
OUT_DIR="${2:-$ROOT/dist-release}"
ASSET_NAME="pc-part-grader-linux-x86_64.tar.gz"

if [[ ! -x "$BINARY" ]]; then
  echo "error: binary not found or not executable: $BINARY" >&2
  exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp "$BINARY" "$STAGE/pc-part-grader"
chmod 755 "$STAGE/pc-part-grader"
cp "$ROOT/packaging/pc-part-grader.desktop" "$STAGE/pc-part-grader.desktop"
cp "$ROOT/src-tauri/icons/128x128.png" "$STAGE/pc-part-grader.png"

mkdir -p "$OUT_DIR"
tar -czf "$OUT_DIR/$ASSET_NAME" -C "$STAGE" pc-part-grader pc-part-grader.desktop pc-part-grader.png
echo "Wrote $OUT_DIR/$ASSET_NAME"
