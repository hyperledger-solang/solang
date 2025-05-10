// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{
    symbol_short,
    testutils::{AuthorizedFunction, AuthorizedInvocation},
    Address, IntoVal, Val,
};

#[test]
fn require_auth() {
    let runtime = build_solidity(
        r#"contract auth {

    address public owner = address"GDRIX624OGPQEX264NY72UKOJQUASHU3PYKL6DDPGSTWXWJSBOTR6N7W";

 
    uint64 public instance counter = 20;

    function increment() public returns (uint64) {

        owner.requireAuth();

        counter = counter + 1;

        return counter;
       
    }
} "#,
        |_| {},
    );

    runtime.env.mock_all_auths();

    let authed_addr = Address::from_str(
        &runtime.env,
        "GDRIX624OGPQEX264NY72UKOJQUASHU3PYKL6DDPGSTWXWJSBOTR6N7W",
    );

    let addr = runtime.contracts.first().unwrap();
    let res = runtime.invoke_contract(addr, "increment", vec![]);
    let expected: Val = 21_u64.into_val(&runtime.env);

    assert!(expected.shallow_eq(&res));

    let auths = runtime.env.auths();

    let authed_invokation = AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            addr.clone(),
            symbol_short!("increment"),
            soroban_sdk::vec![&runtime.env],
        )),
        sub_invocations: vec![],
    };

    assert_eq!(auths, vec![(authed_addr.clone(), authed_invokation)]);
}

/// This is a demo of a deeper chain of cross contract calls.
/// A -> B -> C. The auth_as_curr_contract only takes the C contract invokation
/// Because the B call is authorized by default.
/// A soroban example could be found at: https://github.com/stellar/soroban-examples/blob/main/deep_contract_auth/src/lib.rs
#[test]
fn auth_as_curr_contract() {
    let mut runtime = build_solidity(
        r#"contract a {
    function call_b (address b, address c) public returns (uint64) {
        address addr = address(this);
        // authorize contract c to be called, with function name "get_num" and "a" as an arg.
        // get_num calls a.require_auth()
        auth.authAsCurrContract(c, "get_num", addr);
        bytes payload = abi.encode("increment", addr, c);
        (bool suc, bytes returndata) = b.call(payload);
        uint64 result = abi.decode(returndata, (uint64));
        return result;
    }
}"#,
        |_| {},
    );

    let b = runtime.deploy_contract(
        r#"contract b {
 
    uint64 public instance counter = 20;

    function increment(address a, address c) public returns (uint64) {

        a.requireAuth();
        bytes payload = abi.encode("get_num", a);
        (bool suc, bytes returndata) = c.call(payload);
        uint64 result = abi.decode(returndata, (uint64));

        counter = counter + 2;

        return counter;
       
    }
} "#,
    );

    let c = runtime.deploy_contract(
        r#"contract c {
    function get_num(address a) public returns (uint64) {
        a.requireAuth();
        return 2;
    }
}"#,
    );

    // same as a, but with the auth_as_curr_contract line commented out.
    let a_invalid = runtime.deploy_contract(
        r#"contract a {
    function call_b (address b, address c) public returns (uint64) {
        address addr = address(this);
        // authorize contract c to be called, with function name "get_num" and "a" as an arg.
        // get_num calls a.require_auth()
        // auth.authAsCurrContract(c, "get_num", addr);
        bytes payload = abi.encode("increment", addr, c);
        (bool suc, bytes returndata) = b.call(payload);
        uint64 result = abi.decode(returndata, (uint64));
        return result;
    }
}"#,
    );

    let a = &runtime.contracts[0];

    let ret = runtime.invoke_contract(
        a,
        "call_b",
        vec![b.into_val(&runtime.env), c.into_val(&runtime.env)],
    );

    let expected: Val = 22_u64.into_val(&runtime.env);

    assert!(expected.shallow_eq(&ret));

    let errors = runtime.invoke_contract_expect_error(
        &a_invalid,
        "call_b",
        vec![b.into_val(&runtime.env), c.into_val(&runtime.env)],
    );

    assert!(errors[0].contains("Failed Diagnostic Event (not emitted)] contract:CAJXGFIU32R2SF4BVXV2EB2XSSUPUBQMNXWJWB5GYS7WE76TFPPR7Q7P, topics:[log], data:[\"VM call trapped with HostError\", get_num, Error(Auth, InvalidAction)"))
}
