#!/bin/bash
# Build the SPKG, stream Solana blocks 385870151..385870156 (inclusive), and write JSONL output.
#
# Requires: Rust (rustup), substreams CLI, buf, jq, and a Substreams API key in .substreams.env
# (created by `substreams auth`, gitignored). Run from anywhere: paths are resolved from the repo root.
set -euo pipefail

repository_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repository_root"

start_block=385870151
last_block=385870156
stop_block=$((last_block + 1)) # `substreams run --stop-block` is exclusive

package_file="solana-transfers-v0.1.0.spkg"
raw_output_file="artifacts/substreams_output.jsonl"
transfers_file="artifacts/transfers.jsonl"

if [ ! -f .substreams.env ]; then
  echo "Missing .substreams.env: run 'substreams auth' (or 'substreams auth --paste') in $repository_root" >&2
  exit 1
fi
# shellcheck disable=SC1091
source .substreams.env

mkdir -p artifacts
substreams build
cp "$package_file" "artifacts/$package_file"

substreams run "artifacts/$package_file" map_transfers \
  --start-block "$start_block" --stop-block "$stop_block" --output jsonl > "$raw_output_file"

# One line per transfer with exactly {from, to, amount}. Protobuf JSON encodes uint64 as a string
# and omits zero values, so default to "0" and unquote textually (no float conversion, no precision loss).
jq -c '.["@data"].transfers[]? | {from, to, amount: (.amount // "0")}' "$raw_output_file" \
  | sed -E 's/"amount":"([0-9]+)"/"amount":\1/' > "$transfers_file"

echo "Blocks streamed: $(wc -l < "$raw_output_file" | tr -d ' ')  Transfers: $(wc -l < "$transfers_file" | tr -d ' ')"
echo "Output: $repository_root/$transfers_file"
