use crate::{build_solidity, first_error, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::Target;

#[test]
fn restrictions() {
    let ns = parse_and_resolve(
        r#"
        library c {
            constructor() {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "constructor not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            receive() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "receive not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            fallback() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "fallback not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            function f() public payable {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function in a library cannot be payable"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            function foo() virtual public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "functions in a library cannot be virtual"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            function foo() override public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function in a library cannot override"
    );

    let ns = parse_and_resolve(
        r#"
        library c is x {
            fallback() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library ‘c’ cannot have a base contract"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            function foo() public { }
        }

        contract a is c {
            function bar() public { }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library ‘c’ cannot be used as base contract for contract ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        library c {
            int x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library ‘c’ is not allowed to have state variable ‘x’"
    );
}

#[test]
fn simple() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo(uint64 x) public pure returns (uint64) {
                return ints.max(x, 65536);
            }

            function bar() public pure returns (uint64) {
                return ints.bar();
            }
        }

        library ints {
            uint64 constant CONSTANT_BAR = 102;

            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }

            function bar() internal pure returns (uint64) {
                return CONSTANT_BAR;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", Val(102).encode());

    assert_eq!(runtime.vm.output, Val(65536).encode());

    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val(102).encode());
}

#[test]
fn using() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            using x for x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "library ‘x’ not found");

    let ns = parse_and_resolve(
        r#"
        contract x {
            constructor() {}
        }

        contract c {
            using x for x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library expected but contract ‘x’ found"
    );

    let ns = parse_and_resolve(
        r#"
        library x {
            function max(uint64 a, uint64 b) private pure returns (uint64) {
                return a > b ? a : b;
            }
        }

        contract c {
            using x for asdf;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "type ‘asdf’ not found");

    let ns = parse_and_resolve(
        r#"
        library x {
            function max(uint64 a, uint64 b) private pure returns (uint64) {
                return a > b ? a : b;
            }
        }

        contract c {
            using x for x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "using library ‘x’ to extend library type not possible"
    );

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let mut runtime = build_solidity(
        r##"
        contract test {
            using ints for uint64;
            function foo(uint64 x) public pure returns (uint64) {
                return x.max(65536);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", Val(102).encode());

    assert_eq!(runtime.vm.output, Val(65536).encode());

    // the using directive can specify a different type than the function in the library,
    // as long as it casts implicitly and matches the type of method call _exactly_
    let mut runtime = build_solidity(
        r##"
        contract test {
            using ints for uint32;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", 102u32.encode());

    assert_eq!(runtime.vm.output, Val(65536).encode());

    let mut runtime = build_solidity(
        r##"
        contract test {
            using lib for int32[100];
            bool i_exists_to_make_bar_have_non_zero_storage_slot;
            int32[100] bar;

            function foo() public returns (int64) {
                    bar.set(10, 571);

                    return bar[10];
            }
        }

        library lib {
            function set(int32[100] storage a, uint index, int32 val) internal {
                    a[index] = val;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", Vec::new());

    assert_eq!(runtime.vm.output, Val(571).encode());

    let ns = parse_and_resolve(
        r##"
        contract test {
            using ints for uint64;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "method ‘max’ does not exist");

    let ns = parse_and_resolve(
        r##"
        contract test {
            using ints for uint32;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536, 2);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library function expects 2 arguments, 3 provided (including self)"
    );

    let ns = parse_and_resolve(
        r##"
        contract test {
            using ints for uint32;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536, 2);
            }
        }

        library ints {
            uint64 constant nada = 0;

            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
            function max(uint64 a) internal pure returns (uint64) {
                return a;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot find overloaded library function which matches signature"
    );
}

#[test]
fn using_in_base() {
    let mut runtime = build_solidity(
        r#"
        contract r is base {
            function baz(uint64 arg) public returns (bool) {
                    bar(arg);

                    return x;
            }
        }

        library Lib {
                function foo(uint64 a, uint64 b) internal returns (bool) {
                        return a == b;
                }
        }

        contract base {
                using Lib for *;
                bool x;

                function bar(uint64 arg) internal {
                        x = arg.foo(102);
                }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("baz", 102u64.encode());

    assert_eq!(runtime.vm.output, true.encode());
}
