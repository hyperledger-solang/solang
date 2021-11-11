use crate::{account_new, build_solidity, AccountState};
use ethabi::Token;

#[test]
fn msg_value() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public payable returns (uint64) {
                return msg.value * 3;
            }
        }"#,
    );

    vm.constructor("c", &[], 0);

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    let returns = vm.function("test", &[], &[], 102, None);

    assert_eq!(returns[0], Token::Uint(ethereum_types::U256::from(306)));

    assert_eq!(vm.account_data[&vm.origin].lamports, 312 - 102);
    assert_eq!(vm.account_data[&vm.stack[0].data].lamports, 102);
}

#[test]
fn msg_value_not_enough() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public payable {}
        }"#,
    );

    vm.constructor("c", &[], 0);

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 5;

    let res = vm.function_must_fail("test", &[], &[], 102, None);
    assert!(res.is_err());
}

#[test]
#[should_panic]
fn default_constructor_not_payable() {
    let mut vm = build_solidity(r#"contract c {}"#);

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 2;

    vm.constructor("c", &[], 1);
}

#[test]
#[should_panic]
fn constructor_not_payable() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor () {}
        }
    "#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 2;

    vm.constructor("c", &[], 1);
}

#[test]
fn function_not_payable() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public {}
        }
    "#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 200;

    vm.constructor("c", &[], 0);

    let res = vm.function_must_fail("test", &[], &[], 102, None);

    assert_eq!(res.ok(), Some(4294967296));
}

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

    vm.constructor("c", &[], 0);

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 102,
        },
    );

    let returns = vm.function("test", &[], &[], 0, Some(&new));

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(102))]);
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

    vm.constructor("c", &[], 0);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Bool(false)]);
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

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[], 103);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Bool(true)]);

    assert_eq!(
        vm.account_data.get_mut(&vm.origin).unwrap().lamports,
        312 - 103
    );

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
            constructor() payable {}

            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[], 103);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Bool(false)]);

    assert_eq!(
        vm.account_data.get_mut(&vm.origin).unwrap().lamports,
        312 - 103
    );

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
            constructor() payable {}

            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[], 103);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(
        vm.account_data.get_mut(&vm.origin).unwrap().lamports,
        312 - 103
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
            constructor() payable {}

            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[], 103);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(104)),
        ],
        &[],
        0,
        None,
    );
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

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 312;

    vm.constructor("c", &[], 103);

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
            Token::FixedBytes(new.to_vec()),
            Token::Uint(ethereum_types::U256::from(104)),
        ],
        &[],
        0,
        None,
    );
    assert!(res.is_err());
}
