use crate::{build_solidity, first_error, parse_and_resolve};
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
