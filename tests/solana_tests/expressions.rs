use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use ethabi::Token;
use solang::Target;

#[test]
fn interfaceid() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function get() public returns (bytes4) {
                return type(foo).interfaceId;
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "type(…).interfaceId is permitted on interface, not contract foo"
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function get() public returns (bytes4) {
                return type(I).interfaceId;
            }
        }

        interface I {
            function bar(int) external;
            function baz(bytes) external returns (int);
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("get", &[], &[]);

    assert_eq!(
        returns,
        vec![Token::FixedBytes(0xc78d9f3au32.to_be_bytes().to_vec())]
    );
}

#[test]
fn const_in_type() {
    let ns = parse_and_resolve(
        r#"
        contract x {
            uint public constant Y = 24;

            constructor(bytes32[Y] memory foo) {}
        }"#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn bytes32_0() {
    let ns = parse_and_resolve(
        r#"
        contract x {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(0);
            }

            function b4() public pure returns (bytes4 r) {
                r = bytes4(0xcafedead);
            }

            function b3() public pure returns (bytes3 r) {
                r = bytes3(0x012233);
            }
        }"#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(0xffee);
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "number of 2 bytes cannot be converted to type ‘bytes32’"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(-1);
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "negative number cannot be converted to type ‘bytes32’"
    );
}

#[test]
fn contract_no_init() {
    let ns = parse_and_resolve(
        r#"
        contract other {
            int public a;
        }

        contract testing {
            function test(int x) public returns (int) {
                other o;
                do {
                    x--;
                    o = new other();
                }while(x > 0);

                return o.a();
            }
        }"#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn selector_in_free_function() {
    let ns = parse_and_resolve(
        r#"
        interface I {
            function X(bytes) external;
        }

        function x() returns (bytes4) {
            return I.X.selector;
        }

        contract foo {}
        "#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        interface I {
            function X(bytes) external;
        }

        contract X {
            function x() public returns (bytes4) {
                return I.X.selector;
            }
        }"#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract I {
            function X() external {}
        }

        contract foo {
            function f(I t) public returns (bytes4) {
                return t.X.selector;
            }
        }
        "#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);
}
