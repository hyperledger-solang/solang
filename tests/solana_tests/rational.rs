// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::ethereum_types::U256;

#[test]
fn rational() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = .5 * 8;
                return x;
            }

            function test2() public returns (uint) {
                uint x = .4 * 8 + 0.8;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(4))]);

    let returns = vm.function("test2", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(4))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 + 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(5))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 / 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(24))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 % 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(0))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 5.2 - 1.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(4))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4 + 1.6;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(3))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4e4 + 1.6e3;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(15600))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint64 x) public returns (uint64, uint) {
                return (x * 961748941, 2.5 + 3.5 - 1);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test",
        &[ethabi::Token::Uint(U256::from(982451653))],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![
            ethabi::Token::Uint(U256::from(961748941u64 * 982451653u64)),
            ethabi::Token::Uint(U256::from(5))
        ]
    );
}
