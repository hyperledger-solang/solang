// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use indexmap::Equivalent;
use soroban_sdk::{testutils::Address as _, Address, FromVal, IntoVal, String, Val};

#[test]
fn constructor_profile_test() {
    let mut runtime = SorobanEnv::new();

    let user = Address::generate(&runtime.env);
    let name = String::from_str(&runtime.env, "Alice");
    let age: Val = 30_u32.into_val(&runtime.env);

    let contract_src = r#"
        contract profile {
            address public user;
            string public name;
            uint32 public age;

            constructor(address _user, string memory _name, uint32 _age) {
                user = _user;
                name = _name;
                age = _age;
            }
        }
    "#;

    let addr = runtime.deploy_contract_with_args(contract_src, (user.clone(), name.clone(), age));

    let user_ret = runtime.invoke_contract(&addr, "user", vec![]);
    let name_ret = runtime.invoke_contract(&addr, "name", vec![]);
    let age_ret = runtime.invoke_contract(&addr, "age", vec![]);

    let expected_user = Address::from_val(&runtime.env, &user_ret);
    assert!(expected_user.equivalent(&user));

    let expected_name = String::from_val(&runtime.env, &name_ret);
    assert!(expected_name.equivalent(&name));

    let expected_age: u32 = FromVal::from_val(&runtime.env, &age_ret);
    assert_eq!(expected_age, 30);
}
