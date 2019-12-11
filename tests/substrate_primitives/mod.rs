
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Encode, Decode};

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[test]
fn various_constants() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct FooReturn(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo64Return(i64);

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, FooReturn(2).encode());

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            function foo() public returns (uint32) {
                return 0xdeadcafe;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, FooReturn(0xdeadcafe).encode());

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            function foo() public returns (int64) {
                return -0x7afedeaddeedcafe;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, Foo64Return(-0x7afedeaddeedcafe).encode());
}

#[test]
fn test_literal_overflow() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint8 foo = 300;
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would truncate from uint16 to uint8");

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint16 foo = 0x10000;
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would truncate from uint24 to uint16");

    let (_, errors) = parse_and_resolve(
        "contract test {
            int8 foo = 0x80;
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would truncate from uint8 to int8");

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint8 foo = -130;
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion cannot change negative number to uint8");

    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 18446744073709551616;
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would truncate from uint72 to int64");
}