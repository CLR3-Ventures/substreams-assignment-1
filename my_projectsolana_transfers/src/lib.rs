#[allow(unused)]
mod pb;

use pb::{mydata::v1 as mydata, sf::substreams::solana::v1::Transactions};

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

fn add_system_transfer(
    program_id_index: u32,
    accounts: &[u8],
    data: &[u8],
    account_keys: &[&[u8]],
    transfers: &mut Vec<mydata::Transfer>,
) {
    let Some(program_id) = account_keys.get(program_id_index as usize) else {
        return;
    };
    if substreams_solana::base58::encode(program_id) != SYSTEM_PROGRAM || data.len() != 12 {
        return;
    }

    if u32::from_le_bytes(data[0..4].try_into().unwrap()) != 2 {
        return;
    }

    let (Some(from), Some(to)) = (
        accounts
            .first()
            .and_then(|index| account_keys.get(*index as usize)),
        accounts
            .get(1)
            .and_then(|index| account_keys.get(*index as usize)),
    ) else {
        return;
    };

    transfers.push(mydata::Transfer {
        from: substreams_solana::base58::encode(from),
        to: substreams_solana::base58::encode(to),
        amount: u64::from_le_bytes(data[4..12].try_into().unwrap()),
    });
}

#[substreams::handlers::map]
fn map_my_data(transactions: Transactions) -> mydata::MyData {
    let mut transfers = Vec::new();

    for tx in &transactions.transactions {
        let Some(meta) = tx.meta.as_option() else {
            continue;
        };
        if meta.err.is_set() {
            continue;
        }

        let Some(transaction) = tx.transaction.as_option() else {
            continue;
        };
        let Some(message) = transaction.message.as_option() else {
            continue;
        };

        let mut account_keys: Vec<&[u8]> = message.account_keys.iter().map(Vec::as_slice).collect();
        account_keys.extend(meta.loaded_writable_addresses.iter().map(Vec::as_slice));
        account_keys.extend(meta.loaded_readonly_addresses.iter().map(Vec::as_slice));

        for (index, instruction) in message.instructions.iter().enumerate() {
            add_system_transfer(
                instruction.program_id_index,
                &instruction.accounts,
                &instruction.data,
                &account_keys,
                &mut transfers,
            );

            for inner_group in meta
                .inner_instructions
                .iter()
                .filter(|group| group.index as usize == index)
            {
                for instruction in &inner_group.instructions {
                    add_system_transfer(
                        instruction.program_id_index,
                        &instruction.accounts,
                        &instruction.data,
                        &account_keys,
                        &mut transfers,
                    );
                }
            }
        }
    }

    mydata::MyData { transfers }
}
