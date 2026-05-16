#!/usr/bin/env bash
set -xeu -o pipefail
export STATIC_DIR=/tmp/sunwet_tmp_static_dir
mkdir -p $STATIC_DIR
rm -f generated/jsonschema/*.json
rm -f generated/ts/sub/*.ts
(cd native; cargo run --bin generate_jsonschema)
(cd native; TS_RS_EXPORT_DIR=../generated/ts/sub cargo test export_bindings)

# Generate browser extension config (form/view IDs + parameter types)
sunwet_bin=$(cd native; cargo build --bin sunwet --message-format json 2>/dev/null | jq -rs '[.[] | select(.executable != null) | .executable] | last')
PATH="$(dirname "$sunwet_bin"):$PATH" npx tsx generate_extension_config.ts
