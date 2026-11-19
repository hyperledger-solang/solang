// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{Address, FromVal, IntoVal, TryFromVal};

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

const OTHER: &str = r#"
contract Other {
    function add(uint64 a, uint64 b) public pure returns (uint64) { return a + b; }
    function hello() public pure returns (string memory) { return "hi"; }
    function pair(uint64 x) public pure returns (uint64, bool) { return (x, true); }
    function list() public pure returns (uint64[] memory) {
        uint64[] memory a = new uint64[](2);
        a[0] = 1;
        a[1] = 2;
        return a;
    }
}"#;

const CALLER: &str = r#"
contract Caller {
    function add(Other o, uint64 a, uint64 b) public returns (uint64) { return o.add(a, b); }
    function hello(Other o) public returns (string memory) { return o.hello(); }
    function list(Other o) public returns (uint64[] memory) { return o.list(); }
    function pair(Other o, uint64 x) public returns (uint64) {
        (uint64 a, bool b) = o.pair(x);
        return b ? a + 1 : 0;
    }
}"#;

#[test]
fn typed_calls_through_contract_values() {
    // Caller comes first, so build_solidity deploys Caller
    let mut runtime = build_solidity(&format!("{CALLER}{OTHER}"), |_| {});
    let caller = runtime.contracts.last().unwrap().clone();
    let other = runtime.deploy_contract(OTHER);
    let env = &runtime.env;

    let args = vec![
        other.into_val(env),
        2_u64.into_val(env),
        3_u64.into_val(env),
    ];
    let res = runtime.invoke_contract(&caller, "add", args);
    assert_eq!(u64::from_val(env, &res), 5);

    let res = runtime.invoke_contract(&caller, "hello", vec![other.into_val(env)]);
    assert_eq!(
        soroban_sdk::String::from_val(env, &res),
        soroban_sdk::String::from_str(env, "hi")
    );

    let res = runtime.invoke_contract(&caller, "list", vec![other.into_val(env)]);
    assert_eq!(
        soroban_sdk::Vec::<u64>::from_val(env, &res),
        soroban_sdk::vec![env, 1_u64, 2]
    );

    let res = runtime.invoke_contract(
        &caller,
        "pair",
        vec![other.into_val(env), 41_u64.into_val(env)],
    );
    assert_eq!(u64::from_val(env, &res), 42);
}
