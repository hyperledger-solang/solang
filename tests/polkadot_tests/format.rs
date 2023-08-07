// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::Encode;

#[test]
fn output() {
    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(bool x) public {
                print("val:{}".format(x));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", true.encode());

    assert_eq!(runtime.debug_buffer(), "print: val:true,\n");

    runtime.debug_buffer().truncate(0);

    runtime.function("foo", false.encode());

    assert_eq!(runtime.debug_buffer(), "print: val:false,\n");

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(bytes bar) public {
                print("bar:{}".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", b"ABCD".to_vec().encode());

    assert_eq!(runtime.debug_buffer(), "print: bar:41424344,\n");

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(bytes5 bar) public {
                print("bar:{}".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", b"\x01\x03\xfe\x07\x09".encode());

    assert_eq!(runtime.debug_buffer(), "print: bar:0103fe0709,\n");

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(string bar) public {
                print("bar:{} address:{}".format(bar, this));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", "ladida".encode());

    assert_eq!(
        runtime.debug_buffer(),
        format!(
            "print: bar:ladida address:{},\n",
            hex::encode(runtime.caller())
        )
    );

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(uint64 bar) public {
                print("bar:{:x}".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", 0xcafedu64.encode());

    assert_eq!(runtime.debug_buffer(), "print: bar:0xcafed,\n");

    runtime.debug_buffer().truncate(0);

    runtime.function("foo", 0x1u64.encode());

    assert_eq!(runtime.debug_buffer(), "print: bar:0x1,\n");

    runtime.debug_buffer().truncate(0);

    runtime.function("foo", 0x0u64.encode());

    assert_eq!(runtime.debug_buffer(), "print: bar:0x0,\n");

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(int128 bar) public {
                print("bar:{:x}".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (-0xca5cadab1efeeb1eeffab1ei128).encode());

    assert_eq!(
        runtime.debug_buffer(),
        "print: bar:-0xca5cadab1efeeb1eeffab1e,\n"
    );

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(int128 bar) public {
                print("there is an old android joke which goes {:b} ".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (0x3fi128).encode());
    assert!(runtime.debug_buffer().contains("goes 0b111111 ,"));

    runtime.function("foo", (-0x3fi128).encode());
    assert!(runtime.debug_buffer().contains("goes -0b111111 ,"));

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(int64 bar) public {
                print("number:{} ".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (102i64).encode());
    assert!(runtime.debug_buffer().contains("print: number:102 ,"));

    runtime.function("foo", (-102i64).encode());
    assert!(runtime.debug_buffer().contains("print: number:-102 ,"));

    let mut runtime = build_solidity(
        r#"
        contract format {
            function foo(int128 bar) public {
                print("number:{} ".format(bar));
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (8462643383279502884i128).encode());

    assert_eq!(
        runtime.debug_buffer(),
        "print: number:8462643383279502884 ,\n"
    );

    runtime.debug_buffer().truncate(0);

    runtime.function("foo", (18462643383279502884i128).encode());

    assert_eq!(
        runtime.debug_buffer(),
        "print: number:18462643383279502884 ,\n"
    );

    runtime.debug_buffer().truncate(0);

    runtime.function("foo", (3141592653589793238462643383279502884i128).encode());
    assert!(runtime
        .debug_buffer()
        .contains("number:3141592653589793238462643383279502884 ,"));
    runtime.function("foo", (-3141592653589793238462643383279502884i128).encode());
    assert!(runtime
        .debug_buffer()
        .contains("number:-3141592653589793238462643383279502884 ,"));

    runtime.debug_buffer().truncate(0);

    let mut runtime = build_solidity(
        r#"
        contract format {
            enum enum1 { bar1, bar2, bar3 }
            function foo(int256 bar) public {
                print("number:{} ".format(bar));
            }
            function hex(int256 bar) public {
                print("number:{:x} ".format(bar));
            }
            function unsigned(uint256 bar) public {
                print("number:{} ".format(bar));
            }
            function e() public returns (string) {
                return "number<{}>".format(enum1.bar3);
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (0u128, 102u128).encode());
    assert!(runtime
        .debug_buffer()
        .contains("number:34708801425935723273264209958040357568512 ,"));
    runtime.function("foo", (0u128, -102i128).encode());

    assert!(runtime
        .debug_buffer()
        .contains("number:-34708801425935723273264209958040357568512 ,"));

    runtime.debug_buffer().truncate(0);

    runtime.function("hex", (0u128, 0x102u128).encode());
    assert!(runtime
        .debug_buffer()
        .contains("number:0x10200000000000000000000000000000000 ,"));

    runtime.function("unsigned", (0u128, 102i128).encode());
    assert!(runtime
        .debug_buffer()
        .contains("number:34708801425935723273264209958040357568512 ,"));

    runtime.function("e", Vec::new());

    assert_eq!(runtime.output(), "number<2>".encode());
}

#[test]
fn div128() {
    let mut runtime = build_solidity(
        r##"
        contract div {
            function foo(uint128 bar) public returns (uint128) {
                return bar / 1_00000_00000_00000_00000;
            }

            function rem(uint128 bar) public returns (uint128) {
                return bar % 1_00000_00000_00000_00000;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (3141592653589793238462643383279502884u128).encode());

    assert_eq!(runtime.output(), 31415926535897932u128.encode());

    runtime.function("rem", (3141592653589793238462643383279502884u128).encode());

    assert_eq!(runtime.output(), 38462643383279502884u128.encode());

    runtime.function("rem", (18462643383279502884i128).encode());

    assert_eq!(runtime.output(), 18462643383279502884i128.encode());
}
