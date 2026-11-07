#[allow(unused)]
mod pb;

use pb::{mydata::v1 as mydata, sf::substreams::solana::v1::Transactions};
use substreams_solana::pb::sf::solana::r#type::v1::ConfirmedTransaction;

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

#[substreams::handlers::map]
fn map_my_data(transactions: Transactions) -> mydata::MyData {
    let mut output = mydata::MyData::default();

    for transaction in &transactions.transactions {
        if !transaction.is_successful() {
            continue;
        }
        extract_transaction_transfers(transaction, &mut output);
    }

    output
}

fn extract_transaction_transfers(
    transaction: &ConfirmedTransaction,
    output: &mut mydata::MyData,
) {
    for instruction in transaction.walk_instructions() {
        let program_id = instruction.program_id().to_string();
        let data = instruction.data();
        let accounts = instruction.accounts();

        if program_id == SYSTEM_PROGRAM {
            // SystemInstruction::Transfer is encoded as u32 discriminator 2 + u64 lamports.
            if data.len() >= 12
                && u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == 2
                && accounts.len() >= 2
            {
                let amount = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11],
                ]);
                output.transfers.push(mydata::Transfer {
                    from: accounts[0].to_string(),
                    to: accounts[1].to_string(),
                    amount,
                });
            }
        } else if program_id == TOKEN_PROGRAM || program_id == TOKEN_2022_PROGRAM {
            // SPL Token Transfer (3) and TransferChecked (12) both carry u64 amount
            // immediately after the one-byte instruction discriminator.
            if data.len() >= 9 && (data[0] == 3 || data[0] == 12) && accounts.len() >= 3 {
                let amount = u64::from_le_bytes([
                    data[1], data[2], data[3], data[4],
                    data[5], data[6], data[7], data[8],
                ]);
                // SPL transfer endpoints are token-account addresses, not owner wallets.
                let (source_index, destination_index) = if data[0] == 3 { (0, 1) } else { (0, 2) };
                if accounts.len() > destination_index {
                    output.transfers.push(mydata::Transfer {
                        from: accounts[source_index].to_string(),
                        to: accounts[destination_index].to_string(),
                        amount,
                    });
                }
            }
        }
    }
}
