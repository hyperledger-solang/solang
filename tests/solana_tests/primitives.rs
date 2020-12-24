use crate::build_solidity;
use num_bigint::{BigInt, BigUint};
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

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
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

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
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

    vm.constructor(&[]);

    let returns = vm.function("return_true", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(true),]);

    let returns = vm.function("return_false", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(false),]);

    vm.function("true_arg", &[ethabi::Token::Bool(true)]);
    vm.function("false_arg", &[ethabi::Token::Bool(false)]);
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

    vm.constructor(&[]);

    let returns = vm.function("return_address", &[]);

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

    vm.constructor(&[]);

    let returns = vm.function("return_enum", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(9))]
    );

    vm.function(
        "enum_arg",
        &[ethabi::Token::Uint(ethereum_types::U256::from(6))],
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

        vm.constructor(&[]);

        let returns = vm.function("return_literal", &[]);

        assert_eq!(
            returns,
            vec![ethabi::Token::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7]),]
        );

        let returns = vm.function(
            "return_arg",
            &[ethabi::Token::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7])],
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
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a & b).collect();

            assert_eq!(and, vec![ethabi::Token::FixedBytes(res)]);

            let xor = vm.function(
                "xor",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::FixedBytes(b.to_vec()),
                ],
            );

            let res: Vec<u8> = a.iter().zip(b.iter()).map(|(a, b)| a ^ b).collect();

            assert_eq!(xor, vec![ethabi::Token::FixedBytes(res)]);

            let r = rng.gen::<u32>() % (width as u32 * 8);

            println!("w = {} r = {}", width, r);

            let shl = vm.function(
                "shift_left",
                &[
                    ethabi::Token::FixedBytes(a.to_vec()),
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
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
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
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

    for width in &[8, 16, 32, 64, 128, 256] {
        let width: usize = *width;
        let src = r#"
        contract test {
            function add(uintN a, uintN b) public returns (uintN) {
                return a + b;
            }

            function sub(uintN a, uintN b) public returns (uintN) {
                return a - b;
            }

            function mul(uintN a, uintN b) public returns (uintN) {
                return a * b;
            }

            function div(uintN a, uintN b) public returns (uintN) {
                return a / b;
            }

            function mod(uintN a, uintN b) public returns (uintN) {
                return a % b;
            }

            function pow(uintN a, uintN b) public returns (uintN) {
                return a ** b;
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

        vm.constructor(&[]);

        for _ in 0..10 {
            let mut a = Vec::new();
            let mut b = Vec::new();

            a.resize(width / 8, 0);
            b.resize(width / 8, 0);

            rng.fill(&mut a[..]);
            rng.fill(&mut b[..]);

            let mut a = ethereum_types::U256::from_big_endian(&a);
            let mut b = ethereum_types::U256::from_big_endian(&b);

            rng.fill(&mut a.0[..]);
            rng.fill(&mut b.0[..]);

            truncate_uint(&mut a, width);
            truncate_uint(&mut b, width);

            let add = vm.function("add", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let (mut res, _) = a.overflowing_add(b);

            truncate_uint(&mut res, width);

            assert_eq!(add, vec![ethabi::Token::Uint(res)]);

            let sub = vm.function("sub", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let (mut res, _) = a.overflowing_sub(b);

            truncate_uint(&mut res, width);

            assert_eq!(sub, vec![ethabi::Token::Uint(res)]);

            let mul = vm.function("mul", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let (mut res, _) = a.overflowing_mul(b);

            truncate_uint(&mut res, width);

            assert_eq!(mul, vec![ethabi::Token::Uint(res)]);

            let pow = vm.function("pow", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let (mut res, _) = a.overflowing_pow(b);

            truncate_uint(&mut res, width);

            assert_eq!(pow, vec![ethabi::Token::Uint(res)]);

            if b != ethereum_types::U256::zero() {
                let div = vm.function("div", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

                let mut res = a.div(b);

                truncate_uint(&mut res, width);

                assert_eq!(div, vec![ethabi::Token::Uint(res)]);

                let add = vm.function("mod", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

                let mut res = a.rem(b);

                truncate_uint(&mut res, width);

                assert_eq!(add, vec![ethabi::Token::Uint(res)]);
            }

            let or = vm.function("or", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let mut res = ethereum_types::U256([
                a.0[0] | b.0[0],
                a.0[1] | b.0[1],
                a.0[2] | b.0[2],
                a.0[3] | b.0[3],
            ]);

            truncate_uint(&mut res, width);

            assert_eq!(or, vec![ethabi::Token::Uint(res)]);

            let and = vm.function("and", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let mut res = ethereum_types::U256([
                a.0[0] & b.0[0],
                a.0[1] & b.0[1],
                a.0[2] & b.0[2],
                a.0[3] & b.0[3],
            ]);

            truncate_uint(&mut res, width);

            assert_eq!(and, vec![ethabi::Token::Uint(res)]);

            let xor = vm.function("xor", &[ethabi::Token::Uint(a), ethabi::Token::Uint(b)]);

            let mut res = ethereum_types::U256([
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
                &[
                    ethabi::Token::Uint(a),
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
            );

            let mut res = a.shl(r);

            truncate_uint(&mut res, width);

            assert_eq!(shl, vec![ethabi::Token::Uint(res)]);

            let shr = vm.function(
                "shift_right",
                &[
                    ethabi::Token::Uint(a),
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
            );

            let mut res = a.shr(r);

            truncate_uint(&mut res, width);

            assert_eq!(shr, vec![ethabi::Token::Uint(res)]);
        }
    }
}

fn truncate_uint(n: &mut ethereum_types::U256, width: usize) {
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
fn int() {
    let mut rng = rand::thread_rng();

    for width in &[8, 16, 32, 64, 128, 256] {
        let width: usize = *width;
        let src = r#"
        contract test {
            function add(intN a, intN b) public returns (intN) {
                return a + b;
            }

            function sub(intN a, intN b) public returns (intN) {
                return a - b;
            }

            function mul(intN a, intN b) public returns (intN) {
                return a * b;
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

        vm.constructor(&[]);

        for _ in 0..10 {
            let mut a_bs = Vec::new();
            let mut b_bs = Vec::new();

            a_bs.resize(width / 8, 0);
            b_bs.resize(width / 8, 0);

            rng.fill(&mut a_bs[..]);
            rng.fill(&mut b_bs[..]);

            let mut a = ethereum_types::U256::from_big_endian(&a_bs);
            let mut b = ethereum_types::U256::from_big_endian(&b_bs);

            truncate_int(&mut a, width);
            truncate_int(&mut b, width);

            let big_a = eth_to_bigint(&a, width);
            let big_b = eth_to_bigint(&b, width);

            let add = vm.function("add", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let res = big_a.clone().add(&big_b);

            let res = bigint_to_eth(&res, width);

            assert_eq!(add, vec![ethabi::Token::Int(res)]);

            let sub = vm.function("sub", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let res = bigint_to_eth(&big_a.clone().sub(&big_b), width);

            assert_eq!(sub, vec![ethabi::Token::Int(res)]);

            let mul = vm.function("mul", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let res = bigint_to_eth(&big_a.clone().mul(&big_b), width);

            assert_eq!(mul, vec![ethabi::Token::Int(res)]);

            if b != ethereum_types::U256::zero() {
                let div = vm.function("div", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

                let res = bigint_to_eth(&big_a.clone().div(&big_b), width);

                assert_eq!(div, vec![ethabi::Token::Int(res)]);

                let add = vm.function("mod", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

                let res = big_a.clone().rem(&big_b);

                let res = bigint_to_eth(&res, width);

                assert_eq!(add, vec![ethabi::Token::Int(res)]);
            }

            let or = vm.function("or", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let mut res = ethereum_types::U256([
                a.0[0] | b.0[0],
                a.0[1] | b.0[1],
                a.0[2] | b.0[2],
                a.0[3] | b.0[3],
            ]);

            truncate_int(&mut res, width);

            assert_eq!(or, vec![ethabi::Token::Int(res)]);

            let and = vm.function("and", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let mut res = ethereum_types::U256([
                a.0[0] & b.0[0],
                a.0[1] & b.0[1],
                a.0[2] & b.0[2],
                a.0[3] & b.0[3],
            ]);

            truncate_int(&mut res, width);

            assert_eq!(and, vec![ethabi::Token::Int(res)]);

            let xor = vm.function("xor", &[ethabi::Token::Int(a), ethabi::Token::Int(b)]);

            let mut res = ethereum_types::U256([
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
                &[
                    ethabi::Token::Int(a),
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
            );

            let mut res = a.shl(r);

            truncate_int(&mut res, width);

            assert_eq!(shl, vec![ethabi::Token::Int(res)]);

            let shr = vm.function(
                "shift_right",
                &[
                    ethabi::Token::Int(a),
                    ethabi::Token::Uint(ethereum_types::U256::from(r)),
                ],
            );

            let res = bigint_to_eth(&big_a.clone().shr(r), width);

            assert_eq!(shr, vec![ethabi::Token::Int(res)]);
        }
    }
}

fn truncate_int(n: &mut ethereum_types::U256, width: usize) {
    let sign =
        n.bitand(ethereum_types::U256::from(1) << (width - 1)) != ethereum_types::U256::zero();

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

fn bigint_to_eth(v: &BigInt, width: usize) -> ethereum_types::U256 {
    let mut buf = v.to_signed_bytes_be();
    let width = width / 8;

    while buf.len() > width {
        buf.remove(0);
    }

    let sign = if (buf[0] & 128) != 0 { 0xff } else { 0 };

    while buf.len() < 32 {
        buf.insert(0, sign);
    }

    ethereum_types::U256::from_big_endian(&buf)
}

fn eth_to_bigint(v: &ethereum_types::U256, width: usize) -> BigInt {
    let mut buf = Vec::new();

    buf.resize(32, 0);

    v.to_big_endian(&mut buf);

    let width = width / 8;

    while buf.len() > width {
        buf.remove(0);
    }

    BigInt::from_signed_bytes_be(&buf)
}
