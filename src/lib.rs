//! Substreams map module that extracts native SOL and SPL token transfers from Solana blocks.
//!
//! Covered instructions (top-level and inner/CPI):
//! - System Program `Transfer` and `TransferWithSeed` (amount in lamports)
//! - SPL Token / Token-2022 `Transfer` and `TransferChecked` (amount in raw token base units)
//!
//! Only successful transactions are processed. SPL `from`/`to` are the owner wallets of the
//! source/destination token accounts (resolved from the transaction's token balances, or from the
//! `InitializeAccount` instruction for accounts created and closed within the transaction).
mod pb;

use std::collections::HashMap;

use pb::transfers::v1::{Transfer, Transfers};
use substreams::errors::Error;
use substreams_solana::{b58, base58, pb::sf::solana::r#type::v1 as solana};

const SYSTEM_PROGRAM_ID: [u8; 32] = b58!("11111111111111111111111111111111");
const TOKEN_PROGRAM_ID: [u8; 32] = b58!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM_ID: [u8; 32] = b58!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

// System Program data is bincode: u32 little-endian discriminator, then the payload.
const SYSTEM_TRANSFER: u32 = 2;
const SYSTEM_TRANSFER_WITH_SEED: u32 = 11;

// SPL Token data: u8 tag, then the payload. Token-2022 uses the same tags.
const TOKEN_TRANSFER: u8 = 3;
const TOKEN_TRANSFER_CHECKED: u8 = 12;
const TOKEN_INITIALIZE_ACCOUNT: u8 = 1;
const TOKEN_INITIALIZE_ACCOUNT_2: u8 = 16;
const TOKEN_INITIALIZE_ACCOUNT_3: u8 = 18;

#[substreams::handlers::map]
fn map_transfers(block: solana::Block) -> Result<Transfers, Error> {
    Ok(Transfers {
        transfers: block_transfers(&block),
    })
}

/// `Block::transactions()` yields only successful transactions (meta present and `meta.err` empty).
fn block_transfers(block: &solana::Block) -> Vec<Transfer> {
    block.transactions().flat_map(transaction_transfers).collect()
}

/// Transfers of one transaction in execution order: each top-level instruction, then its inner
/// instructions. Every instruction is visited exactly once, so nothing is emitted twice.
fn transaction_transfers(transaction: &solana::ConfirmedTransaction) -> Vec<Transfer> {
    let message = transaction.transaction.as_ref().and_then(|t| t.message.as_ref());
    let (Some(meta), Some(message)) = (transaction.meta.as_ref(), message) else {
        return Vec::new();
    };

    // Instruction account indexes point into this combined list: static message keys, then
    // address-lookup-table writable addresses, then lookup-table readonly addresses.
    let account_keys: Vec<&Vec<u8>> = message
        .account_keys
        .iter()
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .collect();
    let instructions: Vec<RawInstruction> = message
        .instructions
        .iter()
        .enumerate()
        .flat_map(|(top_level_index, top_level)| {
            let inner = meta
                .inner_instructions
                .iter()
                .filter(move |group| group.index as usize == top_level_index)
                .flat_map(|group| &group.instructions)
                .map(|inner| (inner.program_id_index, inner.accounts.as_slice(), inner.data.as_slice()));
            std::iter::once((top_level.program_id_index, top_level.accounts.as_slice(), top_level.data.as_slice()))
                .chain(inner)
        })
        .collect();
    let owners_by_account_index = token_account_owners(meta, &instructions, &account_keys);

    instructions
        .iter()
        .filter_map(|&(program_id_index, account_indexes, data)| {
            decode_transfer(program_id_index, account_indexes, data, &account_keys, &owners_by_account_index)
        })
        .collect()
}

/// (program id index, account indexes, data) of a top-level or inner instruction.
type RawInstruction<'a> = (u32, &'a [u8], &'a [u8]);

/// Token account index -> owner wallet (base58).
///
/// Primary source: pre/post token balances (pre covers accounts closed in the transaction, post
/// covers accounts created in it). Accounts both created and closed within the same transaction
/// (e.g. temporary wrapped-SOL accounts) appear in neither, so their owner is taken from the
/// SPL `InitializeAccount` instruction that created them.
fn token_account_owners(
    meta: &solana::TransactionStatusMeta,
    instructions: &[RawInstruction],
    account_keys: &[&Vec<u8>],
) -> HashMap<u32, String> {
    let mut owners: HashMap<u32, String> = instructions
        .iter()
        .filter_map(|&(program_id_index, account_indexes, data)| {
            initialized_token_account_owner(program_id_index, account_indexes, data, account_keys)
        })
        .collect();
    owners.extend(
        meta.pre_token_balances
            .iter()
            .chain(&meta.post_token_balances)
            .filter(|balance| !balance.owner.is_empty())
            .map(|balance| (balance.account_index, balance.owner.clone())),
    );
    owners
}

/// Decodes SPL `InitializeAccount` variants into (token account index, owner wallet).
fn initialized_token_account_owner(
    program_id_index: u32,
    account_indexes: &[u8],
    data: &[u8],
    account_keys: &[&Vec<u8>],
) -> Option<(u32, String)> {
    let program_id = account_keys.get(program_id_index as usize)?.as_slice();
    if program_id != TOKEN_PROGRAM_ID && program_id != TOKEN_2022_PROGRAM_ID {
        return None;
    }
    let token_account_index = *account_indexes.first()?;
    let owner = match *data.first()? {
        // InitializeAccount: accounts [account, mint, owner, rent sysvar], no payload.
        TOKEN_INITIALIZE_ACCOUNT => base58::encode(account_keys.get(*account_indexes.get(2)? as usize)?),
        // InitializeAccount2/3: owner pubkey is the 32-byte payload after the tag.
        TOKEN_INITIALIZE_ACCOUNT_2 | TOKEN_INITIALIZE_ACCOUNT_3 => base58::encode(data.get(1..33)?),
        _ => return None,
    };
    Some((token_account_index as u32, owner))
}

/// Decodes one instruction into a transfer. Returns `None` for unrelated programs/instructions
/// and for malformed ones (short data, missing accounts, out-of-range account indexes).
fn decode_transfer(
    program_id_index: u32,
    account_indexes: &[u8],
    data: &[u8],
    account_keys: &[&Vec<u8>],
    owners_by_account_index: &HashMap<u32, String>,
) -> Option<Transfer> {
    let program_id = account_keys.get(program_id_index as usize)?.as_slice();
    let address_at = |position: usize| -> Option<String> {
        let account_index = *account_indexes.get(position)?;
        account_keys.get(account_index as usize).map(base58::encode)
    };

    if program_id == SYSTEM_PROGRAM_ID {
        // Layout: [discriminator u32][lamports u64][...]. Trailing bytes are tolerated, as the runtime does.
        let (from_position, to_position) = match read_u32_le(data, 0)? {
            SYSTEM_TRANSFER => (0, 1),           // accounts: [from, to]
            SYSTEM_TRANSFER_WITH_SEED => (0, 2), // accounts: [from (seed-derived), base, to]
            _ => return None,
        };
        return Some(Transfer {
            from: address_at(from_position)?,
            to: address_at(to_position)?,
            amount: read_u64_le(data, 4)?,
        });
    }

    if program_id == TOKEN_PROGRAM_ID || program_id == TOKEN_2022_PROGRAM_ID {
        // Layout: [tag u8][amount u64] (+ [decimals u8] for TransferChecked).
        let (source_position, destination_position) = match *data.first()? {
            TOKEN_TRANSFER => (0, 1), // accounts: [source, destination, authority, ...signers]
            TOKEN_TRANSFER_CHECKED if data.len() >= 10 => (0, 2), // [source, mint, destination, authority, ...]
            _ => return None,
        };
        // Report the owner wallet; fall back to the token account address if the owner is unknown.
        let wallet_at = |position: usize| -> Option<String> {
            let account_index = *account_indexes.get(position)?;
            match owners_by_account_index.get(&(account_index as u32)) {
                Some(owner) => Some(owner.clone()),
                None => address_at(position),
            }
        };
        return Some(Transfer {
            from: wallet_at(source_position)?,
            to: wallet_at(destination_position)?,
            amount: read_u64_le(data, 1)?,
        });
    }

    None
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)?.try_into().ok().map(u32::from_le_bytes)
}

fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8)?.try_into().ok().map(u64::from_le_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    fn system_transfer_data(lamports: u64) -> Vec<u8> {
        [SYSTEM_TRANSFER.to_le_bytes().as_slice(), &lamports.to_le_bytes()].concat()
    }

    fn token_balance(account_index: u32, owner: &str) -> solana::TokenBalance {
        solana::TokenBalance {
            account_index,
            owner: owner.to_string(),
            ..Default::default()
        }
    }

    /// Keys: 0 payer, 1 recipient, 2 system program, 3 token program, 4 source ATA, 5 mint,
    /// lookup-table writable: 6 destination ATA.
    fn transaction(
        instructions: Vec<solana::CompiledInstruction>,
        inner: Vec<solana::InnerInstructions>,
        failed: bool,
    ) -> solana::ConfirmedTransaction {
        solana::ConfirmedTransaction {
            transaction: Some(solana::Transaction {
                message: Some(solana::Message {
                    account_keys: vec![key(1), key(2), SYSTEM_PROGRAM_ID.to_vec(), TOKEN_PROGRAM_ID.to_vec(), key(4), key(5)],
                    instructions,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            meta: Some(solana::TransactionStatusMeta {
                err: failed.then(solana::TransactionError::default),
                inner_instructions: inner,
                loaded_writable_addresses: vec![key(6)],
                pre_token_balances: vec![token_balance(4, "SourceOwner")],
                post_token_balances: vec![token_balance(6, "DestinationOwner")],
                ..Default::default()
            }),
        }
    }

    fn instruction(program_id_index: u32, accounts: Vec<u8>, data: Vec<u8>) -> solana::CompiledInstruction {
        solana::CompiledInstruction { program_id_index, accounts, data }
    }

    #[test]
    fn extracts_system_and_token_transfers_including_inner_and_lookup_accounts() {
        let token_transfer_checked = [vec![TOKEN_TRANSFER_CHECKED], 7_000u64.to_le_bytes().to_vec(), vec![6]].concat();
        let inner = solana::InnerInstructions {
            index: 0,
            instructions: vec![solana::InnerInstruction {
                program_id_index: 3,
                accounts: vec![4, 5, 6, 0],
                data: token_transfer_checked,
                stack_height: Some(2),
            }],
        };
        let block = solana::Block {
            transactions: vec![
                transaction(vec![instruction(2, vec![0, 1], system_transfer_data(325_253))], vec![inner], false),
                transaction(vec![instruction(2, vec![0, 1], system_transfer_data(999))], vec![], true),
            ],
            ..Default::default()
        };

        let transfers = block_transfers(&block);

        assert_eq!(transfers.len(), 2, "failed transaction must be skipped");
        assert_eq!(transfers[0].from, base58::encode(key(1)));
        assert_eq!(transfers[0].to, base58::encode(key(2)));
        assert_eq!(transfers[0].amount, 325_253);
        assert_eq!(transfers[1].from, "SourceOwner");
        assert_eq!(transfers[1].to, "DestinationOwner");
        assert_eq!(transfers[1].amount, 7_000);
    }

    #[test]
    fn resolves_owner_of_token_account_created_and_closed_in_same_transaction() {
        // Account 1 is initialized via InitializeAccount3 (owner in payload) and has no token balance entry.
        let initialize = [vec![TOKEN_INITIALIZE_ACCOUNT_3], key(7)].concat();
        let transfer = [vec![TOKEN_TRANSFER], 42u64.to_le_bytes().to_vec()].concat();
        let transaction = solana::ConfirmedTransaction {
            transaction: Some(solana::Transaction {
                message: Some(solana::Message {
                    account_keys: vec![TOKEN_PROGRAM_ID.to_vec(), key(8), key(5), key(9)],
                    instructions: vec![instruction(0, vec![1, 2], initialize), instruction(0, vec![1, 3, 1], transfer)],
                    ..Default::default()
                }),
                ..Default::default()
            }),
            meta: Some(solana::TransactionStatusMeta::default()),
        };

        let transfers = transaction_transfers(&transaction);

        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].from, base58::encode(key(7)));
        assert_eq!(transfers[0].to, base58::encode(key(9)), "unknown owner falls back to token account");
        assert_eq!(transfers[0].amount, 42);
    }

    #[test]
    fn token_transfer_falls_back_to_token_account_without_owner() {
        let data = [vec![TOKEN_TRANSFER], 5u64.to_le_bytes().to_vec()].concat();
        let (token_program, source, destination) = (TOKEN_PROGRAM_ID.to_vec(), key(8), key(9));
        let keys: Vec<&Vec<u8>> = vec![&token_program, &source, &destination];

        let transfer = decode_transfer(0, &[1, 2, 1], &data, &keys, &HashMap::new()).unwrap();

        assert_eq!(transfer.from, base58::encode(key(8)));
        assert_eq!(transfer.to, base58::encode(key(9)));
        assert_eq!(transfer.amount, 5);
    }

    #[test]
    fn ignores_malformed_and_unrelated_instructions() {
        let system_program = SYSTEM_PROGRAM_ID.to_vec();
        let token_program = TOKEN_PROGRAM_ID.to_vec();
        let wallet = key(1);
        let keys: Vec<&Vec<u8>> = vec![&system_program, &token_program, &wallet, &wallet];
        let owners = HashMap::new();

        // Truncated amount.
        assert!(decode_transfer(0, &[2, 3], &system_transfer_data(1)[..8], &keys, &owners).is_none());
        // Account index out of range.
        assert!(decode_transfer(0, &[2, 42], &system_transfer_data(1), &keys, &owners).is_none());
        // Missing destination account.
        assert!(decode_transfer(0, &[2], &system_transfer_data(1), &keys, &owners).is_none());
        // System CreateAccount (discriminator 0) is not a transfer.
        assert!(decode_transfer(0, &[2, 3], &[0u8; 52], &keys, &owners).is_none());
        // SPL MintTo (tag 7) is not a transfer.
        assert!(decode_transfer(1, &[2, 3, 2], &[7, 1, 0, 0, 0, 0, 0, 0, 0], &keys, &owners).is_none());
        // TransferChecked without decimals byte.
        assert!(decode_transfer(1, &[2, 3, 3, 2], &[12, 1, 0, 0, 0, 0, 0, 0, 0], &keys, &owners).is_none());
        // Program id index out of range.
        assert!(decode_transfer(99, &[2, 3], &system_transfer_data(1), &keys, &owners).is_none());
    }
}
