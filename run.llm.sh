#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_BIN="$HOME/.cargo/bin"
export PATH="$CARGO_BIN:$PATH"

echo "=== 1. Running rust-analyzer analysis-stats ==="
cd "$SCRIPT_DIR/src-tauri"
if command -v rust-analyzer &> /dev/null; then
    rust-analyzer analysis-stats .
elif [ -f "$CARGO_BIN/rust-analyzer.exe" ]; then
    "$CARGO_BIN/rust-analyzer.exe" analysis-stats .
elif [ -f "$CARGO_BIN/rust-analyzer" ]; then
    "$CARGO_BIN/rust-analyzer" analysis-stats .
fi

echo "=== 2. Running cargo check ==="
cargo check

echo "=== 3. Running cargo test ==="
cargo test

echo "=== 4. Running frontend build ==="
cd "$SCRIPT_DIR"
pnpm build

echo "=== All validation checks passed successfully! ==="
