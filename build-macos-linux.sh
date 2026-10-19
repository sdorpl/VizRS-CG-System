#!/bin/sh
# Budowanie ze źródeł (macOS / inne Linuksy). Wymaga Rust: https://rustup.rs
set -e
cd "$(dirname "$0")/source"
cargo build --release
mkdir -p ../bin/native && cp target/release/vizrs ../bin/native/vizrs
echo "Gotowe: bin/native/vizrs  ->  ./bin/native/vizrs --data ./vizrs-data"
