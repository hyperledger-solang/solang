#![cfg(test)]
use super::expression_values::expression_values;
use super::{highest_set_bit, Variables};
use crate::ast::{Namespace, Type};
use crate::codegen::strength_reduce::value::Value;
use crate::codegen::Expression;
use bitvec::prelude::BitArray;
use num_bigint::BigInt;
use std::collections::{HashMap, HashSet};

#[test]
fn test_highest_bit() {
    assert_eq!(highest_set_bit(&[0, 0, 0]), 0);
    assert_eq!(highest_set_bit(&[0, 1, 0]), 8);
    assert_eq!(highest_set_bit(&[0, 0x80, 0]), 15);
    assert_eq!(highest_set_bit(&[0, 0, 0, 1, 0]), 24);
    assert_eq!(highest_set_bit(&[0x80, 0xff, 0xff]), 23);
    assert_eq!(
        highest_set_bit(
            &hex::decode("fcff030000000000000000000000000000000000000000000000000000000000")
                .unwrap()
        ),
        17
    );
}

#[test]
fn expresson_known_bits() {
    use crate::Target;
    use solang_parser::pt::Loc;

    let ns = Namespace::new(Target::default_substrate());
    let loc = Loc::Codegen;

    let mut vars: Variables = HashMap::new();

    // zero extend 1
    let expr = Expression::ZeroExt(
        loc,
        Type::Uint(128),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Uint(64),
            BigInt::from(16),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);

    let v = res.iter().next().unwrap();

    assert!(v.all_known());
    assert!(v.value[4]);

    // zero extend unknown value
    let expr = Expression::ZeroExt(
        loc,
        Type::Uint(128),
        Box::new(Expression::FunctionArg(loc, Type::Uint(64), 0)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);

    let v = res.iter().next().unwrap();

    assert!(!v.all_known());
    assert!(!v.known_bits[0..63].all());
    assert!(v.known_bits[64..128].all());
    assert!(!v.value.all());

    // sign extend unknown value
    let expr = Expression::SignExt(
        loc,
        Type::Int(128),
        Box::new(Expression::FunctionArg(loc, Type::Int(64), 0)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);

    let v = res.iter().next().unwrap();

    assert!(!v.known_bits.all());
    assert!(!v.value.all());

    // get the sign.

    let expr =
        Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(0x8000_0000_0000_0000u64));

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert_eq!(v.sign(), (true, true));

    // test: bitwise or
    // sign extend unknown value with known sign
    let expr = Expression::SignExt(
        loc,
        Type::Int(128),
        Box::new(Expression::BitwiseOr(
            loc,
            Type::Int(64),
            Box::new(Expression::FunctionArg(loc, Type::Int(64), 0)),
            Box::new(Expression::NumberLiteral(
                loc,
                Type::Int(64),
                BigInt::from(0x8000_0000_0000_0000u64),
            )),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0..62].all());
    assert!(v.known_bits[63..128].all());
    assert!(!v.value[0..62].all());
    assert!(v.value[63..128].all());

    // test: trunc
    let expr = Expression::Trunc(
        loc,
        Type::Int(32),
        Box::new(Expression::FunctionArg(loc, Type::Int(64), 0)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0..32].all());
    assert!(v.known_bits[32..256].all());
    assert!(!v.value.all());

    // test: bitwise and
    // lets put unknown in a variable amd
    let res = expression_values(&Expression::FunctionArg(loc, Type::Int(32), 0), &vars, &ns);

    vars.insert(0, res);

    let expr = Expression::BitwiseAnd(
        loc,
        Type::Int(32),
        Box::new(Expression::Variable(loc, Type::Int(32), 0)),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(0xffff),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0..16].all());
    assert!(v.known_bits[16..256].all());
    assert!(!v.value.all());

    // test: bitwise xor
    let vars = HashMap::new();

    let expr = Expression::BitwiseXor(
        loc,
        Type::Int(32),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(-0x10000),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(0xff0000),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits.all());
    assert!(!v.value[0..24].all());
    assert!(v.value[24..32].all());

    // test: add
    // first try some constants
    let expr = Expression::Add(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(123456),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits.all());

    let mut bs = (123456u32 + 7899900u32).to_le_bytes().to_vec();
    bs.resize(32, 0);

    assert_eq!(v.value.as_buffer().to_vec(), bs);

    // add: unknown plus constant
    let expr = Expression::Add(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::FunctionArg(loc, Type::Int(32), 0)),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits.all());

    // add: unknown plus constant
    let expr = Expression::Add(
        loc,
        Type::Uint(32),
        false,
        Box::new(Expression::ZeroExt(
            loc,
            Type::Uint(32),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Uint(32),
            BigInt::from(7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0..17].all());
    assert!(v.known_bits[17..32].all());
    let mut value = BigInt::from_signed_bytes_le(v.value.as_buffer());

    // mask off the unknown bits and compare
    value &= BigInt::from(!0x1ffff);

    assert_eq!(value, BigInt::from(7899900 & !0x1ffff));

    // test: substrate
    // first try some constants
    let expr = Expression::Subtract(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(123456),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(-7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits.all());

    let mut bs = (123456i32 - -7899900i32).to_le_bytes().to_vec();
    bs.resize(32, 0);

    assert_eq!(v.value.as_buffer().to_vec(), bs);

    // substract: unknown minus constant
    let expr = Expression::Subtract(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::SignExt(
            loc,
            Type::Uint(32),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Uint(32),
            BigInt::from(7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    // we can't know anything since the sign extend made L unknown
    assert!(!v.known_bits.all());

    let mut vars = HashMap::new();

    // substrate: 2 values and 2 values -> 4 values (with dedup)
    let mut val1 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(1)),
        &vars,
        &ns,
    );

    let val2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(2)),
        &vars,
        &ns,
    );

    let mut val3 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(3)),
        &vars,
        &ns,
    );

    let val4 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(4)),
        &vars,
        &ns,
    );

    val1.extend(val4);

    vars.insert(0, val1);

    val3.extend(val2);

    vars.insert(1, val3);
    // now we have: var 0 => 1, 4 and var 1 => 3, 2

    let expr = Expression::Subtract(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::Variable(loc, Type::Uint(32), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(32), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    // { 1, 4 } - { 3, 2 } => { -2, -1, 1, 2 }
    assert_eq!(res.len(), 4);

    let mut cmp_set = HashSet::new();

    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(-2)),
        &vars,
        &ns,
    ));
    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(-1)),
        &vars,
        &ns,
    ));
    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(1)),
        &vars,
        &ns,
    ));
    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(32), BigInt::from(2)),
        &vars,
        &ns,
    ));

    assert_eq!(cmp_set, res);

    // test: multiply
    // constants signed
    let expr = Expression::Multiply(
        loc,
        Type::Int(32),
        false,
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(123456),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Int(32),
            BigInt::from(-7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits.all());

    let mut bs = (123456i64 * -7899900i64).to_le_bytes().to_vec();
    bs.resize(32, 0xff);

    assert_eq!(v.value.as_buffer().to_vec(), bs);

    // constants unsigned
    let expr = Expression::Multiply(
        loc,
        Type::Uint(32),
        false,
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Uint(32),
            BigInt::from(123456),
        )),
        Box::new(Expression::NumberLiteral(
            loc,
            Type::Uint(32),
            BigInt::from(7899900),
        )),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits.all());

    let mut bs = (123456i64 * 7899900i64).to_le_bytes().to_vec();
    bs.resize(32, 0);

    assert_eq!(v.value.as_buffer().to_vec(), bs);

    // multiply a bunch of numbers, known or not
    let mut vars = HashMap::new();

    let mut var1 = expression_values(
        &Expression::ZeroExt(
            loc,
            Type::Uint(64),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        ),
        &vars,
        &ns,
    );

    var1.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(4)),
        &vars,
        &ns,
    ));

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(3)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(0x20_0000)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    let expr = Expression::Multiply(
        loc,
        Type::Uint(64),
        false,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    // { 3, 0x20_0000 } * { 4, 0xffffUKNOWN }
    assert_eq!(res.len(), 4);

    let mut cmp_set = HashSet::new();

    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(3 * 4)),
        &vars,
        &ns,
    ));
    cmp_set.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(0x20_0000 * 4)),
        &vars,
        &ns,
    ));

    let mut known_bits = BitArray::new([0u8; 32]);
    // 0xffff * 3 = 0x2fffd =17 bits
    known_bits[18..64].set_all(true);

    cmp_set.insert(Value {
        known_bits,
        value: BitArray::new([0u8; 32]),
        bits: 64,
    });

    let mut known_bits = BitArray::new([0u8; 32]);
    // 0xffff * 0x2000 = 0x1fffe00000 = 36 bits
    known_bits[37..64].set_all(true);

    cmp_set.insert(Value {
        known_bits,
        value: BitArray::new([0u8; 32]),
        bits: 64,
    });

    assert_eq!(cmp_set, res);

    /////////////
    // test: more
    /////////////
    let mut vars = HashMap::new();

    let mut var1 = expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(102)),
        &vars,
        &ns,
    );

    var1.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(512)),
        &vars,
        &ns,
    ));

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(3)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(0)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should always be true
    let expr = Expression::UnsignedMore(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits[0]);
    assert!(v.value[0]);

    /////////////
    // test: moreequal
    /////////////
    let mut vars = HashMap::new();

    let mut var1 = expression_values(
        &Expression::ZeroExt(
            loc,
            Type::Int(64),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        ),
        &vars,
        &ns,
    );

    var1.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(512)),
        &vars,
        &ns,
    ));

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(3)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(0)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should always be true
    let expr = Expression::UnsignedMore(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits[0]);
    assert!(v.value[0]);

    /////////////
    // test: less
    /////////////
    let mut vars = HashMap::new();

    let var1 = expression_values(
        &Expression::Subtract(
            loc,
            Type::Int(64),
            false,
            Box::new(Expression::ZeroExt(
                loc,
                Type::Int(64),
                Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
            )),
            Box::new(Expression::NumberLiteral(
                loc,
                Type::Int(64),
                BigInt::from(2),
            )),
        ),
        &vars,
        &ns,
    );

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(-1)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(-4)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should always be true
    let expr = Expression::UnsignedLess(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0]);
    assert!(!v.value[0]);

    /////////////
    // test: lessequal
    /////////////
    let mut vars = HashMap::new();

    let var1 = expression_values(
        &Expression::ZeroExt(
            loc,
            Type::Int(64),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        ),
        &vars,
        &ns,
    );

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(-2)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(0)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should always be true
    let expr = Expression::LessEqual(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 2);

    // can be both unknown or true
    let mut cmp_set = HashSet::new();

    // unknown
    cmp_set.insert(Value {
        known_bits: BitArray::new([0u8; 32]),
        value: BitArray::new([0u8; 32]),
        bits: 1,
    });

    let mut known_bits = BitArray::new([0u8; 32]);
    known_bits.set(0, true);

    let mut value = BitArray::new([0u8; 32]);
    value.set(0, true);

    cmp_set.insert(Value {
        known_bits,
        value,
        bits: 1,
    });

    assert_eq!(res, cmp_set);

    /////////////
    // test: equal
    /////////////

    let mut vars = HashMap::new();

    let var1 = expression_values(
        &Expression::ZeroExt(
            loc,
            Type::Int(64),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        ),
        &vars,
        &ns,
    );

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(0)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(-4)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should be unkown or false
    let expr = Expression::Equal(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 2);
    // can be both unknown, false
    let mut cmp_set = HashSet::new();

    // unknown
    cmp_set.insert(Value {
        known_bits: BitArray::new([0u8; 32]),
        value: BitArray::new([0u8; 32]),
        bits: 1,
    });

    let mut known_bits = BitArray::new([0u8; 32]);
    known_bits.set(0, true);

    let value = BitArray::new([0u8; 32]);

    cmp_set.insert(Value {
        known_bits,
        value,
        bits: 1,
    });

    assert_eq!(res, cmp_set);

    /////////////
    // test: notequal
    /////////////

    let mut vars = HashMap::new();

    let var1 = expression_values(
        &Expression::ZeroExt(
            loc,
            Type::Int(64),
            Box::new(Expression::FunctionArg(loc, Type::Uint(16), 0)),
        ),
        &vars,
        &ns,
    );

    vars.insert(0, var1);

    let mut var2 = expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(0x1000000)),
        &vars,
        &ns,
    );

    var2.extend(expression_values(
        &Expression::NumberLiteral(loc, Type::Int(64), BigInt::from(-4)),
        &vars,
        &ns,
    ));

    vars.insert(1, var2);

    // should be true
    let expr = Expression::NotEqual(
        loc,
        Box::new(Expression::Variable(loc, Type::Uint(64), 0)),
        Box::new(Expression::Variable(loc, Type::Uint(64), 1)),
    );

    let res = expression_values(&expr, &vars, &ns);

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits[0]);
    assert!(v.value[0]);
}
