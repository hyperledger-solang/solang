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
            (bool ok, bytes memory ret) = pause_contract.call(payload);

            bool is_paused = abi.decode(ret, (bool));
            require(!is_paused, "Paused");

            count += 1;
            return count;
        }
    }
"#;

#[test]
fn increment_with_pause_counts_up() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));

    let ret: u32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 1);

    let ret: u32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 2);

    let ret: u32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 3);
}

#[test]
#[should_panic]
fn increment_traps_when_paused() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));

    let paused_val: Val = true.into_val(&runtime.env);
    runtime.invoke_contract(&pause_addr, "set", vec![paused_val]);

    runtime.invoke_contract(&inc_addr, "increment", vec![]);
}

#[test]
fn increment_resumes_after_unpause() {
    let mut runtime = build_solidity(PAUSE_SRC, |_| {});
    let pause_addr = runtime.contracts.last().unwrap().clone();
    let inc_addr = runtime.deploy_contract_with_args(INCREMENT_SRC, (pause_addr.clone(),));

    let ret: u32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 1);

    runtime.invoke_contract(&pause_addr, "set", vec![true.into_val(&runtime.env)]);

    runtime.invoke_contract(&pause_addr, "set", vec![false.into_val(&runtime.env)]);

    let ret: u32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(&inc_addr, "increment", vec![]),
    );
    assert_eq!(ret, 2);
}
