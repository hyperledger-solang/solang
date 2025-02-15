// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::storage::{Instance, Persistent, Temporary};
use soroban_sdk::testutils::Ledger;

/// This test is adapted from
/// [Stellar Soroban Examples](https://github.com/stellar/soroban-examples/blob/f595fb5df06058ec0b9b829e9e4d0fe0513e0aa8/ttl).
///
/// It shows testing the TTL extension for persistent storage keys using the `extendPersistentTTL` built-in function
#[test]
fn ttl_basic_persistent() {
    let runtime = build_solidity(
        r#"contract counter {
            /// Variable to track the count. Stored in persistent storage
            uint64 public persistent count = 11;

            /// Extends the TTL for the `count` persistent key to 5000 ledgers
            /// if the current TTL is smaller than 1000 ledgers
            function extend_ttl() public view returns (int64) {
                return count.extendTtl(1000, 5000);
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

#[test]
fn ttl_basic_temporary() {
    let runtime = build_solidity(
        r#"contract temp_counter {
            /// Variable stored in temporary storage
            uint64 temporary tempCount = 7;

            /// Extend the temporary entry TTL to become at least 7000 ledgers,
            /// when its TTL is smaller than 3000 ledgers.
            function extend_temp_ttl() public view returns (int64) {
                return tempCount.extendTtl(3000, 7000);
            }
        }"#,
        |env| {
            env.env.ledger().with_mut(|li| {
                // Current ledger sequence - the TTL is the number of
                // ledgers from the `sequence_number` (exclusive) until
                // the last ledger sequence where entry is still considered
                // alive.
                li.sequence_number = 100_000;
                // Minimum TTL for temporary entries - new temporary
                // entries will have this TTL when created.
                li.min_temp_entry_ttl = 100;
            });
        },
    );

    let addr = runtime.contracts.last().unwrap();

    // initial TTL
    runtime.env.as_contract(addr, || {
        let key = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().temporary().get_ttl(&key), 99);
    });

    // Extend temporary entry TTL to 7000 ledgers - now it is 7000.
    runtime.invoke_contract(addr, "extend_temp_ttl", vec![]);

    runtime.env.as_contract(addr, || {
        let key = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().temporary().get_ttl(&key), 7000);
    });
}

#[test]
#[should_panic(
    expected = "Calling extendTtl() on instance storage is not allowed. Use `extendInstanceTtl()` instead."
)]
fn ttl_instance_wrong() {
    let _runtime = build_solidity(
        r#"contract instance_counter {
            uint64 instance instanceCount = 3;
            
            function extend_instance_ttl() public view returns (int64) {
                return instanceCount.extendTtl(700, 3000);
            }
        }"#,
        |env| {
            env.env.ledger().with_mut(|li| {
                li.sequence_number = 100_000;
            });
        },
    );
}
