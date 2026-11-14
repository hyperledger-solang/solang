// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, build_wasm};
use soroban_sdk::xdr::{Limited, Limits, ReadXdr, ScSpecEntry, ScSpecTypeDef, ScSpecTypeVec};
use soroban_sdk::{contracttype, Bytes, FromVal, IntoVal, String, Symbol, TryFromVal, Val, Vec};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: u64,
    pub flag: bool,
}

const SRC: &str = r#"contract test {
    struct Item {
        uint64 id;
        bool flag;
    }

    function pair() public pure returns (uint64, bool) {
        return (7, true);
    }

    function swap(uint64 a, uint64 b) public pure returns (uint64, uint64) {
        return (b, a);
    }

    function mixed(int128 x) public pure returns (int128, string memory, uint32) {
        return (-x, "hello", 42);
    }

    function named() public pure returns (uint64 a, bool b, bytes memory c) {
        a = 3;
        b = false;
        c = hex"beef";
    }

    function composite(uint64 n) public pure returns (Item memory, uint64[] memory, uint64) {
        uint64[] memory values = new uint64[](n);
        for (uint64 i = 0; i < n; i++) {
            values[i] = i * 10;
        }
        return (Item({ id: n, flag: true }), values, n + 1);
    }

    function checked(uint64 x) public pure returns (uint64, uint64) {
        require(x > 0, "x must be positive");
        return (x, x * 2);
    }

    function internal_tuple() public pure returns (uint64, uint64) {
        (uint64 a, uint64 b) = helper();
        return (a + 1, b + 1);
    }

    function helper() internal pure returns (uint64, uint64) {
        return (10, 20);
    }
}"#;

fn as_vec(runtime: &crate::SorobanEnv, val: Val) -> Vec<Val> {
    Vec::<Val>::try_from_val(&runtime.env, &val).expect("multiple returns must come back as a Vec")
}

#[test]
fn values_come_back_as_vec_in_order() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = as_vec(&runtime, runtime.invoke_contract(addr, "pair", vec![]));
    assert_eq!(res.len(), 2);
    assert_eq!(u64::from_val(env, &res.get(0).unwrap()), 7);
    assert!(bool::from_val(env, &res.get(1).unwrap()));

    let args = vec![1_u64.into_val(env), 2_u64.into_val(env)];
    let res = as_vec(&runtime, runtime.invoke_contract(addr, "swap", args));
    assert_eq!(res.len(), 2);
    assert_eq!(u64::from_val(env, &res.get(0).unwrap()), 2);
    assert_eq!(u64::from_val(env, &res.get(1).unwrap()), 1);
}

#[test]
fn values_of_different_types() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "mixed", vec![5_i128.into_val(env)]);
    let (a, b, c) = <(i128, String, u32)>::try_from_val(env, &res).unwrap();
    assert_eq!(a, -5);
    assert_eq!(b, String::from_str(env, "hello"));
    assert_eq!(c, 42);

    let res = runtime.invoke_contract(addr, "named", vec![]);
    let (a, b, c) = <(u64, bool, Bytes)>::try_from_val(env, &res).unwrap();
    assert_eq!(a, 3);
    assert!(!b);
    assert_eq!(c, Bytes::from_slice(env, &[0xbe, 0xef]));
}

#[test]
fn struct_and_array_among_values() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "composite", vec![3_u64.into_val(env)]);
    let (item, values, next) = <(Item, Vec<u64>, u64)>::try_from_val(env, &res).unwrap();
    assert_eq!(item, Item { id: 3, flag: true });
    assert_eq!(values, soroban_sdk::vec![env, 0_u64, 10, 20]);
    assert_eq!(next, 4);
}

#[test]
fn revert_returns_error() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "checked", vec![4_u64.into_val(env)]);
    assert_eq!(<(u64, u64)>::try_from_val(env, &res).unwrap(), (4, 8));

    let args = soroban_sdk::vec![env, 0_u64.into_val(env)];
    let res = env.try_invoke_contract::<Val, Val>(addr, &Symbol::new(env, "checked"), args);
    assert!(res.is_err());
    let logs = runtime.invoke_contract_expect_error(addr, "checked", vec![0_u64.into_val(env)]);
    assert!(
        logs.iter()
            .any(|log| log.contains("require condition failed")),
        "{logs:?}"
    );
}

#[test]
fn internal_calls_still_destructure() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "internal_tuple", vec![]);
    assert_eq!(<(u64, u64)>::try_from_val(env, &res).unwrap(), (11, 21));
}

fn spec_outputs(wasm: &[u8], name: &str) -> std::vec::Vec<ScSpecTypeDef> {
    let section = wasmparser::Parser::new(0)
        .parse_all(wasm)
        .find_map(|payload| match payload.unwrap() {
            wasmparser::Payload::CustomSection(c) if c.name() == "contractspecv0" => {
                Some(c.data().to_vec())
            }
            _ => None,
        })
        .expect("contractspecv0 section");

    let mut reader = Limited::new(section.as_slice(), Limits::none());
    for entry in ScSpecEntry::read_xdr_iter(&mut reader) {
        if let ScSpecEntry::FunctionV0(f) = entry.unwrap() {
            if f.name.to_utf8_string_lossy() == name {
                return f.outputs.to_vec();
            }
        }
    }
    panic!("function {name} not in spec");
}

#[test]
fn spec_has_one_tuple_output() {
    let (wasm, _) = build_wasm(SRC);

    let tuple = |outputs: std::vec::Vec<ScSpecTypeDef>| match outputs.as_slice() {
        [ScSpecTypeDef::Tuple(t)] => t.value_types.to_vec(),
        other => panic!("expected one tuple output, got {other:?}"),
    };

    assert_eq!(
        tuple(spec_outputs(&wasm, "pair")),
        vec![ScSpecTypeDef::U64, ScSpecTypeDef::Bool]
    );
    assert_eq!(
        tuple(spec_outputs(&wasm, "mixed")),
        vec![
            ScSpecTypeDef::I128,
            ScSpecTypeDef::String,
            ScSpecTypeDef::U32
        ]
    );

    let composite = tuple(spec_outputs(&wasm, "composite"));
    assert_eq!(composite.len(), 3);
    assert!(
        matches!(&composite[0], ScSpecTypeDef::Udt(u) if u.name.to_utf8_string_lossy() == "Item")
    );
    assert_eq!(
        composite[1],
        ScSpecTypeDef::Vec(Box::new(ScSpecTypeVec {
            element_type: Box::new(ScSpecTypeDef::U64)
        }))
    );
    assert_eq!(composite[2], ScSpecTypeDef::U64);
}

#[test]
#[ignore = "calls through internal function pointers are not supported on Soroban yet, even with one return value"]
fn function_pointer_call_returns_all_values() {
    let runtime = build_solidity(
        r#"contract test {
            function first(uint64 x) internal pure returns (uint64, uint64) {
                return (x, x + 1);
            }

            function second(uint64 x) internal pure returns (uint64, uint64) {
                return (x * 2, x * 3);
            }

            function via_pointer(uint64 x, bool pick_first) public pure returns (uint64, uint64) {
                function(uint64) internal pure returns (uint64, uint64) f = second;
                if (pick_first) {
                    f = first;
                }
                (uint64 a, uint64 b) = f(x);
                return (a, b);
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let args = vec![5_u64.into_val(env), true.into_val(env)];
    let res = runtime.invoke_contract(addr, "via_pointer", args);
    assert_eq!(<(u64, u64)>::try_from_val(env, &res).unwrap(), (5, 6));

    let args = vec![5_u64.into_val(env), false.into_val(env)];
    let res = runtime.invoke_contract(addr, "via_pointer", args);
    assert_eq!(<(u64, u64)>::try_from_val(env, &res).unwrap(), (10, 15));
}
