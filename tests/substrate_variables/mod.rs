extern crate solang;

use super::{first_error, parse_and_resolve};
use solang::Target;

#[test]
fn test_variable_errors() {
    let ns = parse_and_resolve(
        "contract test {
            // solc 0.4.25 compiles this to 30.
            function foo() public pure returns (int32) {
                int32 a = b + 3;
                int32 b = a + 7;

                return a * b;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`b' is not declared");
}

#[test]
fn test_variable_initializer_errors() {
    // cannot read contract storage in constant
    let ns = parse_and_resolve(
        "contract test {
            uint x = 102;
            uint constant y = x + 5;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot read contract variable ‘x’ in constant expression"
    );

    // cannot read contract storage in constant
    let ns = parse_and_resolve(
        "contract test {
            function foo() public pure returns (uint) {
                return 102;
            }
            uint constant y = foo() + 5;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot call function in constant expression"
    );

    // cannot refer to variable declared later
    let ns = parse_and_resolve(
        "contract test {
            uint x = y + 102;
            uint y = 102;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`y' is not declared");

    // cannot refer to variable declared later (constant)
    let ns = parse_and_resolve(
        "contract test {
            uint x = y + 102;
            uint constant y = 102;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`y' is not declared");

    // cannot refer to yourself
    let ns = parse_and_resolve(
        "contract test {
            uint x = x + 102;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`x' is not declared");
}
