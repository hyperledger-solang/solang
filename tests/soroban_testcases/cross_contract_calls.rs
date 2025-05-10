// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn simple_cross_contract() {
    let mut runtime = build_solidity(
        r#"contract math {
        function max(uint64 a, uint64 b) public returns (uint64) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    }"#,
        |_| {},
    );

    let caller = runtime.deploy_contract(
        r#"contract mcaller {
    function call_max(
        address addr,
        uint64 a,
        uint64 b
    ) public returns (uint64) {
        bytes payload = abi.encode("max", a, b);
        (bool success, bytes returndata) = addr.call(payload);
        uint64 result = abi.decode(returndata, (uint64));
        return result;
    }
}
"#,
    );

    let arg: Val = 3_u64.into_val(&runtime.env);
    let arg2: Val = 1_u64.into_val(&runtime.env);

    let addr = runtime.contracts.first().unwrap();
    let res = runtime.invoke_contract(addr, "max", vec![arg, arg2]);
    println!("first res {:?}", res);

    let expected: Val = 3_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(
        &caller,
        "call_max",
        vec![addr.into_val(&runtime.env), arg, arg2],
    );

    println!("second res {:?}", res);
    assert!(expected.shallow_eq(&res));
}
