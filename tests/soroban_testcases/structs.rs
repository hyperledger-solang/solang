// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use indexmap::Equivalent;
use soroban_sdk::{
    contracttype, testutils::Address as _, Address, Bytes, FromVal, IntoVal, Val, U256,
};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub zebra: u64,
    pub apple: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct IntRec {
    pub a: i32,
    pub b: i64,
    pub c: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ValRec {
    pub a: bool,
    pub b: u32,
    pub c: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: u64,
    pub y: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub a: Point,
    pub b: Point,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Wide {
    pub big: u128,
    pub neg: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Big256 {
    pub v: U256,
}

#[test]
fn get_fields_via_dot() {
    let runtime = build_solidity(
        r#"
        contract locker {
            struct Lock {
                uint64 release_time;
                address beneficiary;
                uint64 amount;
            }

            mapping(address => Lock) locks;

            function create_lock(
                uint64 release_time,
                address beneficiary,
                uint64 amount
            ) public returns (uint64) {
                Lock memory l = Lock({
                    release_time: release_time,
                    beneficiary: beneficiary,
                    amount: amount
                });

                locks[beneficiary] = l;
                return l.amount;
            }

            function get_lock_amount(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].amount;
            }

            function get_lock_release(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].release_time;
            }

            function get_lock_beneficiary(address key) public view returns (address) {
                return locks[key].beneficiary;
            }

            // Extended functionality: increase amount in-place and return new total
            function increase_lock_amount(address beneficiary, uint64 delta) public returns (uint64) {
                locks[beneficiary].amount += delta;
                return locks[beneficiary].amount;
            }

            // Extended functionality: move a lock to a different beneficiary
            function move_lock(address from, address to) public {
                Lock memory l = locks[from];
                require(l.amount != 0, "no lock");
                l.beneficiary = to;
                locks[to] = l;
                // emulate delete by zeroing fields
                locks[from].amount = 0;
                locks[from].release_time = 0;
            }

            // Extended functionality: clear lock for a beneficiary
            function clear_lock(address beneficiary) public {
                // emulate delete by zeroing fields
                locks[beneficiary].amount = 0;
                locks[beneficiary].release_time = 0;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let user1 = Address::generate(&runtime.env);
    let user2 = Address::generate(&runtime.env);

    let release_time: Val = 1_000_u64.into_val(&runtime.env);
    let amount: Val = 500_u64.into_val(&runtime.env);

    // Create a new lock for user1
    let create_args = vec![release_time, user1.clone().into_val(&runtime.env), amount];
    let res = runtime.invoke_contract(addr, "create_lock", create_args);
    assert!(amount.shallow_eq(&res));

    // Verify getters
    let get_amt_args = vec![user1.clone().into_val(&runtime.env)];
    let get_rel_args = vec![user1.clone().into_val(&runtime.env)];
    let get_ben_args = vec![user1.clone().into_val(&runtime.env)];
    let got_amount = runtime.invoke_contract(addr, "get_lock_amount", get_amt_args);
    let got_release = runtime.invoke_contract(addr, "get_lock_release", get_rel_args);
    let got_beneficiary = runtime.invoke_contract(addr, "get_lock_beneficiary", get_ben_args);
    assert!(amount.shallow_eq(&got_amount));
    assert!(release_time.shallow_eq(&got_release));
    let addr_val = Address::from_val(&runtime.env, &got_beneficiary);
    assert!(addr_val.equivalent(&user1));

    // Increase amount and verify new total
    let delta: Val = 250_u64.into_val(&runtime.env);
    let inc_args = vec![user1.clone().into_val(&runtime.env), delta];
    let new_total = runtime.invoke_contract(addr, "increase_lock_amount", inc_args);
    let expected_total: Val = 750_u64.into_val(&runtime.env);
    assert!(expected_total.shallow_eq(&new_total));

    // Move lock from user1 to user2
    let move_args = vec![
        user1.clone().into_val(&runtime.env),
        user2.clone().into_val(&runtime.env),
    ];
    let _ = runtime.invoke_contract(addr, "move_lock", move_args);

    // After moving, user1 should have no lock (amount == 0)
    let zero: Val = 0_u64.into_val(&runtime.env);
    let amt_user1 = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user1.clone().into_val(&runtime.env)],
    );
    assert!(zero.shallow_eq(&amt_user1));

    // And user2 should now hold the moved lock with the updated total amount
    let amt_user2 = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user2.clone().into_val(&runtime.env)],
    );
    assert!(expected_total.shallow_eq(&amt_user2));

    // Beneficiary for user2's lock should be user2
    let ben_user2 = runtime.invoke_contract(
        addr,
        "get_lock_beneficiary",
        vec![user2.clone().into_val(&runtime.env)],
    );
    let ben2 = Address::from_val(&runtime.env, &ben_user2);
    assert!(ben2.equivalent(&user2));

    // Clear user2's lock and verify
    let _ = runtime.invoke_contract(
        addr,
        "clear_lock",
        vec![user2.clone().into_val(&runtime.env)],
    );
    let amt_user2_after_clear =
        runtime.invoke_contract(addr, "get_lock_amount", vec![user2.into_val(&runtime.env)]);
    assert!(zero.shallow_eq(&amt_user2_after_clear));
}

// Removed: keep only two tests as requested

#[test]
fn get_whole_struct() {
    let runtime = build_solidity(
        r#"
        contract locker {
            struct Lock {
                uint64 release_time;
                address beneficiary;
                uint64 amount;
            }

            mapping(address => Lock) locks;

            function create_lock(
                uint64 release_time,
                address beneficiary,
                uint64 amount
            ) public returns (uint64) {
                Lock memory l = Lock({
                    release_time: release_time,
                    beneficiary: beneficiary,
                    amount: amount
                });

                locks[beneficiary] = l;
                return l.amount;
            }

            function get_lock_amount(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].amount;
            }

            function get_lock_release(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].release_time;
            }

            function get_lock_beneficiary(address key) public view returns (address) {
                return locks[key].beneficiary;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let user = Address::generate(&runtime.env);
    let release_time: Val = 42_u64.into_val(&runtime.env);
    let amount: Val = 7_u64.into_val(&runtime.env);

    // Create lock
    let _ = runtime.invoke_contract(
        addr,
        "create_lock",
        vec![release_time, user.clone().into_val(&runtime.env), amount],
    );

    // Retrieve each field via accessors (no multiple returns)
    let rt_val = runtime.invoke_contract(
        addr,
        "get_lock_release",
        vec![user.clone().into_val(&runtime.env)],
    );
    let ben_val = runtime.invoke_contract(
        addr,
        "get_lock_beneficiary",
        vec![user.clone().into_val(&runtime.env)],
    );
    let amt_val = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user.clone().into_val(&runtime.env)],
    );

    let rt: u64 = FromVal::from_val(&runtime.env, &rt_val);
    let ben = Address::from_val(&runtime.env, &ben_val);
    let amt: u64 = FromVal::from_val(&runtime.env, &amt_val);

    assert_eq!(rt, 42);
    assert!(ben.equivalent(&user));
    assert_eq!(amt, 7);
}

#[test]
fn abi_struct_return_encodes_as_map() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Rec {
                uint64 zebra;
                uint64 apple;
            }

            function make(uint64 z, uint64 a) public pure returns (Rec memory) {
                return Rec({ zebra: z, apple: a });
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let z: Val = 7_u64.into_val(&runtime.env);
    let a: Val = 9_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "make", vec![z, a]);
    let got = Rec::from_val(&runtime.env, &res);
    assert_eq!(got, Rec { zebra: 7, apple: 9 });
}

#[test]
fn abi_struct_return_mixed_integers() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S {
                int64 b;
                int128 c;
                int32 a;
            }

            function make(int32 a, int64 b, int128 c) public pure returns (S memory) {
                return S({ a: a, b: b, c: c });
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let a: Val = (-7i32).into_val(&runtime.env);
    let b: Val = (-9000i64).into_val(&runtime.env);
    let c: Val = 100_000_000_000_000_000_000i128.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "make", vec![a, b, c]);

    let got = IntRec::from_val(&runtime.env, &res);
    assert_eq!(
        got,
        IntRec {
            a: -7,
            b: -9000,
            c: 100_000_000_000_000_000_000i128,
        }
    );
}

#[test]
fn abi_struct_return_value_types() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S {
                bytes4 c;
                bool a;
                uint32 b;
            }

            function make() public pure returns (S memory) {
                bool a = true;
                uint32 b = 42;
                bytes4 c = 0xDEADBEEF;
                return S({ a: a, b: b, c: c });
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "make", vec![]);
    let got = ValRec::from_val(&runtime.env, &res);

    assert!(got.a);
    assert_eq!(got.b, 42);
    assert_eq!(
        got.c,
        Bytes::from_array(&runtime.env, &[0xDE, 0xAD, 0xBE, 0xEF])
    );
}

#[test]
fn abi_struct_decode_param_sum() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct P {
                uint64 y;
                uint64 x;
            }

            function sum(P memory p) public pure returns (uint64) {
                return p.x + p.y;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let arg: Val = Point { x: 3, y: 4 }.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "sum", vec![arg]);

    let got: u64 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(got, 7);
}

#[test]
fn abi_struct_decode_nested() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Point {
                uint64 y;
                uint64 x;
            }

            struct Line {
                Point a;
                Point b;
            }

            function span(Line memory l) public pure returns (uint64) {
                return l.a.x + l.b.y;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let input = Line {
        a: Point { x: 3, y: 7 },
        b: Point { x: 13, y: 6 },
    };
    let res = runtime.invoke_contract(addr, "span", vec![input.into_val(&runtime.env)]);

    let got: u64 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(got, 9);
}
