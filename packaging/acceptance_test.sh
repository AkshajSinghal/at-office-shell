#!/usr/bin/env bash
set -euo pipefail

# Basic local acceptance script: run trushell --version and a simple command
if ! command -v trushell >/dev/null 2>&1; then
  echo "trushell is not installed or not in PATH"
  exit 2
fi

trushell --version || true

# Try a simple non-interactive command if CLI supports -c or similar
if trushell -c 'echo packaging-smoke' >/dev/null 2>&1; then
  echo "Smoke command succeeded"
else
  echo "Smoke command didn't run or returned non-zero exit"
fi
