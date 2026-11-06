# solana-transfers (Substreams)

Extracts System Program `Transfer` instructions (top-level + inner, successful txs only)
for Solana blocks 385870151-385870156.

## Prereqs
    rustup target add wasm32-unknown-unknown
    # substreams CLI: https://docs.substreams.dev/ (brew install streamingfast/tap/substreams, or GitHub release)
    # API key (free): https://thegraph.market  -> then:
    export SUBSTREAMS_API_TOKEN=$(curl -s https://auth.pinax.network/v1/auth/issue -d '{"api_key":"<KEY>"}' | jq -r .token)   # or per provider docs

## Build / pack / run
    cargo build --release --target wasm32-unknown-unknown
    substreams pack substreams.yaml            # -> solana-transfers-v0.1.0.spkg (name may vary)
    substreams run -e solana.substreams.pinax.network:443 solana_transfers-v0.1.0.spkg map_transfers \
      --start-block 385870151 --stop-block 385870157 -o jsonl
