use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use parity_scale_codec::Encode;
use solang::Target;

#[test]
fn parse() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "foo";

                s.format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "format only allowed on string literals"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "foo{".format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "missing closing ‘}’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "foo{d".format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "unexpected format char ‘d’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "foo{:".format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "missing format specifier");

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "foo{:}s".format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "missing argument to format");

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "f{{oo}s".format();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "unmatched ‘}’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "f{{oo}}s".format(true);
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "too many argument for format string"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, true);
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "argument must be signed or unsigned integer type"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, 0xcafe);
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    no_errors(ns.diagnostics);
}

#[test]
fn output() {
    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(bool x) public {
                print("val:{}".format(x));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", true.encode());

    assert_eq!(runtime.printbuf, "val:true");

    runtime.printbuf.truncate(0);

    runtime.function("foo", false.encode());

    assert_eq!(runtime.printbuf, "val:false");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(bytes bar) public {
                print("bar:{}".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", b"ABCD".to_vec().encode());

    assert_eq!(runtime.printbuf, "bar:41424344");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(bytes5 bar) public {
                print("bar:{}".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", b"\x01\x03\xfe\x07\x09".encode());

    assert_eq!(runtime.printbuf, "bar:0103fe0709");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(string bar) public {
                print("bar:{} address:{}".format(bar, this));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", "ladida".encode());

    assert_eq!(
        runtime.printbuf,
        format!("bar:ladida address:{}", hex::encode(&runtime.vm.address))
    );

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(uint64 bar) public {
                print("bar:{:x}".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", 0xcafedu64.encode());

    assert_eq!(runtime.printbuf, "bar:0xcafed");

    runtime.printbuf.truncate(0);

    runtime.function("foo", 0x1u64.encode());

    assert_eq!(runtime.printbuf, "bar:0x1");

    runtime.printbuf.truncate(0);

    runtime.function("foo", 0x0u64.encode());

    assert_eq!(runtime.printbuf, "bar:0x0");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(int128 bar) public {
                print("bar:{:x}".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (-0xca5cadab1efeeb1eeffab1ei128).encode());

    assert_eq!(runtime.printbuf, "bar:-0xca5cadab1efeeb1eeffab1e");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(int128 bar) public {
                print("there is an old android joke which goes {:b} ".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (0x3fi128).encode());
    runtime.function("foo", (-0x3fi128).encode());

    assert_eq!(
        runtime.printbuf,
        "there is an old android joke which goes 0b111111 there is an old android joke which goes -0b111111 "
    );

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(int64 bar) public {
                print("number:{} ".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (102i64).encode());
    runtime.function("foo", (-102i64).encode());

    assert_eq!(runtime.printbuf, "number:102 number:-102 ");

    let mut runtime = build_solidity(
        r##"
        contract format {
            function foo(int128 bar) public {
                print("number:{} ".format(bar));
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (8462643383279502884i128).encode());

    assert_eq!(runtime.printbuf, "number:8462643383279502884 ");

    runtime.printbuf.truncate(0);

    runtime.function("foo", (18462643383279502884i128).encode());

    assert_eq!(runtime.printbuf, "number:18462643383279502884 ");

    runtime.printbuf.truncate(0);

    runtime.function("foo", (3141592653589793238462643383279502884i128).encode());
    runtime.function("foo", (-3141592653589793238462643383279502884i128).encode());

    assert_eq!(runtime.printbuf, "number:3141592653589793238462643383279502884 number:-3141592653589793238462643383279502884 ");

    runtime.printbuf.truncate(0);

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", (0u128, 102u128).encode());
    runtime.function("foo", (0u128, -102i128).encode());

    assert_eq!(
        runtime.printbuf,
        "number:34708801425935723273264209958040357568512 number:-34708801425935723273264209958040357568512 "
    );

    runtime.printbuf.truncate(0);

    runtime.function("hex", (0u128, 0x102u128).encode());
    runtime.function("unsigned", (0u128, 102i128).encode());

    assert_eq!(
        runtime.printbuf,
        "number:0x10200000000000000000000000000000000 number:34708801425935723273264209958040357568512 "
    );

    runtime.function("e", Vec::new());

    assert_eq!(runtime.vm.output, "number<2>".encode());
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

    assert_eq!(runtime.vm.output, 31415926535897932u128.encode());

    runtime.function("rem", (3141592653589793238462643383279502884u128).encode());

    assert_eq!(runtime.vm.output, 38462643383279502884u128.encode());

    runtime.function("rem", (18462643383279502884i128).encode());

    assert_eq!(runtime.vm.output, 18462643383279502884i128.encode());
}
