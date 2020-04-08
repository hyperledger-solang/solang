use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, no_errors};
use solang::{parse_and_resolve, Target};

#[test]
fn various_constants() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct FooReturn(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo64Return(i64);

    // parse
    let (runtime, mut store) = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, FooReturn(2).encode());

    // parse
    let (runtime, mut store) = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 0xdeadcafe;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, FooReturn(0xdead_cafe).encode());

    // parse
    let (runtime, mut store) = build_solidity(
        "
        contract test {
            function foo() public returns (int64) {
                return -0x7afedeaddeedcafe;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    assert_eq!(store.scratch, Foo64Return(-0x7afe_dead_deed_cafe).encode());
}

#[test]
fn test_literal_overflow() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            uint8 foo = 300;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion would truncate from uint16 to uint8"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint16 foo = 0x10000;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion would truncate from uint24 to uint16"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            int8 foo = 0x8_0;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion would truncate from uint8 to int8"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            int8 foo = 127;
        }",
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        "contract test {
            int8 foo = -128;
        }",
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint8 foo = 255;
        }",
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint8 foo = -1_30;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion cannot change negative number to uint8"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 1844674_4073709551616;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
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
    let (runtime, mut store) = build_solidity(
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

    runtime.function(&mut store, "const3", Vec::new());

    assert_eq!(store.scratch, Bytes3([0x11, 0x22, 0x33]).encode());

    runtime.function(&mut store, "const4", Vec::new());

    assert_eq!(store.scratch, Bytes4(*b"ABCD").encode());

    runtime.function(&mut store, "const32", Vec::new());

    assert_eq!(
        store.scratch,
        Bytes32(*b"The quick brown fox jumped over ").encode()
    );

    runtime.function(&mut store, "test4", Test4args(1, *b"abcd").encode());
    runtime.function(&mut store, "test4", Test4args(2, *b"ABCD").encode());

    // Casting to larger bytesN should insert stuff on the right
    runtime.function(&mut store, "test7", Bytes7(*b"1234567").encode());
    assert_eq!(
        store.scratch,
        Bytes32(*b"1234567\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").encode()
    );

    runtime.function(&mut store, "test3", Bytes3(*b"XYZ").encode());
    assert_eq!(store.scratch, Bytes7(*b"XYZ\0\0\0\0").encode());

    // truncating should drop values on the right
    runtime.function(&mut store, "test7trunc", Bytes7(*b"XYWOLEH").encode());
    assert_eq!(store.scratch, Bytes3(*b"XYW").encode());
}

#[test]
fn address() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            address  foo = 0x1844674_4073709551616;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion from uint80 to address not allowed"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            address foo = 0x8617E340B3D01FA5F11F306F4090FD50E238070d;
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "address literal has incorrect checksum, expected ‘0x8617E340B3D01FA5F11F306F4090FD50E238070D’");

    let (_, errors) = parse_and_resolve(
        "contract test {
            uint160 foo = 0x8617E340B3D01FA5F11F306F4090FD50E238070D;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion would truncate from address to uint160"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            address foo = 0x8617E340B3D01FA5F11F306F4090FD50E238070D;

            function bar() private returns (bool) {
                return foo > address(0);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "expression of type address not allowed"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            address foo = 0x8617E340B3D01FA5F11F306F4090FD50E238070D;

            function bar() private returns (address) {
                return foo + address(1);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "expression of type address not allowed"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            address foo = 0x8617E340B3D01FA5F11F306F4090FD50E238070D;

            function bar() private returns (address) {
                return foo | address(1);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "expression of type address not allowed"
    );

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Address([u8; 20]);

    // parse
    let (runtime, mut store) = build_solidity(
        "
        contract test {
            function check_return() public returns (address) {
                return 0xde709f2102306220921060314715629080e2fb77;
            }

            function check_param(address a) public {
                assert(a == 0xE9430d8C01C4E4Bb33E44fd7748942085D82fC91);
            }
        }",
    );

    runtime.function(&mut store, "check_return", Vec::new());

    assert_eq!(
        store.scratch,
        Address([
            0xde, 0x70, 0x9f, 0x21, 0x02, 0x30, 0x62, 0x20, 0x92, 0x10, 0x60, 0x31, 0x47, 0x15,
            0x62, 0x90, 0x80, 0xe2, 0xfb, 0x77
        ])
        .encode()
    );

    let val = Address([
        0xE9, 0x43, 0x0d, 0x8c, 0x01, 0xc4, 0xe4, 0xbb, 0x33, 0xE4, 0x4f, 0xd7, 0x74, 0x89, 0x42,
        0x08, 0x5D, 0x82, 0xfc, 0x91,
    ])
    .encode();

    runtime.function(&mut store, "check_param", val);
}
