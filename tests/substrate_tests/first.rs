// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};

use crate::build_solidity;

#[test]
fn simple_solidiy_compile_and_run() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct FooReturn {
        value: u32,
    }

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

    let ret = FooReturn { value: 2 };

    assert_eq!(runtime.output(), ret.encode());
}

#[test]
fn flipper() {
    // parse
    let mut runtime = build_solidity(
        "
        contract flipper {
            bool private value;

            constructor(bool initvalue) public {
                value = initvalue;
            }

            function flip() public {
                value = !value;
            }

            function get() public view returns (bool) {
                return value;
            }
        }
        ",
    );

    runtime.function("get", Vec::new());
    assert_eq!(runtime.output(), false.encode());

    runtime.function("flip", Vec::new());
    runtime.function("flip", Vec::new());
    runtime.function("flip", Vec::new());
    runtime.function("get", Vec::new());
    assert_eq!(runtime.output(), true.encode());
}

#[test]
fn contract_storage_initializers() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct FooReturn {
        value: u32,
    }

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 a = 100;
            uint32 b = 200;

            constructor() public {
                b = 300;
            }

            function foo() public returns (uint32) {
                return a + b;
            }
        }",
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", Vec::new());

    let ret = FooReturn { value: 400 };

    assert_eq!(runtime.output(), ret.encode());
}

#[test]
fn contract_constants() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct FooReturn {
        value: u32,
    }

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 constant a = 300 + 100;

            function foo() public pure returns (uint32) {
                uint32 ret = a;
                return ret;
            }
        }",
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", Vec::new());

    let ret = FooReturn { value: 400 };

    assert_eq!(runtime.output(), ret.encode());
}

#[test]
fn large_contract_variables() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct ValBool(u8);

    // parse
    let mut runtime = build_solidity("
        contract test {
            int constant large = 0x7fff0000_7fff0000_7fff0000_7fff0000__7fff0000_7fff0000_7fff0000_7fff0000;
            int bar = large + 10;

            function foo() public view returns (int) {
                return (bar - 10);
            }
        }",
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", Vec::new());

    assert_eq!(runtime.output(), b"\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f\x00\x00\xff\x7f");
}

#[test]
fn assert_ok() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public pure returns (uint32) {
                assert(true);
                return 0;
            }
        }",
    );

    runtime.constructor(0, Vec::new());

    runtime.function("foo", Vec::new());
}

#[test]
fn assert_not_ok() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public pure returns (uint32) {
                assert(false);
                return 0;
            }
        }",
    );

    runtime.constructor(0, Vec::new());
    runtime.function_expect_failure("foo", Vec::new());
}
