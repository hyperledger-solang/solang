// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::Encode;

#[test]
fn for_loop_no_cond_or_next() {
    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo(bool x) public {
                for (;;) {
                    if (x)
                        break;
                }
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", true.encode());
}
