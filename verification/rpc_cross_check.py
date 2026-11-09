"""Independent cross-check of the Substreams output against Solana JSON-RPC.

The Substreams module decodes raw instruction bytes in Rust. This script instead uses the RPC
node's own `jsonParsed` instruction decoder, applies the same inclusion rules, and compares the
two transfer multisets block by block. It is verification tooling only; extraction is the Rust module.

Usage:
    python3 verification/rpc_cross_check.py <rpc_blocks_dir> <substreams_output.jsonl> <report.md>

<rpc_blocks_dir> holds <slot>.json files from `getBlock` with encoding=jsonParsed,
maxSupportedTransactionVersion=0. <substreams_output.jsonl> is raw `substreams run -o jsonl` output.
"""
import json
import sys
from collections import Counter
from pathlib import Path

SYSTEM_PROGRAM = "11111111111111111111111111111111"
TOKEN_PROGRAMS = {"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"}


def all_instructions(transaction):
    """Top-level instructions, each followed by its inner instructions (same order as the module)."""
    inner_by_index = {group["index"]: group["instructions"] for group in transaction["meta"].get("innerInstructions") or []}
    for index, instruction in enumerate(transaction["transaction"]["message"]["instructions"]):
        yield instruction
        yield from inner_by_index.get(index, [])


def token_owners(transaction):
    """Token account -> owner. InitializeAccount* first (temporary accounts), token balances override."""
    account_keys = [key["pubkey"] for key in transaction["transaction"]["message"]["accountKeys"]]
    owners = {}
    for instruction in all_instructions(transaction):
        parsed = instruction.get("parsed")
        if (instruction.get("programId") in TOKEN_PROGRAMS and isinstance(parsed, dict)
                and str(parsed.get("type", "")).startswith("initializeAccount")):
            owners[parsed["info"]["account"]] = parsed["info"]["owner"]
    for balance in (transaction["meta"].get("preTokenBalances") or []) + (transaction["meta"].get("postTokenBalances") or []):
        if balance.get("owner"):
            owners[account_keys[balance["accountIndex"]]] = balance["owner"]
    return owners


def expected_transfers(transaction):
    owners = token_owners(transaction)
    for instruction in all_instructions(transaction):
        parsed = instruction.get("parsed")
        if not isinstance(parsed, dict):
            continue
        info, kind = parsed.get("info", {}), parsed.get("type")
        if instruction["programId"] == SYSTEM_PROGRAM and kind in ("transfer", "transferWithSeed"):
            yield info["source"], info["destination"], int(info["lamports"])
        elif instruction["programId"] in TOKEN_PROGRAMS and kind == "transfer":
            yield owners.get(info["source"], info["source"]), owners.get(info["destination"], info["destination"]), int(info["amount"])
        elif instruction["programId"] in TOKEN_PROGRAMS and kind == "transferChecked":
            yield owners.get(info["source"], info["source"]), owners.get(info["destination"], info["destination"]), int(info["tokenAmount"]["amount"])


def main(rpc_dir, transfers_path, report_path):
    # Raw `substreams run -o jsonl` lines. Protobuf JSON prints uint64 as a string and omits zero values.
    extracted_by_block = {}
    for line in Path(transfers_path).read_text().splitlines():
        record = json.loads(line)
        transfers = record["@data"].get("transfers", [])
        extracted_by_block[record["@block"]] = Counter((t["from"], t["to"], int(t.get("amount", "0"))) for t in transfers)

    rows, all_match = [], True
    for block_file in sorted(Path(rpc_dir).glob("*.json")):
        slot = int(block_file.stem)
        block = json.loads(block_file.read_text())["result"]
        transactions = block["transactions"]
        successful = [t for t in transactions if t["meta"]["err"] is None]
        expected = Counter(transfer for t in successful for transfer in expected_transfers(t))
        extracted = extracted_by_block.get(slot, Counter())
        missing, unexpected = expected - extracted, extracted - expected
        all_match &= not missing and not unexpected
        rows.append((slot, len(transactions), len(successful), len(transactions) - len(successful),
                     sum(expected.values()), sum(extracted.values()), sum(missing.values()), sum(unexpected.values())))
        for transfer in list(missing)[:5]:
            print(f"{slot} missing from Substreams: {transfer}")
        for transfer in list(unexpected)[:5]:
            print(f"{slot} not in RPC: {transfer}")

    header = ("| Block | Transactions examined | Successful | Failed (skipped) | Transfers per RPC | "
              "Transfers extracted | Missing | Unexpected |\n|---|---|---|---|---|---|---|---|\n")
    table = header + "".join("| " + " | ".join(map(str, row)) + " |\n" for row in rows)
    Path(report_path).write_text(table)
    print(table)
    print("RESULT:", "MATCH" if all_match else "MISMATCH")
    return 0 if all_match else 1


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:4]))
