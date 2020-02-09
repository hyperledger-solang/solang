use parity_scale_codec::Encode;
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

    // is it impossible to define recursive structs
    let (_, errors) = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                Foo y;
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "type ‘Foo’ not found");
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

#[test]
fn structs_as_ref_args() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bool x;
                uint32 y;
            }
        
            function func(foo f) private {
                f.x = true;
                f.y = 64;
            }
        
            function test() public {
                foo f;
        
                func(f);
        
                assert(f.x == true);
                assert(f.y == 64);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}

#[test]
fn structs_encode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: bool,
    };

    let (runtime, mut store) = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                bool f2;
            }
                
            function test(foo f) public {
                assert(f.f1 == "ABC");
                assert(f.f2 == true);
            }
        }"##,
    );

    runtime.function(
        &mut store,
        "test",
        Foo {
            f1: [0x41, 0x42, 0x43],
            f2: true,
        }
        .encode(),
    );
}

#[test]
fn structs_decode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: i32,
    };

    let (runtime, mut store) = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                int32 f2;
            }
                
            function test() public returns (foo) {
                foo f;

                f.f1 = hex"f33ec3";
                f.f2 = 0xfd7f;

                return f;
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());

    assert_eq!(
        store.scratch,
        Foo {
            f1: [0xf3, 0x3e, 0xc3],
            f2: 0xfd7f,
        }
        .encode(),
    );
}
