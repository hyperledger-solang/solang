use super::first_error;
use solang::{parse_and_resolve, Target};

#[test]
fn missing_array_index() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public returns (uint) {
                    uint8[4] memory bar = [ 1, 2, 3, 4 ];

                    return bar[];
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "expected expression before ‘]’ token");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public returns (uint8) {
                    uint8[4] memory bar = [ 1, 2, 3, 4, 5 ];

                    return bar[0];
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "conversion from uint8[5] to uint8[4] not possible"
    );
}
