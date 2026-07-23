// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::{FromVal, IntoVal, Val};

const CONTRACT: &str = r#"
    contract IncrementContract {
        uint32 public instance count = 0;
        event IncrementEvent(string indexed action, string indexed method, uint32 count);

        function increment() public returns (uint32) {
            count += 1;
            emit IncrementEvent("COUNTER", "increment", count);
            return count;
        }
    }
"#;

#[test]
fn example_events_increment_accumulates() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let ret: u32 = FromVal::from_val(env, &runtime.invoke_contract(addr, "increment", vec![]));
    assert_eq!(ret, 1);

    let ret: u32 = FromVal::from_val(env, &runtime.invoke_contract(addr, "increment", vec![]));
    assert_eq!(ret, 2);

    let ret: u32 = FromVal::from_val(env, &runtime.invoke_contract(addr, "increment", vec![]));
    assert_eq!(ret, 3);
}

#[test]
fn example_events_emits_correct_topic_and_data() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    for expected_count in 1_u32..=3 {
        runtime.invoke_contract(addr, "increment", vec![]);

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, data) = events.get(0).unwrap();

        assert_eq!(topics.len(), 2);
        let action: soroban_sdk::String =
            soroban_sdk::String::from_val(env, &topics.get(0).unwrap());
        assert_eq!(action.to_string(), "COUNTER");
        let method: soroban_sdk::String =
            soroban_sdk::String::from_val(env, &topics.get(1).unwrap());
        assert_eq!(method.to_string(), "increment");
        let expected_data: Val = expected_count.into_val(env);
        assert!(expected_data.shallow_eq(&data));
    }
}
