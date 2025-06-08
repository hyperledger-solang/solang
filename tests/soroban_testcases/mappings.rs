// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{testutils::Address as _, Address, IntoVal, Val};

#[test]
fn balance_and_allowance_test() {
    let runtime = build_solidity(
        r#"
        contract mapper {
            mapping(address => uint64) public balances;
            mapping(address => mapping(address => uint64)) public allowances;

            function setBalance(address addr, uint64 amount) public {
                balances[addr] = amount;
            }

            function getBalance(address addr) public view returns (uint64) {
                return balances[addr];
            }

            function setAllowance(address owner, address spender, uint64 amount) public {
                allowances[owner][spender] = amount;
            }

            function getAllowance(address owner, address spender) public view returns (uint64) {
                return allowances[owner][spender];
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let user1 = Address::generate(&runtime.env);
    let user2 = Address::generate(&runtime.env);

    let bal: Val = 100_u64.into_val(&runtime.env);
    let get_args = vec![user1.clone().into_val(&runtime.env)];

    // Set and get balance
    let set_args = vec![user1.clone().into_val(&runtime.env), bal];
    runtime.invoke_contract(addr, "setBalance", set_args);
    let res = runtime.invoke_contract(addr, "getBalance", get_args.clone());
    assert!(bal.shallow_eq(&res));

    // Set and get allowance
    let allowance_val: Val = 77_u64.into_val(&runtime.env);
    let set_allow_args = vec![
        user1.clone().into_val(&runtime.env),
        user2.clone().into_val(&runtime.env),
        allowance_val,
    ];
    runtime.invoke_contract(addr, "setAllowance", set_allow_args);

    let get_allow_args = vec![user1.into_val(&runtime.env), user2.into_val(&runtime.env)];
    let res = runtime.invoke_contract(addr, "getAllowance", get_allow_args);
    assert!(allowance_val.shallow_eq(&res));
}
