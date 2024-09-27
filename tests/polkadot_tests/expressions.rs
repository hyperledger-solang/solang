// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, build_solidity_with_options};
use num_bigint::{BigInt, BigUint, RandBigInt, Sign};
use parity_scale_codec::{Decode, Encode};
use rand::seq::SliceRandom;
use rand::Rng;
use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Sub;

#[test]
fn celcius_and_fahrenheit() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function celcius2fahrenheit(int32 celcius) pure public returns (int32) {
                int32 fahrenheit = celcius * 9 / 5 + 32;

                return fahrenheit;
            }

            function fahrenheit2celcius(uint32 fahrenheit) pure public returns (uint32) {
                return (fahrenheit - 32) * 5 / 9;
            }
        }",
    );

    runtime.function("celcius2fahrenheit", Val(10).encode());

    assert_eq!(runtime.output(), Val(50).encode());

    runtime.function("fahrenheit2celcius", Val(50).encode());

    assert_eq!(runtime.output(), Val(10).encode());
}

#[test]
fn digits() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function digitslen(uint64 val) pure public returns (uint32) {
                uint32 count = 0;
                while (val > 0) {
                    count++;
                    val /= 10;
                }
                if (count == 0) {
                    count = 1;
                }
                return count;
            }

            function sumdigits(int64 val) pure public returns (uint32) {
                uint32 sum = 0;

                while (val > 0) {
                    sum += uint32(val % 10);
                    val= val / 10;
                }

                return sum;
            }
        }",
    );

    runtime.function("digitslen", Val64(1234567).encode());

    assert_eq!(runtime.output(), Val32(7).encode());

    runtime.function("sumdigits", Val64(123456789).encode());

    assert_eq!(runtime.output(), Val32(45).encode());
}

#[test]
fn large_loops() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo(uint val) pure public returns (uint) {
                for (uint i =0 ; i < 100; i++ ) {
                    val += i + 10;
                }

                return val;
            }

            function baz(int val) pure public returns (int) {
                return val * 1000_000;
            }

            function bar() public {
                assert(foo(10) == 5960);
                assert(baz(7_000_123) == 7_000_123_000_000);
                assert(baz(7_000_123_456_678) == 7_000_123_456_678_000_000);
            }
        }",
    );

    runtime.function("bar", Vec::new());

    let mut args = Val64(7000).encode();
    args.resize(32, 0);

    runtime.function("baz", args);

    let mut rets = Val64(7000000000).encode();
    rets.resize(32, 0);

    assert_eq!(runtime.output(), rets);
}

#[test]
fn expressions() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val16(u16);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val8(u8);

    // parse
    let mut runtime = build_solidity("
        contract test {
            // this is 2^254
            int constant large_value = 14474011154664524427946373126085988481658748083205070504932198000989141204992;

            function add_100(uint16 a) pure public returns (uint16) {
                unchecked {
                    a -= 200;
                    a += 300;
                }
                return a;
            }

            function clear_digit(uint8 a) pure public returns (uint8) {
                a /= 10;
                a *= 10;
                return a;
            }

            function low_digit(uint8 a) pure public returns (uint8) {
                a %= 10;
                return a;
            }

            function test_comparisons() pure public {
                {
                    // test comparisons work, if will work even if sign/unsigned is broken
                    uint64 left = 102;
                    uint64 right = 103;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == 103);
                    assert(left >= 102);
                    assert(right <= 103);
                    assert(!(right <= 102));
                }

                {
                    // check if unsigned compare works correctly (will fail if signed compare is done)
                    uint16 left = 102;
                    uint16 right = 0x8001;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == 0x8001);
                    assert(left >= 102);
                    assert(right <= 0x8001);
                    assert(!(right <= 102));
                }

                {
                    // check if signed compare works correctly (will fail if unsigned compare is done)
                    int left = -102;
                    int right = large_value;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == large_value);
                    assert(left >= -102);
                    assert(right <= large_value);
                    assert(!(right <= -102));
                }
            }

            function increments() public {
                uint a = 1;

                assert(a-- == 1);
                assert(a == 0);

                assert(a++ == 0);
                assert(a == 1);

                assert(--a == 0);
                assert(a == 0);

                assert(++a == 1);
                assert(a == 1);
            }
        }",
    );

    runtime.function("add_100", Val16(0xffc0).encode());

    assert_eq!(runtime.output(), Val16(36).encode());

    runtime.function("clear_digit", Val8(25).encode());

    assert_eq!(runtime.output(), Val8(20).encode());

    runtime.function("low_digit", Val8(25).encode());

    assert_eq!(runtime.output(), Val8(5).encode());

    runtime.function("test_comparisons", Vec::new());

    runtime.function("increments", Vec::new());
}

#[test]
#[should_panic(expected = "IntegerDivisionByZero")]
fn divisions_by_zero() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint256 divisor = 0;
            function do_test() public returns (uint256 result){
                result = 100 / divisor;
            }
        }",
    );

    runtime.function_expect_failure("do_test", Vec::new());
}

#[test]
fn divisions() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint constant large = 101213131318098987934191741;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn divisions64() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 constant large = 101213131318098987;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn divisions128() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(i128, i128);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Rets(i128);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint128 constant large = 101213131318098987;
            uint128 constant small = 99;

            int128 constant signed_large = 101213131318098987;
            int128 constant neg_signed_large = -101213131318098987;
            int128 constant signed_small = 99;

            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                assert(small / 10 == 9);
                assert(small % 10 == 9);

                assert(large / 100000 == 1012131313180);
                assert(large % 100000 == 98987);

                return 0;
            }

            function do_signed_test() public returns (uint) {
                assert(signed_large / 1 == signed_large);
                assert(signed_large / (signed_large + 102) == 0);
                assert(signed_large / signed_large == 1);

                assert(signed_large % 1 == 0);
                assert(signed_large % (signed_large + 102) == signed_large);
                assert(signed_large % signed_large == 0);

                assert(signed_small / 10 == 9);
                assert(signed_small % 10 == 9);

                assert(signed_large / 100000 == 1012131313180);
                assert(signed_large % 100000 == 98987);

                assert(neg_signed_large / -100000 == 1012131313180);
                assert(signed_large / -100000 == -1012131313180);
                assert(-signed_large / 100000 == -1012131313180);

                assert(signed_large % -100000 == 98987);
                assert(-signed_large % 100000 == -98987);
                assert(-signed_large % -100000 == -98987);


                return 0;
            }

            function do_div(int128 x, int128 y) public returns (int128) {
                return x / y;
            }

            function return_neg() public returns (int128) {
                return -100;
            }

            function return_pos() public returns (int128) {
                return 255;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    runtime.function("return_neg", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.output()[..]) {
        assert_eq!(r, -100);
    } else {
        panic!();
    }

    runtime.function("return_pos", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.output()[..]) {
        assert_eq!(r, 255);
    } else {
        panic!();
    }

    runtime.function("do_div", Args(-9900, -100).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.output()[..]) {
        assert_eq!(r, 99);
    } else {
        panic!();
    }

    runtime.function("do_div", Args(-101213131318098987, -100000).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.output()[..]) {
        assert_eq!(r, 1012131313180);
    } else {
        panic!();
    }

    runtime.function("do_signed_test", Vec::new());
}

#[test]
fn divisions256() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint256 constant large = 101213131318098987;
            uint256 constant small = 99;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                assert(small / 10 == 9);
                assert(small % 10 == 9);

                assert(large / 100000 == 1012131313180);
                assert(large % 100000 == 98987);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn complement() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0;
                assert(~x1 == 255);
                int32 x2 = 0x7fefabcd;
                assert(uint32(~x2) == 0x80105432);
            }

            function do_complement(uint256 foo) public returns (uint) {
                return ~foo;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let args = vec![0; 32];

    runtime.function("do_complement", args);

    let ret = runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.into_iter().filter(|x| *x == 255).count() == 32);
}

#[test]
fn bitwise() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                assert(x1 | x2 == 0xff);
                assert(x1 ^ x2 == 0xff);
                assert(x1 & x2 == 0x00);
                assert(x1 ^ 0 == x1);

                int32 x3 = 0x7fefabcd;
                assert(x3 & 0xffff == 0xabcd);
            }

            function do_or(uint256 a, uint256 b) public returns (uint) {
                return a | b;
            }

            function do_and(uint256 a, uint256 b) public returns (uint) {
                return a & b;
            }

            function do_xor(uint256 a, uint256 b) public returns (uint) {
                return a ^ b;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_xor", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_or", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_and", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn shift() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                assert(x1 >> 4 == 0x0f);
                assert(x2 << 4 == 0xf0);

                int x3 = -16;
                assert(x3 >> 2 == -4);

                uint x5 = 0xdead_0000_0000_0000_0000;
                assert(x5 >> 64 == 0xdead);

                x5 = 0xdead;
                assert(x5 << 64 == 0xdead_0000_0000_0000_0000);

            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn assign_bitwise() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                x1 |= x2;
                assert(x1 == 0xff);
                x1 = 0xf0; x2 = 0x0f;
                x1 ^= x2;
                assert(x1 == 0xff);
                x1 = 0xf0; x2 = 0x0f;
                x1 &= x2;
                assert(x1 == 0x00);
                x1 = 0xf0; x2 = 0x0f;
                x1 ^= 0;
                assert(x1 == x1);

                int32 x3 = 0x7fefabcd;
                x3 &= 0xffff;
                assert(x3 == 0xabcd);
            }

            function do_or(uint256 a, uint256 b) public returns (uint) {
                a |= b;
                return a;
            }

            function do_and(uint256 a, uint256 b) public returns (uint) {
                a &= b;
                return a;
            }

            function do_xor(uint256 a, uint256 b) public returns (uint) {
                a ^= b;
                return a;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_xor", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_or", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = vec![0; 32];
    args.resize(64, 0xff);

    runtime.function("do_and", args);

    let ret = &runtime.output();

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn assign_shift() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                x1 >>= 4;
                x2 <<= 4;
                assert(x1 == 0x0f);
                assert(x2 == 0xf0);

                int x3 = -16;
                x3 >>= 2;
                assert(x3 == -4);

                uint x5 = 0xdead_0000_0000_0000_0000;
                x5 >>= 64;
                assert(x5 == 0xdead);

                x5 = 0xdead;
                x5 <<= 64;
                assert(x5 == 0xdead_0000_0000_0000_0000);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn ternary() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;

                assert((false ? x1 : x2) == x2);
                assert((true ? x1 : x2) == x1);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn short_circuit_or() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 counter;

            function increase_counter() private returns (bool) {
                counter += 1;
                return true;
            }

            function increase_counter2() private returns (bool) {
                counter++;
                return true;
            }

            function do_test() public {
                assert(counter == 0);

                // if left of or is true, right is not evaluated
                assert(true || increase_counter());
                assert(counter == 0);

                assert(false || increase_counter2());
                assert(counter == 1);

                false && increase_counter();
                assert(counter == 1);

                true && increase_counter();
                assert(counter == 2);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn short_circuit_and() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 counter;

            function increase_counter() private returns (bool) {
                counter |= 1;
                return false;
            }

            function increase_counter2() private returns (bool) {
                ++counter;
                return false;
            }

            function do_test() public {
                assert(counter == 0);

                increase_counter2();
                increase_counter2();

                assert(counter == 2);

                increase_counter();

                assert(counter == 3);

                counter = 0;

                // if left hand side is false, right hand side is not evaluated
                assert(!(false && increase_counter()));
                assert(counter == 0);
                assert(!(true && increase_counter2()));
                assert(counter == 1);
                false && increase_counter2();
                assert(counter == 1);
                counter = 0;
                true && increase_counter();
                assert(counter == 1);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn power() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function power(uint64 base, uint64 exp) public returns (uint64) {
                return base ** exp;
            }

            function power_with_cast() public returns (uint64) {
                return uint64(2 ** 32);
            }
        }",
    );

    // 4**5 = 1024
    let args = Val(4).encode().into_iter().chain(Val(5).encode()).collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(1024).encode());

    // n ** 1 = n
    let args = Val(2345)
        .encode()
        .into_iter()
        .chain(Val(1).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(2345).encode());

    // n ** 0 = 0
    let args = Val(0xdead_beef)
        .encode()
        .into_iter()
        .chain(Val(0).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(1).encode());

    // 0 ** n = 0
    let args = Val(0)
        .encode()
        .into_iter()
        .chain(Val(0xdead_beef).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(0).encode());

    runtime.function("power_with_cast", Vec::new());

    assert_eq!(runtime.output(), Val(0x1_0000_0000).encode());
}

#[test]
fn large_power() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u128);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function power(uint128 base, uint128 exp) public returns (uint128) {
                return base ** exp;
            }
        }",
    );

    // 4**5 = 1024
    let args = Val(4).encode().into_iter().chain(Val(5).encode()).collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(1024).encode());

    // n ** 1 = n
    let args = Val(2345)
        .encode()
        .into_iter()
        .chain(Val(1).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(2345).encode());

    // n ** 0 = 0
    let args = Val(0xdeadbeef)
        .encode()
        .into_iter()
        .chain(Val(0).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(1).encode());

    // 0 ** n = 0
    let args = Val(0)
        .encode()
        .into_iter()
        .chain(Val(0xdeadbeef).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.output(), Val(0).encode());

    // 10 ** 36 = 1000000000000000000000000000000000000
    let args = Val(10)
        .encode()
        .into_iter()
        .chain(Val(36).encode())
        .collect();

    runtime.function("power", args);

    assert_eq!(
        runtime.output(),
        Val(1000000000000000000000000000000000000).encode()
    );
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
        .replace("intN", &format!("int{width}"));

        let mut contract = build_solidity_with_options(&src, false);

        let base = BigUint::from(2_u32);
        let mut base_data = base.to_bytes_le();

        let exp = BigUint::from(width - 1);
        let mut exp_data = exp.to_bytes_le();

        let width_rounded = (width / 8usize).next_power_of_two();

        base_data.resize(width_rounded, 0);
        exp_data.resize(width_rounded, 0);

        contract.function(
            "pow",
            base_data.clone().into_iter().chain(exp_data).collect(),
        );

        let res = BigUint::from(2_usize).pow((width - 1).try_into().unwrap());
        let mut res_data = res.to_bytes_le();
        res_data.resize(width / 8, 0);

        assert_eq!(contract.output()[..width / 8], res_data);

        let exp = exp.add(1_usize);
        let mut exp_data = exp.to_bytes_le();
        exp_data.resize(width_rounded, 0);

        contract.function_expect_failure("pow", base_data.into_iter().chain(exp_data).collect());
    }
}

#[test]
fn multiply() {
    let mut rng = rand::thread_rng();
    let size = 32;

    let mut runtime = build_solidity(
        "
        contract c {
            function multiply(uint a, uint b) public returns (uint) {
                unchecked {
                return a * b;
                }
            }

            function multiply_with_cast() public returns (uint64) {
                return uint64(255 * 255);
            }
        }",
    );

    runtime.function("multiply_with_cast", Vec::new());

    assert_eq!(runtime.output(), 65025u64.encode());

    let mut rand = || -> (BigInt, Vec<u8>) {
        let length = rng.gen::<usize>() % size;

        let mut data = Vec::new();

        data.resize_with(length + 1, || rng.gen());

        data.resize(size, 0);

        (BigInt::from_bytes_le(Sign::Plus, &data), data)
    };

    for _ in 0..1000 {
        let (a, a_data) = rand();
        let (b, b_data) = rand();

        println!("in: a:{a_data:?} b:{b_data:?}");

        runtime.function("multiply", a_data.into_iter().chain(b_data).collect());

        println!("out: res:{:?}", runtime.output());

        let res = BigInt::from_bytes_le(Sign::Plus, &runtime.output());

        println!("{res} = {a} * {b}");

        // the result is truncated to $size bytes. We do this here by converting to Vec<u8> and truncating
        // it. A truncating bigint multiply would be nicer.
        let (_, mut res) = (a * b).to_bytes_le();
        res.resize(size, 0);

        assert_eq!(res, runtime.output());
    }
}

#[test]
fn test_mul_within_range_signed() {
    // We generate a random value that fits N bits. Then, we multiply that value by 1, -1 or 0.
    let mut rng = rand::thread_rng();
    for width in (8..=256).step_by(8) {
        let src = r#"
        contract test {
            function mul(intN a, intN b) public returns (intN) {
                return a * b;
            }
        }"#
        .replace("intN", &format!("int{width}"));

        let width_rounded = (width / 8_usize).next_power_of_two();

        let mut runtime = build_solidity(&src);

        let upper_bound = BigInt::from(2).pow(width as u32 - 1); // Upper bound is exclusive
        let lower_bound = upper_bound.clone().mul(-1);
        let a = rng.gen_bigint_range(&lower_bound, &upper_bound);
        let a_sign = a.sign();
        let mut a_data = a.to_signed_bytes_le();

        let side = [-1, 0, 1];
        let b = BigInt::from(*side.choose(&mut rng).unwrap());
        let b_sign = b.sign();
        let mut b_data = b.to_signed_bytes_le();

        a_data.resize(width_rounded, sign_extend(a_sign));
        b_data.resize(width_rounded, sign_extend(b_sign));

        runtime.function("mul", a_data.into_iter().chain(b_data).collect());

        let value = a * b;
        let value_sign = value.sign();

        let mut value_data = value.to_signed_bytes_le();
        value_data.resize(width / 8, sign_extend(value_sign));

        assert_eq!(value_data, runtime.output()[..width / 8]);
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
        .replace("intN", &format!("int{width}"));

        let width_rounded = (width / 8usize).next_power_of_two();
        let mut runtime = build_solidity(&src);

        // The range of values that can be held in unsigned N bits is [0, 2^N-1]. Here we generate a random number within this range and multiply it by 1
        let a = rng.gen_biguint((width).try_into().unwrap());

        let mut a_data = a.to_bytes_le();

        let b = BigUint::from(1_u32);

        let mut b_data = b.to_bytes_le();
        a_data.resize(width_rounded, 0);
        b_data.resize(width_rounded, 0);

        runtime.function("mul", a_data.into_iter().chain(b_data).collect());

        let value = a * b;

        let mut value_data = value.to_bytes_le();
        value_data.resize(width / 8, 0);

        assert_eq!(value_data, runtime.output()[..width / 8]);
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
        .replace("intN", &format!("int{width}"));
        let mut contract = build_solidity_with_options(&src, false);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1]. We generate these boundaries:
        let upper_boundary = BigInt::from(2_u32).pow(width - 1).sub(1_u32);
        let mut up_data = upper_boundary.to_signed_bytes_le();

        let lower_boundary = BigInt::from(2_u32).pow(width - 1).mul(-1_i32);
        let mut low_data = lower_boundary.to_signed_bytes_le();

        let second_op = BigInt::from(1_u32);
        let mut sec_data = second_op.to_signed_bytes_le();

        let width_rounded = (width as usize / 8).next_power_of_two();

        up_data.resize(width_rounded, 0);
        low_data.resize(width_rounded, 255);
        sec_data.resize(width_rounded, 0);

        // Multiply the boundaries by 1.
        contract.function(
            "mul",
            up_data
                .clone()
                .into_iter()
                .chain(sec_data.clone())
                .collect(),
        );

        let res = upper_boundary.clone().mul(1_u32);
        let mut res_data = res.to_signed_bytes_le();
        res_data.resize((width / 8) as usize, 0);

        assert_eq!(res_data, contract.output()[..(width / 8) as usize]);

        contract.function(
            "mul",
            low_data
                .clone()
                .into_iter()
                .chain(sec_data.clone())
                .collect(),
        );

        let res = lower_boundary.clone().mul(1_u32);
        let mut res_data = res.to_signed_bytes_le();
        res_data.resize((width / 8) as usize, 0);

        assert_eq!(res_data, contract.output()[..(width / 8) as usize]);

        let upper_boundary_plus_one = BigInt::from(2_u32).pow(width - 1);

        // We subtract 2 instead of one to make the number even, so that no rounding occurs when we divide by 2 later on.
        let lower_boundary_minus_two = BigInt::from(2_u32).pow(width - 1).mul(-1_i32).sub(2_i32);

        let upper_second_op = upper_boundary_plus_one.div(2_u32);
        let mut upper_second_op_data = upper_second_op.to_signed_bytes_le();

        let lower_second_op = lower_boundary_minus_two.div(2_u32);
        let mut lower_second_op_data = lower_second_op.to_signed_bytes_le();

        let mut two_data = BigInt::from(2_u32).to_signed_bytes_le();

        upper_second_op_data.resize(width_rounded, 0);
        two_data.resize(width_rounded, 0);
        lower_second_op_data.resize(width_rounded, 255);

        // This will generate a value more than the upper boundary.
        contract.function_expect_failure(
            "mul",
            upper_second_op_data
                .clone()
                .into_iter()
                .chain(two_data.clone())
                .collect(),
        );

        // Generate a value less than the lower boundary
        contract.function_expect_failure(
            "mul",
            lower_second_op_data
                .clone()
                .into_iter()
                .chain(two_data.clone())
                .collect(),
        );

        // Upper boundary * Upper boundary
        contract.function_expect_failure(
            "mul",
            up_data.clone().into_iter().chain(up_data.clone()).collect(),
        );

        // Lower boundary * Lower boundary
        contract.function_expect_failure(
            "mul",
            low_data
                .clone()
                .into_iter()
                .chain(low_data.clone())
                .collect(),
        );

        // Lower boundary * Upper boundary
        contract.function_expect_failure(
            "mul",
            low_data
                .clone()
                .into_iter()
                .chain(up_data.clone())
                .collect(),
        );
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
        .replace("intN", &format!("int{width}"));
        let mut contract = build_solidity_with_options(&src, false);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1] .Generate a value that will overflow this range:
        let limit = BigInt::from(2_u32).pow(width - 1).sub(1_u32);

        // Generate a random number within the the range [(2^N-1)/2, (2^N-1) -1]
        let first_operand_rand =
            rng.gen_bigint_range(&(limit.clone().div(2usize)).add(1usize), &limit);

        let first_op_sign = first_operand_rand.sign();
        let mut first_op_data = first_operand_rand.to_signed_bytes_le();

        let width_rounded = (width as usize / 8).next_power_of_two();
        first_op_data.resize(width_rounded, sign_extend(first_op_sign));

        // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
        let second_operand_rand = rng.gen_bigint_range(&BigInt::from(2usize), &limit);

        let second_op_sign = second_operand_rand.sign();
        let mut second_op_data = second_operand_rand.to_signed_bytes_le();
        second_op_data.resize(width_rounded, sign_extend(second_op_sign));

        contract.function_expect_failure(
            "mul",
            first_op_data
                .into_iter()
                .chain(second_op_data.clone())
                .collect(),
        );

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1] .
        let lower_limit = BigInt::from(2_u32).pow(width - 1).sub(1usize).mul(-1_i32);

        // Generate a random number within the the range [-(2^N-1), -(2^N-1)/2]
        let first_operand_rand =
            rng.gen_bigint_range(&lower_limit, &(lower_limit.clone().div(2usize)).add(1usize));

        let first_op_sign = first_operand_rand.sign();
        let mut first_op_data = first_operand_rand.to_signed_bytes_le();
        first_op_data.resize(width_rounded, sign_extend(first_op_sign));

        contract.function_expect_failure(
            "mul",
            first_op_data.into_iter().chain(second_op_data).collect(),
        );
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
        .replace("intN", &format!("int{width}"));
        let mut contract = build_solidity_with_options(&src, false);

        // The range of values that can be held in signed N bits is [-2^(N-1), 2^(N-1)-1].
        let limit = BigUint::from(2_u32).pow(width).sub(1_u32);

        // Generate a random number within the the range [(2^N-1)/2, 2^N -1]
        let first_operand_rand =
            rng.gen_biguint_range(&(limit.clone().div(2usize)).add(1usize), &limit);

        let mut first_op_data = first_operand_rand.to_bytes_le();

        let width_rounded = (width as usize / 8).next_power_of_two();
        first_op_data.resize(width_rounded, 0);

        // Calculate a number that when multiplied by first_operand_rand, the result will overflow N bits
        let second_operand_rand = rng.gen_biguint_range(&BigUint::from(2usize), &limit);

        let mut second_op_data = second_operand_rand.to_bytes_le();
        second_op_data.resize(width_rounded, 0);

        contract.function_expect_failure(
            "mul",
            first_op_data.into_iter().chain(second_op_data).collect(),
        );
    }
}

#[test]
fn bytes_bitwise() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes3([u8; 3]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Bytes5([u8; 5]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct BytesArray([u8; 7], u32);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function or(bytes5 x) public returns (bytes5 y) {
                y = x | hex\"80808080\";
            }

            function and(bytes5 x) public returns (bytes5) {
                return x & hex\"FFFF\";
            }

            function xor(bytes5 x) public returns (bytes5) {
                x ^= 0xFF00000000;

                return x;
            }

            function shift_left(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a) << 8;
            }

            function shift_right(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a) >> 8;
            }

            function shift_left2(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a);
                b <<= 8;
            }

            function shift_right2(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a);
                b >>= 8;
            }

            function bytes_length() public {
                bytes4 b4;

                assert(b4.length == 4);
            }

            function complement(bytes3 a) public returns (bytes3) {
                return ~a;
            }

            function bytes_array(bytes7 foo, uint32 index) public returns (bytes1) {
                return foo[index];
            }
        }",
    );

    runtime.function("or", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0x81, 0x81, 0x81, 0x81, 0x01]).encode()
    );

    runtime.function("and", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(runtime.output(), Bytes5([0x01, 0x01, 0, 0, 0]).encode());

    runtime.function("xor", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0xfe, 0x01, 0x01, 0x01, 0x01]).encode()
    );

    // shifty-shift
    runtime.function("shift_left", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0x7d, 0x03, 0x00, 0x00, 0x00]).encode()
    );

    runtime.function("shift_right", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0x00, 0xf3, 0x7d, 0x03, 0x00]).encode()
    );

    // assignment versions
    runtime.function("shift_left2", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0x7d, 0x03, 0x00, 0x00, 0x00]).encode()
    );

    runtime.function("shift_right2", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.output(),
        Bytes5([0x00, 0xf3, 0x7d, 0x03, 0x00]).encode()
    );

    // check length
    runtime.function("bytes_length", Vec::new());

    // complement
    runtime.function("complement", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(runtime.output(), Bytes3([0x0c, 0x82, 0xfc]).encode());

    // array access
    let bytes7 = *b"NAWABRA";
    for i in 0..6 {
        runtime.function("bytes_array", BytesArray(bytes7, i).encode());

        assert_eq!(runtime.output(), [bytes7[i as usize]]);
    }
}

#[test]
fn bytesn_underflow_index_acccess() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct BytesArray([u8; 7], i32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function bytes_array(bytes7 foo, int32 index) public returns (bytes1) {
                return foo[uint(index)];
            }
        }",
    );

    runtime.function_expect_failure("bytes_array", BytesArray(*b"nawabra", -1).encode());
}

#[test]
fn bytesn_overflow_index_acccess() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct BytesArray([u8; 7], i32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function bytes_array(bytes7 foo, int32 index) public returns (byte) {
                return foo[uint(index)];
            }
        }",
    );

    runtime.function_expect_failure("bytes_array", BytesArray(*b"nawabra", 7).encode());
}

#[test]
fn negation_and_subtract() {
    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                uint32 x = 10-10;
                assert(x == 0);
                int32 y = -10-10;
                assert(y == -20);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn div() {
    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test1() public {
                // see https://solidity.readthedocs.io/en/latest/types.html#modulo
                assert(int256(5) % int256(2) == int256(1));
                assert(int256(5) % int256(-2) == int256(1));
                assert(int256(-5) % int256(2) == int256(-1));
                assert(int256(-5) % int256(-2) == int256(-1));

                assert(int64(5) % int64(2) == int64(1));
                assert(int64(5) % int64(-2) == int64(1));
                assert(int64(-5) % int64(2) == int64(-1));
                assert(int64(-5) % int64(-2) == int64(-1));
            }

            function test2() public {
                // see https://github.com/hyperledger-solang/burrow/pull/1367#issue-399914366
                assert(int256(7) / int256(3) == int256(2));
                assert(int256(7) / int256(-3) == int256(-2));
                assert(int256(-7) / int256(3) == int256(-2));
                assert(int256(-7) / int256(-3) == int256(2));

                assert(int256(7) % int256(3) == int256(1));
                assert(int256(7) % int256(-3) == int256(1));
                assert(int256(-7) % int256(3) == int256(-1));
                assert(int256(-7) % int256(-3) == int256(-1));

                assert(int64(7) / int64(3) == int64(2));
                assert(int64(7) / int64(-3) == int64(-2));
                assert(int64(-7) / int64(3) == int64(-2));
                assert(int64(-7) / int64(-3) == int64(2));

                assert(int64(7) % int64(3) == int64(1));
                assert(int64(7) % int64(-3) == int64(1));
                assert(int64(-7) % int64(3) == int64(-1));
                assert(int64(-7) % int64(-3) == int64(-1));
            }
        }"#,
    );

    runtime.function("test1", Vec::new());

    runtime.function("test2", Vec::new());
}

#[test]
fn destructure() {
    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                int a;
                int b;

                // test one
                (a, b) = (102, 3);

                assert(b == 3 && a == 102);

                // test missing one
                (a, , b) = (1, 2, 3);

                assert(a == 1 && b == 3);

                // test single one
                (a) = 5;

                assert(a == 5);

                // or like so
                (a) = (105);

                assert(a == 105);
            }

            function swap() public {
                int32 a;
                int32 b;

                // test one
                (a, b) = (102, 3);

                // test swap
                (b, a) = (a, b);

                assert(a == 3 && b == 102);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    runtime.function("swap", Vec::new());

    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                // test one
                (int32 a, int32 b) = (102, 3);

                assert(b == 3 && a == 102);

                // test missing one
                (a, , b) = (1, 2, 3);

                assert(a == 1 && b == 3);

                // test single one
                (a) = 5;

                assert(a == 5);

                // or like so
                (a) = (105);

                assert(a == 105);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn addition_overflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract overflow {
            function foo(uint8 x) internal returns (uint8) {
                uint8 y = x + 1;
                return y;
            }

            function bar() public {
                foo(255);
            }
        }
        "#,
        false,
    );

    runtime.function_expect_failure("bar", Vec::new());
}

#[test]
fn unchecked_addition_overflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract overflow {
            function foo(uint8 x) internal returns (uint8) {
                unchecked {
                    uint8 y = x + 1;
                    return y;
                }
            }

            function bar() public {
                foo(255);
            }
        }
        "#,
        false,
    );

    runtime.function("bar", Vec::new());
}

#[test]
fn subtraction_underflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract underflow {
            function foo(uint64 x) internal returns (uint64) {
                uint64 y = x - 1;
                return y;
            }

            function bar() public {
                foo(0);
            }
        }
        "#,
        false,
    );

    runtime.function_expect_failure("bar", Vec::new());
}

#[test]
fn unchecked_subtraction_underflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract underflow {
            function foo(uint64 x) internal returns (uint64) {
                unchecked {
                    uint64 y = x - 1;
                    return y;
                }
            }

            function bar() public {
                foo(0);
            }
        }
        "#,
        false,
    );

    runtime.function("bar", Vec::new());
}

#[test]
fn multiplication_overflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract overflow {
            function foo(int8 x) internal returns (int8) {
                int8 y = x * int8(64);
                return y;
            }

            function bar() public {
                foo(8);
            }
        }
        "#,
        false,
    );

    runtime.function_expect_failure("bar", Vec::new());
}

#[test]
fn unchecked_multiplication_overflow() {
    let mut runtime = build_solidity_with_options(
        r#"
        contract overflow {
            function foo(int8 x) internal returns (int8) {
                unchecked {
                    int8 y = x * int8(64);
                    return y;
                }
            }

            function bar() public {
                foo(8);
            }
        }
        "#,
        false,
    );

    runtime.function("bar", Vec::new());
}

#[test]
fn address_compare() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args([u8; 32], [u8; 32]);

    let mut runtime = build_solidity(
        r#"
        contract addr {
            function bar() public {
                address left = address(1);
                address right = address(2);

                assert(left < right);
                assert(left <= right);
                assert(right > left);
                assert(right >= left);
            }

            function order(address tokenA, address tokenB) external returns (address token0, address token1) {
                require(tokenA != tokenB, 'UniswapV2: IDENTICAL_ADDRESSES');
                (token0,  token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
            }
        }"#,
    );

    runtime.function("bar", Vec::new());

    let address0: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ];

    let address1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 2,
    ];

    runtime.function("order", Args(address0, address1).encode());

    assert_eq!(runtime.output(), Args(address0, address1).encode());

    runtime.function("order", Args(address1, address0).encode());

    assert_eq!(runtime.output(), Args(address0, address1).encode());
}

#[test]
fn address_pass_by_value() {
    let mut runtime = build_solidity(
        r#"
        contract addr {
            function bar() public {
                address left = address(1);

                foo(left);

                assert(left == address(1));
            }

            function foo(address a) internal {
                a = address(2);
            }
        }"#,
    );

    runtime.function("bar", Vec::new());
}

fn sign_extend(sign: Sign) -> u8 {
    if sign == Sign::Minus {
        255
    } else {
        0
    }
}

/// Given a chain of assignments, with the leftmost hand being a return parameter.
/// It should compile fine and all values in the chain should be assigned the right most value.
#[test]
fn assign_chained() {
    let mut runtime = build_solidity(
        r#"
    contract C {
        uint64 public foo;
        uint64 public bar;
    
        function f(uint64 x) public returns (uint64) {
            return foo = bar = x;
        }
    }
    "#,
    );

    let expected_output = 42u64.encode();

    runtime.function("f", expected_output.clone());
    assert_eq!(runtime.output(), &expected_output[..]);

    runtime.function("foo", Vec::new());
    assert_eq!(runtime.output(), &expected_output[..]);

    runtime.function("bar", Vec::new());
    assert_eq!(runtime.output(), &expected_output[..]);
}
