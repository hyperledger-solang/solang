use crate::{first_error, parse_and_resolve};
use solang::Target;

#[test]
fn parse() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function get() public returns (bytes4) {
                assembly {
                    let returndata_size := mload(returndata)
                    revert(add(32, returndata), returndata_size)
                }
            }
        }"#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "evm assembly not supported on target solana"
    );
}
