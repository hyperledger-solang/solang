// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, String as SString, TryFromVal, Vec as SVec};

#[test]
fn hello_world() {
    let runtime = build_solidity(
        r#"
        contract HelloWorld {
            function hello(string memory to) public pure returns (string[] memory) {
                string[] memory res = new string[](2);
                res[0] = "Hello";
                res[1] = to;
                return res;
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let to = SString::from_str(env, "Soroban");
    let res = runtime.invoke_contract(addr, "hello", vec![to.into_val(env)]);
    let got = SVec::<SString>::try_from_val(env, &res).unwrap();
    assert_eq!(
        got,
        soroban_sdk::vec![
            env,
            SString::from_str(env, "Hello"),
            SString::from_str(env, "Soroban"),
        ]
    );
}
