// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn simple() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
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

    assert_eq!(runtime.output(), Val(65536).encode());

    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), Val(102).encode());
}

#[test]
fn using() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
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

    assert_eq!(runtime.output(), Val(65536).encode());

    // the using directive can specify a different type than the function in the library,
    // as long as it casts implicitly and matches the type of method call _exactly_
    let mut runtime = build_solidity(
        r##"
        contract test {
            using {ints.max} for uint32;
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

    assert_eq!(runtime.output(), Val(65536).encode());

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

    assert_eq!(runtime.output(), Val(571).encode());
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

        abstract contract base {
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

    assert_eq!(runtime.output(), true.encode());
}
