// SPDX-License-Identifier: Apache-2.0

use super::value::Value;
use super::{bigint_to_bitarr, highest_set_bit, track, Bits, Variables};
use crate::codegen::Expression;
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{Namespace, Type};
use bitvec::prelude::*;
use itertools::Itertools;
use num_bigint::{BigInt, Sign};
use std::collections::HashSet;

pub(super) fn expression_values(
    expr: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    match expr {
        Expression::NumberLiteral { ty, value, .. } => number_literal_values(ty, value, ns),
        Expression::BoolLiteral { value, .. } => bool_literal_values(*value),
        Expression::ZeroExt { ty, expr, .. } => zero_ext_values(ty, expr, vars, ns),
        Expression::SignExt { ty, expr, .. } => sign_ext_values(ty, expr, vars, ns),
        Expression::Trunc { ty, expr, .. } => trunc_values(ty, expr, vars, ns),
        Expression::BitwiseOr { left, right, .. } => bitwise_or_values(left, right, vars, ns),
        Expression::BitwiseAnd { left, right, .. } => bitwise_and_values(left, right, vars, ns),
        Expression::BitwiseXor { left, right, .. } => bitwise_xor_values(left, right, vars, ns),
        Expression::Add {
            ty, left, right, ..
        } => add_values(ty, left, right, vars, ns),
        Expression::Subtract {
            ty, left, right, ..
        } => subtract_values(ty, left, right, vars, ns),
        Expression::Multiply {
            ty, left, right, ..
        } => multiply_values(ty, left, right, vars, ns),
        Expression::More {
            left,
            right,
            signed,
            ..
        } => more_values(left, right, *signed, vars, ns),
        Expression::MoreEqual {
            left,
            right,
            signed,
            ..
        } => more_equal_values(left, right, *signed, vars, ns),
        Expression::Less {
            left,
            right,
            signed,
            ..
        } => less_values(left, right, *signed, vars, ns),
        Expression::LessEqual {
            left,
            right,
            signed,
            ..
        } => less_equal_values(left, right, *signed, vars, ns),
        Expression::Equal {
            left: left_expr,
            right: right_expr,
            ..
        } => equal_values(left_expr, right_expr, vars, ns),
        Expression::NotEqual {
            left: left_expr,
            right: right_expr,
            ..
        } => not_equal_values(left_expr, right_expr, vars, ns),
        Expression::Not { expr, .. } => not_values(expr, vars, ns),
        Expression::BitwiseNot { expr, .. } => complement_values(expr, vars, ns),
        Expression::Variable { var_no, .. } => variable_values(*var_no, vars),
        Expression::InternalFunctionCfg { .. } => {
            // reference to a function; ignore
            HashSet::new()
        }
        Expression::Undefined { ty } => {
            // If the variable is undefined, we can return the default value to optimize operations
            if let Some(default_expr) = ty.default(ns) {
                return expression_values(&default_expr, vars, ns);
            }

            HashSet::new()
        }
        e => {
            let ty = e.ty();
            let mut set = HashSet::new();

            if track(&ty) {
                // the all bits known
                let mut known_bits = BitArray::new([!0u8; 32]);

                let bits = ty.bits(ns) as usize;

                // set the bits from the value to unknown
                known_bits[0..bits].fill(false);

                set.insert(Value {
                    known_bits,
                    value: BitArray::new([0u8; 32]),
                    bits,
                });
            }

            set
        }
    }
}

fn number_literal_values(ty: &Type, v: &BigInt, ns: &Namespace) -> HashSet<Value> {
    let mut set = HashSet::new();
    let bits = ty.bits(ns) as usize;

    set.insert(Value {
        known_bits: BitArray::new([!0u8; 32]),
        value: bigint_to_bitarr(v, bits),
        bits,
    });

    set
}

fn bool_literal_values(v: bool) -> HashSet<Value> {
    let mut set = HashSet::new();

    let mut value = BitArray::new([0u8; 32]);
    value.set(0, v);
    let mut known_bits = BitArray::new([0u8; 32]);
    known_bits.set(0, true);

    set.insert(Value {
        known_bits,
        value,
        bits: 1,
    });

    set
}

fn zero_ext_values(
    ty: &Type,
    expr: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let vals = expression_values(expr, vars, ns);
    let bits_after = ty.bits(ns) as usize;

    vals.into_iter()
        .map(|mut v| {
            let bits_before = v.bits;
            v.known_bits[bits_before..bits_after].fill(true);
            v.bits = bits_after;
            v
        })
        .collect()
}

fn sign_ext_values(
    ty: &Type,
    expr: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let vals = expression_values(expr, vars, ns);
    let bits_after = ty.bits(ns) as usize;

    vals.into_iter()
        .map(|mut v| {
            let bits_before = v.bits;
            // copy the sign known bit over
            let sign_known = v.known_bits[bits_before - 1];
            v.known_bits[bits_before..bits_after].fill(sign_known);

            // copy the sign bit over
            let sign = v.value[bits_before - 1];
            v.value[bits_before..bits_after].fill(sign);

            v.bits = bits_after;
            v
        })
        .collect()
}

fn trunc_values(ty: &Type, expr: &Expression, vars: &Variables, ns: &Namespace) -> HashSet<Value> {
    let vals = expression_values(expr, vars, ns);
    let bits_after = ty.bits(ns) as usize;

    vals.into_iter()
        .map(|mut v| {
            let bits_before = v.bits;
            v.known_bits[bits_after..bits_before].fill(true);
            v.value[bits_after..bits_before].fill(false);
            v.bits = bits_after;
            v
        })
        .collect()
}

fn bitwise_or_values(
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| Value {
            value: l.value | (r.value & r.known_bits),
            known_bits: l.known_bits | (r.value & r.known_bits),
            bits: l.bits,
        })
        .collect()
}

fn bitwise_and_values(
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    // bitwise and
    // value bits become 0 if right known_bit and !value
    // known_bits because more if known_bit & !value
    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| Value {
            value: l.value & (r.known_bits & !r.value),
            known_bits: l.known_bits | (r.known_bits & !r.value),
            bits: l.bits,
        })
        .collect()
}

fn bitwise_xor_values(
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    // bitwise and
    // value bits become 0 if right known_bit and !value
    // known_bits because more if known_bit & !value
    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut value = l.value ^ r.value;
            value[l.bits..].fill(false);
            Value {
                value,
                known_bits: l.known_bits & r.known_bits,
                bits: l.bits,
            }
        })
        .collect()
}

fn add_values(
    ty: &Type,
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut min_possible =
                (BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                    + BigInt::from_signed_bytes_le(&r.get_unsigned_min_value().into_inner()))
                .to_signed_bytes_le();
            let sign = if (min_possible.last().unwrap() & 0x80) != 0 {
                u8::MAX
            } else {
                u8::MIN
            };
            min_possible.resize(32, sign);

            let mut min_possible: Bits = BitArray::new(min_possible.try_into().unwrap());
            min_possible[ty.bits(ns) as usize..].fill(false);

            let mut max_possible =
                (BigInt::from_signed_bytes_le(&l.get_unsigned_max_value().into_inner())
                    + BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner()))
                .to_signed_bytes_le();
            let sign = if (max_possible.last().unwrap() & 0x80) != 0 {
                u8::MAX
            } else {
                u8::MIN
            };
            max_possible.resize(32, sign);

            let mut max_possible: Bits = BitArray::new(max_possible.try_into().unwrap());
            max_possible[ty.bits(ns) as usize..].fill(false);

            let known_bits = !(min_possible ^ max_possible) & l.known_bits & r.known_bits;

            if known_bits.all() {
                assert_eq!(min_possible, max_possible);
            }

            Value {
                value: min_possible,
                known_bits,
                bits: l.bits,
            }
        })
        .collect()
}

fn subtract_values(
    ty: &Type,
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut min_possible =
                (BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                    - BigInt::from_signed_bytes_le(&r.get_unsigned_min_value().into_inner()))
                .to_signed_bytes_le();
            let sign = if (min_possible.last().unwrap() & 0x80) != 0 {
                u8::MAX
            } else {
                u8::MIN
            };
            min_possible.resize(32, sign);

            let mut min_possible: Bits = BitArray::new(min_possible.try_into().unwrap());
            min_possible[ty.bits(ns) as usize..].fill(false);

            let mut max_possible =
                (BigInt::from_signed_bytes_le(&l.get_unsigned_max_value().into_inner())
                    - BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner()))
                .to_signed_bytes_le();
            let sign = if (max_possible.last().unwrap() & 0x80) != 0 {
                u8::MAX
            } else {
                u8::MIN
            };
            max_possible.resize(32, sign);

            let mut max_possible: Bits = BitArray::new(max_possible.try_into().unwrap());
            max_possible[ty.bits(ns) as usize..].fill(false);

            let known_bits = !(min_possible ^ max_possible) & l.known_bits & r.known_bits;

            Value {
                value: min_possible,
                known_bits,
                bits: l.bits,
            }
        })
        .collect()
}

fn multiply_values(
    ty: &Type,
    left: &Expression,
    right: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut known_bits = BitArray::new([0u8; 32]);

            if ty.is_signed_int(ns) {
                match (l.sign(), r.sign()) {
                    ((true, left_sign), (true, right_sign)) => {
                        let left = if left_sign {
                            l.get_signed_min_value()
                        } else {
                            l.get_signed_max_value()
                        };

                        let right = if right_sign {
                            r.get_signed_min_value()
                        } else {
                            r.get_signed_max_value()
                        };

                        let max_possible = BigInt::from_signed_bytes_le(&left.into_inner())
                            * BigInt::from_signed_bytes_le(&right.into_inner());

                        let (sign, bs) = max_possible.to_bytes_le();
                        let top_bit = highest_set_bit(&bs);

                        let mut max_possible = max_possible.to_signed_bytes_le();

                        max_possible.resize(32, if sign == Sign::Minus { u8::MAX } else { 0 });

                        if l.known_bits[0..l.bits].all() && r.known_bits[0..r.bits].all() {
                            // constants
                            known_bits.fill(true);
                        } else {
                            known_bits[top_bit + 1..l.bits].fill(true);
                        }

                        Value {
                            value: BitArray::new(max_possible.try_into().unwrap()),
                            known_bits,
                            bits: l.bits,
                        }
                    }
                    _ => {
                        // if we don't know either of the signs, we can't say anything about the result
                        Value {
                            value: BitArray::new([0u8; 32]),
                            known_bits,
                            bits: l.bits,
                        }
                    }
                }
            } else {
                let mut max_possible =
                    (BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_max_value().into_inner())
                        * BigInt::from_bytes_le(
                            Sign::Plus,
                            &r.get_unsigned_max_value().into_inner(),
                        ))
                    .to_signed_bytes_le();

                if l.known_bits[0..l.bits].all() && r.known_bits[0..r.bits].all() {
                    // constants
                    max_possible.resize(32, 0);

                    known_bits.fill(true);

                    Value {
                        value: BitArray::new(max_possible.try_into().unwrap()),
                        known_bits,
                        bits: l.bits,
                    }
                } else {
                    let top_bit = highest_set_bit(&max_possible);

                    // one above the top bit and higher will be known (i.e. all zeros)
                    if top_bit < l.bits {
                        debug_assert_eq!(l.bits, r.bits);

                        known_bits[top_bit + 1..l.bits].fill(true);
                    }

                    Value {
                        value: BitArray::new([0u8; 32]),
                        known_bits,
                        bits: l.bits,
                    }
                }
            }
        })
        .collect()
}

fn more_values(
    left: &Expression,
    right: &Expression,
    signed: bool,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            // is l more than r
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let is_true = if signed {
                BigInt::from_signed_bytes_le(&l.get_signed_max_value().into_inner())
                    > BigInt::from_signed_bytes_le(&r.get_signed_min_value().into_inner())
            } else {
                BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_max_value().into_inner())
                    > BigInt::from_bytes_le(Sign::Plus, &r.get_unsigned_min_value().into_inner())
            };

            if is_true {
                // we know that this comparison is always true
                known_bits.set(0, true);
                value.set(0, true);
            } else {
                // maybe the comparison is always false
                let is_false = if signed {
                    BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        <= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                } else {
                    BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_min_value().into_inner())
                        <= BigInt::from_bytes_le(
                            Sign::Plus,
                            &r.get_unsigned_max_value().into_inner(),
                        )
                };

                if is_false {
                    // we know that this comparison is always false
                    known_bits.set(0, true);
                }
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn more_equal_values(
    left: &Expression,
    right: &Expression,
    signed: bool,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            // is l more than or equal r
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let is_true = if signed {
                BigInt::from_signed_bytes_le(&l.get_signed_max_value().into_inner())
                    >= BigInt::from_signed_bytes_le(&r.get_signed_min_value().into_inner())
            } else {
                BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_max_value().into_inner())
                    >= BigInt::from_bytes_le(Sign::Plus, &r.get_unsigned_min_value().into_inner())
            };

            if is_true {
                // we know that this comparison is always true
                known_bits.set(0, true);
                value.set(0, true);
            } else {
                // maybe the comparison is always false
                let is_false = if signed {
                    BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        < BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                } else {
                    BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_min_value().into_inner())
                        < BigInt::from_bytes_le(
                            Sign::Plus,
                            &r.get_unsigned_max_value().into_inner(),
                        )
                };

                if is_false {
                    // we know that this comparison is always false
                    known_bits.set(0, true);
                }
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn less_values(
    left: &Expression,
    right: &Expression,
    signed: bool,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            // is l less than r
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let is_true = if signed {
                BigInt::from_signed_bytes_le(&l.get_signed_max_value().into_inner())
                    < BigInt::from_signed_bytes_le(&r.get_signed_min_value().into_inner())
            } else {
                BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_max_value().into_inner())
                    < BigInt::from_bytes_le(Sign::Plus, &r.get_unsigned_min_value().into_inner())
            };

            if is_true {
                // we know that this comparison is always true
                known_bits.set(0, true);
                value.set(0, true);
            } else {
                // maybe the comparison is always false
                let is_false = if signed {
                    BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        >= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                } else {
                    BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_min_value().into_inner())
                        >= BigInt::from_bytes_le(
                            Sign::Plus,
                            &r.get_unsigned_max_value().into_inner(),
                        )
                };

                if is_false {
                    // we know that this comparison is always false
                    known_bits.set(0, true);
                }
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn less_equal_values(
    left: &Expression,
    right: &Expression,
    signed: bool,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left, vars, ns);
    let right = expression_values(right, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            // is l less than r
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let is_true = if signed {
                BigInt::from_signed_bytes_le(&l.get_signed_max_value().into_inner())
                    <= BigInt::from_signed_bytes_le(&r.get_signed_min_value().into_inner())
            } else {
                BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_max_value().into_inner())
                    <= BigInt::from_bytes_le(Sign::Plus, &r.get_unsigned_min_value().into_inner())
            };

            if is_true {
                // we know that this comparison is always true
                known_bits.set(0, true);
                value.set(0, true);
            } else {
                // maybe the comparison is always false
                let is_false = if signed {
                    BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        > BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                } else {
                    BigInt::from_bytes_le(Sign::Plus, &l.get_unsigned_min_value().into_inner())
                        > BigInt::from_bytes_le(
                            Sign::Plus,
                            &r.get_unsigned_max_value().into_inner(),
                        )
                };

                if is_false {
                    // we know that this comparison is always false
                    known_bits.set(0, true);
                }
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn equal_values(
    left_expr: &Expression,
    right_expr: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left_expr, vars, ns);
    let right = expression_values(right_expr, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let could_be_equal = if left_expr.ty().is_signed_int(ns) {
                BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                    >= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                    && BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        <= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
            } else {
                BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                    >= BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner())
                    && BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                        <= BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner())
            };

            if !could_be_equal || l.all_known() && r.all_known() {
                known_bits.set(0, true);
                value.set(0, could_be_equal);
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn not_equal_values(
    left_expr: &Expression,
    right_expr: &Expression,
    vars: &Variables,
    ns: &Namespace,
) -> HashSet<Value> {
    let left = expression_values(left_expr, vars, ns);
    let right = expression_values(right_expr, vars, ns);

    left.iter()
        .cartesian_product(right.iter())
        .map(|(l, r)| {
            let mut known_bits = BitArray::new([0u8; 32]);
            let mut value = BitArray::new([0u8; 32]);

            let could_be_equal = if left_expr.ty().is_signed_int(ns) {
                BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                    >= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
                    && BigInt::from_signed_bytes_le(&l.get_signed_min_value().into_inner())
                        <= BigInt::from_signed_bytes_le(&r.get_signed_max_value().into_inner())
            } else {
                BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                    >= BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner())
                    && BigInt::from_signed_bytes_le(&l.get_unsigned_min_value().into_inner())
                        <= BigInt::from_signed_bytes_le(&r.get_unsigned_max_value().into_inner())
            };

            if !could_be_equal || l.all_known() && r.all_known() {
                known_bits.set(0, true);
                value.set(0, !could_be_equal);
            }

            Value {
                value,
                known_bits,
                bits: 1,
            }
        })
        .collect()
}

fn not_values(expr: &Expression, vars: &Variables, ns: &Namespace) -> HashSet<Value> {
    let vals = expression_values(expr, vars, ns);

    vals.into_iter()
        .map(|mut v| {
            if v.known_bits[0] {
                let bit = v.value[0];

                v.value.set(0, !bit);
            }
            v
        })
        .collect()
}

fn complement_values(expr: &Expression, vars: &Variables, ns: &Namespace) -> HashSet<Value> {
    let vals = expression_values(expr, vars, ns);

    vals.into_iter()
        .map(|mut v| {
            // just invert the known bits
            let cmpl = !v.value & v.known_bits;
            v.value &= v.known_bits;
            v.value |= cmpl;
            v
        })
        .collect()
}

fn variable_values(var_no: usize, vars: &Variables) -> HashSet<Value> {
    if let Some(v) = vars.get(&var_no) {
        v.clone()
    } else {
        HashSet::new()
    }
}
