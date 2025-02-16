// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::storage::{Instance, Persistent, Temporary};
use soroban_sdk::testutils::Ledger;

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
            
            function extendInstanceTtl() public view returns (int64) {
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

#[test]
fn ttl_instance_correct() {
    let runtime = build_solidity(
        r#"contract instance_counter {
            /// Variable stored in instance storage
            uint64 instance instanceCount = 3;

            /// Extends the TTL for the instance storage to 10000 ledgers
            /// if the current TTL is smaller than 2000 ledgers
            function extendInstanceTtl() public view returns (int64) {
                return extendInstanceTtl(2000, 10000);
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
                // Minimum TTL for temporary entries - new temporary
                // entries will have this TTL when created.
                li.min_temp_entry_ttl = 100;
                // Maximum TTL of any entry. Note, that entries can have their TTL
                // extended indefinitely, but each extension can be at most
                // `max_entry_ttl` ledger from the current `sequence_number`.
                li.max_entry_ttl = 15000;
            });
        },
    );

    let addr = runtime.contracts.last().unwrap();

    // Initial TTL for instance storage
    runtime.env.as_contract(addr, || {
        assert_eq!(runtime.env.storage().instance().get_ttl(), 499);
    });

    // Extend instance TTL to 10000 ledgers
    runtime.invoke_contract(addr, "extendInstanceTtl", vec![]);
    runtime.env.as_contract(addr, || {
        assert_eq!(runtime.env.storage().instance().get_ttl(), 10000);
    });
}

/// This test is adapted from
/// [Stellar Soroban Examples](https://github.com/stellar/soroban-examples/blob/f595fb5df06058ec0b9b829e9e4d0fe0513e0aa8/ttl).
#[test]
fn ttl_combined() {
    let runtime = build_solidity(
        r#"
        contract ttl_storage {
            uint64 public persistent pCount = 11;
            uint64 temporary tCount = 7;
            uint64 instance iCount = 3;

            function extend_persistent_ttl() public view returns (int64) {
                return pCount.extendTtl(1000, 5000);
            }

            function extend_temp_ttl() public view returns (int64) {
                return tCount.extendTtl(3000, 7000);
            }

            function extendInstanceTtl() public view returns (int64) {
                return extendInstanceTtl(2000, 10000);
            }
        }"#,
        |env| {
            env.env.ledger().with_mut(|li| {
                li.sequence_number = 100_000;
                li.min_persistent_entry_ttl = 500;
                li.min_temp_entry_ttl = 100;
                li.max_entry_ttl = 15000;
            });
        },
    );

    let addr = runtime.contracts.last().unwrap();

    // Verify initial TTLs
    runtime.env.as_contract(addr, || {
        let pkey = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        let tkey = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&pkey), 499);
        assert_eq!(runtime.env.storage().instance().get_ttl(), 499);
        assert_eq!(runtime.env.storage().temporary().get_ttl(&tkey), 99);
    });

    // Extend persistent storage TTL
    runtime.invoke_contract(addr, "extend_persistent_ttl", vec![]);
    runtime.env.as_contract(addr, || {
        let pkey = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&pkey), 5000);
    });

    // Extend instance storage TTL
    runtime.invoke_contract(addr, "extendInstanceTtl", vec![]);
    runtime.env.as_contract(addr, || {
        assert_eq!(runtime.env.storage().instance().get_ttl(), 10000);
    });

    // Extend temporary storage TTL
    runtime.invoke_contract(addr, "extend_temp_ttl", vec![]);
    runtime.env.as_contract(addr, || {
        let tkey = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().temporary().get_ttl(&tkey), 7000);
    });

    // Bump ledger sequence by 5000
    runtime.env.ledger().with_mut(|li| {
        li.sequence_number = 105_000;
    });

    // Verify TTL after ledger increment
    runtime.env.as_contract(addr, || {
        let pkey = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        let tkey = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&pkey), 0);
        assert_eq!(runtime.env.storage().instance().get_ttl(), 5000);
        assert_eq!(runtime.env.storage().temporary().get_ttl(&tkey), 2000);
    });

    // Re-extend all TTLs
    runtime.invoke_contract(addr, "extend_persistent_ttl", vec![]);
    runtime.invoke_contract(addr, "extendInstanceTtl", vec![]);
    runtime.invoke_contract(addr, "extend_temp_ttl", vec![]);

    // Final TTL verification
    runtime.env.as_contract(addr, || {
        let pkey = runtime
            .env
            .storage()
            .persistent()
            .all()
            .keys()
            .first()
            .unwrap();
        let tkey = runtime
            .env
            .storage()
            .temporary()
            .all()
            .keys()
            .first()
            .unwrap();
        assert_eq!(runtime.env.storage().persistent().get_ttl(&pkey), 5000);
        assert_eq!(runtime.env.storage().instance().get_ttl(), 5000); // Threshold not met, remains the same
        assert_eq!(runtime.env.storage().temporary().get_ttl(&tkey), 7000);
    });
}

#[test]
#[should_panic(expected = "[testing-only] Accessed contract instance key that has been archived.")]
fn test_persistent_entry_archival() {
    let runtime = build_solidity(
        r#"
        contract persistent_cleanup {
            uint64 public persistent pCount = 11;

            function extend_persistent_ttl() public view returns (int64) {
                return pCount.extendTtl(1000, 10000);
            }

            function extendInstanceTtl() public view returns (int64) {
                return extendInstanceTtl(2000, 10000);
            }
        }"#,
        |env| {
            env.env.ledger().with_mut(|li| {
                li.sequence_number = 100_000;
                li.min_persistent_entry_ttl = 500;
                li.min_temp_entry_ttl = 100;
                li.max_entry_ttl = 15000;
            });
        },
    );

    let addr = runtime.contracts.last().unwrap();

    // Extend instance TTL
    runtime.invoke_contract(addr, "extendInstanceTtl", vec![]);

    // Bump ledger sequence by 10001 (one past persistent TTL)
    runtime.env.ledger().with_mut(|li| {
        li.sequence_number = 110_001;
    });

    // This should panic as the persistent entry is archived
    runtime.invoke_contract(addr, "extend_persistent_ttl", vec![]);
}
