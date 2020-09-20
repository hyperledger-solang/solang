use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, first_warning, no_errors, parse_and_resolve};
use solang::Target;

#[test]
fn various_constants() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct FooReturn(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
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

    assert_eq!(runtime.vm.scratch, FooReturn(2).encode());

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

    assert_eq!(runtime.vm.scratch, FooReturn(0xdead_cafe).encode());

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

    assert_eq!(runtime.vm.scratch, FooReturn(1000).encode());

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

    assert_eq!(runtime.vm.scratch, Foo64Return(-7000).encode());

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
        runtime.vm.scratch,
        Foo64Return(-0x7afe_dead_deed_cafe).encode()
    );
}

#[test]
fn test_literal_overflow() {
    let ns = parse_and_resolve(
        "contract test {
            uint8 foo = 300;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would truncate from uint16 to uint8"
    );

    let ns = parse_and_resolve(
        "contract test {
            uint16 foo = 0x10000;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would truncate from uint24 to uint16"
    );

    let ns = parse_and_resolve(
        "contract test {
            int8 foo = 0x8_0;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would truncate from uint8 to int8"
    );

    let ns = parse_and_resolve(
        "contract test {
            int8 foo = 127;
        }",
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        "contract test {
            int8 foo = -128;
        }",
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        "contract test {
            uint8 foo = 255;
        }",
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        "contract test {
            uint8 foo = -1_30;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion cannot change negative number to uint8"
    );

    let ns = parse_and_resolve(
        "contract test {
            int64 foo = 1844674_4073709551616;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would truncate from uint72 to int64"
    );
}

#[test]
fn bytes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes3([u8; 3]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes4([u8; 4]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes7([u8; 7]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes32([u8; 32]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Test4args(u32, [u8; 4]);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function const3() public returns (bytes3) {
                return hex\"112233\";
            }

            function const4() public returns (bytes4) {
                return \"ABCD\";
            }

            function const32() public returns (bytes32) {
                return \"The quick brown fox jumped over \";
            }

            function test4(uint32 x, bytes4 foo) public {
                if (x == 1)
                    assert(foo == \"abcd\");
                else if (x == 2)
                    assert(foo == \"ABCD\");
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
        }",
    );

    runtime.function("const3", Vec::new());

    assert_eq!(runtime.vm.scratch, Bytes3([0x11, 0x22, 0x33]).encode());

    runtime.function("const4", Vec::new());

    assert_eq!(runtime.vm.scratch, Bytes4(*b"ABCD").encode());

    runtime.function("const32", Vec::new());

    assert_eq!(
        runtime.vm.scratch,
        Bytes32(*b"The quick brown fox jumped over ").encode()
    );

    runtime.function("test4", Test4args(1, *b"abcd").encode());
    runtime.function("test4", Test4args(2, *b"ABCD").encode());

    // Casting to larger bytesN should insert stuff on the right
    runtime.function("test7", Bytes7(*b"1234567").encode());
    assert_eq!(
        runtime.vm.scratch,
        Bytes32(*b"1234567\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").encode()
    );

    runtime.function("test3", Bytes3(*b"XYZ").encode());
    assert_eq!(runtime.vm.scratch, Bytes7(*b"XYZ\0\0\0\0").encode());

    // truncating should drop values on the right
    runtime.function("test7trunc", Bytes7(*b"XYWOLEH").encode());
    assert_eq!(runtime.vm.scratch, Bytes3(*b"XYW").encode());
}

#[test]
fn address() {
    let ns = parse_and_resolve(
        "contract test {
            address  foo = 0x1844674_4073709551616;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion from uint80 to address not allowed"
    );

    let ns = parse_and_resolve(
        "contract test {
            address foo = 0xa368df6dfcd5ba7b0bc108af09e98e4655e35a2c3b2e2d5e3eae6c6f7cd8d2d4;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "address literal has incorrect checksum, expected ‘0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4’");

    let ns = parse_and_resolve(
        "contract test {
            uint256 foo = 0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would truncate from address to uint256"
    );

    let ns = parse_and_resolve(
        "contract test {
            address foo = 0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4;

            function bar() private returns (bool) {
                return foo > address(0);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expression of type address not allowed"
    );

    let ns = parse_and_resolve(
        "contract test {
            address foo = 0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4;

            function bar() private returns (address) {
                return foo + address(1);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expression of type address not allowed"
    );

    let ns = parse_and_resolve(
        "contract test {
            address foo = 0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4;

            function bar() private returns (address) {
                return foo | address(1);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expression of type address not allowed"
    );

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Address([u8; 32]);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function check_return() public returns (address) {
                return 0x7d5839e24ACaDa338c257643a7d2e025453F77D058b8335C1c3791Bc6742b320;
            }

            function check_param(address a) public {
                assert(a == 0x8D166E028f3148854F2427d29B8755F617EED0651Bc6C8809b189200A4E3aaa9);
            }
        }",
    );

    runtime.function("check_return", Vec::new());

    assert_eq!(
        runtime.vm.scratch,
        Address([
            0x20, 0xb3, 0x42, 0x67, 0xbc, 0x91, 0x37, 0x1c, 0x5C, 0x33, 0xb8, 0x58, 0xD0, 0x77,
            0x3F, 0x45, 0x25, 0xe0, 0xd2, 0xa7, 0x43, 0x76, 0x25, 0x8c, 0x33, 0xda, 0xca, 0x4A,
            0xe2, 0x39, 0x58, 0x7d
        ])
        .encode()
    );

    let val = Address([
        0xa9, 0xaa, 0xE3, 0xA4, 0x00, 0x92, 0x18, 0x9b, 0x80, 0xC8, 0xc6, 0x1B, 0x65, 0xD0, 0xEE,
        0x17, 0xF6, 0x55, 0x87, 0x9B, 0xd2, 0x27, 0x24, 0x4F, 0x85, 0x48, 0x31, 0x8f, 0x02, 0x6E,
        0x16, 0x8D,
    ])
    .encode();

    runtime.function("check_param", val);
}

#[test]
fn address_payable_type() {
    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address payable a) public {
                address b = a;
            }
        }"##,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion to contract other from address not allowed"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address payable a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion to contract other from address payable not allowed"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address payable a) public {
                other b = other(a);
            }
        }

        contract other {
            function test() public {
            }
        }"##,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address payable a) public {
                address b = address(a);
            }
        }"##,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(payable a) public {
                address b = a;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘payable’ cannot be used for type declarations, only casting. use ‘address payable’"
    );

    // note: this is not possible in solc yet
    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(address a) public {
                address payable b = address payable(a);
            }
        }"##,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn type_name() {
    // parse
    let mut runtime = build_solidity(
        r##"
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

        contract foobar {
            int32 a;
        }"##,
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

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                int32 x = type(bool).max;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "type ‘bool’ does not have type function max"
    );
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
                assert(1 szabo == 1000_000_000_000);
                assert(1 finney == 1000_000_000_000_000);
                assert(1 ether == 1000_000_000_000_000_000);
            }
        }"##,
    );

    runtime.function("foo", Vec::new());

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                int32 x = 1 ether;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "ethereum currency unit used while not targetting ethereum"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                int32 x = 0xa days;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "hexadecimal numbers cannot be used with unit denominations"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                int32 x = (1 + 2) days;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unit denominations can only be used with number literals"
    );
}

#[test]
fn literal_bytes_cast() {
    // parse
    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo() public {
                bytes4 x = bytes4(hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");

                assert(x == hex"acaf_3289");


                bytes32 y = hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f";

                assert(bytes4(x) == hex"acaf_3289");
            }
        }"##,
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
#[should_panic]
fn implicit_bytes_cast_incompatible_size() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                bytes b1 = hex"01020304";

                bytes3 b2 = b1;
            }
        }
        "#,
    );

    runtime.function("test", Vec::new());
}
