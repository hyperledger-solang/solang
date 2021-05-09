use crate::{build_solidity, first_error, parse_and_resolve};
use ethabi::Token;
use solang::Target;

#[test]
fn calltys() {
    let ns = parse_and_resolve(
        r#"
        contract main {
            function test() public {
                address x = address(0);

                x.staticcall(hex"1222");
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘staticcall’ does not exist"
    );

    let ns = parse_and_resolve(
        r#"
        contract main {
            function test() public {
                address x = address(0);

                x.delegatecall(hex"1222");
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘delegatecall’ does not exist"
    );

    let ns = parse_and_resolve(
        r#"
        contract main {
            function test() public {
                address x = address(0);

                (bool success, bytes bs) = x.call{gas: 5}(hex"1222");
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘gas’ not permitted for external calls on solana"
    );
}

#[test]
fn simple_external_call() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_bar(string v) public {
                print("bar0 says: " + v);
            }

            function test_other(bar1 x) public {
                x.test_bar("cross contract call");
            }
        }

        contract bar1 {
            function test_bar(string v) public {
                print("bar1 says: " + v);
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("test_bar", &[Token::String(String::from("yo"))]);

    assert_eq!(vm.printbuf, "bar1 says: yo");

    vm.printbuf.truncate(0);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor(&[]);

    vm.function("test_bar", &[Token::String(String::from("uncle beau"))]);

    assert_eq!(vm.printbuf, "bar0 says: uncle beau");

    vm.printbuf.truncate(0);

    vm.function("test_other", &[Token::FixedBytes(bar1_account.to_vec())]);

    assert_eq!(vm.printbuf, "bar1 says: cross contract call");
}

#[test]
fn external_call_with_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(bar1 x) public returns (int64) {
                return x.test_bar(7) + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    vm.constructor(&[]);

    let res = vm.function("test_bar", &[Token::Int(ethereum_types::U256::from(21))]);

    assert_eq!(res, vec![Token::Int(ethereum_types::U256::from(24))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor(&[]);

    let res = vm.function("test_other", &[Token::FixedBytes(bar1_account.to_vec())]);

    assert_eq!(res, vec![Token::Int(ethereum_types::U256::from(15))]);
}

#[test]
fn external_call_with_string_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(bar1 x) public returns (string) {
                string y = x.test_bar(7);
                print(y);
                return y;
            }

            function test_this(bar1 x) public {
                address a = x.who_am_i();
                assert(a == address(x));
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (string) {
                return "foo:{}".format(y);
            }

            function who_am_i() public returns (address) {
                return address(this);
            }
        }"#,
    );

    vm.constructor(&[]);

    let res = vm.function("test_bar", &[Token::Int(ethereum_types::U256::from(22))]);

    assert_eq!(res, vec![Token::String(String::from("foo:22"))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor(&[]);

    let res = vm.function("test_other", &[Token::FixedBytes(bar1_account.to_vec())]);

    assert_eq!(res, vec![Token::String(String::from("foo:7"))]);

    vm.function("test_this", &[Token::FixedBytes(bar1_account.to_vec())]);
}
