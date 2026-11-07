# Solana Transfer Extraction

Substreams package for extracting successful Solana transfer instructions.

## Scope

The map module recognizes:
- Native SOL System Program `Transfer` instructions.
- SPL Token `Transfer` and `TransferChecked` instructions.
- Token-2022 `Transfer` and `TransferChecked` instructions.

`amount` is emitted in the raw smallest unit: lamports for native SOL and base units for SPL tokens. For SPL transfers, `from` and `to` are the source and destination token-account addresses, not the wallet owner addresses.

The configured filter includes the System, SPL Token, and Token-2022 programs. Failed transactions are skipped, and inner instructions are included.

## Build

Run from this directory with Substreams CLI and Buf configured:

```sh
substreams build
```

## Run the requested block range

Authenticate first if required by your Substreams endpoint, then run:

```sh
substreams run substreams.yaml map_my_data --start-block 385870151 --stop-block 385870156
```

The package build and block execution must be performed in an environment with the Substreams CLI, Rust WASM target, Buf, and access to the Solana Substreams endpoint.
