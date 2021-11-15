use crate::{no_errors, parse_and_resolve};
use solang::Target;

#[test]
fn edge() {
    let ns = parse_and_resolve(
        r#"
        contract Test {
            function test(bool _b) public {
                uint24 n1 = 1;
                uint112 n2 = 2;
                uint256 r = (_b == true ? n1 : n2);
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}
