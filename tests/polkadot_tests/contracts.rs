// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn external_call() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(runtime.output(), Ret(1020).encode());
}

#[test]
fn revert_external_call() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                revert("The reason why");
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("test", Vec::new());
}

#[test]
fn revert_constructor() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            b x;
            constructor() public {
            }
            function test() public returns (int32) {
                x = new b(102);
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                require(a == 0, "Hello,\
 World!");
            }

            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function_expect_failure("test", Vec::new());
}

#[test]
fn external_datatypes() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(u64);

    let mut runtime = build_solidity(
        r#"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }

            function test() public returns (int64) {
                strukt k = x.get_x(10, "foobar", true, strukt({ f1: "abcd", f2: address(555555), f3: -1 }));

                assert(k.f1 == "1234");
                assert(k.f2 == address(102));
                return int64(k.f3);
            }
        }

        contract b {
            int x;
            constructor(int a) public {
                x = a;
            }

            function get_x(int t, string s, bool y, strukt k) public returns (strukt) {
                assert(y == true);
                assert(t == 10);
                assert(s == "foobar");
                assert(k.f1 == "abcd");

                return strukt({ f1: "1234", f2: address(102), f3: x * t });
            }
        }

        struct strukt {
            bytes4 f1;
            address f2;
            int f3;
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(runtime.output(), Ret(1020).encode());
}

#[test]
fn creation_code() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public returns (bytes) {
                bytes runtime = type(b).runtimeCode;

                assert(runtime[0] == 0);
                assert(runtime[1] == 0x61); // a
                assert(runtime[2] == 0x73); // s
                assert(runtime[3] == 0x6d); // m

                bytes creation = type(b).creationCode;

                // on Polkadot, they are the same
                assert(creation == runtime);

                return creation;
            }
        }

        contract b {
            int public x;
            constructor(int a) public {
                x = a;
            }
        }"##,
    );

    runtime.function("test", Vec::new());
    assert_eq!(runtime.output(), runtime.contracts()[1].code.blob.encode());
}

#[test]
fn issue666() {
    let mut runtime = build_solidity(
        r#"
        contract Flipper {
            function flip () pure public {
                print("flip");
            }
        }

        contract Inc {
            Flipper _flipper;

            constructor (Flipper _flipperContract) {
                _flipper = _flipperContract;
            }

            function superFlip () view public {
                _flipper.flip();
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    let flipper_address = runtime.caller();

    println!("flipper_address={}", hex::encode(flipper_address));

    runtime.set_account(1);

    runtime.constructor(0, flipper_address.to_vec());

    runtime.function("superFlip", Vec::new());

    assert!(runtime.output().is_empty());
}

#[test]
fn mangle_function_names_in_abi() {
    let runtime = build_solidity(
        r##"
        enum E { v1, v2 }
        struct S { int256 i; bool b; address a; }

        contract C {
            // foo_
            function foo() public pure {}

            // foo_uint256_addressArray2Array
            function foo(uint256 i, address[2][] memory a) public pure {}

            // foo_uint8Array2__int256_bool_address
            function foo(E[2] memory e, S memory s) public pure {}
        }"##,
    );

    let _ = runtime.contracts()[0].code.messages["foo_"];
    let _ = runtime.contracts()[0].code.messages["foo_uint256_addressArray2Array"];
    let _ = runtime.contracts()[0].code.messages["foo_uint8Array2__int256_bool_address"];
    assert!(!runtime.contracts()[0].code.messages.contains_key("foo"));
}

#[test]
fn mangle_overloaded_function_names_in_abi() {
    let runtime = build_solidity(
        r##"
        contract A {
            function foo(bool x) public {}
        }

        contract B is A {
            function foo(int x) public {}
        }"##,
    );

    let _ = runtime.contracts()[0].code.messages["foo"];
    assert!(!runtime.contracts()[0]
        .code
        .messages
        .contains_key("foo_bool"));

    let _ = runtime.contracts()[1].code.messages["foo_bool"];
    assert!(!runtime.contracts()[1].code.messages.contains_key("foo"));
}
