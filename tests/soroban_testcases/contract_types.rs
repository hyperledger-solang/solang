// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{Address, IntoVal, TryFromVal};

// C comes first, so build_solidity deploys C (it deploys the first contract in the file)
const SRC: &str = r#"contract C {
    Other saved;

    function echo(Other o) public pure returns (Other) {
        return o;
    }

    function to_address(Other o) public pure returns (address) {
        return address(o);
    }

    function save(Other o) public {
        saved = o;
    }

    function load() public view returns (Other) {
        return saved;
    }
}

contract Other {
    function seven() public pure returns (uint64) {
        return 7;
    }
}"#;

#[test]
fn contract_values_cross_the_boundary() {
    let mut runtime = build_solidity(SRC, |_| {});
    let c = runtime.contracts.last().unwrap().clone();
    let other = runtime.deploy_contract(
        r#"contract Other {
        function seven() public pure returns (uint64) {
            return 7;
        }
    }"#,
    );
    let env = &runtime.env;
    let arg = || vec![other.into_val(env)];

    // in as a parameter, out as a return value
    let res = runtime.invoke_contract(&c, "echo", arg());
    assert_eq!(Address::try_from_val(env, &res).unwrap(), other);

    // address(o) is the same address
    let res = runtime.invoke_contract(&c, "to_address", arg());
    assert_eq!(Address::try_from_val(env, &res).unwrap(), other);

    // into storage and back
    runtime.invoke_contract(&c, "save", arg());
    let res = runtime.invoke_contract(&c, "load", vec![]);
    assert_eq!(Address::try_from_val(env, &res).unwrap(), other);
}
