use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[derive(Debug, PartialEq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn contract_name() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            function test() public {}
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            enum test { a}
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            bool test;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            struct test { bool a; }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "test is already defined as a contract name"
    );
}

#[test]
fn revert() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract bar {
            function test() public {
                revert("yo!");
            }

            function a() public {
                b();
            }

            function b() public {
                c();
            }

            function c() public {
                d();
            }

            function d() public {
                revert("revert value has to be passed down the stack");
            }
        }"##,
    );

    runtime.function_expect_revert(&mut store, "test", Vec::new());

    assert_eq!(
        store.scratch,
        RevertReturn(0x08c3_79a0, "yo!".to_string()).encode()
    );

    runtime.function_expect_revert(&mut store, "a", Vec::new());

    assert_eq!(
        store.scratch,
        RevertReturn(
            0x08c3_79a0,
            "revert value has to be passed down the stack".to_string()
        )
        .encode()
    );

    let (runtime, mut store) = build_solidity(
        r##"
        contract c {
            function test() public {
                revert();
            }
        }"##,
    );

    runtime.function_expect_revert(&mut store, "test", Vec::new());

    assert_eq!(store.scratch.len(), 0);
}

#[test]
fn require() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract c {
            function test1() public {
                require(false, "Program testing can be used to show the presence of bugs, but never to show their absence!");
            }

            function test2() public {
                require(true, "Program testing can be used to show the presence of bugs, but never to show their absence!");
            }
        }"##,
    );

    runtime.function_expect_revert(&mut store, "test1", Vec::new());

    assert_eq!(
        store.scratch,
        RevertReturn(0x08c3_79a0, "Program testing can be used to show the presence of bugs, but never to show their absence!".to_string()).encode()
    );

    runtime.function(&mut store, "test2", Vec::new());

    assert_eq!(store.scratch.len(), 0);
}
