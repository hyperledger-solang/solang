// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity_with_options;

#[test]
fn debug_buffer_format() {
    let mut runtime = build_solidity_with_options(
        r#"contract DebugBuffer {
            function multiple_prints() public {
                print("Hello!");
                print("I call seal_debug_message under the hood!");
            }

            function multiple_prints_then_revert() public {
                print("Hello!");
                print("I call seal_debug_message under the hood!");
                revert("sesa!!!");
            }
        }
    "#,
        true,
        true,
    );

    runtime.function("multiple_prints", [].to_vec());
    assert_eq!(
        runtime.debug_buffer(),
        r#"print: Hello!,
call: seal_debug_message=0,
print: I call seal_debug_message under the hood!,
call: seal_debug_message=0,
"#
    );

    runtime.function_expect_failure("multiple_prints_then_revert", [].to_vec());
    assert_eq!(
        runtime.debug_buffer(),
        r#"print: Hello!,
call: seal_debug_message=0,
print: I call seal_debug_message under the hood!,
call: seal_debug_message=0,
runtime_error: sesa!!! revert encountered in test.sol:10:17-34,
call: seal_debug_message=0,
"#
    );
}
