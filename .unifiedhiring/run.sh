#!/bin/bash
# Build, pack and run the Solana transfers Substreams for blocks 385870151-385870156.
# Needs: rust + wasm target, substreams CLI, and SUBSTREAMS_API_TOKEN set (see README.md).
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown
cargo test
cargo build --release --target wasm32-unknown-unknown
substreams pack substreams.yaml -o solana-transfers.spkg

# stop-block is exclusive, so 385870157 covers block 385870156
substreams run -e solana.substreams.pinax.network:443 solana-transfers.spkg map_transfers \
  --start-block 385870151 --stop-block 385870157 -o jsonl
