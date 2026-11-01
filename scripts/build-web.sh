#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --lib --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/nes_rs.wasm web/index.html web/app.js web/*.mjs web/credits.html dist/
cp -R web/roms dist/
touch dist/.nojekyll
