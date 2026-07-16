use crate::build_solidity;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{FromVal, Val};

#[test]
fn get_ledger_sequence() {
    let runtime = build_solidity(
        r#"
        contract LedgerSequence {
            function get_ledger_sequence() public view returns (uint64) {
                return block.number;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let samples: Vec<u64> = vec![
        1,
        99,
        2,
        33,
        13,
        9,
        15,
        0,
        10001,
        1_000_000_000,
        2 << 16,
        2 << 24,
    ];
    for number in samples {
        runtime.env.ledger().set_sequence_number(number as u32);
        let block_number_val: Val = runtime.invoke_contract(addr, "get_ledger_sequence", vec![]);
        let block_number_u64: u64 = FromVal::from_val(&runtime.env, &block_number_val);
        assert_eq!(number, block_number_u64);
    }
}
