// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{
    contracttype, testutils::Address as _, Address, Bytes, BytesN, FromVal, IntoVal, String, I256,
    U256,
};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SBool {
    pub v: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SU32 {
    pub v: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SU64 {
    pub v: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SU128 {
    pub v: u128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SU256 {
    pub v: U256,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SI32 {
    pub v: i32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SI64 {
    pub v: i64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SI128 {
    pub v: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SI256 {
    pub v: I256,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SAddr {
    pub v: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SBytesN {
    pub v: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SString {
    pub v: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SDynBytes {
    pub v: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SInner {
    pub v: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SOuter {
    pub v: SInner,
}

#[test]
fn single_bool_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { bool v; }
            function make() public pure returns (S memory) { return S(true); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function flip(S memory s) public pure returns (S memory) { s.v = !s.v; return s; }
            function via_local(S memory s) public pure returns (S memory) {
                bool b = s.v;
                b = !b;
                return S(b);
            }
            function combine(S memory s, bool o) public pure returns (S memory) {
                return S(s.v && o);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SBool::from_val(env, &res), SBool { v: true });

    for input in [SBool { v: true }, SBool { v: false }] {
        let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
        assert_eq!(SBool::from_val(env, &res), input);
    }

    let res = runtime.invoke_contract(addr, "flip", vec![SBool { v: true }.into_val(env)]);
    assert_eq!(SBool::from_val(env, &res), SBool { v: false });

    let res = runtime.invoke_contract(addr, "via_local", vec![SBool { v: true }.into_val(env)]);
    assert_eq!(SBool::from_val(env, &res), SBool { v: false });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SBool { v: true }.into_val(env), true.into_val(env)],
    );
    assert_eq!(SBool::from_val(env, &res), SBool { v: true });
    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SBool { v: true }.into_val(env), false.into_val(env)],
    );
    assert_eq!(SBool::from_val(env, &res), SBool { v: false });
}

#[test]
fn single_uint32_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { uint32 v; }
            function make() public pure returns (S memory) { return S(12345); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                uint32 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, uint32 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 12345 });

    let input = SU32 { v: 99999 };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "twice", vec![SU32 { v: 1000 }.into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 2000 });

    let res = runtime.invoke_contract(addr, "inc_local", vec![SU32 { v: 1000 }.into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 1001 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SU32 { v: 1000 }.into_val(env), 5u32.into_val(env)],
    );
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 1005 });
}

#[test]
fn single_uint64_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { uint64 v; }
            function make() public pure returns (S memory) { return S(1000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                uint64 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, uint64 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SU64::from_val(env, &res), SU64 { v: 1000 });

    let input = SU64 {
        v: 0x0123_4567_89AB_CDEF,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SU64::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "twice", vec![SU64 { v: 1_000_000_000 }.into_val(env)]);
    assert_eq!(SU64::from_val(env, &res), SU64 { v: 2_000_000_000 });

    let res = runtime.invoke_contract(
        addr,
        "inc_local",
        vec![SU64 { v: 1_000_000_000 }.into_val(env)],
    );
    assert_eq!(SU64::from_val(env, &res), SU64 { v: 1_000_000_001 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SU64 { v: 1_000_000_000 }.into_val(env), 5u64.into_val(env)],
    );
    assert_eq!(SU64::from_val(env, &res), SU64 { v: 1_000_000_005 });
}

#[test]
fn single_uint128_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { uint128 v; }
            function make() public pure returns (S memory) { return S(1000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                uint128 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, uint128 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SU128::from_val(env, &res), SU128 { v: 1000 });

    let big = 100_000_000_000_000_000_000u128;
    let res = runtime.invoke_contract(addr, "echo", vec![SU128 { v: big }.into_val(env)]);
    assert_eq!(SU128::from_val(env, &res), SU128 { v: big });

    let res = runtime.invoke_contract(addr, "twice", vec![SU128 { v: big }.into_val(env)]);
    assert_eq!(SU128::from_val(env, &res), SU128 { v: big * 2 });

    let res = runtime.invoke_contract(addr, "inc_local", vec![SU128 { v: big }.into_val(env)]);
    assert_eq!(SU128::from_val(env, &res), SU128 { v: big + 1 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SU128 { v: big }.into_val(env), 50u128.into_val(env)],
    );
    assert_eq!(SU128::from_val(env, &res), SU128 { v: big + 50 });
}

#[test]
fn single_uint256_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { uint256 v; }
            function make() public pure returns (S memory) { return S(100000000000000000000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                uint256 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, uint256 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let u256 = |n: u128| U256::from_u128(env, n);

    let big = 100_000_000_000_000_000_000u128;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SU256::from_val(env, &res), SU256 { v: u256(big) });

    let res = runtime.invoke_contract(addr, "echo", vec![SU256 { v: u256(big) }.into_val(env)]);
    assert_eq!(SU256::from_val(env, &res), SU256 { v: u256(big) });

    let res = runtime.invoke_contract(addr, "twice", vec![SU256 { v: u256(big) }.into_val(env)]);
    assert_eq!(SU256::from_val(env, &res), SU256 { v: u256(big * 2) });

    let res = runtime.invoke_contract(
        addr,
        "inc_local",
        vec![SU256 { v: u256(big) }.into_val(env)],
    );
    assert_eq!(SU256::from_val(env, &res), SU256 { v: u256(big + 1) });

    let d = 50_000_000_000_000_000_000u128;
    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SU256 { v: u256(big) }.into_val(env), u256(d).into_val(env)],
    );
    assert_eq!(SU256::from_val(env, &res), SU256 { v: u256(big + d) });
}

#[test]
fn single_int32_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { int32 v; }
            function make() public pure returns (S memory) { return S(-123); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                int32 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, int32 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SI32::from_val(env, &res), SI32 { v: -123 });

    let input = SI32 { v: -123_456 };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SI32::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "twice", vec![SI32 { v: -1000 }.into_val(env)]);
    assert_eq!(SI32::from_val(env, &res), SI32 { v: -2000 });

    let res = runtime.invoke_contract(addr, "inc_local", vec![SI32 { v: -1000 }.into_val(env)]);
    assert_eq!(SI32::from_val(env, &res), SI32 { v: -999 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SI32 { v: -1000 }.into_val(env), (-5i32).into_val(env)],
    );
    assert_eq!(SI32::from_val(env, &res), SI32 { v: -1005 });
}

#[test]
fn single_int64_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { int64 v; }
            function make() public pure returns (S memory) { return S(-1000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                int64 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, int64 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SI64::from_val(env, &res), SI64 { v: -1000 });

    let input = SI64 {
        v: -1_234_567_890_123,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SI64::from_val(env, &res), input);

    let res = runtime.invoke_contract(
        addr,
        "twice",
        vec![SI64 { v: -1_000_000_000 }.into_val(env)],
    );
    assert_eq!(SI64::from_val(env, &res), SI64 { v: -2_000_000_000 });

    let res = runtime.invoke_contract(
        addr,
        "inc_local",
        vec![SI64 { v: -1_000_000_000 }.into_val(env)],
    );
    assert_eq!(SI64::from_val(env, &res), SI64 { v: -999_999_999 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![
            SI64 { v: -1_000_000_000 }.into_val(env),
            (-5i64).into_val(env),
        ],
    );
    assert_eq!(SI64::from_val(env, &res), SI64 { v: -1_000_000_005 });
}

#[test]
fn single_int128_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { int128 v; }
            function make() public pure returns (S memory) { return S(-1000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                int128 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, int128 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SI128::from_val(env, &res), SI128 { v: -1000 });

    let big = -100_000_000_000_000_000_000i128;
    let res = runtime.invoke_contract(addr, "echo", vec![SI128 { v: big }.into_val(env)]);
    assert_eq!(SI128::from_val(env, &res), SI128 { v: big });

    let res = runtime.invoke_contract(addr, "twice", vec![SI128 { v: big }.into_val(env)]);
    assert_eq!(SI128::from_val(env, &res), SI128 { v: big * 2 });

    let res = runtime.invoke_contract(addr, "inc_local", vec![SI128 { v: big }.into_val(env)]);
    assert_eq!(SI128::from_val(env, &res), SI128 { v: big + 1 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SI128 { v: big }.into_val(env), (-50i128).into_val(env)],
    );
    assert_eq!(SI128::from_val(env, &res), SI128 { v: big - 50 });
}

#[test]
fn single_int256_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { int256 v; }
            function make() public pure returns (S memory) { return S(-100000000000000000000); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                int256 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, int256 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let i256 = |n: i128| I256::from_i128(env, n);

    let big = -100_000_000_000_000_000_000i128;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SI256::from_val(env, &res), SI256 { v: i256(big) });

    let res = runtime.invoke_contract(addr, "echo", vec![SI256 { v: i256(big) }.into_val(env)]);
    assert_eq!(SI256::from_val(env, &res), SI256 { v: i256(big) });

    let res = runtime.invoke_contract(addr, "twice", vec![SI256 { v: i256(big) }.into_val(env)]);
    assert_eq!(SI256::from_val(env, &res), SI256 { v: i256(big * 2) });

    let res = runtime.invoke_contract(
        addr,
        "inc_local",
        vec![SI256 { v: i256(big) }.into_val(env)],
    );
    assert_eq!(SI256::from_val(env, &res), SI256 { v: i256(big + 1) });

    let d = -50_000_000_000_000_000_000i128;
    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SI256 { v: i256(big) }.into_val(env), i256(d).into_val(env)],
    );
    assert_eq!(SI256::from_val(env, &res), SI256 { v: i256(big + d) });
}

#[test]
fn single_address_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { address v; }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function via_local(S memory s) public pure returns (S memory) {
                address a = s.v; return S(a);
            }
            function swap(S memory s, address other) public pure returns (S memory) {
                s.v = other; return s;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let a = Address::generate(env);
    let b = Address::generate(env);

    let input = SAddr { v: a.clone() };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SAddr::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(SAddr::from_val(env, &res), input);

    let res = runtime.invoke_contract(
        addr,
        "swap",
        vec![input.into_val(env), b.clone().into_val(env)],
    );
    assert_eq!(SAddr::from_val(env, &res), SAddr { v: b });
}

#[test]
fn single_bytes4_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { bytes4 v; }
            function make() public pure returns (S memory) { bytes4 b = 0x01020304; return S(b); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function mask(S memory s) public pure returns (S memory) {
                s.v = s.v & 0xff00ff00; return s;
            }
            function via_local(S memory s) public pure returns (S memory) {
                bytes4 b = s.v; return S(b);
            }
            function swap(S memory s, bytes4 other) public pure returns (S memory) {
                s.v = other; return s;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let b4 = |bytes: &[u8; 4]| Bytes::from_array(env, bytes);

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(
        SBytesN::from_val(env, &res),
        SBytesN {
            v: b4(&[1, 2, 3, 4])
        }
    );

    let input = SBytesN {
        v: b4(&[0xDE, 0xAD, 0xBE, 0xEF]),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SBytesN::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "mask", vec![input.clone().into_val(env)]);
    assert_eq!(
        SBytesN::from_val(env, &res),
        SBytesN {
            v: b4(&[0xDE, 0x00, 0xBE, 0x00])
        }
    );

    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(SBytesN::from_val(env, &res), input);

    let other = b4(&[0x11, 0x22, 0x33, 0x44]);
    let res = runtime.invoke_contract(
        addr,
        "swap",
        vec![input.into_val(env), other.clone().into_val(env)],
    );
    assert_eq!(SBytesN::from_val(env, &res), SBytesN { v: other });
}

#[test]
fn single_bytes32_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { bytes32 v; }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function via_local(S memory s) public pure returns (S memory) {
                bytes32 b = s.v; return S(b);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let bytes: [u8; 32] = core::array::from_fn(|i| i as u8);
    let input = SBytesN {
        v: Bytes::from_array(env, &bytes),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SBytesN::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(SBytesN::from_val(env, &res), input);
}

#[test]
fn single_uint8_rounds_to_u32() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { uint8 v; }
            function make() public pure returns (S memory) { return S(100); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function twice(S memory s) public pure returns (S memory) { s.v = s.v + s.v; return s; }
            function inc_local(S memory s) public pure returns (S memory) {
                uint8 x = s.v; x = x + 1; return S(x);
            }
            function combine(S memory s, uint8 d) public pure returns (S memory) {
                return S(s.v + d);
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 100 });

    let input = SU32 { v: 250 };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "twice", vec![SU32 { v: 100 }.into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 200 });

    let res = runtime.invoke_contract(addr, "inc_local", vec![SU32 { v: 100 }.into_val(env)]);
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 101 });

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![SU32 { v: 100 }.into_val(env), 20u32.into_val(env)],
    );
    assert_eq!(SU32::from_val(env, &res), SU32 { v: 120 });
}

#[test]
fn single_string_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { string v; }
            function make() public pure returns (S memory) { return S("hello"); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function len(S memory s) public pure returns (uint64) {
                return uint64(bytes(s.v).length);
            }
            function append(S memory s) public pure returns (S memory) {
                s.v = string.concat(s.v, "!!");
                return s;
            }
            function combine(S memory s, string memory extra) public pure returns (S memory) {
                s.v = string.concat(s.v, extra);
                return s;
            }
            function push_char(S memory s) public pure returns (S memory) {
                bytes memory b = bytes(s.v);
                b.push(0x21); // '!'
                s.v = string(b);
                return s;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let s = |t: &str| String::from_str(env, t);

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SString::from_val(env, &res).v, s("hello"));

    let input = SString { v: s("Solang!") };

    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SString::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "len", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 7);

    let res = runtime.invoke_contract(addr, "append", vec![input.clone().into_val(env)]);
    assert_eq!(SString::from_val(env, &res).v, s("Solang!!!"));

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![input.clone().into_val(env), s(" rocks").into_val(env)],
    );
    assert_eq!(SString::from_val(env, &res).v, s("Solang! rocks"));

    let res = runtime.invoke_contract(addr, "push_char", vec![input.clone().into_val(env)]);
    assert_eq!(SString::from_val(env, &res).v, s("Solang!!"));
}

#[test]
fn single_dyn_bytes_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S { bytes v; }
            function make() public pure returns (S memory) { return S(hex"aabbcc"); }
            function echo(S memory s) public pure returns (S memory) { return s; }
            function len(S memory s) public pure returns (uint64) { return uint64(s.v.length); }
            function push_byte(S memory s, bytes1 x) public pure returns (S memory) {
                bytes memory b = s.v;
                b.push(x);
                s.v = b;
                return s;
            }
            function pop_byte(S memory s) public pure returns (S memory) {
                bytes memory b = s.v;
                b.pop();
                s.v = b;
                return s;
            }
            function first(S memory s) public pure returns (uint64) {
                return uint64(uint8(s.v[0]));
            }
            function combine(S memory s, bytes memory extra) public pure returns (S memory) {
                s.v = bytes.concat(s.v, extra);
                return s;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let b = |bytes: &[u8]| Bytes::from_slice(env, bytes);

    let res = runtime.invoke_contract(addr, "make", vec![]);
    assert_eq!(SDynBytes::from_val(env, &res).v, b(&[0xAA, 0xBB, 0xCC]));

    let input = SDynBytes {
        v: b(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE]),
    };

    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(SDynBytes::from_val(env, &res), input);

    let res = runtime.invoke_contract(addr, "len", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 5);

    let res = runtime.invoke_contract(
        addr,
        "push_byte",
        vec![
            input.clone().into_val(env),
            BytesN::from_array(env, &[0x99]).into_val(env),
        ],
    );
    assert_eq!(
        SDynBytes::from_val(env, &res).v,
        b(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x99])
    );

    let res = runtime.invoke_contract(addr, "pop_byte", vec![input.clone().into_val(env)]);
    assert_eq!(
        SDynBytes::from_val(env, &res).v,
        b(&[0xAA, 0xBB, 0xCC, 0xDD])
    );

    let res = runtime.invoke_contract(addr, "first", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 0xAA);

    let res = runtime.invoke_contract(
        addr,
        "combine",
        vec![input.clone().into_val(env), b(&[0x11, 0x22]).into_val(env)],
    );
    assert_eq!(
        SDynBytes::from_val(env, &res).v,
        b(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x11, 0x22])
    );
}

#[test]
fn single_nested_struct_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Inner { uint64 v; }
            struct Outer { Inner v; }
            function make_val() public pure returns (uint64) {
                Outer memory o = Outer(Inner(42));
                return o.v.v;
            }
            function get(Outer memory o) public pure returns (uint64) {
                return o.v.v;
            }
            function bump(Outer memory o) public pure returns (uint64) {
                o.v.v = o.v.v + 1;
                return o.v.v;
            }
            function local_val(Outer memory o) public pure returns (uint64) {
                Inner memory i = o.v;
                return i.v;
            }
            function mix(Outer memory o, uint64 d) public pure returns (uint64) {
                return o.v.v + d;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "make_val", vec![]);
    assert_eq!(u64::from_val(env, &res), 42);

    let input = SOuter {
        v: SInner { v: 987_654_321 },
    };
    let res = runtime.invoke_contract(addr, "get", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 987_654_321);

    let res = runtime.invoke_contract(addr, "bump", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 987_654_322);

    let res = runtime.invoke_contract(addr, "local_val", vec![input.clone().into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 987_654_321);

    let res = runtime.invoke_contract(addr, "mix", vec![input.into_val(env), 9u64.into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 987_654_330);
}
