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
        r##"
        string constant foo = "FOO";
        contract error {
            function test() public payable {
                assert(foo == "FOO");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        string constant foo = "FOO";
        contract a {
            function test(uint64 error) public payable {
                assert(error == 0);
                assert(foo == "FOO");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", 0u64.encode());
}
