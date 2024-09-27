// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn various_constants() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct FooReturn(u32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Foo64Return(i64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    runtime.function("foo", Vec::new());

    assert_eq!(runtime.output(), FooReturn(2).encode());

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 0xdeadcafe;
            }
        }",
    );

    runtime.function("foo", Vec::new());

    assert_eq!(runtime.output(), FooReturn(0xdead_cafe).encode());

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 1e3;
            }
        }",
    );

    runtime.function("foo", Vec::new());

    assert_eq!(runtime.output(), FooReturn(1000).encode());

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (int64) {
                return -7e3;
            }
        }",
    );

    runtime.function("foo", Vec::new());

    assert_eq!(runtime.output(), Foo64Return(-7000).encode());

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (int64) {
                return -0x7afedeaddeedcafe;
            }
        }",
    );

    runtime.function("foo", Vec::new());

    assert_eq!(
        runtime.output(),
        Foo64Return(-0x7afe_dead_deed_cafe).encode()
    );
}

#[test]
fn bytes() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes3([u8; 3]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes4([u8; 4]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes7([u8; 7]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes32([u8; 32]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Test4args(u32, [u8; 4]);

    // parse
    let mut runtime = build_solidity(
        r#"
        contract test {
            function const3() public returns (bytes3) {
                return hex"112233";
            }

            function const4() public returns (bytes4) {
                return "ABCD";
            }

            function const32() public returns (bytes32) {
                return "The quick brown fox jumped over ";
            }

            function test4(uint32 x, bytes4 foo) public {
                if (x == 1)
                    assert(foo == "abcd");
                else if (x == 2)
                    assert(foo == "ABCD");
                else
                    assert(false);
            }

            function test7(bytes7 foo) public returns (bytes32) {
                return bytes32(foo);
            }

            function test3(bytes3 foo) public returns (bytes7) {
                return bytes7(foo);
            }

            function test7trunc(bytes7 foo) public returns (bytes3) {
                return bytes3(foo);
            }

            function hex_lit_leading_zero() public pure {
                assert(bytes4(0x00) == hex"00000000");
                assert(
                    bytes32(0x00d4f4fc2f5752f06faf7ece82edbdcd093e8ee1144d482ea5820899b3520315)
                    ==
                    hex"00d4f4fc2f5752f06faf7ece82edbdcd093e8ee1144d482ea5820899b3520315"
                );
            }
        }"#,
    );

    runtime.function("const3", Vec::new());

    assert_eq!(runtime.output(), Bytes3([0x11, 0x22, 0x33]).encode());

    runtime.function("const4", Vec::new());

    assert_eq!(runtime.output(), Bytes4(*b"ABCD").encode());

    runtime.function("const32", Vec::new());

    assert_eq!(
        runtime.output(),
        Bytes32(*b"The quick brown fox jumped over ").encode()
    );

    runtime.function("test4", Test4args(1, *b"abcd").encode());
    runtime.function("test4", Test4args(2, *b"ABCD").encode());

    // Casting to larger bytesN should insert stuff on the right
    runtime.function("test7", Bytes7(*b"1234567").encode());
    assert_eq!(
        runtime.output(),
        Bytes32(*b"1234567\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").encode()
    );

    runtime.function("test3", Bytes3(*b"XYZ").encode());
    assert_eq!(runtime.output(), Bytes7(*b"XYZ\0\0\0\0").encode());

    // truncating should drop values on the right
    runtime.function("test7trunc", Bytes7(*b"XYWOLEH").encode());
    assert_eq!(runtime.output(), Bytes3(*b"XYW").encode());

    runtime.function("hex_lit_leading_zero", vec![]);
}

#[test]
fn address() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Address([u8; 32]);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function check_return() public returns (address) {
                return address(0x7d5839e24ACaDa338c257643a7d2e025453F77D058b8335C1c3791Bc6742b320);
            }

            function check_param(address a) public {
                assert(a == address(0x8D166E028f3148854F2427d29B8755F617EED0651Bc6C8809b189200A4E3aaa9));
            }
        }",
    );

    runtime.function("check_return", Vec::new());

    assert_eq!(
        runtime.output(),
        Address([
            0x7d, 0x58, 0x39, 0xe2, 0x4a, 0xca, 0xda, 0x33, 0x8c, 0x25, 0x76, 0x43, 0xa7, 0xd2,
            0xe0, 0x25, 0x45, 0x3f, 0x77, 0xd0, 0x58, 0xb8, 0x33, 0x5c, 0x1c, 0x37, 0x91, 0xbc,
            0x67, 0x42, 0xb3, 0x20,
        ])
        .encode()
    );

    let val = Address([
        0x8d, 0x16, 0x6e, 0x2, 0x8f, 0x31, 0x48, 0x85, 0x4f, 0x24, 0x27, 0xd2, 0x9b, 0x87, 0x55,
        0xf6, 0x17, 0xee, 0xd0, 0x65, 0x1b, 0xc6, 0xc8, 0x80, 0x9b, 0x18, 0x92, 0x0, 0xa4, 0xe3,
        0xaa, 0xa9,
    ])
    .encode();

    runtime.function("check_param", val);
}

#[test]
fn type_name() {
    // parse
    let mut runtime = build_solidity(
        r#"
        contract test {
            function foo() public returns (uint32) {
                assert(type(foobar).name == "foobar");
                assert(type(uint8).min == 0);
                assert(type(uint128).min == 0);
                assert(type(uint256).min == 0);
                assert(type(uint48).min == 0);
                return 2;
            }
        }

        abstract contract foobar {
            int32 a;
        }"#,
    );

    runtime.function("foo", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract test {
            function min() public returns (uint32) {
                assert(type(int8).min == -128);
                assert(type(int16).min == -32768);
                assert(type(int64).min == -9223372036854775808);
                assert(type(int48).min == -140737488355328);
                return 2;
            }

            function max_int() public returns (uint32) {
                assert(type(int8).max == 127);
                assert(type(int16).max == 32767);
                assert(type(int64).max == 9223372036854775807);
                assert(type(int48).max == 140737488355327);
                return 2;
            }

            function max_uint() public returns (uint32) {
                assert(type(uint8).max == 255);
                assert(type(uint16).max == 65535);
                assert(type(uint64).max == 18446744073709551615);
                assert(type(uint48).max == 281474976710655);
                return 2;
            }
        }"##,
    );

    runtime.function("min", Vec::new());
    runtime.function("max_int", Vec::new());
    runtime.function("max_uint", Vec::new());
}

#[test]
fn units() {
    // parse
    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo() public {
                assert(10 seconds == 10);
                assert(1 minutes == 60);
                assert(60 minutes == 1 hours);
                assert(48 hours == 2 days);
                assert(14 days == 2 weeks);
            }
        }"##,
    );

    runtime.function("foo", Vec::new());

    // parse
    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo() public {
                assert(10 wei == 10);
                assert(1 gwei == 1000_000_000);
                assert(1 ether == 1000_000_000_000_000_000);
            }
        }"##,
    );

    runtime.function("foo", Vec::new());
}

#[test]
fn literal_bytes_cast() {
    // parse
    let mut runtime = build_solidity(
        r#"
        contract test {
            function foo() public {
                bytes4 x = bytes4(hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");

                assert(x == hex'acaf_3289');


                bytes32 y = hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f";

                assert(bytes4(x) == hex"acaf_3289");
            }
        }"#,
    );

    runtime.function("foo", Vec::new());
}

#[test]
fn implicit_bytes_cast() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                bytes4 b1 = hex"01020304";

                bytes b2 = b1;

                assert(b2 == hex"01020304");
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                bytes b1 = hex"01020304";

                bytes4 b2 = b1;

                assert(b2 == hex"01020304");
            }
        }
        "#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn implicit_bytes_cast_incompatible_size() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public returns (bytes3) {
                bytes b1 = hex"01020304";

                bytes3 b2 = b1;
                return b2;
            }
        }
        "#,
    );

    runtime.function_expect_failure("test", Vec::new());
}

#[test]
fn signed_literal_unsigned_cast() {
    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo() public {
                assert(uint16(-1) == 0xffff);
                assert(uint8(-2) == 0xfe);
                assert(uint32(-3) == 0xffff_fffd);
                assert(uint8(-4000) == 96);
            }
        }"##,
    );

    runtime.function("foo", Vec::new());
}

#[test]
fn mul() {
    // https://github.com/hyperledger-solang/solang/issues/1507
    let mut runtime = build_solidity(
        r#"
        contract Test {
            function test()
                external view
                returns (uint256 result)
            {
                return f(10_000_000_000);
            }

            function f(uint256 x)
                internal pure
                returns (uint256)
            {
                return x * x / 10_000_000_000;
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.output(),
        (10000000000u64, 0u64, 0u64, 0u64).encode()
    );
}
