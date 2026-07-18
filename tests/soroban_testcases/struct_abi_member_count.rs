// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{
    contracttype, testutils::Address as _, Address, Bytes, FromVal, IntoVal, String,
};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S1 {
    pub a: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S2 {
    pub a: u64,
    pub b: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S3 {
    pub a: u64,
    pub b: bool,
    pub c: i32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S4 {
    pub a: u64,
    pub b: bool,
    pub c: i32,
    pub d: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S5 {
    pub a: u64,
    pub b: bool,
    pub c: i32,
    pub d: Address,
    pub e: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S6 {
    pub a: u64,
    pub b: bool,
    pub c: i32,
    pub d: Address,
    pub e: Bytes,
    pub f: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S7 {
    pub a: u64,
    pub b: bool,
    pub c: i32,
    pub d: Address,
    pub e: Bytes,
    pub f: String,
    pub g: Bytes,
}

const FNS: &str = r#"
            function echo(S memory s) public pure returns (S memory) { return s; }
            function via_local(S memory s) public pure returns (S memory) {
                S memory t = s;
                return t;
            }
"#;

#[test]
fn struct_1_member() {
    let runtime = build_solidity(
        &format!(r#"contract test {{ struct S {{ uint64 a; }} {FNS} }}"#,),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S1 {
        a: 0x0123_4567_89AB_CDEF,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S1::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S1::from_val(env, &res), input);
}

#[test]
fn struct_2_member() {
    let runtime = build_solidity(
        &format!(r#"contract test {{ struct S {{ uint64 a; bool b; }} {FNS} }}"#,),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S2 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S2::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S2::from_val(env, &res), input);
}

#[test]
fn struct_3_member() {
    let runtime = build_solidity(
        &format!(r#"contract test {{ struct S {{ uint64 a; bool b; int32 c; }} {FNS} }}"#,),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S3 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
        c: -12345,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S3::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S3::from_val(env, &res), input);
}

#[test]
fn struct_4_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 a; bool b; int32 c; address d; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S4 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
        c: -12345,
        d: Address::generate(env),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S4::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S4::from_val(env, &res), input);
}

#[test]
fn struct_5_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 a; bool b; int32 c; address d; bytes4 e; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S5 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
        c: -12345,
        d: Address::generate(env),
        e: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S5::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S5::from_val(env, &res), input);
}

#[test]
fn struct_6_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 a; bool b; int32 c; address d; bytes4 e; string f; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S6 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
        c: -12345,
        d: Address::generate(env),
        e: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
        f: String::from_str(env, "Solang!"),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S6::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S6::from_val(env, &res), input);
}

#[test]
fn struct_7_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 a; bool b; int32 c; address d; bytes4 e; string f; bytes g; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S7 {
        a: 0x0123_4567_89AB_CDEF,
        b: true,
        c: -12345,
        d: Address::generate(env),
        e: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
        f: String::from_str(env, "Solang!"),
        g: Bytes::from_array(env, &[0x01, 0x02, 0x03]),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S7::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S7::from_val(env, &res), input);
}
