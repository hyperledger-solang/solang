use crate::{build_solidity, first_error, parse_and_resolve};
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

    assert_eq!(first_error(ns.diagnostics), "`b' is not found");
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

    assert_eq!(first_error(ns.diagnostics), "`y' is not found");

    // cannot refer to variable declared later (constant)
    let ns = parse_and_resolve(
        "contract test {
            uint x = y + 102;
            uint constant y = 102;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`y' is not found");

    // cannot refer to yourself
    let ns = parse_and_resolve(
        "contract test {
            uint x = x + 102;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`x' is not found");
}

#[test]
fn global_constants() {
    let ns = parse_and_resolve("uint x = 102;", Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "global variable must be constant"
    );

    let ns = parse_and_resolve("uint constant public x = 102;", Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "‘public’: global variable cannot have visibility specifier"
    );

    let ns = parse_and_resolve("uint constant external x = 102;", Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "‘external’: global variable cannot have visibility specifier"
    );

    let ns = parse_and_resolve("uint constant x;", Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "missing initializer for constant"
    );

    let ns = parse_and_resolve(
        "uint constant test = 5; contract test {}",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "test is already defined as a contract name"
    );

    let mut runtime = build_solidity(
        r##"
        int32 constant foo = 102 + 104;
        contract a {
            function test() public payable {
                assert(foo == 206);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        string constant foo = "FOO";
        contract a {
            function test() public payable {
                assert(foo == "FOO");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());
}
