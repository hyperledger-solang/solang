// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::ethereum_types::U256;
use num_bigint::{BigInt, BigUint, RandBigInt, Sign};
use rand::seq::SliceRandom;
use rand::Rng;
use std::ops::Add;
use std::ops::BitAnd;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Rem;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Sub;

#[test]
#[should_panic]
fn assert_false() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(false, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function("assert_fails", &[], &[], None);
}

#[test]
fn assert_true() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(true, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function("assert_fails", &[], &[], None);
}

#[test]
fn boolean() {
    // we need to test: literals
    // passing address around
    // abi encoding/decoding address
    // comparing address to another
    let mut vm = build_solidity(
        r#"
        contract foo {
            function return_true() public returns (bool) {
                return true;
            }

            function return_false() public returns (bool) {
                return false;
            }

            function true_arg(bool b) public {
                assert(b);
            }

            function false_arg(bool b) public {
                assert(!b);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("return_true", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Bool(true),]);

    let returns = vm.function("return_false", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Bool(false),]);

    vm.function("true_arg", &[ethabi::Token::Bool(true)], &[], None);
    vm.function("false_arg", &[ethabi::Token::Bool(false)], &[], None);
}

#[test]
fn address() {
    // we need to test: literals
    // passing address around
    // abi encoding/decoding address
    // comparing address to another

    let mut vm = build_solidity(
        r#"
        contract foo {
            function return_address() public returns (address) {
                return address'CXQw5tfeRKKzV4hk6PcdyKyANSvFxoZCKwHkVXAhAYSJ';
            }

            function address_arg(address a) public {
                assert(a == address'66Eh1STPJgabub73TP8YbN7VNCwjaVTEJGHRxCLeBJ4A');
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("return_address", &[], &[], None);

    assert_eq!(
        returns,
        vec![ethabi::Token::FixedBytes(vec![
            171, 59, 10, 127, 211, 122, 217, 123, 53, 213, 159, 40, 54, 36, 50, 52, 196, 144, 17,
            226, 97, 168, 69, 213, 79, 14, 6, 232, 165, 44, 58, 31
        ]),]
    );

    vm.function(
        "address_arg",
        &[ethabi::Token::FixedBytes(vec![
            75, 161, 209, 89, 47, 84, 50, 13, 23, 127, 94, 21, 50, 249, 250, 185, 117, 49, 186,
            134, 82, 130, 112, 97, 218, 24, 157, 198, 40, 105, 118, 27,
        ])],
        &[],
        None,
    );
}

#[test]
fn test_enum() {
    // we need to test enum literals
    // abi encoding/decode literals
    // comparing enums

    let mut vm = build_solidity(
        r#"
        contract foo {
            enum bar { bar0, bar1, bar2, bar3, bar4, bar5, bar6, bar7, bar8, bar9, bar10 }

            function return_enum() public returns (bar) {
                return bar.bar9;
            }

            function enum_arg(bar a) public {
                assert(a == bar.bar6);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("return_enum", &[], &[], None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(9))]);

    vm.function("enum_arg", &[ethabi::Token::Uint(U256::from(6))], &[], None);
}

#[test]
fn bytes() {
    let mut rng = rand::thread_rng();

    for width in 1..32 {
        let src = r#"
        contract test {
            function return_literal() public returns (bytes7) {
                return hex"01020304050607";
            }

            function return_arg(bytes7 x) public returns (bytes7) {
                return x;
            }

            function or(bytesN a, bytesN b) public returns (bytesN) {
                return a | b;
            }

            function and(bytesN a, bytesN b) public returns (bytesN) {
                return a & b;
            }

            function xor(bytesN a, bytesN b) public returns (bytesN) {
                return a ^ b;
            }

            function shift_left(bytesN a, uint32 r) public returns (bytesN) {
                return a << r;
            }

            function shift_right(bytesN a, uint32 r) public returns (bytesN) {
                return a >> r;
            }
        }"#
        .replace("bytesN", &format!("bytes{}", width));

        let mut vm = build_solidity(&src);

        vm.constructor("test", &[]);

        let returns = vm.function("return_literal", &[], &[], None);

        assert_eq!(
            returns,
            vec![ethabi::Token::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7]),]
        );

        let returns = vm.function(
            "return_arg",
            &[ethabi::Token::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7])],
            &[],
            None,
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7])]
        );

        for _ in 0..10 {
            let mut a = Vec::new();
            let mut b = Vec::new();

            a.resize(width, 0);
            b.resize(width, 0);

            rng.fill(&mut a[..]);
            rng.fill(&mut b[..]);

            let or = vm.function(
                "or",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::FixedBytes(b.to_vec()),
                ],
                &[],
                None,
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a | b).collect();

            println!(
                "{} | {} = {}",
                hex::encode(&a),
                hex::encode(&b),
                hex::encode(&res)
            );

            assert_eq!(or, vec![ethabi::Token::FixedBytes(res)]);

            let and = vm.function(
                "and",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::FixedBytes(b.to_vec()),
                ],
                &[],
                None,
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a & b).collect();

            assert_eq!(and, vec![ethabi::Token::FixedBytes(res)]);

            let xor = vm.function(
                "xor",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::FixedBytes(b.to_vec()),
                ],
                &[],
                None,
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a ^ b).collect();

            assert_eq!(xor, vec![ethabi::Token::FixedBytes(res)]);

            let r = rng.gen::<u32>() % (width as u32 * 8);

            println!("w = {} r = {}", width, r);

            let shl = vm.function(
                "shift_left",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::Uint(U256::from(r)),
                ],
                &[],
                None,
            );

            let mut res = (BigUint::from_bytes_be(&a) << r).to_bytes_be();

            while res.len() > width {
                res.remove(0);
            }

            while res.len() < width {
                res.insert(0, 0);
            }

            assert_eq!(shl, vec![ethabi::Token::FixedBytes(res)]);

            let shr = vm.function(
                "shift_right",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::Uint(U256::from(r)),
                ],
                &[],
                None,
            );

            let mut res = (BigUint::from_bytes_be(&a) >> r).to_bytes_be();

            while res.len() < width {
                res.insert(0, 0);
            }

            assert_eq!(shr, vec![ethabi::Token::FixedBytes(res)]);
        }
    }
}

#[test]
fn uint() {
    let mut rng = rand::thread_rng();

    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function pass(uintN a) public returns (uintN) {
                print("x:{:x}".format(uint64(a)));
                return 0x7f;
            }

            function add(uintN a, uintN b) public returns (uintN) {
                return a + b;
            }

            function sub(uintN a, uintN b) public returns (uintN) {
                return a - b;
            }

            function mul(uintN a, uintN b) public returns (uintN) {
                unchecked {
                return a * b;
                }
            }

            function div(uintN a, uintN b) public returns (uintN) {
                return a / b;
            }

            function mod(uintN a, uintN b) public returns (uintN) {
                return a % b;
            }

            function pow(uintN a, uintN b) public returns (uintN) {
                unchecked {
                return a ** b;
                }
            }

            function or(uintN a, uintN b) public returns (uintN) {
                return a | b;
            }

            function and(uintN a, uintN b) public returns (uintN) {
                return a & b;
            }

            function xor(uintN a, uintN b) public returns (uintN) {
                return a ^ b;
            }

            function shift_left(uintN a, uint32 r) public returns (uintN) {
                return a << r;
            }

            function shift_right(uintN a, uint32 r) public returns (uintN) {
                return a >> r;
            }
        }"#
        .replace("uintN", &format!("uint{}", width));

        let mut vm = build_solidity(&src);

        vm.constructor("test", &[]);

        println!("width:{}", width);

        for _ in 0..10 {
            let mut a = Vec::new();
            let mut b = Vec::new();

            a.resize(width / 8, 0);
            b.resize(width / 8, 0);

            rng.fill(&mut a[..]);
            rng.fill(&mut b[..]);

            let mut a = U256::from_big_endian(&a);
            let mut b = U256::from_big_endian(&b);

            rng.fill(&mut a.0[..]);
            rng.fill(&mut b.0[..]);

            truncate_uint(&mut a, width);
            truncate_uint(&mut b, width);

            let res = vm.function("pass", &[ethabi::Token::Uint(a)], &[], None);

            println!("{:x} = {:?} o", a, res);

            let add = vm.function(
                "add",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let (mut res, _) = a.overflowing_add(b);

            truncate_uint(&mut res, width);

            println!("{:x} + {:x} = {:?} or {:x}", a, b, add, res);

            assert_eq!(add, vec![ethabi::Token::Uint(res)]);

            let sub = vm.function(
                "sub",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let (mut res, _) = a.overflowing_sub(b);

            truncate_uint(&mut res, width);

            assert_eq!(sub, vec![ethabi::Token::Uint(res)]);

            let mul = vm.function(
                "mul",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let (mut res, _) = a.overflowing_mul(b);

            truncate_uint(&mut res, width);

            assert_eq!(mul, vec![ethabi::Token::Uint(res)]);

            let pow = vm.function(
                "pow",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let (mut res, _) = a.overflowing_pow(b);

            truncate_uint(&mut res, width);

            assert_eq!(pow, vec![ethabi::Token::Uint(res)]);

            if b != U256::zero() {
                let div = vm.function(
                    "div",
                    &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                    &[],
                    None,
                );

                let mut res = a.div(b);

                truncate_uint(&mut res, width);

                assert_eq!(div, vec![ethabi::Token::Uint(res)]);

                let add = vm.function(
                    "mod",
                    &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                    &[],
                    None,
                );

                let mut res = a.rem(b);

                truncate_uint(&mut res, width);

                assert_eq!(add, vec![ethabi::Token::Uint(res)]);
            }

            let or = vm.function(
                "or",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] | b.0[0],
                a.0[1] | b.0[1],
                a.0[2] | b.0[2],
                a.0[3] | b.0[3],
            ]);

            truncate_uint(&mut res, width);

            assert_eq!(or, vec![ethabi::Token::Uint(res)]);

            let and = vm.function(
                "and",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] & b.0[0],
                a.0[1] & b.0[1],
                a.0[2] & b.0[2],
                a.0[3] & b.0[3],
            ]);

            truncate_uint(&mut res, width);

            assert_eq!(and, vec![ethabi::Token::Uint(res)]);

            let xor = vm.function(
                "xor",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] ^ b.0[0],
                a.0[1] ^ b.0[1],
                a.0[2] ^ b.0[2],
                a.0[3] ^ b.0[3],
            ]);

            truncate_uint(&mut res, width);

            assert_eq!(xor, vec![ethabi::Token::Uint(res)]);

            let r = rng.gen::<u32>() % (width as u32);

            let shl = vm.function(
                "shift_left",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(U256::from(r))],
                &[],
                None,
            );

            let mut res = a.shl(r);

            truncate_uint(&mut res, width);

            assert_eq!(shl, vec![ethabi::Token::Uint(res)]);

            let shr = vm.function(
                "shift_right",
                &[ethabi::Token::Uint(a), ethabi::Token::Uint(U256::from(r))],
                &[],
                None,
            );

            let mut res = a.shr(r);

            truncate_uint(&mut res, width);

            assert_eq!(shr, vec![ethabi::Token::Uint(res)]);
        }
    }
}

fn truncate_uint(n: &mut U256, width: usize) {
    let mut bits = 256 - width;

    let mut offset = 3;

    while bits > 64 {
        n.0[offset] = 0;

        offset -= 1;
        bits -= 64;
    }

    if bits > 0 {
        n.0[offset] &= (1 << (64 - bits)) - 1;
    }
}

#[test]
fn test_power_overflow_boundaries() {
    for width in (72..=256).step_by(8) {
        let src = r#"
        contract test {
            function pow(uintN a, uintN b) public returns (uintN) { 
                return a ** b;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut contract = build_solidity(&src);
        contract.constructor("test", &[]);

        let return_value = contract.function(
            "pow",
            &[
                ethabi::Token::Uint(U256::from(2)),
                ethabi::Token::Uint(U256::from(width - 1)),
            ],
            &[],
            None,
        );

        let res = BigUint::from(2_u32).pow((width - 1) as u32);

        assert_eq!(
            return_value,
            vec![ethabi::Token::Uint(U256::from_big_endian(
                &res.to_bytes_be()
            ))]
        );

        let sesa = contract.function_must_fail(
            "pow",
            &[
                ethabi::Token::Uint(U256::from(2)),
                ethabi::Token::Uint(U256::from(width)),
            ],
            &[],
            None,
        );

        assert_ne!(sesa, Ok(0));
    }
}

#[test]
fn test_overflow_boundaries() {
    // For bit sizes from 8..64, LLVM has intrinsic functions for multiplication with overflow. Testing starts from int types of 72 and up. There is no need to test intrinsic LLVM functions.
    for width in (72..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity(&src);

        // The max value held in signed N bits is - 2^(N-1)..2^(N-1)-1 .We generate theses boundaries:
        let upper_boundary = BigInt::from(2_u32).pow(width - 1).sub(1);
        let lower_boundary = BigInt::from(2_u32).pow(width - 1).mul(-1);
        let second_op = BigInt::from(1_u32);

        // Multiply the boundaries by 1.
        contract.constructor("test", &[]);
        let return_value = contract.function(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(&upper_boundary, width.try_into().unwrap())),
                ethabi::Token::Int(bigint_to_eth(&second_op, width.try_into().unwrap())),
            ],
            &[],
            None,
        );
        assert_eq!(
            return_value,
            vec![ethabi::Token::Int(bigint_to_eth(
                &upper_boundary,
                width.try_into().unwrap(),
            ))]
        );

        let return_value = contract.function(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(&lower_boundary, width.try_into().unwrap())),
                ethabi::Token::Int(bigint_to_eth(&second_op, width.try_into().unwrap())),
            ],
            &[],
            None,
        );
        assert_eq!(
            return_value,
            vec![ethabi::Token::Int(bigint_to_eth(
                &lower_boundary,
                width.try_into().unwrap(),
            ))]
        );

        let upper_boundary_plus_one = BigInt::from(2_u32).pow(width - 1);

        // We subtract 2 instead of one to make the number even, so that no rounding occurs when we divide by 2 later on.
        let lower_boundary_minus_two = BigInt::from(2_u32).pow(width - 1).mul(-1_i32).sub(2_i32);

        let upper_second_op = upper_boundary_plus_one.div(2);

        let lower_second_op = lower_boundary_minus_two.div(2);

        let res = contract.function_must_fail(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(&upper_second_op, width.try_into().unwrap())),
                ethabi::Token::Int(bigint_to_eth(&BigInt::from(2), width.try_into().unwrap())),
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        let res = contract.function_must_fail(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(&lower_second_op, width.try_into().unwrap())),
                ethabi::Token::Int(bigint_to_eth(&BigInt::from(2), width.try_into().unwrap())),
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));
    }
}

#[test]
fn test_mul_within_range_signed() {
    let mut rng = rand::thread_rng();
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut contract = build_solidity(&src);

        // Signed N bits will fit the range: 2^(N-1) -1 +/-. Here we generate a random number within this range and multiply it by -1, 1 or 0.
        let first_operand_rand = rng.gen_bigint(width - 1).sub(1_u32);

        let side = vec![-1, 0, 1];
        // -1, 1 or 0
        let second_op = BigInt::from(*side.choose(&mut rng).unwrap() as i32);

        contract.constructor("test", &[]);
        let return_value = contract.function(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(
                    &first_operand_rand,
                    width.try_into().unwrap(),
                )),
                ethabi::Token::Int(bigint_to_eth(&second_op, width.try_into().unwrap())),
            ],
            &[],
            None,
        );

        let res = first_operand_rand.mul(second_op);
        assert_eq!(
            return_value,
            vec![ethabi::Token::Int(bigint_to_eth(
                &res,
                width.try_into().unwrap(),
            ))]
        );
    }
}

#[test]
fn test_mul_within_range() {
    let mut rng = rand::thread_rng();
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(uintN a, uintN b) public returns (uintN) { 
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut contract = build_solidity(&src);
        contract.constructor("test", &[]);
        for _ in 0..10 {
            // Max number to fit unsigned N bits is 0..(2^N)-1
            let limit = BigUint::from(2_u32).pow(width).sub(1_u32);

            // Generate a random number within the the range 0..2^N -1
            let first_operand_rand = rng.gen_biguint(width.into());

            // Calculate a number that when multiplied by first_operand_rand, the result will not overflow N bits (the result of this division will cast the float result to int result, therefore lowering it. The result of multiplication will never overflow).
            let second_operand_rand = limit.div(&first_operand_rand);

            let return_value = contract.function(
                "mul",
                &[
                    ethabi::Token::Uint(U256::from_big_endian(&first_operand_rand.to_bytes_be())),
                    ethabi::Token::Uint(U256::from_big_endian(&second_operand_rand.to_bytes_be())),
                ],
                &[],
                None,
            );
            let res = first_operand_rand * second_operand_rand;

            assert_eq!(
                return_value,
                vec![ethabi::Token::Uint(U256::from_big_endian(
                    &res.to_bytes_be()
                ))]
            );
        }
    }
}

#[test]
fn test_overflow_detect_signed() {
    let mut rng = rand::thread_rng();
    // For bit sizes from 8..64, LLVM has intrinsic functions for multiplication with overflow. Testing starts from int types of 72 and up. There is no need to test intrinsic LLVM functions.
    for width in (72..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity(&src);

        contract.constructor("test", &[]);

        // The max value held in signed N bits is - 2^(N-1)..2^(N-1)-1 .Generate a value that will overflow this range:
        let limit = BigInt::from(2_u32).pow(width - 1).add(1_u32);

        // Divide Limit by a random number
        let first_operand_rand = rng.gen_bigint((width - 1).into()).sub(1_u32);

        // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
        let mut second_operand_rand = limit / &first_operand_rand;

        if let Sign::Minus = second_operand_rand.sign() {
            // Decrease by 1 if negative, this is to make sure the result will overflow
            second_operand_rand = second_operand_rand.sub(1);
        } else {
            // Increase by 1 if psotive
            second_operand_rand = second_operand_rand.add(1);
        }

        let res = contract.function_must_fail(
            "mul",
            &[
                ethabi::Token::Int(bigint_to_eth(
                    &first_operand_rand,
                    width.try_into().unwrap(),
                )),
                ethabi::Token::Int(bigint_to_eth(
                    &second_operand_rand,
                    width.try_into().unwrap(),
                )),
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));
    }
}

#[test]
fn test_overflow_detect_unsigned() {
    let mut rng = rand::thread_rng();
    for width in (72..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(uintN a, uintN b) public returns (uintN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity(&src);

        contract.constructor("test", &[]);

        for _ in 0..10 {
            // N bits can hold the range 0..(2^N)-1 . Generate a value that overflows N bits
            let limit = BigUint::from(2_u32).pow(width);

            // Generate a random number within the the range 0..2^N
            let first_operand_rand = rng.gen_biguint(width.into());

            // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
            let second_operand_rand = limit.div(&first_operand_rand).add(1_usize);

            let res = contract.function_must_fail(
                "mul",
                &[
                    ethabi::Token::Uint(U256::from_big_endian(&first_operand_rand.to_bytes_be())),
                    ethabi::Token::Uint(U256::from_big_endian(&second_operand_rand.to_bytes_be())),
                ],
                &[],
                None,
            );
            assert_ne!(res, Ok(0));
        }
    }
}

#[test]
fn int() {
    let mut rng = rand::thread_rng();

    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function add(intN a, intN b) public returns (intN) {
                return a + b;
            }

            function sub(intN a, intN b) public returns (intN) {
                return a - b;
            }

            function mul(intN a, intN b) public returns (intN) {
                unchecked {
                return a * b;
                }
            }

            function div(intN a, intN b) public returns (intN) {
                 return a / b;
            }

            function mod(intN a, intN b) public returns (intN) {
                return a % b;
            }

            function or(intN a, intN b) public returns (intN) {
                return a | b;
            }

            function and(intN a, intN b) public returns (intN) {
                return a & b;
            }

            function xor(intN a, intN b) public returns (intN) {
                return a ^ b;
            }

            function shift_left(intN a, uint32 r) public returns (intN) {
                return a << r;
            }

            function shift_right(intN a, uint32 r) public returns (intN) {
                return a >> r;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut vm = build_solidity(&src);

        vm.constructor("test", &[]);

        for _ in 0..10 {
            let mut a_bs = Vec::new();
            let mut b_bs = Vec::new();

            a_bs.resize(width / 8, 0);
            b_bs.resize(width / 8, 0);

            rng.fill(&mut a_bs[..]);
            rng.fill(&mut b_bs[..]);

            let mut a = U256::from_big_endian(&a_bs);
            let mut b = U256::from_big_endian(&b_bs);

            truncate_int(&mut a, width);
            truncate_int(&mut b, width);

            let big_a = eth_to_bigint(&a, width);
            let big_b = eth_to_bigint(&b, width);

            let add = vm.function(
                "add",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let res = big_a.clone().add(&big_b);

            let res = bigint_to_eth(&res, width);

            assert_eq!(add, vec![ethabi::Token::Int(res)]);

            let sub = vm.function(
                "sub",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let res = bigint_to_eth(&big_a.clone().sub(&big_b), width);

            assert_eq!(sub, vec![ethabi::Token::Int(res)]);

            let mul = vm.function(
                "mul",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let res = bigint_to_eth(&big_a.clone().mul(&big_b), width);

            assert_eq!(mul, vec![ethabi::Token::Int(res)]);

            if b != U256::zero() {
                let div = vm.function(
                    "div",
                    &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                    &[],
                    None,
                );

                let res = bigint_to_eth(&big_a.clone().div(&big_b), width);

                assert_eq!(div, vec![ethabi::Token::Int(res)]);

                let add = vm.function(
                    "mod",
                    &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                    &[],
                    None,
                );

                let res = big_a.clone().rem(&big_b);

                let res = bigint_to_eth(&res, width);

                assert_eq!(add, vec![ethabi::Token::Int(res)]);
            }

            let or = vm.function(
                "or",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] | b.0[0],
                a.0[1] | b.0[1],
                a.0[2] | b.0[2],
                a.0[3] | b.0[3],
            ]);

            truncate_int(&mut res, width);

            assert_eq!(or, vec![ethabi::Token::Int(res)]);

            let and = vm.function(
                "and",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] & b.0[0],
                a.0[1] & b.0[1],
                a.0[2] & b.0[2],
                a.0[3] & b.0[3],
            ]);

            truncate_int(&mut res, width);

            assert_eq!(and, vec![ethabi::Token::Int(res)]);

            let xor = vm.function(
                "xor",
                &[ethabi::Token::Int(a), ethabi::Token::Int(b)],
                &[],
                None,
            );

            let mut res = U256([
                a.0[0] ^ b.0[0],
                a.0[1] ^ b.0[1],
                a.0[2] ^ b.0[2],
                a.0[3] ^ b.0[3],
            ]);

            truncate_int(&mut res, width);

            assert_eq!(xor, vec![ethabi::Token::Int(res)]);

            let r = rng.gen::<u32>() % (width as u32);

            let shl = vm.function(
                "shift_left",
                &[ethabi::Token::Int(a), ethabi::Token::Uint(U256::from(r))],
                &[],
                None,
            );

            let mut res = a.shl(r);

            truncate_int(&mut res, width);

            assert_eq!(shl, vec![ethabi::Token::Int(res)]);

            let shr = vm.function(
                "shift_right",
                &[ethabi::Token::Int(a), ethabi::Token::Uint(U256::from(r))],
                &[],
                None,
            );

            let res = bigint_to_eth(&big_a.clone().shr(r), width);

            assert_eq!(shr, vec![ethabi::Token::Int(res)]);
        }
    }
}

fn truncate_int(n: &mut U256, width: usize) {
    let sign = n.bitand(U256::from(1) << (width - 1)) != U256::zero();

    let mut bits = 256 - width;

    let mut offset = 3;

    while bits > 64 {
        n.0[offset] = if sign { u64::MAX } else { 0 };

        offset -= 1;
        bits -= 64;
    }

    if bits > 0 {
        if sign {
            n.0[offset] |= !((1 << (64 - bits)) - 1);
        } else {
            n.0[offset] &= (1 << (64 - bits)) - 1;
        }
    }
}

fn bigint_to_eth(v: &BigInt, width: usize) -> U256 {
    let mut buf = v.to_signed_bytes_be();
    let width = width / 8;

    while buf.len() > width {
        buf.remove(0);
    }

    let sign = if (buf[0] & 128) != 0 { 0xff } else { 0 };

    while buf.len() < 32 {
        buf.insert(0, sign);
    }

    U256::from_big_endian(&buf)
}

fn eth_to_bigint(v: &U256, width: usize) -> BigInt {
    let mut buf = Vec::new();

    buf.resize(32, 0);

    v.to_big_endian(&mut buf);

    let width = width / 8;

    while buf.len() > width {
        buf.remove(0);
    }

    BigInt::from_signed_bytes_be(&buf)
}
