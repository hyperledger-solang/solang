use crate::{build_solidity, first_error, parse_and_resolve};
use parity_scale_codec::Encode;
use solang::Target;

#[test]
fn test_infinite_loop() {
    let ns = parse_and_resolve(
        "contract test3 {
            // The resolver should figure out how many breaks there
            // in the for loop; if there are none, then the basic block
            // after the loop need not be created
            function halting_problem() public returns (uint32) {
                for (;;) {
                }
                return 0;
            }
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "unreachable statement");
}

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
