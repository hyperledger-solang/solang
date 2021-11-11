use crate::{build_solidity, first_error, parse_and_resolve};
use ethabi::Token;
use solang::Target;

#[test]
fn return_single() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint) {
                return 2;
            }

            function g() public returns (uint) {
                return false? 2 : 3;
            }

            function h() public returns (uint) {
                return true? f() : g();
            }

            function i() public returns (uint) {
                int a = 24;
                return uint(a);
            }

            function j() public returns (uint) {
                return 2 + 3;
            }
        }"#,
    );
    vm.constructor("foo", &[], 0);

    let returns = vm.function("f", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2)),]);

    let returns = vm.function("g", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(3)),]);

    let returns = vm.function("h", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2)),]);

    let returns = vm.function("i", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(24)),]);

    let returns = vm.function("j", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(5)),]);
}

#[test]
fn return_ternary() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint, uint) {
                return true ? (false ? (1, 2) : (3, 4)) : (5, 6);
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);
    let returns = vm.function("f", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(3)),
            Token::Uint(ethereum_types::U256::from(4)),
        ]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint, uint) {
                return true ? (1 + 2 + 3, 2 * 2) : (22 + 6, 1996);
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);
    let returns = vm.function("f", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(6)),
            Token::Uint(ethereum_types::U256::from(4)),
        ]
    );
}

#[test]
fn return_err() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            uint private val = 0;

            function get() public {
                return val;
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(first_error(ns.diagnostics), "function has no return values",);

    let ns = parse_and_resolve(
        r#"
        contract foo {
            uint private val = 0;

            function get() public returns (uint, uint) {
                return (val, val, val);
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "incorrect number of return values, expected 2 but got 3",
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            uint private val = 0;
            function f() public returns (uint, uint, uint) {
                return (val, val, val);
            }

            function get() public returns (uint, uint) {
                return f();
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "incorrect number of return values, expected 2 but got 3",
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            uint private val = 0;
            function f() public returns (uint, uint, uint) {
                return (val, val, val);
            }

            function get() public {
                return f();
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(first_error(ns.diagnostics), "function has no return values",);

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function f() public returns (uint, uint, uint) {
                int a = 43;
                return uint(a);
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "incorrect number of return values, expected 3 but got 1",
    );
}

#[test]
fn return_nothing() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            uint private val = 0;

            function inc() public {
                val += 1;
            }

            function get() public returns (uint) {
                return val;
            }

            function strange() public {
                return inc();
            }

        }"#,
    );

    vm.constructor("foo", &[], 0);
    let _returns = vm.function("strange", &[], &[], 0, None);
    let _returns = vm.function("inc", &[], &[], 0, None);
    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2)),]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            uint a = 4;

            function inc() internal {
                a += 1;
            }

            function dec() internal {
                a -= 1;
            }

            function get() public returns (uint) {
                return a;
            }

            function f() public {
                return true ? inc() : dec();
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);
    let _returns = vm.function("f", &[], &[], 0, None);
    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(5)),]);
}

#[test]
fn return_function() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function g() public returns (uint, uint) {
                return (1, 2);
            }

            function f() public returns (uint, uint) {
                return g();
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);
    let returns = vm.function("f", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(1)),
            Token::Uint(ethereum_types::U256::from(2)),
        ]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function g() public returns (uint, uint) {
                return (1, 2);
            }

            function f() public returns (uint, uint) {
                return true? g() : (0, 0);
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);
    let returns = vm.function("f", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(1)),
            Token::Uint(ethereum_types::U256::from(2)),
        ]
    );
}
