#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"
wasm-pack build crates/corewar-viz --target web --out-dir ../../wasm/pkg -- --features wasm
cp "$SCRIPT_DIR/index.html" "$SCRIPT_DIR/pkg/index.html"
echo "Built WASM package in $SCRIPT_DIR/pkg"
