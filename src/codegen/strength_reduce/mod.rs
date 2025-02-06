// SPDX-License-Identifier: Apache-2.0

mod expression_values;
mod reaching_values;
mod tests;
mod value;

use super::cfg::{ControlFlowGraph, Instr};
use crate::codegen::Expression;
use crate::sema::ast::{ExternalCallAccounts, Namespace, Type};
use bitvec::prelude::*;
use expression_values::expression_values;
use num_bigint::{BigInt, Sign};
use num_traits::{One, ToPrimitive};
use reaching_values::{reaching_values, transfer};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use value::{get_max_signed, get_max_unsigned, is_single_constant, Value};

/*
  Strength Reduce optimization pass - replace expensive arithmetic operations with cheaper ones

  Currently implemented:
  - Replace 256/128 bit multiply/divide/modulo with smaller width operations

*/

/*
  In order to know whether e.g. a 256 multiply can be replaced with a 64 bit one, we need to know
  the maximum value its arguments can have. For this, we first use reaching definitions to calculate
  the known bits of variables. Then, we walk the expressions and do the replacements.

  For example:


    contract test {
        function f() public {
            for (uint i = 0; i < 10; i++) {
                // this multiply can be done with a 64 bit instruction
                print("i:{}".format(i * 100));
            }
        }
    }


   Here we need to collect all the possible values i can have. Here the loop has clear bounds. However,


    contract test {
        function f(bool x) public {
            uint i = 0;

            for (;;) {
                 print("i:{}".format(i * 100));
                 i += 1;
                 if (x)
                    break;
            }
        }
    }


  Here we have no idea what the upper bound of i might be, as there is none. We iterate until we have
  MAX_VALUES of values, and then the value i becomes a set with the single value unknown. If the multiplication
  was "(i & 255) * 100" then we know that the upper bound of i is 255, and the multiply can be done with 64
  bit operations again.

  TODO/ideas to explore
  - In the first example above, the variable i can be replaced with a 64 bit. Check each assignment to i
    and check if the result fits into 64 bit
  - Conditions like "if (i < 100) { ... }" are not used to know the bounds of i
  - The pass does not work across function calls
  - Can we replace Expression::Power() with a cheaper one
  - Can we replace Expression::BitwiseAnd() with a cheaper one if either side fits into u64
*/

// Iterate over the cfg until we have 100 possible values, if we have more give up and assume unknown. This
// is to prevent infinite loops in our pass.
const MAX_VALUES: usize = 100;

/// some information when hovering over a variable.
pub fn strength_reduce(cfg: &mut ControlFlowGraph, ns: &mut Namespace) {
    // reaching definitions for integer calculations
    let mut block_vars = HashMap::new();
    let mut vars = HashMap::new();

    reaching_values(0, cfg, &mut vars, &mut block_vars, ns);

    // now we have all the reaching values for the top of each block
    // we can now step through each block and do any strength reduction where possible
    for (block_no, vars) in block_vars {
        block_reduce(block_no, cfg, vars, ns);
    }
}

/// Walk through all the expressions in a block, and find any expressions which can be
/// replaced with cheaper ones.
fn block_reduce(
    block_no: usize,
    cfg: &mut ControlFlowGraph,
    mut vars: Variables,
    ns: &mut Namespace,
) {
    for instr in &mut cfg.blocks[block_no].instr {
        match instr {
            Instr::Set { expr, .. } => {
                *expr = expression_reduce(expr, &vars, ns);
            }
            Instr::Call { args, .. } => {
                *args = args
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();
            }
            Instr::Return { value } => {
                *value = value
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();
            }
            Instr::Store { dest, data } => {
                *dest = expression_reduce(dest, &vars, ns);
                *data = expression_reduce(data, &vars, ns);
            }
            Instr::AssertFailure {
                encoded_args: Some(expr),
            } => {
                *expr = expression_reduce(expr, &vars, ns);
            }
            Instr::Print { expr } => {
                *expr = expression_reduce(expr, &vars, ns);
            }
            Instr::ClearStorage { storage, .. } => {
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::SetStorage { storage, value, .. } => {
                *value = expression_reduce(value, &vars, ns);
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::SetStorageBytes {
                storage,
                value,
                offset,
                ..
            } => {
                *value = expression_reduce(value, &vars, ns);
                *storage = expression_reduce(storage, &vars, ns);
                *offset = expression_reduce(offset, &vars, ns);
            }
            Instr::PushStorage { storage, value, .. } => {
                if let Some(value) = value {
                    *value = expression_reduce(value, &vars, ns);
                }
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::PopStorage { storage, .. } => {
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::PushMemory { value, .. } => {
                *value = Box::new(expression_reduce(value, &vars, ns));
            }
            Instr::Constructor {
                encoded_args,
                value,
                gas,
                salt,
                accounts,
                ..
            } => {
                *encoded_args = expression_reduce(encoded_args, &vars, ns);
                if let Some(value) = value {
                    *value = expression_reduce(value, &vars, ns);
                }
                if let Some(salt) = salt {
                    *salt = expression_reduce(salt, &vars, ns);
                }
                if let ExternalCallAccounts::Present(accounts) = accounts {
                    *accounts = expression_reduce(accounts, &vars, ns);
                }
                *gas = expression_reduce(gas, &vars, ns);
            }
            Instr::ExternalCall {
                address,
                payload,
                value,
                gas,
                ..
            } => {
                *value = expression_reduce(value, &vars, ns);
                if let Some(address) = address {
                    *address = expression_reduce(address, &vars, ns);
                }
                *payload = expression_reduce(payload, &vars, ns);
                *gas = expression_reduce(gas, &vars, ns);
            }
            Instr::ValueTransfer { address, value, .. } => {
                *address = expression_reduce(address, &vars, ns);
                *value = expression_reduce(value, &vars, ns);
            }
            Instr::EmitEvent { topics, data, .. } => {
                *topics = topics
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();
                *data = expression_reduce(data, &vars, ns);
            }
            Instr::WriteBuffer { offset, .. } => {
                *offset = expression_reduce(offset, &vars, ns);
            }
            _ => (),
        }

        transfer(instr, &mut vars, ns);
    }
}

/// Walk through an expression, and do the replacements for the expensive operations
fn expression_reduce(expr: &Expression, vars: &Variables, ns: &mut Namespace) -> Expression {
    let filter = |expr: &Expression, ns: &mut Namespace| -> Expression {
        match expr {
            Expression::Multiply {
                loc,
                ty,
                overflowing,
                left,
                right,
            } => {
                let bits = ty.bits(ns) as usize;
                if bits >= 128 {
                    let left_values = expression_values(left, vars, ns);
                    let right_values = expression_values(right, vars, ns);

                    match is_single_constant(&right_values) {
                        Some(right) if *overflowing => {
                            // is it a power of two
                            // replace with a shift
                            let mut shift = BigInt::one();
                            let mut cmp = BigInt::from(2);

                            for _ in 1..bits {
                                if cmp == right {
                                    ns.hover_overrides.insert(
                                        *loc,
                                        format!(
                                            "{} multiply optimized to shift left {}",
                                            ty.to_string(ns),
                                            shift
                                        ),
                                    );

                                    return Expression::ShiftLeft {
                                        loc: *loc,
                                        ty: ty.clone(),
                                        left: left.clone(),
                                        right: Box::new(Expression::NumberLiteral {
                                            loc: *loc,
                                            ty: ty.clone(),
                                            value: shift,
                                        }),
                                    };
                                }

                                cmp *= 2;
                                shift += 1;
                            }
                        }
                        _ => (), // SHL would disable overflow check
                    }

                    if ty.is_signed_int(ns) {
                        if let (Some(left_max), Some(right_max)) =
                            (get_max_signed(&left_values), get_max_signed(&right_values))
                        {
                            // We can safely replace this with a 64 bit multiply which can be encoded in a single wasm/bpf instruction
                            if (left_max * right_max).to_i64().is_some() {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} multiply optimized to int64 multiply",
                                        ty.to_string(ns),
                                    ),
                                );

                                return Expression::SignExt {
                                    loc: *loc,
                                    ty: ty.clone(),
                                    expr: Box::new(Expression::Multiply {
                                        loc: *loc,
                                        ty: Type::Int(64),
                                        overflowing: *overflowing,
                                        left: Box::new(
                                            left.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                        right: Box::new(
                                            right.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                    }),
                                };
                            }
                        }
                    } else {
                        let left_max = get_max_unsigned(&left_values);
                        let right_max = get_max_unsigned(&right_values);

                        // We can safely replace this with a 64 bit multiply which can be encoded in a single wasm/bpf instruction
                        if left_max * right_max <= BigInt::from(u64::MAX) {
                            ns.hover_overrides.insert(
                                *loc,
                                format!(
                                    "{} multiply optimized to uint64 multiply",
                                    ty.to_string(ns),
                                ),
                            );

                            return Expression::ZeroExt {
                                loc: *loc,
                                ty: ty.clone(),
                                expr: Box::new(Expression::Multiply {
                                    loc: *loc,
                                    ty: Type::Uint(64),
                                    overflowing: *overflowing,
                                    left: Box::new(left.as_ref().clone().cast(&Type::Uint(64), ns)),
                                    right: Box::new(
                                        right.as_ref().clone().cast(&Type::Uint(64), ns),
                                    ),
                                }),
                            };
                        }
                    }
                }

                expr.clone()
            }
            Expression::UnsignedDivide {
                loc,
                ty,
                left,
                right,
            }
            | Expression::SignedDivide {
                loc,
                ty,
                left,
                right,
            } => {
                let bits = ty.bits(ns) as usize;

                if bits >= 128 {
                    let left_values = expression_values(left, vars, ns);
                    let right_values = expression_values(right, vars, ns);

                    if let Some(right) = is_single_constant(&right_values) {
                        // is it a power of two
                        // replace with a shift
                        let mut shift = BigInt::one();
                        let mut cmp = BigInt::from(2);

                        for _ in 1..bits {
                            if cmp == right {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} divide optimized to shift right {}",
                                        ty.to_string(ns),
                                        shift
                                    ),
                                );

                                return Expression::ShiftRight {
                                    loc: *loc,
                                    ty: ty.clone(),
                                    left: left.clone(),
                                    right: Box::new(Expression::NumberLiteral {
                                        loc: *loc,
                                        ty: ty.clone(),
                                        value: shift,
                                    }),
                                    signed: ty.is_signed_int(ns),
                                };
                            }

                            cmp *= 2;
                            shift += 1;
                        }
                    }

                    if ty.is_signed_int(ns) {
                        if let (Some(left_max), Some(right_max)) =
                            (get_max_signed(&left_values), get_max_signed(&right_values))
                        {
                            if left_max.to_i64().is_some() && right_max.to_i64().is_some() {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} divide optimized to int64 divide",
                                        ty.to_string(ns),
                                    ),
                                );

                                return Expression::SignExt {
                                    loc: *loc,
                                    ty: ty.clone(),
                                    expr: Box::new(Expression::UnsignedDivide {
                                        loc: *loc,
                                        ty: Type::Int(64),
                                        left: Box::new(
                                            left.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                        right: Box::new(
                                            right.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                    }),
                                };
                            }
                        }
                    } else {
                        let left_max = get_max_unsigned(&left_values);
                        let right_max = get_max_unsigned(&right_values);

                        // If both values fit into u64, then the result must too
                        if left_max.to_u64().is_some() && right_max.to_u64().is_some() {
                            ns.hover_overrides.insert(
                                *loc,
                                format!("{} divide optimized to uint64 divide", ty.to_string(ns),),
                            );

                            return Expression::ZeroExt {
                                loc: *loc,
                                ty: ty.clone(),
                                expr: Box::new(Expression::UnsignedDivide {
                                    loc: *loc,
                                    ty: Type::Uint(64),
                                    left: Box::new(left.as_ref().clone().cast(&Type::Uint(64), ns)),
                                    right: Box::new(
                                        right.as_ref().clone().cast(&Type::Uint(64), ns),
                                    ),
                                }),
                            };
                        }
                    }
                }

                expr.clone()
            }
            Expression::SignedModulo {
                loc,
                ty,
                left,
                right,
            }
            | Expression::UnsignedModulo {
                loc,
                ty,
                left,
                right,
            } => {
                let bits = ty.bits(ns) as usize;

                if bits >= 128 {
                    let left_values = expression_values(left, vars, ns);
                    let right_values = expression_values(right, vars, ns);

                    if let Some(right) = is_single_constant(&right_values) {
                        // is it a power of two
                        // replace with an bitwise and
                        // e.g. (foo % 16) becomes (foo & 15)
                        let mut cmp = BigInt::one();

                        for _ in 1..bits {
                            if cmp == right {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} modulo optimized to bitwise and {}",
                                        ty.to_string(ns),
                                        cmp.clone() - 1
                                    ),
                                );

                                return Expression::BitwiseAnd {
                                    loc: *loc,
                                    ty: ty.clone(),
                                    left: left.clone(),
                                    right: Box::new(Expression::NumberLiteral {
                                        loc: *loc,
                                        ty: ty.clone(),
                                        value: cmp - 1,
                                    }),
                                };
                            }

                            cmp *= 2;
                        }
                    }

                    if ty.is_signed_int(ns) {
                        if let (Some(left_max), Some(right_max)) =
                            (get_max_signed(&left_values), get_max_signed(&right_values))
                        {
                            if left_max.to_i64().is_some() && right_max.to_i64().is_some() {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} modulo optimized to int64 modulo",
                                        ty.to_string(ns),
                                    ),
                                );

                                return Expression::SignExt {
                                    loc: *loc,
                                    ty: ty.clone(),
                                    expr: Box::new(Expression::SignedModulo {
                                        loc: *loc,
                                        ty: Type::Int(64),
                                        left: Box::new(
                                            left.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                        right: Box::new(
                                            right.as_ref().clone().cast(&Type::Int(64), ns),
                                        ),
                                    }),
                                };
                            }
                        }
                    } else {
                        let left_max = get_max_unsigned(&left_values);
                        let right_max = get_max_unsigned(&right_values);

                        // If both values fit into u64, then the result must too
                        if left_max.to_u64().is_some() && right_max.to_u64().is_some() {
                            ns.hover_overrides.insert(
                                *loc,
                                format!("{} modulo optimized to uint64 modulo", ty.to_string(ns)),
                            );

                            return Expression::ZeroExt {
                                loc: *loc,
                                ty: ty.clone(),
                                expr: Box::new(Expression::UnsignedModulo {
                                    loc: *loc,
                                    ty: Type::Uint(64),
                                    left: Box::new(left.as_ref().clone().cast(&Type::Uint(64), ns)),
                                    right: Box::new(
                                        right.as_ref().clone().cast(&Type::Uint(64), ns),
                                    ),
                                }),
                            };
                        }
                    }
                }

                expr.clone()
            }
            _ => expr.clone(),
        }
    };

    expr.copy_filter(ns, filter)
}

/// This optimization pass only tracks bools and integers variables.
/// Other types (e.g. bytes) is not relevant for strength reduce. Bools are only
/// tracked so we can following branching after integer compare.
fn track(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Uint(_) | Type::Int(_) | Type::Bool | Type::Value | Type::UserType(_)
    )
}

// A variable can
type Variables = HashMap<usize, HashSet<Value>>;
type Bits = BitArray<[u8; 32], Lsb0>;

fn highest_set_bit(bs: &[u8]) -> usize {
    for (i, b) in bs.iter().enumerate().rev() {
        if *b != 0 {
            return (i + 1) * 8 - bs[i].leading_zeros() as usize - 1;
        }
    }

    0
}

fn bigint_to_bitarr(v: &BigInt, bits: usize) -> BitArray<[u8; 32], Lsb0> {
    let mut bs = v.to_signed_bytes_le();

    bs.resize(
        32,
        if v.sign() == Sign::Minus {
            u8::MAX
        } else {
            u8::MIN
        },
    );

    let mut ba = BitArray::new(bs.try_into().unwrap());

    if bits < 256 {
        ba[bits..256].fill(false);
    }

    ba
}
