use crate::{build_solidity, first_error, parse_and_resolve};

#[test]
fn rational() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint y = 0.1;
                return y;
            }
        }"#,
        crate::Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion to uint256 from rational not allowed"
    );

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

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(4))]
    );

    let returns = vm.function("test2", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(4))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 + 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(5))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 / 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(24))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 % 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(0))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 5.2 - 1.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(4))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4 + 1.6;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(3))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4e4 + 1.6e3;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(15600))]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint64 x) public returns (uint64, uint) {
                return (x * 961748941, 2.5 + 3.5 - 1);
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function(
        "test",
        &[ethabi::Token::Uint(ethereum_types::U256::from(982451653))],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![
            ethabi::Token::Uint(ethereum_types::U256::from(961748941u64 * 982451653u64)),
            ethabi::Token::Uint(ethereum_types::U256::from(5))
        ]
    );
}
