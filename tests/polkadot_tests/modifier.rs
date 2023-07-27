// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn chain() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            uint16 public var;

            modifier foo() {
                bool boom = true;
                if (boom) {
                    _;
                }
            }

            function bar() foo() public {
                var = 7;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("bar", Vec::new());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(7, 0));

    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1 {
                var = 3;
                _;
                var = 5;
            }

            function test() mod1 public {
                assert(var == 3);
                var = 7;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", Vec::new());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(5, 0));

    // now test modifier with argument and test that function argument is passed on
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1(uint16 v) {
                var = 3;
                _;
                assert(var == 11);
                var = v;
            }

            function test(uint16 x) mod1(x - 6) public {
                assert(var == 3);
                var = x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", 11u16.encode());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(5, 0));

    // now test modifier with argument and test that function argument is passed on
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1(uint16 v) {
                assert(var == 0);

                var = 3;
                _;
                assert(var == 17);
                var = v;
            }

            modifier mod2(uint16 v) {
                assert(var == 3);
                var = 9;
                _;
                assert(var == 11);
                var = v;
            }

            function test(uint16 x) mod1(x - 6) mod2(x + 6) public {
                assert(var == 9);
                var = x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", 11u16.encode());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(5, 0));

    // two placeholders means the following function is called twice.
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier m {
                _;
                _;
            }

            function test() m public {
                var += 3;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", Vec::new());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(6, 0));
}

#[test]
fn inherit_modifier() {
    let mut runtime = build_solidity(
        r##"
        contract c is base {
            function test() md2 public {
                    assert(s2 == 2);
                    s2 += 3;
            }
        }

        abstract contract base {
                bool private s1;
                int32 internal s2;

                modifier md2 {
                        s2 += 2;
                        _;
                        s2 += 2;
                }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];
    slot[0] = 1;

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", Vec::new());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(7, 0, 0, 0));

    // now override it
    let mut runtime = build_solidity(
        r##"
        contract c is base {
            function test() md2 public {
                    assert(s2 == 2);
                    s2 += 3;
            }

            modifier md2 override {
                s2 += 2;
                _;
                s2 += 5;
            }
        }

        abstract contract base {
            bool private s1;
            int32 internal s2;

            modifier md2 virtual {
                    s2 += 1;
                    _;
                    s2 += 1;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];
    slot[0] = 1;

    assert_eq!(runtime.storage().get(&slot), None);

    runtime.function("test", Vec::new());

    assert_eq!(runtime.storage().get(&slot).unwrap(), &vec!(10, 0, 0, 0));
}

#[test]
fn return_values() {
    // in the modifier syntax, there are no return values
    // however, the generated cfg has the same arguments/returns the function is on attached

    // return simple value
    let mut runtime = build_solidity(
        r##"
        contract c {
            int64 s2;

            function test() md2 public returns (int64) {
                    assert(s2 == 2);
                    s2 += 3;
                    return s2;
            }

            modifier md2 {
                    s2 += 2;
                    _;
                    s2 += 2;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u64);

    assert_eq!(runtime.output(), Val(5).encode());

    let mut runtime = build_solidity(
        r#"
        struct S {
            int64 f1;
            string f2;
        }

        contract c {
            int64 s2;

            function test() md2 public returns (bool, S) {
                    assert(s2 == 2);
                    s2 += 3;
                    return (true, S({ f1: s2, f2: "Hello, World!" }));
            }

            modifier md2 {
                    s2 += 2;
                    _;
                    s2 += 2;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct StructS(bool, u64, String);

    assert_eq!(
        runtime.output(),
        StructS(true, 5, String::from("Hello, World!")).encode()
    );
}

#[test]
fn repeated_modifier() {
    let mut runtime = build_solidity(
        r#"
        contract Test {
            modifier notZero(uint64 num) {
                require(num != 0, "invalid number");
                _;
            }

            function contfunc(uint64 num1, uint64 num2) public notZero(num1) notZero(num2) {
                // any code
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("contfunc", (1u64, 0u64).encode());
    runtime.function_expect_failure("contfunc", (0u64, 0u64).encode());
    runtime.function_expect_failure("contfunc", (0u64, 1u64).encode());
    runtime.function("contfunc", (1u64, 1u64).encode());
}

#[test]
fn modifier_in_library() {
    let mut runtime = build_solidity(
        r##"
        library LibWidthMod {
            modifier m(uint64 v) {
                require(v > 100);
                _;
            }

            function withMod(uint64 self) m(self) internal view {
                require(self > 10);
            }
        }

        contract Test {
            using LibWidthMod for uint64;

            function test(uint64 n) external {
                n.withMod();
                LibWidthMod.withMod(n);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("test", 0u64.encode());
    runtime.function_expect_failure("test", 50u64.encode());
    runtime.function_expect_failure("test", 100u64.encode());
    runtime.function("test", 200u64.encode());
}
