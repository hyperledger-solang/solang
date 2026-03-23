// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{Address, FromVal, Val};

fn now_ts(runtime: &crate::SorobanEnv, addr: &Address) -> u64 {
    let out: Val = runtime.invoke_contract(addr, "now_ts", vec![]);
    FromVal::from_val(&runtime.env, &out)
}

#[test]
fn block_timestamp_tracks_ledger_across_u64_encodings() {
    let runtime = build_solidity(
        r#"contract timestamp_test {
            function now_ts() public view returns (uint64) {
                return block.timestamp;
            }
        }"#,
        |env| {
            env.env.ledger().set_timestamp(0);
        },
    );

    let addr = runtime.contracts.last().unwrap();
    // Cover both Soroban u64 representations: values below 2^56 use the small
    // immediate form, while values at/above 2^56 use the object form.
    let samples = [
        0_u64,
        42_u64,
        (1_u64 << 56) - 1,
        1_u64 << 56,
        (1_u64 << 60) + 1234,
        (1_u64 << 56) + 7,
    ];

    for ts in samples {
        runtime.env.ledger().set_timestamp(ts);
        assert_eq!(now_ts(&runtime, addr), ts, "timestamp mismatch for {ts}");
    }
}
