// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::Encode;

#[test]
fn global_constants() {
    // test that error is allowed as a variable name/contract name
    let mut runtime = build_solidity(
        r##"
        int32 constant error = 102 + 104;
        contract a {
            function test() public payable {
                assert(error == 206);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        string constant foo = "FOO";
        contract error {
            function test() public payable {
                assert(foo == "FOO");
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        string constant foo = "FOO";
        contract a {
            function test(uint64 error) public payable {
                assert(error == 0);
                assert(foo == "FOO");
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", 0u64.encode());
}

#[test]
fn ext_fn_call_is_reading_variable() {
    let mut runtime = build_solidity(
        r##"contract C {
            function ext_func_call(uint32 arg) public payable returns (uint32) {
                A a = new A();
                function(uint32) external returns (uint32) func = a.a;
                return func(arg);
            }
        }
        
        contract A {
            function a(uint32 arg) public pure returns (uint32) {
                return arg;
            }
        }"##,
    );

    runtime.set_transferred_value(1000);
    runtime.function("ext_func_call", 0xdeadbeefu32.encode());
    assert_eq!(runtime.output(), 0xdeadbeefu32.encode())
}
