#!/bin/bash
# Build WASM modules for 021-basic-wasm-faas
# Requires: rustup target add wasm32-unknown-unknown wasm32-wasip1
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$SCRIPT_DIR/src"
OUT="$SCRIPT_DIR/out"
mkdir -p "$OUT"

echo "=== Building WASM modules for 021-basic-wasm-faas ==="

# pricing + validation: pure functions (call_i32), no WASI needed
for module in pricing validation; do
    echo -n "  Building ${module}.wasm (call_i32)... "
    rustc --target wasm32-unknown-unknown -O \
        --crate-type cdylib \
        "$SRC/${module}.rs" \
        -o "$OUT/${module}.wasm" 2>&1
    size=$(wc -c < "$OUT/${module}.wasm")
    echo "OK (${size} bytes)"
done

# transform: WASI binary (stdin/stdout), needs wasm32-wasip1
echo -n "  Building transform.wasm (WASI stdin/stdout)... "
rustc --target wasm32-wasip1 --edition 2021 -O \
    "$SRC/transform.rs" \
    -o "$OUT/transform.wasm" 2>&1
size=$(wc -c < "$OUT/transform.wasm")
echo "OK (${size} bytes)"

echo ""
echo "All WASM modules built in: $OUT/"
ls -lh "$OUT/"*.wasm
