#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
cd "`dirname $0`"
cargo build --workspace --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/*.wasm ./res/
