#!/usr/bin/env bash
set -euo pipefail

BIN="${1:-$(pwd)/target/release/hyprshot-rs}"
OUT_DIR="${2:-$(pwd)}"

if [[ ! -x "$BIN" ]]; then
  echo "Binary not found or not executable: $BIN"
  exit 1
fi

echo "Using binary: $BIN"
echo "Output dir:   $OUT_DIR"
echo

echo "1) Capture output (select output if needed)"
"$BIN" -m output
echo "OK"
echo

echo "2) Capture active output (under cursor)"
"$BIN" -m output -m active
echo "OK"
echo

echo "3) Capture active window"
"$BIN" -m window -m active
echo "OK"
echo

echo "4) Capture output by name"
read -r -p "Enter output name (example: DP-1): " OUTPUT_NAME
"$BIN" -m output -m "$OUTPUT_NAME"
echo "OK"
echo

echo "5) Capture region (raw -> file)"
OUT_FILE="$OUT_DIR/output.png"
rm -f "$OUT_FILE"
"$BIN" -m region -r >"$OUT_FILE"
if [[ ! -s "$OUT_FILE" ]]; then
  echo "Region capture failed: $OUT_FILE is empty"
  exit 1
fi
echo "OK: $OUT_FILE"
echo

echo "6) Two quick captures (filename collision check)"
"$BIN" -m region >/dev/null
"$BIN" -m region >/dev/null
echo "OK"
echo

echo "7) Clipboard/notify (best-effort) sanity"
echo "Check for notification and clipboard content manually."
"$BIN" -m region
echo

echo "All manual Wayland tests completed."
