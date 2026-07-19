// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal};

const CONTRACT: &str = r#"
    contract Pause {
        bool instance paused_flag = false;

        function paused() public view returns (bool) {
            return paused_flag;
        }

        function set(bool paused) public {
            paused_flag = paused;
        }
    }
"#;

#[test]
fn example_pause_initial_state_is_not_paused() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let is_paused: bool = FromVal::from_val(env, &runtime.invoke_contract(addr, "paused", vec![]));
    assert!(!is_paused);
}

#[test]
fn example_pause_set_paused_then_unpaused() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    runtime.invoke_contract(addr, "set", vec![true.into_val(env)]);
    let is_paused: bool = FromVal::from_val(env, &runtime.invoke_contract(addr, "paused", vec![]));
    assert!(is_paused);

    runtime.invoke_contract(addr, "set", vec![false.into_val(env)]);
    let is_paused: bool = FromVal::from_val(env, &runtime.invoke_contract(addr, "paused", vec![]));
    assert!(!is_paused);
}
