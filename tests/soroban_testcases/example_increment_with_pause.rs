// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal, Val};

const PAUSE_SRC: &str = r#"
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

const INCREMENT_SRC: &str = r#"
    contract IncrementContract {
        address public instance pause_contract;
        uint32 public instance count = 0;

        constructor(address _pause) {
            pause_contract = _pause;
        }

        function increment() public returns (uint32) {
            bytes payload = abi.encode("paused");
            (, bytes memory ret) = pause_contract.call(payload);
            bool is_paused = abi.decode(ret, (bool));
            require(!is_paused, "Paused");

            count += 1;
            extendInstanceTtl(50, 100);
            return count;
        }
    }
"#;

#[test]
fn example_increment_with_pause_counts_up() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));
    let env = &runtime.env;

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 1);

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 2);

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 3);
}

#[test]
#[should_panic]
fn example_increment_with_pause_traps_when_paused() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));
    let env = &runtime.env;
    let paused_val: Val = true.into_val(env);
    runtime.invoke_contract(&pause_addr, "set", vec![paused_val]);
    runtime.invoke_contract(&inc_addr, "increment", vec![]);
}

#[test]
fn example_increment_with_pause_resumes_after_unpause() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));
    let env = &runtime.env;

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 1);

    runtime.invoke_contract(&pause_addr, "set", vec![true.into_val(env)]);
    runtime.invoke_contract(&pause_addr, "set", vec![false.into_val(env)]);

    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 2);
}

#[test]
fn example_increment_with_pause_extends_instance_ttl() {
    use soroban_sdk::testutils::storage::Instance;
    use soroban_sdk::testutils::Ledger;

    let mut runtime = build_solidity(PAUSE_SRC, |env| {
        env.env.ledger().with_mut(|li| {
            li.sequence_number = 100_000;
            li.min_persistent_entry_ttl = 10;
            li.max_entry_ttl = 200;
        });
    });
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));
    let env = &runtime.env;

    env.as_contract(&inc_addr, || {
        assert_eq!(env.storage().instance().get_ttl(), 9);
    });

    runtime.invoke_contract(&inc_addr, "increment", vec![]);

    env.as_contract(&inc_addr, || {
        assert_eq!(env.storage().instance().get_ttl(), 100);
    });
}
