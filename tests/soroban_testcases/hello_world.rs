// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::Logs;
use soroban_sdk::{IntoVal, String, Vec};

#[test]
#[should_panic(expected = "unsupported return type Array")]
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

    let to_str = String::from_str(&runtime.env, "Dev");
    let res = runtime.invoke_contract(addr, "hello", vec![to_str.into_val(&runtime.env)]);

    let vec_res: Vec<String> = res.into_val(&runtime.env);
    println!("Logs: {:?}", runtime.env.logs().all());
    assert_eq!(vec_res.len(), 2);

    // Check elements
    let _str0 = vec_res.get(0).unwrap();
    let _str1 = vec_res.get(1).unwrap();
}
