#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <path-to-release-binary> [out-dir]"
  exit 1
fi

BINARY_PATH="$1"
OUT_DIR="${2:-target/rpm}"

mkdir -p "$OUT_DIR"

if [ ! -f "$BINARY_PATH" ]; then
  echo "Binary not found: $BINARY_PATH"
  exit 2
fi

# Try to extract version from Cargo.toml
VERSION="0.0.0"
if [ -f Cargo.toml ]; then
  VERSION_LINE=$(grep -E '^version\s*=\s*"' Cargo.toml | head -n1 || true)
  if [ -n "$VERSION_LINE" ]; then
    VERSION=$(echo "$VERSION_LINE" | cut -d'"' -f2)
  fi
fi

BASENAME=$(basename "$BINARY_PATH")

# Ensure binary is executable
chmod +x "$BINARY_PATH" || true

fpm -s dir -t rpm -n trushell -v "$VERSION" --prefix /usr/local/bin "$BINARY_PATH" -p "$OUT_DIR/trushell_${VERSION}_x86_64.rpm"

echo "RPM created in $OUT_DIR"
