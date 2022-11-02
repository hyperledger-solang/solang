// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity_with_overflow_check;
use crate::{build_solidity, BorshToken};
use num_bigint::{BigInt, BigUint, RandBigInt, ToBigInt};
use num_traits::{ToPrimitive, Zero};
use rand::seq::SliceRandom;
use rand::Rng;
use std::ops::BitAnd;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Rem;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Sub;
use std::ops::{Add, BitOr, BitXor, MulAssign, ShlAssign, ShrAssign, SubAssign};

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

    assert_eq!(returns, vec![BorshToken::Bool(true),]);

    let returns = vm.function("return_false", &[], &[], None);

    assert_eq!(returns, vec![BorshToken::Bool(false),]);

    vm.function("true_arg", &[BorshToken::Bool(true)], &[], None);
    vm.function("false_arg", &[BorshToken::Bool(false)], &[], None);
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
        vec![BorshToken::FixedBytes(vec![
            171, 59, 10, 127, 211, 122, 217, 123, 53, 213, 159, 40, 54, 36, 50, 52, 196, 144, 17,
            226, 97, 168, 69, 213, 79, 14, 6, 232, 165, 44, 58, 31
        ]),]
    );

    vm.function(
        "address_arg",
        &[BorshToken::FixedBytes(vec![
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

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 8,
            value: BigInt::from(9u8)
        }]
    );

    vm.function(
        "enum_arg",
        &[BorshToken::Uint {
            width: 8,
            value: BigInt::from(6u8),
        }],
        &[],
        None,
    );
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
            vec![BorshToken::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7]),]
        );

        let returns = vm.function(
            "return_arg",
            &[BorshToken::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7])],
            &[],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7])]
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
                    BorshToken::FixedBytes(a.to_vec()),
                    BorshToken::FixedBytes(b.to_vec()),
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

            assert_eq!(or, vec![BorshToken::FixedBytes(res)]);

            let and = vm.function(
                "and",
                &[
                    BorshToken::FixedBytes(a.to_vec()),
                    BorshToken::FixedBytes(b.to_vec()),
                ],
                &[],
                None,
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a & b).collect();

            assert_eq!(and, vec![BorshToken::FixedBytes(res)]);

            let xor = vm.function(
                "xor",
                &[
                    BorshToken::FixedBytes(a.to_vec()),
                    BorshToken::FixedBytes(b.to_vec()),
                ],
                &[],
                None,
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a ^ b).collect();

            assert_eq!(xor, vec![BorshToken::FixedBytes(res)]);

            let r = rng.gen::<u32>() % (width as u32 * 8);

            println!("w = {} r = {}", width, r);

            let shl = vm.function(
                "shift_left",
                &[
                    BorshToken::FixedBytes(a.to_vec()),
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
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

            assert_eq!(shl, vec![BorshToken::FixedBytes(res)]);

            let shr = vm.function(
                "shift_right",
                &[
                    BorshToken::FixedBytes(a.to_vec()),
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
                ],
                &[],
                None,
            );

            let mut res = (BigUint::from_bytes_be(&a) >> r).to_bytes_be();

            while res.len() < width {
                res.insert(0, 0);
            }

            assert_eq!(shr, vec![BorshToken::FixedBytes(res)]);
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
            let mut a = rng.gen_biguint(width as u64);
            let mut b = rng.gen_biguint(width as u64);
            if b > a {
                std::mem::swap(&mut a, &mut b);
            }

            let res = vm.function(
                "pass",
                &[BorshToken::Uint {
                    width: width as u16,
                    value: a.to_bigint().unwrap(),
                }],
                &[],
                None,
            );

            println!("{:x} = {:?} o", a, res);

            let add = vm.function(
                "add",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().add(&b);
            truncate_biguint(&mut res, width);

            println!("{:x} + {:x} = {:?} or {:x}", a, b, add, res);

            assert_eq!(
                add,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let sub = vm.function(
                "sub",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().sub(&b);
            truncate_biguint(&mut res, width);

            assert_eq!(
                sub,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let mul = vm.function(
                "mul",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().mul(&b);
            truncate_biguint(&mut res, width);

            assert_eq!(
                mul,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            if let Some(mut n) = b.to_u32() {
                n %= 65536;
                let pow = vm.function(
                    "pow",
                    &[
                        BorshToken::Uint {
                            width: width as u16,
                            value: a.to_bigint().unwrap(),
                        },
                        BorshToken::Uint {
                            width: width as u16,
                            value: BigInt::from(n),
                        },
                    ],
                    &[],
                    None,
                );

                let mut res = a.clone().pow(n);
                truncate_biguint(&mut res, width);

                assert_eq!(
                    pow,
                    vec![BorshToken::Uint {
                        width: width as u16,
                        value: res.to_bigint().unwrap(),
                    }]
                );
            }

            if b != BigUint::zero() {
                let div = vm.function(
                    "div",
                    &[
                        BorshToken::Uint {
                            width: width as u16,
                            value: a.to_bigint().unwrap(),
                        },
                        BorshToken::Uint {
                            width: width as u16,
                            value: b.to_bigint().unwrap(),
                        },
                    ],
                    &[],
                    None,
                );

                let mut res = a.clone().div(&b);

                truncate_biguint(&mut res, width);

                assert_eq!(
                    div,
                    vec![BorshToken::Uint {
                        width: width as u16,
                        value: res.to_bigint().unwrap(),
                    }]
                );

                let add = vm.function(
                    "mod",
                    &[
                        BorshToken::Uint {
                            width: width as u16,
                            value: a.to_bigint().unwrap(),
                        },
                        BorshToken::Uint {
                            width: width as u16,
                            value: b.to_bigint().unwrap(),
                        },
                    ],
                    &[],
                    None,
                );

                let mut res = a.clone().rem(&b);

                truncate_biguint(&mut res, width);

                assert_eq!(
                    add,
                    vec![BorshToken::Uint {
                        width: width as u16,
                        value: res.to_bigint().unwrap(),
                    }]
                );
            }

            let or = vm.function(
                "or",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitor(&b);
            truncate_biguint(&mut res, width);

            assert_eq!(
                or,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let and = vm.function(
                "and",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitand(&b);
            truncate_biguint(&mut res, width);

            assert_eq!(
                and,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let xor = vm.function(
                "xor",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: b.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitxor(&b);
            truncate_biguint(&mut res, width);

            assert_eq!(
                xor,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let r = rng.gen::<u32>() % (width as u32);

            let shl = vm.function(
                "shift_left",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone();
            res.shl_assign(r);
            truncate_biguint(&mut res, width);

            assert_eq!(
                shl,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );

            let shr = vm.function(
                "shift_right",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: a.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone();
            res.shr_assign(&r);
            truncate_biguint(&mut res, width);

            assert_eq!(
                shr,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );
        }
    }
}

fn truncate_biguint(n: &mut BigUint, width: usize) {
    let mut bytes = n.to_bytes_le();
    let byte_width = width / 8;
    if bytes.len() < byte_width {
        return;
    }

    for item in bytes.iter_mut().skip(byte_width) {
        *item = 0;
    }

    *n = BigUint::from_bytes_le(&bytes);
}

#[test]
fn test_power_overflow_boundaries() {
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function pow(uintN a, uintN b) public returns (uintN) {
                return a ** b;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut contract = build_solidity_with_overflow_check(&src, true);
        contract.constructor("test", &[]);

        let return_value = contract.function(
            "pow",
            &[
                BorshToken::Uint {
                    width,
                    value: BigInt::from(2u8),
                },
                BorshToken::Uint {
                    width,
                    value: BigInt::from(width - 1),
                },
            ],
            &[],
            None,
        );

        let res = BigUint::from(2_u32).pow((width - 1) as u32);

        assert_eq!(
            return_value,
            vec![BorshToken::Uint {
                width,
                value: res.to_bigint().unwrap(),
            }]
        );

        let sesa = contract.function_must_fail(
            "pow",
            &[
                BorshToken::Uint {
                    width,
                    value: BigInt::from(2u8),
                },
                BorshToken::Uint {
                    width,
                    value: BigInt::from(width + 1),
                },
            ],
            &[],
            None,
        );

        assert_ne!(sesa, Ok(0));
    }
}

#[test]
fn test_overflow_boundaries() {
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity_with_overflow_check(&src, true);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1]. We generate these boundaries:
        let mut upper_boundary: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);
        upper_boundary.sub_assign(1);
        let mut lower_boundary: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);
        lower_boundary.mul_assign(-1);
        let second_op = BigInt::from(1_u32);

        // Multiply the boundaries by 1.
        contract.constructor("test", &[]);
        let return_value = contract.function(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: upper_boundary.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: second_op.clone(),
                },
            ],
            &[],
            None,
        );
        assert_eq!(
            return_value,
            vec![BorshToken::Int {
                width: width as u16,
                value: upper_boundary.clone(),
            }]
        );

        let return_value = contract.function(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: lower_boundary.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: second_op.clone(),
                },
            ],
            &[],
            None,
        );
        assert_eq!(
            return_value,
            vec![BorshToken::Int {
                width: width as u16,
                value: lower_boundary.clone(),
            },]
        );

        let upper_boundary_plus_one: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);

        // We subtract 2 instead of one to make the number even, so that no rounding occurs when we divide by 2 later on.
        let mut lower_boundary_minus_two: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);
        lower_boundary_minus_two.mul_assign(-1_i32);
        lower_boundary_minus_two.sub_assign(2_i32);

        let upper_second_op = upper_boundary_plus_one.div(2);

        let lower_second_op = lower_boundary_minus_two.div(2);

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: upper_second_op,
                },
                BorshToken::Int {
                    width: width as u16,
                    value: BigInt::from(2u8),
                },
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: lower_second_op,
                },
                BorshToken::Int {
                    width: width as u16,
                    value: BigInt::from(2),
                },
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: upper_boundary.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: upper_boundary.clone(),
                },
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: lower_boundary.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: lower_boundary.clone(),
                },
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: upper_boundary.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: lower_boundary.clone(),
                },
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
                print("{}*{}".format(a, b));
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));

        let mut contract = build_solidity(&src);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1]. Here we generate a random number within this range and multiply it by -1, 1 or 0.
        let first_operand_rand = rng.gen_bigint(width - 1).sub(1_u32);
        println!("First op : {:?}", first_operand_rand);

        let side = vec![-1, 0, 1];
        // -1, 1 or 0
        let second_op = BigInt::from(*side.choose(&mut rng).unwrap());
        println!("second op : {:?}", second_op);

        contract.constructor("test", &[]);
        let return_value = contract.function(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: first_operand_rand.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: second_op.clone(),
                },
            ],
            &[],
            None,
        );

        let res = first_operand_rand.mul(second_op);
        assert_eq!(
            return_value,
            vec![BorshToken::Int {
                width: width as u16,
                value: res,
            }]
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

        let mut contract = build_solidity_with_overflow_check(&src, true);
        contract.constructor("test", &[]);
        for _ in 0..10 {
            // Max number to fit unsigned N bits is (2^N)-1
            let mut limit: BigUint = BigUint::from(2_u32).pow(width as u32);
            limit.sub_assign(1u8);

            // Generate a random number within the the range [0, 2^N -1]
            let first_operand_rand = rng.gen_biguint_range(&BigUint::from(1usize), &limit);

            // Calculate a number that when multiplied by first_operand_rand, the result will not overflow N bits (the result of this division will cast the float result to int result, therefore lowering it. The result of multiplication will never overflow).
            let second_operand_rand = limit.div(&first_operand_rand);

            let return_value = contract.function(
                "mul",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: first_operand_rand.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: second_operand_rand.to_bigint().unwrap(),
                    },
                ],
                &[],
                None,
            );
            let res = first_operand_rand * second_operand_rand;

            assert_eq!(
                return_value,
                vec![BorshToken::Uint {
                    width: width as u16,
                    value: res.to_bigint().unwrap(),
                }]
            );
        }
    }
}

#[test]
fn test_overflow_detect_signed() {
    let mut rng = rand::thread_rng();
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity_with_overflow_check(&src, true);

        contract.constructor("test", &[]);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1] .
        let mut limit: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);
        limit.sub_assign(1u8);

        // Generate a random number within the the range [(2^N-1)/2, (2^N-1) -1]
        let first_operand_rand =
            rng.gen_bigint_range(&(limit.clone().div(2usize)).add(1usize), &limit);

        // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
        let second_operand_rand = rng.gen_bigint_range(&BigInt::from(2usize), &limit);

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: first_operand_rand.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: second_operand_rand.clone(),
                },
            ],
            &[],
            None,
        );

        assert_ne!(res, Ok(0));

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1] .
        let mut lower_limit: BigInt = BigInt::from(2_u32).pow((width - 1) as u32);
        lower_limit.sub_assign(1usize);
        lower_limit.mul_assign(-1_i32);

        // Generate a random number within the the range [-(2^N-1), -(2^N-1)/2]
        let first_operand_rand =
            rng.gen_bigint_range(&lower_limit, &(lower_limit.clone().div(2usize)).add(1usize));

        let res = contract.function_must_fail(
            "mul",
            &[
                BorshToken::Int {
                    width: width as u16,
                    value: first_operand_rand.clone(),
                },
                BorshToken::Int {
                    width: width as u16,
                    value: second_operand_rand.clone(),
                },
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
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(uintN a, uintN b) public returns (uintN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{}", width));
        let mut contract = build_solidity_with_overflow_check(&src, true);

        contract.constructor("test", &[]);

        for _ in 0..10 {
            // N bits can hold the range [0, (2^N)-1]. Generate a value that overflows N bits
            let mut limit: BigUint = BigUint::from(2_u32).pow(width as u32);
            limit.sub_assign(1u8);

            // Generate a random number within the the range [(2^N-1)/2, 2^N -1]
            let first_operand_rand =
                rng.gen_biguint_range(&(limit.clone().div(2usize)).add(1usize), &limit);

            // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
            let second_operand_rand = rng.gen_biguint_range(&BigUint::from(2usize), &limit);

            let res = contract.function_must_fail(
                "mul",
                &[
                    BorshToken::Uint {
                        width: width as u16,
                        value: first_operand_rand.to_bigint().unwrap(),
                    },
                    BorshToken::Uint {
                        width: width as u16,
                        value: second_operand_rand.to_bigint().unwrap(),
                    },
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
            let a = rng.gen_bigint(width - 1);
            let b = rng.gen_bigint(width - 1);

            let add = vm.function(
                "add",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().add(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                add,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let sub = vm.function(
                "sub",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().sub(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                sub,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let mul = vm.function(
                "mul",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().mul(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                mul,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            if b != BigInt::zero() {
                let div = vm.function(
                    "div",
                    &[
                        BorshToken::Int {
                            width: width as u16,
                            value: a.clone(),
                        },
                        BorshToken::Int {
                            width: width as u16,
                            value: b.clone(),
                        },
                    ],
                    &[],
                    None,
                );

                let mut res = a.clone().div(&b);
                truncate_bigint(&mut res, width as usize);

                assert_eq!(
                    div,
                    vec![BorshToken::Int {
                        width: width as u16,
                        value: res,
                    }]
                );

                let add = vm.function(
                    "mod",
                    &[
                        BorshToken::Int {
                            width: width as u16,
                            value: a.clone(),
                        },
                        BorshToken::Int {
                            width: width as u16,
                            value: b.clone(),
                        },
                    ],
                    &[],
                    None,
                );

                let mut res = a.clone().rem(&b);
                truncate_bigint(&mut res, width as usize);

                assert_eq!(
                    add,
                    vec![BorshToken::Int {
                        width: width as u16,
                        value: res,
                    }]
                );
            }

            let or = vm.function(
                "or",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitor(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                or,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let and = vm.function(
                "and",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitand(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                and,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let xor = vm.function(
                "xor",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Int {
                        width: width as u16,
                        value: b.clone(),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().bitxor(&b);
            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                xor,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let r = rng.gen::<u32>() % (width as u32);

            let shl = vm.function(
                "shift_left",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.clone().shl(r);

            truncate_bigint(&mut res, width as usize);

            assert_eq!(
                shl,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );

            let shr = vm.function(
                "shift_right",
                &[
                    BorshToken::Int {
                        width: width as u16,
                        value: a.clone(),
                    },
                    BorshToken::Uint {
                        width: 32,
                        value: BigInt::from(r),
                    },
                ],
                &[],
                None,
            );

            let mut res = a.shr(r);
            truncate_bigint(&mut res, width as usize);
            assert_eq!(
                shr,
                vec![BorshToken::Int {
                    width: width as u16,
                    value: res,
                }]
            );
        }
    }
}

fn truncate_bigint(n: &mut BigInt, width: usize) {
    let mut bytes_le = n.to_signed_bytes_le();
    let bytes_width = width / 8;
    if bytes_le.len() < bytes_width {
        return;
    }
    while bytes_le.len() > bytes_width {
        bytes_le.pop();
    }
    *n = BigInt::from_signed_bytes_le(&bytes_le);
}
