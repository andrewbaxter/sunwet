#!/usr/bin/env bash
set -xeu -o pipefail

cd "$(dirname "$0")"

TARGET_DIR="${CARGO_TARGET_DIR:-/mnt/home-dev/.cargo_target}"
WASM_TARGET="wasm32-unknown-unknown"
BUILD_DIR="build"

# Build browser wasm binaries
cargo build --manifest-path Cargo.toml --target "$WASM_TARGET" --release

# Generate JS bindings with wasm-bindgen
mkdir -p "$BUILD_DIR"
wasm-bindgen --target web --out-dir "$BUILD_DIR" --out-name content2 \
  "$TARGET_DIR/$WASM_TARGET/release/browser-content.wasm"
wasm-bindgen --target web --out-dir "$BUILD_DIR" --out-name options2 \
  "$TARGET_DIR/$WASM_TARGET/release/browser-options.wasm"

# Copy static extension files
cp ext_static/* "$BUILD_DIR/"
cp ../wasm/prestatic/big-icon.svg "$BUILD_DIR/"
cp browser_manifest.json "$BUILD_DIR/manifest.json"

# TypeScript type check JS files
# Note: in Nix environments pkgs.nodejs and pkgs.typescript guarantee node/tsc.
if command -v tsc >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
  echo "Running TypeScript type checks..."
  (cd "$BUILD_DIR" && tsc --noEmit)
else
  echo "Warning: tsc or node not found, skipping TypeScript type checks"
fi

echo "Browser extension build complete in ./$BUILD_DIR/"
