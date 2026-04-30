// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::IntoVal;

#[test]
fn emit_event_no_topics() {
    let src = build_solidity(
        r#"contract EventEmitter {
            event Transfer(uint64 value);

            function doTransfer(uint64 value) public {
                emit Transfer(value);
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();
    src.invoke_contract(addr, "doTransfer", vec![42_u64.into_val(&src.env)]);

    let events = src.env.events().all();
    assert_eq!(events.len(), 1);
}

#[test]
fn emit_event_with_topic() {
    let src = build_solidity(
        r#"contract EventEmitter {
            event Transfer(uint64 indexed from, uint64 value);

            function doTransfer(uint64 from, uint64 value) public {
                emit Transfer(from, value);
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();
    src.invoke_contract(
        addr,
        "doTransfer",
        vec![1_u64.into_val(&src.env), 42_u64.into_val(&src.env)],
    );

    let events = src.env.events().all();
    assert_eq!(events.len(), 1);
    let (_, topics, _) = &events.get(0).unwrap();
    assert_eq!(topics.len(), 1);
}
