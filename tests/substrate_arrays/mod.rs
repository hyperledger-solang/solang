use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val32(u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val8(u8);

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

#[test]
fn const_array_array() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            int8[8] constant bar = [ int8(1), 2, 3, 4, 5, 6, 7, 8 ];

            function f(uint32 i1) public returns (int8) {
                return bar[i1];
            }
        }"##,
    );

    runtime.function(&mut store, "f", Val32(1).encode());

    assert_eq!(store.scratch, Val8(2).encode());
}

#[test]
fn votes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Votes([bool; 11]);

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            /// In a jury, do the ayes have it?
            function f(bool[11] votes) public pure returns (bool) {
                    uint32 i;
                    uint32 ayes = 0;
    
                    for (i=0; i<votes.length; i++) {
                            if (votes[i]) {
                                    ayes += 1;
                            }
                    }
    
                    return ayes > votes.length / 2;
            }
        }"##,
    );

    runtime.function(
        &mut store,
        "f",
        Votes([
            true, true, true, true, true, true, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(store.scratch, true.encode());

    runtime.function(
        &mut store,
        "f",
        Votes([
            true, true, true, true, true, false, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(store.scratch, false.encode());
}

#[test]
fn return_array() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Res([u64; 4]);

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function array() pure public returns (int64[4]) {
                return [ int64(4), 84564, 31213, 1312 ];
        }
        }"##,
    );

    runtime.function(&mut store, "array", Vec::new());

    assert_eq!(store.scratch, Res([4, 84564, 31213, 1312]).encode());
}
