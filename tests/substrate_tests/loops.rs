use crate::{first_error, parse_and_resolve};
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
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "unreachable statement");
}
