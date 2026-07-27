// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn pause_contract() {
    let src = build_solidity(
        r#"contract Pause {
            bool private paused;

            function is_paused() public view returns (bool) {
                return paused;
            }

            function pause() public {
                paused = true;
            }

            function unpause() public {
                paused = false;
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();
    
    // Check initial state
    let res = src.invoke_contract(addr, "is_paused", vec![]);
    let expected: Val = false.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    // Pause
    src.invoke_contract(addr, "pause", vec![]);
    
    // Check paused state
    let res = src.invoke_contract(addr, "is_paused", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    // Unpause
    src.invoke_contract(addr, "unpause", vec![]);

    // Check unpaused state
    let res = src.invoke_contract(addr, "is_paused", vec![]);
    let expected: Val = false.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}
