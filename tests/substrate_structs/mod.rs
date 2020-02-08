use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val32(u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val8(u8);

#[test]
fn parse_structs() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint a;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "struct ‘Foo’ has duplicate struct field ‘a’"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint storage b;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "storage location ‘storage’ not allowed for struct field"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint calldata b;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "storage location ‘calldata’ not allowed for struct field"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool memory a;
                uint calldata b;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "storage location ‘memory’ not allowed for struct field"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "struct definition for ‘Foo’ has no fields"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                boolean x;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "type ‘boolean’ not found");
}

#[test]
fn struct_members() {
    let (runtime, mut store) = build_solidity(
        r##"
        pragma solidity 0;
        pragma experimental ABIEncoderV2;

        contract test_struct_parsing {
                struct foo {
                        bool x;
                        uint32 y;
                        bytes31 d;
                }

                function test() public {
                        foo f;
                        f.x = true;
                        f.y = 64;

                        assert(f.x == true);
                        assert(f.y == 64);
                        assert(f.d.length == 31);
                }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}
