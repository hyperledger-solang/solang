// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal, I256, U256};

#[test]
fn i256_basic_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(int256 a, int256 b) public returns (int256) {
            return a + b;
        }
        function sub(int256 a, int256 b) public returns (int256) {
            return a - b;
        }
        function mul(int256 a, int256 b) public returns (int256) {
            return a * b;
        }
        function div(int256 a, int256 b) public returns (int256) {
            return a / b;
        }
        function mod(int256 a, int256 b) public returns (int256) {
            return a % b;
        }

       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);
    {
        let a = i256(9);
        let b = i256(11);
        let res_add = runtime.invoke_contract(
            addr,
            "add",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(20));
    }

    {
        let a = i256(2i128.pow(70));
        let b = i256(2i128.pow(70));
        let res_add = runtime.invoke_contract(
            addr,
            "add",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(2i128.pow(71)));
    }
    {
        let a = i256(9);
        let b = i256(11);
        let res_add = runtime.invoke_contract(
            addr,
            "sub",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(-2));
    }
    {
        let a = i256(2i128.pow(70));
        let b = i256(2i128.pow(69));
        let res_add = runtime.invoke_contract(
            addr,
            "sub",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(2i128.pow(69)));
    }
    {
        let a = i256(9);
        let b = i256(11);
        let res_add = runtime.invoke_contract(
            addr,
            "mul",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(99));
    }
    {
        let a = i256(2i128.pow(50));
        let b = i256(2i128.pow(73));
        let res_add = runtime.invoke_contract(
            addr,
            "mul",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(2i128.pow(123)));
    }
    {
        let a = i256(99);
        let b = i256(11);
        let res_add = runtime.invoke_contract(
            addr,
            "div",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(9));
    }
    {
        let a = i256(2i128.pow(90));
        let b = i256(2i128.pow(73));
        let res_add = runtime.invoke_contract(
            addr,
            "div",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(2i128.pow(17)));
    }
    {
        let a = i256(17);
        let b = i256(13);
        let res_add = runtime.invoke_contract(
            addr,
            "mod",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res_add), i256(4));
    }
    {
        let a = i256((2 * 2i128.pow(90)) - 1);
        let b = i256(2i128.pow(90));
        let res_add = runtime.invoke_contract(
            addr,
            "mod",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(
            I256::from_val(addr.env(), &res_add),
            i256(2i128.pow(90) - 1)
        );
    }
}

#[test]
fn i256_bitwise_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function and(int256 a, int256 b) public returns (int256) {
            return a & b;
        }
        function or(int256 a, int256 b) public returns (int256) {
            return a | b;
        }
        function xor(int256 a, int256 b) public returns (int256) {
            return a ^ b;
        }
        function shl(int256 a, uint64 b) public returns (int256) {
            return a << b;
        }
        function shr(int256 a, uint64 b) public returns (int256) {
            return a >> b;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    {
        let a = i256(0b1010);
        let b = i256(0b1100);
        let res = runtime.invoke_contract(
            addr,
            "and",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(0b1000));
    }
    {
        let a = i256(2i128.pow(70));
        let b = i256(2i128.pow(70) + 2i128.pow(69));
        let res = runtime.invoke_contract(
            addr,
            "and",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(70)));
    }
    {
        let a = i256(0b1010);
        let b = i256(0b0101);
        let res = runtime.invoke_contract(
            addr,
            "or",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(0b1111));
    }
    {
        let a = i256(2i128.pow(70));
        let b = i256(2i128.pow(69));
        let res = runtime.invoke_contract(
            addr,
            "or",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(
            I256::from_val(addr.env(), &res),
            i256(2i128.pow(70) + 2i128.pow(69))
        );
    }
    {
        let a = i256(0b1010);
        let b = i256(0b1100);
        let res = runtime.invoke_contract(
            addr,
            "xor",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(0b0110));
    }
    {
        let a = i256(2i128.pow(70) + 2i128.pow(69));
        let b = i256(2i128.pow(70));
        let res = runtime.invoke_contract(
            addr,
            "xor",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(69)));
    }
    {
        let a = i256(1);
        let res = runtime.invoke_contract(
            addr,
            "shl",
            vec![a.into_val(addr.env()), 4u64.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(16));
    }
    {
        let a = i256(2i128.pow(60));
        let res = runtime.invoke_contract(
            addr,
            "shl",
            vec![a.into_val(addr.env()), 10u64.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(70)));
    }
    {
        let a = i256(16);
        let res = runtime.invoke_contract(
            addr,
            "shr",
            vec![a.into_val(addr.env()), 4u64.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(1));
    }
    {
        let a = i256(2i128.pow(70));
        let res = runtime.invoke_contract(
            addr,
            "shr",
            vec![a.into_val(addr.env()), 10u64.into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(60)));
    }
}

#[test]
fn u256_basic_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(uint256 a, uint256 b) public returns (uint256) {
            return a + b;
        }
        function sub(uint256 a, uint256 b) public returns (uint256) {
            return a - b;
        }
        function mul(uint256 a, uint256 b) public returns (uint256) {
            return a * b;
        }
        function div(uint256 a, uint256 b) public returns (uint256) {
            return a / b;
        }
        function mod(uint256 a, uint256 b) public returns (uint256) {
            return a % b;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    {
        let a = u256(9);
        let b = u256(11);
        let res = runtime.invoke_contract(
            addr,
            "add",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(20));
    }
    {
        let a = u256(2u128.pow(100));
        let b = u256(2u128.pow(100));
        let res = runtime.invoke_contract(
            addr,
            "add",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(101)));
    }
    {
        let a = u256(11);
        let b = u256(9);
        let res = runtime.invoke_contract(
            addr,
            "sub",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2));
    }
    {
        let a = u256(2u128.pow(100));
        let b = u256(2u128.pow(99));
        let res = runtime.invoke_contract(
            addr,
            "sub",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(99)));
    }
    {
        let a = u256(9);
        let b = u256(11);
        let res = runtime.invoke_contract(
            addr,
            "mul",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(99));
    }
    {
        let a = u256(2u128.pow(50));
        let b = u256(2u128.pow(73));
        let res = runtime.invoke_contract(
            addr,
            "mul",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(123)));
    }
    {
        let a = u256(99);
        let b = u256(11);
        let res = runtime.invoke_contract(
            addr,
            "div",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(9));
    }
    {
        let a = u256(2u128.pow(100));
        let b = u256(2u128.pow(73));
        let res = runtime.invoke_contract(
            addr,
            "div",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(27)));
    }
    {
        let a = u256(17);
        let b = u256(13);
        let res = runtime.invoke_contract(
            addr,
            "mod",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(4));
    }
    {
        let a = u256(2u128.pow(100) + 2u128.pow(99) - 1);
        let b = u256(2u128.pow(100));
        let res = runtime.invoke_contract(
            addr,
            "mod",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(99) - 1));
    }
}

#[test]
fn u256_bitwise_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function and(uint256 a, uint256 b) public returns (uint256) {
            return a & b;
        }
        function or(uint256 a, uint256 b) public returns (uint256) {
            return a | b;
        }
        function xor(uint256 a, uint256 b) public returns (uint256) {
            return a ^ b;
        }
        function shl(uint256 a, uint64 b) public returns (uint256) {
            return a << b;
        }
        function shr(uint256 a, uint64 b) public returns (uint256) {
            return a >> b;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    {
        let a = u256(0b1010);
        let b = u256(0b1100);
        let res = runtime.invoke_contract(
            addr,
            "and",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(0b1000));
    }
    {
        let a = u256(2u128.pow(100));
        let b = u256(2u128.pow(100) + 2u128.pow(99));
        let res = runtime.invoke_contract(
            addr,
            "and",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));
    }
    {
        let a = u256(0b1010);
        let b = u256(0b0101);
        let res = runtime.invoke_contract(
            addr,
            "or",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(0b1111));
    }
    {
        let a = u256(2u128.pow(100));
        let b = u256(2u128.pow(99));
        let res = runtime.invoke_contract(
            addr,
            "or",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(
            U256::from_val(addr.env(), &res),
            u256(2u128.pow(100) + 2u128.pow(99))
        );
    }
    {
        let a = u256(0b1010);
        let b = u256(0b1100);
        let res = runtime.invoke_contract(
            addr,
            "xor",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(0b0110));
    }
    {
        let a = u256(2u128.pow(100) + 2u128.pow(99));
        let b = u256(2u128.pow(100));
        let res = runtime.invoke_contract(
            addr,
            "xor",
            vec![a.into_val(addr.env()), b.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(99)));
    }
    {
        let a = u256(1);
        let res = runtime.invoke_contract(
            addr,
            "shl",
            vec![a.into_val(addr.env()), 4u64.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(16));
    }
    {
        let a = u256(2u128.pow(90));
        let res = runtime.invoke_contract(
            addr,
            "shl",
            vec![a.into_val(addr.env()), 10u64.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));
    }
    {
        let a = u256(16);
        let res = runtime.invoke_contract(
            addr,
            "shr",
            vec![a.into_val(addr.env()), 4u64.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(1));
    }
    {
        let a = u256(2u128.pow(100));
        let res = runtime.invoke_contract(
            addr,
            "shr",
            vec![a.into_val(addr.env()), 10u64.into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(90)));
    }
}

#[test]
fn i256_constants() {
    let runtime = build_solidity(
        r#"contract math {
        function small_pos() public returns (int256) {
            return 42;
        }
        function small_neg() public returns (int256) {
            return -7;
        }
    }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    let res = runtime.invoke_contract(addr, "small_pos", vec![]);
    assert_eq!(I256::from_val(addr.env(), &res), i256(42));

    let res = runtime.invoke_contract(addr, "small_neg", vec![]);
    assert_eq!(I256::from_val(addr.env(), &res), i256(-7));
}

#[test]
fn u256_constants() {
    let runtime = build_solidity(
        r#"contract math {
        function small() public returns (uint256) {
            return 99;
        }
        function large() public returns (uint256) {
            uint256 a = 2**100;
            return a;
        }
        function computed() public returns (uint256) {
            uint256 a = 2**60;
            uint256 b = 2**40;
            return a * b;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    let res = runtime.invoke_contract(addr, "small", vec![]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(99));

    let res = runtime.invoke_contract(addr, "large", vec![]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));

    let res = runtime.invoke_contract(addr, "computed", vec![]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));
}

#[test]
fn i256_storage() {
    let runtime = build_solidity(
        r#"contract math {
        int256 stored;

        function set(int256 val) public {
            stored = val;
        }

        function get() public returns (int256) {
            return stored;
        }

        function accumulate(int256 val) public returns (int256) {
            stored = stored + val;
            return stored;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    let _ = runtime.invoke_contract(addr, "set", vec![i256(42).into_val(addr.env())]);
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(I256::from_val(addr.env(), &res), i256(42));

    let _ = runtime.invoke_contract(addr, "set", vec![i256(2i128.pow(90)).into_val(addr.env())]);
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(90)));

    let _ = runtime.invoke_contract(
        addr,
        "set",
        vec![i256(-(2i128.pow(80))).into_val(addr.env())],
    );
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(I256::from_val(addr.env(), &res), i256(-(2i128.pow(80))));

    let _ = runtime.invoke_contract(addr, "set", vec![i256(2i128.pow(70)).into_val(addr.env())]);
    let res = runtime.invoke_contract(
        addr,
        "accumulate",
        vec![i256(2i128.pow(70)).into_val(addr.env())],
    );
    assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(71)));
}

#[test]
fn u256_storage() {
    let runtime = build_solidity(
        r#"contract math {
        uint256 stored;

        function set(uint256 val) public {
            stored = val;
        }

        function get() public returns (uint256) {
            return stored;
        }

        function accumulate(uint256 val) public returns (uint256) {
            stored = stored + val;
            return stored;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    let _ = runtime.invoke_contract(addr, "set", vec![u256(99).into_val(addr.env())]);
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(99));

    let _ = runtime.invoke_contract(addr, "set", vec![u256(2u128.pow(100)).into_val(addr.env())]);
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));

    let _ = runtime.invoke_contract(addr, "set", vec![u256(2u128.pow(99)).into_val(addr.env())]);
    let res = runtime.invoke_contract(
        addr,
        "accumulate",
        vec![u256(2u128.pow(99)).into_val(addr.env())],
    );
    assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));
}

#[test]
fn i256_abi_edge_cases() {
    let runtime = build_solidity(
        r#"contract math {
        function identity(int256 a) public returns (int256) {
            return a;
        }
        function negate(int256 a) public returns (int256) {
            return -a;
        }
        function to_u256(int256 a) public returns (uint256) {
            return uint256(a);
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);
    let u256 = |n: u128| U256::from_u128(addr.env(), n);
    {
        let res = runtime.invoke_contract(addr, "identity", vec![i256(5).into_val(addr.env())]);
        assert_eq!(I256::from_val(addr.env(), &res), i256(5));
    }
    {
        let res = runtime.invoke_contract(
            addr,
            "identity",
            vec![i256(2i128.pow(80)).into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(80)));
    }
    {
        let res = runtime.invoke_contract(addr, "identity", vec![i256(-1).into_val(addr.env())]);
        assert_eq!(I256::from_val(addr.env(), &res), i256(-1));
    }
    {
        let res = runtime.invoke_contract(
            addr,
            "identity",
            vec![i256(-(2i128.pow(80))).into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(-(2i128.pow(80))));
    }
    {
        let res = runtime.invoke_contract(addr, "negate", vec![i256(7).into_val(addr.env())]);
        assert_eq!(I256::from_val(addr.env(), &res), i256(-7));
    }
    {
        let res = runtime.invoke_contract(
            addr,
            "negate",
            vec![i256(2i128.pow(70)).into_val(addr.env())],
        );
        assert_eq!(I256::from_val(addr.env(), &res), i256(-(2i128.pow(70))));
    }
    {
        let res = runtime.invoke_contract(
            addr,
            "to_u256",
            vec![i256(2i128.pow(80)).into_val(addr.env())],
        );
        assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(80)));
    }
}

#[test]
fn u256_abi_edge_cases() {
    let runtime = build_solidity(
        r#"contract math {
        function identity(uint256 a) public returns (uint256) {
            return a;
        }
        function to_i256(uint256 a) public returns (int256) {
            return int256(a);
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    let res = runtime.invoke_contract(addr, "identity", vec![u256(42).into_val(addr.env())]);
    assert_eq!(U256::from_val(addr.env(), &res), u256(42));

    let res = runtime.invoke_contract(
        addr,
        "identity",
        vec![u256(2u128.pow(100)).into_val(addr.env())],
    );
    assert_eq!(U256::from_val(addr.env(), &res), u256(2u128.pow(100)));

    let res = runtime.invoke_contract(
        addr,
        "to_i256",
        vec![u256(2u128.pow(80)).into_val(addr.env())],
    );
    assert_eq!(I256::from_val(addr.env(), &res), i256(2i128.pow(80)));
}

#[test]
fn i256_small_object_boundary() {
    let runtime = build_solidity(
        r#"contract math {
        function identity(int256 a) public returns (int256) {
            return a;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    let max_small = 2i128.pow(55) - 1;
    let min_small = -(2i128.pow(55));
    let cases = [
        0,
        1,
        -1,
        255,
        256,
        -256,
        max_small,
        min_small,
        max_small + 1,
        min_small - 1,
        2i128.pow(80),
        -(2i128.pow(80)),
    ];

    for value in cases {
        let res = runtime.invoke_contract(addr, "identity", vec![i256(value).into_val(addr.env())]);
        assert_eq!(
            I256::from_val(addr.env(), &res),
            i256(value),
            "int256 round-trip failed for {value}"
        );
    }
}

#[test]
fn u256_small_object_boundary() {
    let runtime = build_solidity(
        r#"contract math {
        function identity(uint256 a) public returns (uint256) {
            return a;
        }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    let max_small = 2u128.pow(56) - 1;
    let cases = [0, 1, 255, 256, max_small, max_small + 1, 2u128.pow(100)];

    for value in cases {
        let res = runtime.invoke_contract(addr, "identity", vec![u256(value).into_val(addr.env())]);
        assert_eq!(
            U256::from_val(addr.env(), &res),
            u256(value),
            "uint256 round-trip failed for {value}"
        );
    }
}

#[test]
fn i256_comparison_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function eq(int256 a, int256 b) public returns (bool) { return a == b; }
        function ne(int256 a, int256 b) public returns (bool) { return a != b; }
        function lt(int256 a, int256 b) public returns (bool) { return a < b; }
        function lte(int256 a, int256 b) public returns (bool) { return a <= b; }
        function gt(int256 a, int256 b) public returns (bool) { return a > b; }
        function gte(int256 a, int256 b) public returns (bool) { return a >= b; }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let i256 = |n: i128| I256::from_i128(addr.env(), n);

    let pairs = [
        (0i128, 0i128),
        (1, -1),
        (-1, 1),
        (5, 5),
        (-7, -7),
        (255, 256),
        (2i128.pow(55) - 1, 2i128.pow(55)),
        (-(2i128.pow(55)), -(2i128.pow(55)) - 1),
        (2i128.pow(90), 2i128.pow(70)),
        (-(2i128.pow(90)), 2i128.pow(70)),
        (2i128.pow(100), 2i128.pow(100)),
    ];

    for (a, b) in pairs {
        let check = |func: &str, expected: bool| {
            let res = runtime.invoke_contract(
                addr,
                func,
                vec![i256(a).into_val(addr.env()), i256(b).into_val(addr.env())],
            );
            assert_eq!(
                bool::from_val(addr.env(), &res),
                expected,
                "{func}({a}, {b})"
            );
        };
        check("eq", a == b);
        check("ne", a != b);
        check("lt", a < b);
        check("lte", a <= b);
        check("gt", a > b);
        check("gte", a >= b);
    }
}

#[test]
fn u256_comparison_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function eq(uint256 a, uint256 b) public returns (bool) { return a == b; }
        function ne(uint256 a, uint256 b) public returns (bool) { return a != b; }
        function lt(uint256 a, uint256 b) public returns (bool) { return a < b; }
        function lte(uint256 a, uint256 b) public returns (bool) { return a <= b; }
        function gt(uint256 a, uint256 b) public returns (bool) { return a > b; }
        function gte(uint256 a, uint256 b) public returns (bool) { return a >= b; }
       }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let u256 = |n: u128| U256::from_u128(addr.env(), n);

    let pairs = [
        (0u128, 0u128),
        (1, 0),
        (5, 5),
        (255, 256),
        (2u128.pow(56) - 1, 2u128.pow(56)),
        (2u128.pow(100), 2u128.pow(70)),
        (2u128.pow(70), 2u128.pow(100)),
        (2u128.pow(100), 2u128.pow(100)),
    ];

    for (a, b) in pairs {
        let check = |func: &str, expected: bool| {
            let res = runtime.invoke_contract(
                addr,
                func,
                vec![u256(a).into_val(addr.env()), u256(b).into_val(addr.env())],
            );
            assert_eq!(
                bool::from_val(addr.env(), &res),
                expected,
                "{func}({a}, {b})"
            );
        };
        check("eq", a == b);
        check("ne", a != b);
        check("lt", a < b);
        check("lte", a <= b);
        check("gt", a > b);
        check("gte", a >= b);
    }
}
