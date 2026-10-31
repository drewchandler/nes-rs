#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --lib --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/nes_rs.wasm web/index.html web/app.js dist/
touch dist/.nojekyll
