#!/usr/bin/bash -xeu
export STATIC_DIR=/tmp/sunwet_tmp_static_dir
mkdir -p $STATIC_DIR
rm -f generated/jsonschema/*.json
cd native
cargo run --bin generate_jsonschema