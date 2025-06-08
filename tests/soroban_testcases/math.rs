// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn math() {
    let runtime = build_solidity(
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

    let arg: Val = 5_u64.into_val(&runtime.env);
    let arg2: Val = 4_u64.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "max", vec![arg, arg2]);

    let expected: Val = 5_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn math_same_name() {
    let src = build_solidity(
        r#"contract math {
        function max(uint64 a, uint64 b) public returns (uint64) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    
        function max(uint64 a, uint64 b, uint64 c) public returns (uint64) {
            if (a > b) {
                if (a > c) {
                    return a;
                } else {
                    return c;
                }
            } else {
                if (b > c) {
                    return b;
                } else {
                    return c;
                }
            }
        }
    }
    "#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let res = src.invoke_contract(addr, "max_uint64_uint64", vec![arg1, arg2]);
    let expected: Val = 5_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let arg3: Val = 6_u64.into_val(&src.env);
    let res = src.invoke_contract(addr, "max_uint64_uint64_uint64", vec![arg1, arg2, arg3]);
    let expected: Val = 6_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i128_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(int128 a, int128 b) public returns (int128) {
            return a + b;
        }

        function sub(int128 a, int128 b) public returns (int128) {
            return a - b;
        }

        function mul(int128 a, int128 b) public returns (int128) {
            return a * b;
        }

        function div(int128 a, int128 b) public returns (int128) {
            return a / b;
        }

        function mod(int128 a, int128 b) public returns (int128) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_i128.into_val(&runtime.env);
    let arg2: Val = 4_i128.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);

    let expected: Val = 9_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);

    let expected: Val = 20_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn u128_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(uint128 a, uint128 b) public returns (uint128) {
            return a + b;
        }

        function sub(uint128 a, uint128 b) public returns (uint128) {
            return a - b;
        }

        function mul(uint128 a, uint128 b) public returns (uint128) {
            return a * b;
        }

        function div(uint128 a, uint128 b) public returns (uint128) {
            return a / b;
        }

        function mod(uint128 a, uint128 b) public returns (uint128) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_u128.into_val(&runtime.env);
    let arg2: Val = 4_u128.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);

    let expected: Val = 9_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);

    let expected: Val = 20_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn bool_roundtrip() {
    let runtime = build_solidity(
        r#"
        contract test {
            function flip(bool x) public returns (bool) {
                return !x;
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let arg_true: Val = true.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "flip", vec![arg_true]);
    let expected: Val = false.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn u32_roundtrip() {
    let runtime = build_solidity(
        r#"
        contract test {
            function id(uint32 x) public returns (uint32) {
                return x;
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let arg: Val = (42_u32).into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "id", vec![arg]);
    let expected: Val = (42_u32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn u32_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(uint32 a, uint32 b) public returns (uint32) {
            return a + b;
        }

        function sub(uint32 a, uint32 b) public returns (uint32) {
            return a - b;
        }

        function mul(uint32 a, uint32 b) public returns (uint32) {
            return a * b;
        }

        function div(uint32 a, uint32 b) public returns (uint32) {
            return a / b;
        }

        function mod(uint32 a, uint32 b) public returns (uint32) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_u32.into_val(&runtime.env);
    let arg2: Val = 4_u32.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();

    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);
    let expected: Val = 9_u32.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);
    let expected: Val = 1_u32.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);
    let expected: Val = 20_u32.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);
    let expected: Val = 1_u32.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);
    let expected: Val = 1_u32.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i32_roundtrip() {
    let runtime = build_solidity(
        r#"
        contract test {
            function id(int32 x) public returns (int32) {
                return x;
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let arg: Val = (42_i32).into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "id", vec![arg]);
    let expected: Val = (42_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i32_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(int32 a, int32 b) public returns (int32) {
            return a + b;
        }

        function sub(int32 a, int32 b) public returns (int32) {
            return a - b;
        }

        function mul(int32 a, int32 b) public returns (int32) {
            return a * b;
        }

        function div(int32 a, int32 b) public returns (int32) {
            return a / b;
        }

        function mod(int32 a, int32 b) public returns (int32) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let arg: Val = (5_i32).into_val(&runtime.env);
    let arg2: Val = (4_i32).into_val(&runtime.env);

    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);
    let expected: Val = (9_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);
    let expected: Val = (1_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);
    let expected: Val = (20_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);
    let expected: Val = (1_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);
    let expected: Val = (1_i32).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i64_roundtrip() {
    let runtime = build_solidity(
        r#"
        contract test {
            function id(int64 x) public returns (int64) {
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let arg: Val = (-42_i64).into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "id", vec![arg]);
    let expected: Val = (-42_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i64_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(int64 a, int64 b) public returns (int64) { return a + b; }
        function sub(int64 a, int64 b) public returns (int64) { return a - b; }
        function mul(int64 a, int64 b) public returns (int64) { return a * b; }
        function div(int64 a, int64 b) public returns (int64) { return a / b; }
        function mod(int64 a, int64 b) public returns (int64) { return a % b; }
    }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let a: Val = (5_i64).into_val(&runtime.env);
    let b: Val = (-4_i64).into_val(&runtime.env);

    let res = runtime.invoke_contract(addr, "add", vec![a, b]);
    let expected: Val = (1_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![a, b]);
    let expected: Val = (9_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![a, b]);
    let expected: Val = (-20_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![a, b]);
    let expected: Val = (-1_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![a, b]);
    let expected: Val = (1_i64).into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}
