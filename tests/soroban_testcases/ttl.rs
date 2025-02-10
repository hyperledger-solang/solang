// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::storage::Persistent;
use soroban_sdk::testutils::Ledger;

/// This test is adapted from
/// [Stellar Soroban Examples](https://github.com/stellar/soroban-examples/blob/f595fb5df06058ec0b9b829e9e4d0fe0513e0aa8/ttl).
///
/// It shows testing the TTL extension for persistent storage keys using the `extendPersistentTTL` built-in function
#[test]
fn ttl_basic() {
    let runtime = build_solidity(
        r#"contract counter {
            /// Variable to track the count. Stored in persistent storage
            uint64 public persistent count = 11;

            /// Extends the TTL for the `count` persistent key to 5000 ledgers
            /// if the current TTL is smaller than 1000 ledgers
            function extend_ttl() public view returns (int64) {
                return count.extendPersistentTtl(1000, 5000);
            }
        }"#,
        |env| {
            env.env.ledger().with_mut(|li| {
                // Current ledger sequence - the TTL is the number of
                // ledgers from the `sequence_number` (exclusive) until
                // the last ledger sequence where entry is still considered
                // alive.
                li.sequence_number = 100_000;
                // Minimum TTL for persistent entries - new persistent (and instance)
                // entries will have this TTL when created.
                li.min_persistent_entry_ttl = 500;
            });
        },
    );

    let addr = runtime.contracts.last().unwrap();

    // initial TTL
    runtime.env.as_contract(addr, || {
        // There is only one key in the persistent storage
        let key = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&key), 499);
    });

    // Extend persistent entry TTL to 5000 ledgers - now it is 5000.
    runtime.invoke_contract(addr, "extend_ttl", vec![]);

    runtime.env.as_contract(addr, || {
        // There is only one key in the persistent storage
        let key = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&key), 5000);
    });
}
