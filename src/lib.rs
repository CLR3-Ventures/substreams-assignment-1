mod pb;

use pb::{Transfer, Transfers};
use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::Block;

/// System Program id: 11111111111111111111111111111111
const SYSTEM_PROGRAM: [u8; 32] = [0u8; 32];
/// System Program instruction index for `Transfer`.
const IX_TRANSFER: u32 = 2;

fn parse_transfer(accounts: &[String], program_idx: u32, ix_accounts: &[u8], data: &[u8]) -> Option<Transfer> {
    let program = bs58::decode(accounts.get(program_idx as usize)?).into_vec().ok()?;
    if program.as_slice() != SYSTEM_PROGRAM {
        return None;
    }
    // Layout: u32 LE discriminator (2 = Transfer) + u64 LE lamports
    if data.len() < 12 || u32::from_le_bytes(data[0..4].try_into().ok()?) != IX_TRANSFER {
        return None;
    }
    let amount = u64::from_le_bytes(data[4..12].try_into().ok()?);
    let from = accounts.get(*ix_accounts.first()? as usize)?.clone();
    let to = accounts.get(*ix_accounts.get(1)? as usize)?.clone();
    Some(Transfer { from, to, amount })
}

#[substreams::handlers::map]
fn map_transfers(block: Block) -> Result<Transfers, Error> {
    let mut transfers = Vec::new();

    for tx in block.transactions {
        // Skip failed transactions
        if tx.meta.as_ref().map_or(true, |m| m.err.is_some()) {
            continue;
        }
        let Some(message) = tx.transaction.as_ref().and_then(|t| t.message.as_ref()) else { continue };
        let meta = tx.meta.as_ref().unwrap();

        // Full account list = static keys + address-table-loaded (v0 txs)
        let accounts: Vec<String> = message
            .account_keys
            .iter()
            .chain(meta.loaded_writable_addresses.iter())
            .chain(meta.loaded_readonly_addresses.iter())
            .map(|k| bs58::encode(k).into_string())
            .collect();

        for ix in &message.instructions {
            if let Some(t) = parse_transfer(&accounts, ix.program_id_index, &ix.accounts, &ix.data) {
                transfers.push(t);
            }
        }
        for inner in &meta.inner_instructions {
            for ix in &inner.instructions {
                if let Some(t) = parse_transfer(&accounts, ix.program_id_index, &ix.accounts, &ix.data) {
                    transfers.push(t);
                }
            }
        }
    }

    Ok(Transfers { transfers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_system_transfer() {
        let from = "Eq1c3uNugn3FRpJ99K2q1ZyQrora3jntUqvkMnGroKkB".to_string();
        let to = "4XpSNfoNfireSgdg8hY6Ng3dWrHjewZhmCancypjbcCN".to_string();
        let sys = "11111111111111111111111111111111".to_string();
        let accounts = vec![from.clone(), to.clone(), sys];
        let mut data = 2u32.to_le_bytes().to_vec();
        data.extend_from_slice(&325253u64.to_le_bytes());
        let t = parse_transfer(&accounts, 2, &[0, 1], &data).unwrap();
        assert_eq!((t.from, t.to, t.amount), (from, to, 325253));
    }

    #[test]
    fn ignores_other_programs_and_instructions() {
        let accounts = vec!["Eq1c3uNugn3FRpJ99K2q1ZyQrora3jntUqvkMnGroKkB".to_string(); 3];
        let mut data = 2u32.to_le_bytes().to_vec();
        data.extend_from_slice(&1u64.to_le_bytes());
        assert!(parse_transfer(&accounts, 2, &[0, 1], &data).is_none());
    }
}
