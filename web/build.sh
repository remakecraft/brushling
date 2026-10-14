#!/usr/bin/env bash
# Build the browser version into web/dist (index.html, brushling.js,
# brushling_bg.wasm): a static folder that works from any sub-path.
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen in Cargo.lock>
set -euo pipefail
cd "$(dirname "$0")/.."

want=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
have=$(wasm-bindgen --version | awk '{print $2}')
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen-cli $have does not match wasm-bindgen $want in Cargo.lock:" >&2
  echo "  cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

cargo build --profile web --target wasm32-unknown-unknown --bin brushling
rm -rf web/dist
mkdir -p web/dist
wasm-bindgen --target web --no-typescript --out-dir web/dist target/wasm32-unknown-unknown/web/brushling.wasm
cp web/index.html web/dist/
ls -l web/dist
