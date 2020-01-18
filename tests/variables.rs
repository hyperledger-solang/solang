extern crate solang;

use solang::output;
use solang::{parse_and_resolve, Target};

fn first_error(errors: Vec<output::Output>) -> String {
    match errors.iter().find(|m| m.level == output::Level::Error) {
        Some(m) => m.message.to_owned(),
        None => panic!("no errors found"),
    }
}

#[test]
fn test_variable_errors() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            // solc 0.4.25 compiles this to 30.
            function foo() public pure returns (int32) {
                int32 a = b + 3;
                int32 b = a + 7;

                return a * b;
            }
        }",
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "`b' is not declared");
}

#[test]
fn test_variable_initializer_errors() {
    // cannot read contract storage in constant
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint x = 102;
            uint constant y = x + 5;
        }",
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "cannot read variable x in constant expression"
    );

    // cannot read contract storage in constant
    let (_, errors) = parse_and_resolve(
        "contract test {
            function foo() public pure returns (uint) {
                return 102;
            }
            uint constant y = foo() + 5;
        }",
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "cannot call function in constant expression"
    );

    // cannot refer to variable declared later
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint x = y + 102;
            uint y = 102;
        }",
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "`y' is not declared");

    // cannot refer to variable declared later (constant)
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint x = y + 102;
            uint constant y = 102;
        }",
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "`y' is not declared");

    // cannot refer to yourself
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint x = x + 102;
        }",
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "`x' is not declared");
}
