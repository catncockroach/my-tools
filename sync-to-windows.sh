#!/usr/bin/env bash
# Copy các extension sang ổ Windows để Chrome "Load unpacked".
# Dùng: ./sync-to-windows.sh            -> copy tất cả extension
#       ./sync-to-windows.sh youtube-tools -> chỉ copy 1 extension
set -euo pipefail

DEST="${WIN_EXT_DIR:-/mnt/c/Users/haihv/Downloads/chrome-extension}"
ROOT="$(cd "$(dirname "$0")" && pwd)"

if [ $# -gt 0 ]; then
  names=("$@")
else
  names=()
  for m in "$ROOT"/*/manifest.json; do names+=("$(basename "$(dirname "$m")")"); done
fi

mkdir -p "$DEST"
for name in "${names[@]}"; do
  [ -f "$ROOT/$name/manifest.json" ] || { echo "Không tìm thấy extension: $name" >&2; exit 1; }
  rm -rf "${DEST:?}/$name"
  cp -r "$ROOT/$name" "$DEST/$name"
  echo "Đã copy $name -> $DEST/$name"
done
