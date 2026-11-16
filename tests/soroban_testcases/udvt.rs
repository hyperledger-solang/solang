// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal};

#[test]
fn user_defined_value_types_cross_the_boundary() {
    let runtime = build_solidity(
        r#"type Price is int128;
        contract C {
            Price saved;
            function echo(Price p) public pure returns (Price) { return p; }
            function double(Price p) public pure returns (Price) {
                return Price.wrap(Price.unwrap(p) * 2);
            }
            function save(Price p) public { saved = p; }
            function load() public view returns (Price) { return saved; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "echo", vec![(-7_i128).into_val(env)]);
    assert_eq!(i128::from_val(env, &res), -7);

    let res = runtime.invoke_contract(addr, "double", vec![21_i128.into_val(env)]);
    assert_eq!(i128::from_val(env, &res), 42);

    runtime.invoke_contract(addr, "save", vec![5_i128.into_val(env)]);
    let res = runtime.invoke_contract(addr, "load", vec![]);
    assert_eq!(i128::from_val(env, &res), 5);
}
