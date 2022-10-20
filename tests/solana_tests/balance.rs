// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountState, BorshToken};
use ethabi::{Function, StateMutability};
use num_bigint::BigInt;

#[test]
fn get_balance() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public view returns (uint64) {
                return msg.sender.balance;
            }
        }"#,
    );

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 102,
        },
    );

    let returns = vm.function("test", &[], &[], Some(&new));

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(102u8),
        }]
    );
}

#[test]
fn send_fails() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 0,
        },
    );

    let returns = vm.function(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(false)]);
}

#[test]
fn send_succeeds() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    let returns = vm.function(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(true)]);

    assert_eq!(vm.account_data.get_mut(&new).unwrap().lamports, 107);

    assert_eq!(
        vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports,
        1
    );
}

#[test]
fn send_overflows() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 101,
        },
    );

    let returns = vm.function(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(false)]);

    assert_eq!(
        vm.account_data.get_mut(&new).unwrap().lamports,
        u64::MAX - 101
    );

    assert_eq!(
        vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports,
        103
    );
}

#[test]
fn transfer_succeeds() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    vm.function(
        "transfer",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(vm.account_data.get_mut(&new).unwrap().lamports, 107);

    assert_eq!(
        vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports,
        1
    );
}

#[test]
fn transfer_fails_not_enough() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    let res = vm.function_must_fail(
        "transfer",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ],
        &[],
        None,
    );
    std::println!("{:?}", res);
    assert!(res.is_err());
}

#[test]
fn transfer_fails_overflow() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 100,
        },
    );

    let res = vm.function_must_fail(
        "transfer",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ],
        &[],
        None,
    );
    assert!(res.is_err());
}

#[test]
fn fallback() {
    let mut vm = build_solidity(
        r#"
        contract c {
            fallback() external {
                print("fallback");
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[]);

    if let Some(abi) = &vm.stack[0].abi {
        let mut abi = abi.clone();

        #[allow(deprecated)]
        abi.functions.insert(
            String::from("extinct"),
            vec![Function {
                name: "extinct".to_string(),
                inputs: vec![],
                outputs: vec![],
                constant: None,
                state_mutability: StateMutability::Payable,
            }],
        );

        vm.stack[0].abi = Some(abi);
    }

    vm.function("extinct", &[], &[], None);

    assert_eq!(vm.logs, "fallback");
}

#[test]
fn value_overflows() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function send(address payable addr, uint128 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports = 103;

    vm.constructor("c", &[]);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 101,
        },
    );

    let res = vm.function_must_fail(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(u64::MAX as u128 + 1),
            },
        ],
        &[],
        None,
    );
    assert_eq!(res.ok(), Some(4294967296));

    let res = vm.function_must_fail(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(u128::MAX),
            },
        ],
        &[],
        None,
    );
    assert_eq!(res.ok(), Some(4294967296));

    let returns = vm.function(
        "send",
        &[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(102u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(false)]);

    assert_eq!(
        vm.account_data.get_mut(&new).unwrap().lamports,
        u64::MAX - 101
    );

    assert_eq!(
        vm.account_data.get_mut(&vm.stack[0].data).unwrap().lamports,
        103
    );
}
