use super::{build_solidity, first_error, no_errors};
use solang::{parse_and_resolve, Target};

#[test]
fn basic_tests() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public {
                    string f = new string(2);

                    f[0] = 102;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "array subscript is not permitted on string"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public {
                    bytes f = new string(2);
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "conversion from string to bytes not possible"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public {
                    string f = new bytes(2);
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "conversion from bytes to string not possible"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public {
                    string f = string(new bytes(2));
            }
        }"#,
        &Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public {
                    bytes f = bytes(new string(2));
            }
        }"#,
        &Target::Substrate,
    );

    no_errors(errors);
}

#[test]
fn more_tests() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function test() public {
                string s = new string(10);

                assert(s.length == 10);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function test() public {
                bytes s = new bytes(2);

                s[0] = 0x41;
                s[1] = 0x42;

                assert(s.length == 2);

                assert(s[0] == 0x41);
                assert(s[1] == 0x42);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function ref_test(bytes n) private {
                n[1] = 102;

                n = new bytes(10);
                // new reference
                n[1] = 104;
            }

            function test() public {
                bytes s = new bytes(2);

                s[0] = 0x41;
                s[1] = 0x42;

                assert(s.length == 2);

                ref_test(s);

                assert(s[0] == 0x41);
                assert(s[1] == 102);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function test() public {
                bytes s = "ABCD";

                assert(s.length == 4);

                s[0] = 0x41;
                s[1] = 0x42;
                s[2] = 0x43;
                s[3] = 0x44;
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}
