use crate::{first_error, no_errors, parse_and_resolve};
use solang::Target;

#[test]
fn call() {
    let ns = parse_and_resolve(
        r#"
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.delegatecall{value: 2}("");
            }
        }
        "#,
        Target::Ewasm,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘delegatecall’ cannot have value specifed"
    );

    let ns = parse_and_resolve(
        r#"
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.staticcall{value: 2}("");
            }
        }
        "#,
        Target::Ewasm,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘staticcall’ cannot have value specifed"
    );

    let ns = parse_and_resolve(
        r#"
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.call{value: 2}("");
            }
        }
        "#,
        Target::Ewasm,
    );

    no_errors(ns.diagnostics);
}
