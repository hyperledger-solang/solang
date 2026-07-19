// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{contracttype, FromVal, IntoVal};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub count: u32,
    pub last_incr: u32,
}

const CONTRACT: &str = r#"
    contract CustomTypes {
        struct State {
            uint32 count;
            uint32 last_incr;
        }
        State state;

        function increment(uint32 incr) public returns (uint32) {
            state.count += incr;
            state.last_incr = incr;
            return state.count;
        }

        function get_state() public view returns (State memory) {
            return state;
        }
    }
"#;

#[test]
fn example_custom_types_increment_accumulates() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(addr, "increment", vec![1_u32.into_val(env)]),
    );
    assert_eq!(ret, 1);

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(addr, "increment", vec![10_u32.into_val(env)]),
    );
    assert_eq!(ret, 11);
}

#[test]
fn example_custom_types_get_state_reflects_increments() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    runtime.invoke_contract(addr, "increment", vec![1_u32.into_val(env)]);
    runtime.invoke_contract(addr, "increment", vec![10_u32.into_val(env)]);

    let s = State::from_val(env, &runtime.invoke_contract(addr, "get_state", vec![]));
    assert_eq!(
        s,
        State {
            count: 11,
            last_incr: 10
        }
    );
}

#[test]
fn example_custom_types_initial_state_is_zero() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let s = State::from_val(env, &runtime.invoke_contract(addr, "get_state", vec![]));
    assert_eq!(
        s,
        State {
            count: 0,
            last_incr: 0
        }
    );
}
