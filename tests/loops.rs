
extern crate solang;

use solang::{parse_and_resolve, Target};
use solang::output;

fn first_error(errors: Vec<output::Output>) -> String {
    for m in errors.iter().filter(|m| m.level == output::Level::Error) {
        return m.message.to_owned();
    }

    panic!("no errors detected");
}

#[test]
fn test_infinite_loop() {
    let (_, errors) = parse_and_resolve(
        "contract test3 {
            // The resolver should figure out how many breaks there 
            // in the for loop; if there are none, then the basic block
            // after the loop need not be created
            function halting_problem() public returns (uint32) {
                for (;;) {
                }
                return 0;
            }
        }", &Target::Substrate);
    
    assert_eq!(first_error(errors), "unreachable statement");
}
