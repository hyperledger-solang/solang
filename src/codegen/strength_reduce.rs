use super::cfg::{ControlFlowGraph, Instr};
use crate::sema::ast::{Expression, Namespace, Type};
use crate::sema::expression::cast;
use bitvec::prelude::*;
use itertools::Itertools;
use num_bigint::{BigInt, Sign};
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fmt;

/*
  Strength Reduce optimization pass - replace expensive arithmetic operations with cheaper ones

  Currently implemented:
  - Replace 256/128 bit multiply/divide/modulo with smaller width operations

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
    for (block_no, vars) in block_vars.into_iter() {
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
            Instr::Store { dest, .. } => {
                *dest = expression_reduce(dest, &vars, ns);
            }
            Instr::AssertFailure { expr: Some(expr) } => {
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
                *value = expression_reduce(value, &vars, ns);
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::PopStorage { storage, .. } => {
                *storage = expression_reduce(storage, &vars, ns);
            }
            Instr::PushMemory { value, .. } => {
                *value = Box::new(expression_reduce(value, &vars, ns));
            }
            Instr::Constructor {
                args,
                value,
                gas,
                salt,
                ..
            } => {
                *args = args
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();
                if let Some(value) = value {
                    *value = expression_reduce(value, &vars, ns);
                }
                if let Some(salt) = salt {
                    *salt = expression_reduce(salt, &vars, ns);
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
            Instr::AbiDecode { data, .. } => {
                *data = expression_reduce(data, &vars, ns);
            }
            Instr::EmitEvent { topics, data, .. } => {
                *topics = topics
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();

                *data = data
                    .iter()
                    .map(|e| expression_reduce(e, &vars, ns))
                    .collect();
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
            Expression::Multiply(loc, ty, unchecked, left, right) => {
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
                                        "{} multiply optimized to shift left {}",
                                        ty.to_string(ns),
                                        shift
                                    ),
                                );

                                return Expression::ShiftLeft(
                                    *loc,
                                    ty.clone(),
                                    left.clone(),
                                    Box::new(Expression::NumberLiteral(*loc, ty.clone(), shift)),
                                );
                            }

                            cmp *= 2;
                            shift += 1;
                        }
                    }

                    if ty.is_signed_int() {
                        if let (Some(left_max), Some(right_max)) =
                            (set_max_signed(&left_values), set_max_signed(&right_values))
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

                                return Expression::SignExt(
                                    *loc,
                                    ty.clone(),
                                    Box::new(Expression::Multiply(
                                        *loc,
                                        Type::Int(64),
                                        *unchecked,
                                        Box::new(
                                            cast(
                                                loc,
                                                left.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                        Box::new(
                                            cast(
                                                loc,
                                                right.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                    )),
                                );
                            }
                        }
                    } else {
                        let left_max = set_max_unsigned(&left_values);
                        let right_max = set_max_unsigned(&right_values);

                        // We can safely replace this with a 64 bit multiply which can be encoded in a single wasm/bpf instruction
                        if left_max * right_max <= BigInt::from(u64::MAX) {
                            ns.hover_overrides.insert(
                                *loc,
                                format!(
                                    "{} multiply optimized to uint64 multiply",
                                    ty.to_string(ns),
                                ),
                            );

                            return Expression::ZeroExt(
                                *loc,
                                ty.clone(),
                                Box::new(Expression::Multiply(
                                    *loc,
                                    Type::Uint(64),
                                    *unchecked,
                                    Box::new(
                                        cast(
                                            loc,
                                            left.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                    Box::new(
                                        cast(
                                            loc,
                                            right.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                )),
                            );
                        }
                    }
                }

                expr.clone()
            }
            Expression::Divide(loc, ty, left, right) => {
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

                                return Expression::ShiftRight(
                                    *loc,
                                    ty.clone(),
                                    left.clone(),
                                    Box::new(Expression::NumberLiteral(*loc, ty.clone(), shift)),
                                    ty.is_signed_int(),
                                );
                            }

                            cmp *= 2;
                            shift += 1;
                        }
                    }

                    if ty.is_signed_int() {
                        if let (Some(left_max), Some(right_max)) =
                            (set_max_signed(&left_values), set_max_signed(&right_values))
                        {
                            if left_max.to_i64().is_some() && right_max.to_i64().is_some() {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} divide optimized to int64 divide",
                                        ty.to_string(ns),
                                    ),
                                );

                                return Expression::SignExt(
                                    *loc,
                                    ty.clone(),
                                    Box::new(Expression::Divide(
                                        *loc,
                                        Type::Int(64),
                                        Box::new(
                                            cast(
                                                loc,
                                                left.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                        Box::new(
                                            cast(
                                                loc,
                                                right.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                    )),
                                );
                            }
                        }
                    } else {
                        let left_max = set_max_unsigned(&left_values);
                        let right_max = set_max_unsigned(&right_values);

                        // If both values fit into u64, then the result must too
                        if left_max.to_u64().is_some() && right_max.to_u64().is_some() {
                            ns.hover_overrides.insert(
                                *loc,
                                format!("{} divide optimized to uint64 divide", ty.to_string(ns),),
                            );

                            return Expression::ZeroExt(
                                *loc,
                                ty.clone(),
                                Box::new(Expression::Divide(
                                    *loc,
                                    Type::Uint(64),
                                    Box::new(
                                        cast(
                                            loc,
                                            left.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                    Box::new(
                                        cast(
                                            loc,
                                            right.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                )),
                            );
                        }
                    }
                }

                expr.clone()
            }
            Expression::Modulo(loc, ty, left, right) => {
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

                                return Expression::BitwiseAnd(
                                    *loc,
                                    ty.clone(),
                                    left.clone(),
                                    Box::new(Expression::NumberLiteral(*loc, ty.clone(), cmp - 1)),
                                );
                            }

                            cmp *= 2;
                        }
                    }

                    if ty.is_signed_int() {
                        if let (Some(left_max), Some(right_max)) =
                            (set_max_signed(&left_values), set_max_signed(&right_values))
                        {
                            if left_max.to_i64().is_some() && right_max.to_i64().is_some() {
                                ns.hover_overrides.insert(
                                    *loc,
                                    format!(
                                        "{} modulo optimized to int64 modulo",
                                        ty.to_string(ns),
                                    ),
                                );

                                return Expression::SignExt(
                                    *loc,
                                    ty.clone(),
                                    Box::new(Expression::Modulo(
                                        *loc,
                                        Type::Int(64),
                                        Box::new(
                                            cast(
                                                loc,
                                                left.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                        Box::new(
                                            cast(
                                                loc,
                                                right.as_ref().clone(),
                                                &Type::Int(64),
                                                false,
                                                ns,
                                                &mut Vec::new(),
                                            )
                                            .unwrap(),
                                        ),
                                    )),
                                );
                            }
                        }
                    } else {
                        let left_max = set_max_unsigned(&left_values);
                        let right_max = set_max_unsigned(&right_values);

                        // If both values fit into u64, then the result must too
                        if left_max.to_u64().is_some() && right_max.to_u64().is_some() {
                            ns.hover_overrides.insert(
                                *loc,
                                format!("{} modulo optimized to uint64 modulo", ty.to_string(ns)),
                            );

                            return Expression::ZeroExt(
                                *loc,
                                ty.clone(),
                                Box::new(Expression::Modulo(
                                    *loc,
                                    Type::Uint(64),
                                    Box::new(
                                        cast(
                                            loc,
                                            left.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                    Box::new(
                                        cast(
                                            loc,
                                            right.as_ref().clone(),
                                            &Type::Uint(64),
                                            false,
                                            ns,
                                            &mut Vec::new(),
                                        )
                                        .unwrap(),
                                    ),
                                )),
                            );
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

/// Step through a block, and calculate the reaching values for all the variables
fn reaching_values(
    block_no: usize,
    cfg: &ControlFlowGraph,
    vars: &mut Variables,
    block_vars: &mut HashMap<usize, Variables>,
    ns: &Namespace,
) {
    // We should merge the incoming set of variables with the existing ones. If there
    // are no changes, then we cease traversing the cfg. The rules are:
    // - If there are more than MAX_VALUES entries in the result, make the set the unknown set
    // - If either the existing set or the incoming set contains unknown, make set the unknown set
    // - If there are no changes to the existing set, record this
    // - This is a very hot code path. This needs to be _FAST_ else compilation time quickly explodes
    if let Some(map) = block_vars.get_mut(&block_no) {
        let mut changes = false;

        for (var_no, set) in vars.iter() {
            if let Some(existing) = map.get_mut(var_no) {
                if existing.iter().next().map_or(false, |v| v.all_unknown()) {
                    // If we already think it is unknown, nothing can improve on that
                } else if let Some(v) = set.iter().find(|v| v.all_unknown()) {
                    // If we are merging an unknown value, set the entire value set to unknown
                    let mut set = HashSet::new();

                    set.insert(v.clone());

                    changes = true;
                    map.insert(*var_no, set);
                } else {
                    for v in set {
                        if !existing.contains(v) {
                            existing.insert(v.clone());
                            changes = true;
                        }
                    }

                    if existing.len() > MAX_VALUES {
                        let bits = existing.iter().next().unwrap().bits;

                        let mut set = HashSet::new();

                        set.insert(Value::unknown(bits));

                        changes = true;
                        map.insert(*var_no, set);
                    }
                }
            } else {
                // We have no existing set. Create one but folding unknown
                changes = true;

                if set.len() > MAX_VALUES || set.iter().any(|v| v.all_unknown()) {
                    let bits = set.iter().next().unwrap().bits;

                    let mut set = HashSet::new();

                    set.insert(Value::unknown(bits));

                    map.insert(*var_no, set);
                } else {
                    map.insert(*var_no, set.clone());
                }
            }
        }

        if !changes {
            // no need to do this again
            return;
        }
    } else {
        block_vars.insert(block_no, vars.clone());
    }

    for instr in &cfg.blocks[block_no].instr {
        transfer(instr, vars, ns);

        match instr {
            Instr::Branch { block } => {
                // must be last in the block
                reaching_values(*block, cfg, vars, block_vars, ns);
            }
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                // must be last in the block
                let v = expression_values(cond, vars, ns);

                if v.len() == 1 {
                    let v = v.iter().next().unwrap();

                    // if we know the value of the condition, follow that path
                    if v.known_bits[0] {
                        reaching_values(
                            if v.value[0] {
                                *true_block
                            } else {
                                *false_block
                            },
                            cfg,
                            vars,
                            block_vars,
                            ns,
                        );

                        continue;
                    }
                }

                // we don't know the value of the condition. Follow both paths
                let mut vars_copy = vars.clone();

                reaching_values(*true_block, cfg, &mut vars_copy, block_vars, ns);

                reaching_values(*false_block, cfg, vars, block_vars, ns);
            }
            Instr::AbiDecode {
                exception_block: Some(block),
                ..
            } => {
                let mut vars = vars.clone();

                reaching_values(*block, cfg, &mut vars, block_vars, ns);
            }

            _ => (),
        }
    }
}

/// For a given instruction, calculate the new reaching values
fn transfer(instr: &Instr, vars: &mut Variables, ns: &Namespace) {
    match instr {
        Instr::Set { res, expr, .. } => {
            let v = expression_values(expr, vars, ns);

            vars.insert(*res, v);
        }
        Instr::AbiDecode { res, tys, .. } => {
            for (i, var_no) in res.iter().enumerate() {
                let ty = &tys[i].ty;

                if track(ty) {
                    let mut set = HashSet::new();

                    let bits = ty.bits(ns) as usize;

                    set.insert(Value::unknown(bits));

                    vars.insert(*var_no, set);
                }
            }
        }
        Instr::Call {
            res, return_tys, ..
        } => {
            for (i, var_no) in res.iter().enumerate() {
                let mut set = HashSet::new();

                let ty = &return_tys[i];

                if track(ty) {
                    let bits = ty.bits(ns) as usize;

                    set.insert(Value::unknown(bits));

                    vars.insert(*var_no, set);
                }
            }
        }
        Instr::PopStorage { res, .. } => {
            let mut set = HashSet::new();

            set.insert(Value::unknown(8));

            vars.insert(*res, set);
        }
        Instr::PopMemory { res, ty, .. } => {
            if track(ty) {
                let mut set = HashSet::new();

                let bits = ty.bits(ns) as usize;

                set.insert(Value::unknown(bits));

                vars.insert(*res, set);
            }
        }
        _ => (),
    }
}

/// This optimization pass only tracks bools and integers variables.
/// Other types (e.g. bytes) is not relevant for strength reduce. Bools are only
/// tracked so we can following branching after integer compare.
fn track(ty: &Type) -> bool {
    matches!(ty, Type::Uint(_) | Type::Int(_) | Type::Bool | Type::Value)
}

#[derive(Eq, Hash, Debug, Clone, PartialEq)]
struct Value {
    // which bits are known
    known_bits: BitArray<Lsb0, [u8; 32]>,
    // value
    value: BitArray<Lsb0, [u8; 32]>,
    // type
    bits: usize,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.all_known() {
            write!(
                f,
                "{}",
                BigInt::from_signed_bytes_le(self.value.as_buffer())
            )
        } else if self.all_unknown() {
            write!(f, "unknown")
        } else {
            write!(
                f,
                "{} k:{}",
                BigInt::from_signed_bytes_le(self.value.as_buffer()),
                hex::encode(self.value[0..self.bits].as_slice())
            )
        }
    }
}

#[allow(unused)]
fn dump_set(name: &str, set: &HashSet<Value>) {
    println!(
        "{}:{}",
        name,
        set.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<String>>()
            .join(",")
    );
}

/// Is the value set just a single constant
fn is_single_constant(set: &HashSet<Value>) -> Option<BigInt> {
    if set.len() == 1 {
        let v = set.iter().next().unwrap();

        if v.all_known() {
            return Some(BigInt::from_signed_bytes_le(v.value[0..v.bits].as_slice()));
        }
    }

    None
}

/// Get the maximum unsigned value in a set
fn set_max_signed(set: &HashSet<Value>) -> Option<BigInt> {
    let mut m = BigInt::zero();

    for v in set {
        let (sign_known, sign) = v.sign();

        if !sign_known {
            return None;
        }

        let v = if sign {
            BigInt::from_signed_bytes_le(v.get_signed_min_value().as_buffer())
        } else {
            BigInt::from_signed_bytes_le(v.get_signed_max_value().as_buffer())
        };

        if v.abs() > m.abs() {
            m = v;
        }
    }

    Some(m)
}

/// Get the maximum signed value in a set
fn set_max_unsigned(set: &HashSet<Value>) -> BigInt {
    let mut m = BigInt::zero();

    for v in set {
        let v = BigInt::from_bytes_le(Sign::Plus, v.get_unsigned_max_value().as_buffer());

        m = std::cmp::max(v, m);
    }

    m
}

impl Value {
    /// Calculate the unsigned min value. Higher bits than the type are 0
    fn get_unsigned_min_value(&self) -> Bits {
        self.value & self.known_bits
    }

    /// Calculate the unsigned max value. Higher bits than the type are 0
    fn get_unsigned_max_value(&self) -> Bits {
        (BitArray::new([!0u8; 32]) & !self.known_bits) | self.value
    }

    /// Return whether the sign is known and what value it is
    fn sign(&self) -> (bool, bool) {
        let sign_bit = self.bits - 1;

        (self.known_bits[sign_bit], self.value[sign_bit])
    }

    /// Calculate the signed max value
    fn get_signed_max_value(&self) -> Bits {
        let negative = match self.sign() {
            (true, sign) => sign,
            (false, _) => false,
        };

        if !negative {
            // we know the value is positive; same as unsigned
            self.get_unsigned_max_value()
        } else {
            // the value might be negative. So, we want to know which bits are zero
            let mut v = self.get_unsigned_min_value();
            v[self.bits - 1..].set_all(true);
            v
        }
    }

    fn get_signed_min_value(&self) -> Bits {
        let negative = match self.sign() {
            (true, sign) => sign,
            (false, _) => true,
        };

        if !negative {
            // we know the value is positive; same as unsigned
            self.get_unsigned_min_value()
        } else {
            // the value might be negative. So, we want to know which bits are zero
            let mut v = self.get_unsigned_max_value();
            v[self.bits - 1..].set_all(true);
            v
        }
    }

    fn all_known(&self) -> bool {
        self.known_bits[0..self.bits].all()
    }

    fn all_unknown(&self) -> bool {
        self.known_bits[0..self.bits].not_any()
    }

    fn unknown(bits: usize) -> Value {
        Value {
            value: BitArray::new([0u8; 32]),
            known_bits: BitArray::new([0u8; 32]),
            bits,
        }
    }
}

// A variable can
type Variables = HashMap<usize, HashSet<Value>>;
type Bits = BitArray<Lsb0, [u8; 32]>;

fn expression_values(expr: &Expression, vars: &Variables, ns: &Namespace) -> HashSet<Value> {
    match expr {
        Expression::NumberLiteral(_, ty, v) => {
            let mut set = HashSet::new();
            let bits = ty.bits(ns) as usize;

            set.insert(Value {
                known_bits: BitArray::new([!0u8; 32]),
                value: bigint_to_bitarr(v, bits),
                bits,
            });

            set
        }
        Expression::BoolLiteral(_, v) => {
            let mut set = HashSet::new();

            let mut value = BitArray::new([0u8; 32]);
            value.set(0, *v);
            let mut known_bits = BitArray::new([0u8; 32]);
            known_bits.set(0, true);

            set.insert(Value {
                known_bits,
                value,
                bits: 1,
            });

            set
        }
        Expression::ZeroExt(_, ty, expr) => {
            let vals = expression_values(expr, vars, ns);
            let bits_after = ty.bits(ns) as usize;

            vals.into_iter()
                .map(|mut v| {
                    let bits_before = v.bits;
                    v.known_bits[bits_before..bits_after].set_all(true);
                    v.bits = bits_after;
                    v
                })
                .collect()
        }
        Expression::SignExt(_, ty, expr) => {
            let vals = expression_values(expr, vars, ns);
            let bits_after = ty.bits(ns) as usize;

            vals.into_iter()
                .map(|mut v| {
                    let bits_before = v.bits;
                    // copy the sign known bit over
                    let sign_known = v.known_bits[bits_before - 1];
                    v.known_bits[bits_before..bits_after].set_all(sign_known);

                    // copy the sign bit over
                    let sign = v.value[bits_before - 1];
                    v.value[bits_before..bits_after].set_all(sign);

                    v.bits = bits_after;
                    v
                })
                .collect()
        }
        Expression::Trunc(_, ty, expr) => {
            let vals = expression_values(expr, vars, ns);
            let bits_after = ty.bits(ns) as usize;

            vals.into_iter()
                .map(|mut v| {
                    let bits_before = v.bits;
                    v.known_bits[bits_after..bits_before].set_all(true);
                    v.value[bits_after..bits_before].set_all(false);
                    v.bits = bits_after;
                    v
                })
                .collect()
        }
        Expression::BitwiseOr(_, _, left, right) => {
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
        Expression::BitwiseAnd(_, _, left, right) => {
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
        Expression::BitwiseXor(_, _, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            // bitwise and
            // value bits become 0 if right known_bit and !value
            // known_bits because more if known_bit & !value
            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut value = l.value ^ r.value;
                    value[l.bits..].set_all(false);
                    Value {
                        value,
                        known_bits: l.known_bits & r.known_bits,
                        bits: l.bits,
                    }
                })
                .collect()
        }
        Expression::Add(_, ty, _, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut min_possible =
                        (BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                            + BigInt::from_signed_bytes_le(r.get_unsigned_min_value().as_buffer()))
                        .to_signed_bytes_le();
                    let sign = if (min_possible.last().unwrap() & 0x80) != 0 {
                        u8::MAX
                    } else {
                        u8::MIN
                    };
                    min_possible.resize(32, sign);

                    let mut min_possible: BitArray<Lsb0, [u8; 32]> =
                        BitArray::new(min_possible.try_into().unwrap());
                    min_possible[ty.bits(ns) as usize..].set_all(false);

                    let mut max_possible =
                        (BigInt::from_signed_bytes_le(l.get_unsigned_max_value().as_buffer())
                            + BigInt::from_signed_bytes_le(r.get_unsigned_max_value().as_buffer()))
                        .to_signed_bytes_le();
                    let sign = if (max_possible.last().unwrap() & 0x80) != 0 {
                        u8::MAX
                    } else {
                        u8::MIN
                    };
                    max_possible.resize(32, sign);

                    let mut max_possible: BitArray<Lsb0, [u8; 32]> =
                        BitArray::new(max_possible.try_into().unwrap());
                    max_possible[ty.bits(ns) as usize..].set_all(false);

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
        Expression::Subtract(_, ty, _, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut min_possible =
                        (BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                            - BigInt::from_signed_bytes_le(r.get_unsigned_min_value().as_buffer()))
                        .to_signed_bytes_le();
                    let sign = if (min_possible.last().unwrap() & 0x80) != 0 {
                        u8::MAX
                    } else {
                        u8::MIN
                    };
                    min_possible.resize(32, sign);

                    let mut min_possible: BitArray<Lsb0, [u8; 32]> =
                        BitArray::new(min_possible.try_into().unwrap());
                    min_possible[ty.bits(ns) as usize..].set_all(false);

                    let mut max_possible =
                        (BigInt::from_signed_bytes_le(l.get_unsigned_max_value().as_buffer())
                            - BigInt::from_signed_bytes_le(r.get_unsigned_max_value().as_buffer()))
                        .to_signed_bytes_le();
                    let sign = if (max_possible.last().unwrap() & 0x80) != 0 {
                        u8::MAX
                    } else {
                        u8::MIN
                    };
                    max_possible.resize(32, sign);

                    let mut max_possible: BitArray<Lsb0, [u8; 32]> =
                        BitArray::new(max_possible.try_into().unwrap());
                    max_possible[ty.bits(ns) as usize..].set_all(false);

                    let known_bits = !(min_possible ^ max_possible) & l.known_bits & r.known_bits;

                    Value {
                        value: min_possible,
                        known_bits,
                        bits: l.bits,
                    }
                })
                .collect()
        }
        Expression::Multiply(_, ty, _, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut known_bits = BitArray::new([0u8; 32]);

                    if ty.is_signed_int() {
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

                                let max_possible = BigInt::from_signed_bytes_le(left.as_buffer())
                                    * BigInt::from_signed_bytes_le(right.as_buffer());

                                let (sign, bs) = max_possible.to_bytes_le();
                                let top_bit = highest_set_bit(&bs);

                                let mut max_possible = max_possible.to_signed_bytes_le();

                                max_possible
                                    .resize(32, if sign == Sign::Minus { u8::MAX } else { 0 });

                                if l.known_bits[0..l.bits].all() && r.known_bits[0..r.bits].all() {
                                    // constants
                                    known_bits.set_all(true);
                                } else {
                                    known_bits[top_bit + 1..l.bits].set_all(true);
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
                            (BigInt::from_signed_bytes_le(l.get_unsigned_max_value().as_buffer())
                                * BigInt::from_signed_bytes_le(
                                    r.get_unsigned_max_value().as_buffer(),
                                ))
                            .to_signed_bytes_le();
                        let sign = if (max_possible.last().unwrap() & 0x80) != 0 {
                            u8::MAX
                        } else {
                            u8::MIN
                        };
                        max_possible.resize(32, sign);

                        if l.known_bits[0..l.bits].all() && r.known_bits[0..r.bits].all() {
                            // constants
                            known_bits.set_all(true);

                            Value {
                                value: BitArray::new(max_possible.try_into().unwrap()),
                                known_bits,
                                bits: l.bits,
                            }
                        } else {
                            let top_bit = highest_set_bit(&max_possible);

                            if top_bit < l.bits {
                                known_bits[top_bit + 1..l.bits].set_all(true);
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
        Expression::More(_, left, right) => {
            let ty = left.ty();

            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    // is l more than r
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let is_true = if ty.is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_max_value().as_buffer())
                            > BigInt::from_signed_bytes_le(r.get_signed_min_value().as_buffer())
                    } else {
                        BigInt::from_bytes_le(Sign::Plus, l.get_unsigned_max_value().as_buffer())
                            > BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_min_value().as_buffer(),
                            )
                    };

                    if is_true {
                        // we know that this comparison is always true
                        known_bits.set(0, true);
                        value.set(0, true);
                    } else {
                        // maybe the comparison is always false
                        let is_false = if ty.is_signed_int() {
                            BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                <= BigInt::from_signed_bytes_le(
                                    r.get_signed_max_value().as_buffer(),
                                )
                        } else {
                            BigInt::from_bytes_le(
                                Sign::Plus,
                                l.get_unsigned_min_value().as_buffer(),
                            ) <= BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_max_value().as_buffer(),
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
        Expression::MoreEqual(_, left, right) => {
            let ty = left.ty();

            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    // is l more than or equal r
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let is_true = if ty.is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_max_value().as_buffer())
                            >= BigInt::from_signed_bytes_le(r.get_signed_min_value().as_buffer())
                    } else {
                        BigInt::from_bytes_le(Sign::Plus, l.get_unsigned_max_value().as_buffer())
                            >= BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_min_value().as_buffer(),
                            )
                    };

                    if is_true {
                        // we know that this comparison is always true
                        known_bits.set(0, true);
                        value.set(0, true);
                    } else {
                        // maybe the comparison is always false
                        let is_false = if ty.is_signed_int() {
                            BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                < BigInt::from_signed_bytes_le(r.get_signed_max_value().as_buffer())
                        } else {
                            BigInt::from_bytes_le(
                                Sign::Plus,
                                l.get_unsigned_min_value().as_buffer(),
                            ) < BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_max_value().as_buffer(),
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
        Expression::Less(_, left, right) => {
            let ty = left.ty();

            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    // is l less than r
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let is_true = if ty.is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_max_value().as_buffer())
                            < BigInt::from_signed_bytes_le(r.get_signed_min_value().as_buffer())
                    } else {
                        BigInt::from_bytes_le(Sign::Plus, l.get_unsigned_max_value().as_buffer())
                            < BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_min_value().as_buffer(),
                            )
                    };

                    if is_true {
                        // we know that this comparison is always true
                        known_bits.set(0, true);
                        value.set(0, true);
                    } else {
                        // maybe the comparison is always false
                        let is_false = if ty.is_signed_int() {
                            BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                >= BigInt::from_signed_bytes_le(
                                    r.get_signed_max_value().as_buffer(),
                                )
                        } else {
                            BigInt::from_bytes_le(
                                Sign::Plus,
                                l.get_unsigned_min_value().as_buffer(),
                            ) >= BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_max_value().as_buffer(),
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
        Expression::LessEqual(_, left, right) => {
            let ty = left.ty();

            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    // is l less than r
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let is_true = if ty.is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_max_value().as_buffer())
                            <= BigInt::from_signed_bytes_le(r.get_signed_min_value().as_buffer())
                    } else {
                        BigInt::from_bytes_le(Sign::Plus, l.get_unsigned_max_value().as_buffer())
                            <= BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_min_value().as_buffer(),
                            )
                    };

                    if is_true {
                        // we know that this comparison is always true
                        known_bits.set(0, true);
                        value.set(0, true);
                    } else {
                        // maybe the comparison is always false
                        let is_false = if ty.is_signed_int() {
                            BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                > BigInt::from_signed_bytes_le(r.get_signed_max_value().as_buffer())
                        } else {
                            BigInt::from_bytes_le(
                                Sign::Plus,
                                l.get_unsigned_min_value().as_buffer(),
                            ) > BigInt::from_bytes_le(
                                Sign::Plus,
                                r.get_unsigned_max_value().as_buffer(),
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
        Expression::Equal(_, left_expr, right_expr) => {
            let left = expression_values(left_expr, vars, ns);
            let right = expression_values(right_expr, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let could_be_equal = if left_expr.ty().is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                            >= BigInt::from_signed_bytes_le(r.get_signed_max_value().as_buffer())
                            && BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                <= BigInt::from_signed_bytes_le(
                                    r.get_signed_max_value().as_buffer(),
                                )
                    } else {
                        BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                            >= BigInt::from_signed_bytes_le(r.get_unsigned_max_value().as_buffer())
                            && BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                                <= BigInt::from_signed_bytes_le(
                                    r.get_unsigned_max_value().as_buffer(),
                                )
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
        Expression::NotEqual(_, left_expr, right_expr) => {
            let left = expression_values(left_expr, vars, ns);
            let right = expression_values(right_expr, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    let could_be_equal = if left_expr.ty().is_signed_int() {
                        BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                            >= BigInt::from_signed_bytes_le(r.get_signed_max_value().as_buffer())
                            && BigInt::from_signed_bytes_le(l.get_signed_min_value().as_buffer())
                                <= BigInt::from_signed_bytes_le(
                                    r.get_signed_max_value().as_buffer(),
                                )
                    } else {
                        BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                            >= BigInt::from_signed_bytes_le(r.get_unsigned_max_value().as_buffer())
                            && BigInt::from_signed_bytes_le(l.get_unsigned_min_value().as_buffer())
                                <= BigInt::from_signed_bytes_le(
                                    r.get_unsigned_max_value().as_buffer(),
                                )
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
        Expression::Not(_, expr) => {
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
        Expression::Or(_, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    if l.known_bits[0] && r.known_bits[0] {
                        known_bits.set(0, true);
                        value.set(0, l.value[0] || r.value[0]);
                    } else if (l.known_bits[0] && l.value[0]) || (r.known_bits[0] && r.value[0]) {
                        known_bits.set(0, true);
                        value.set(0, true);
                    }

                    Value {
                        value,
                        known_bits,
                        bits: 1,
                    }
                })
                .collect()
        }
        Expression::And(_, left, right) => {
            let left = expression_values(left, vars, ns);
            let right = expression_values(right, vars, ns);

            left.iter()
                .cartesian_product(right.iter())
                .map(|(l, r)| {
                    let mut known_bits = BitArray::new([0u8; 32]);
                    let mut value = BitArray::new([0u8; 32]);

                    if l.known_bits[0] && r.known_bits[0] {
                        known_bits.set(0, true);
                        value.set(0, l.value[0] && r.value[0]);
                    } else if (l.known_bits[0] && !l.value[0]) || (r.known_bits[0] && !r.value[0]) {
                        known_bits.set(0, true);
                    }

                    Value {
                        value,
                        known_bits,
                        bits: 1,
                    }
                })
                .collect()
        }
        Expression::Complement(_, _, expr) => {
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
        Expression::Variable(_, _, var_no) => {
            if let Some(v) = vars.get(var_no) {
                v.clone()
            } else {
                HashSet::new()
            }
        }
        Expression::InternalFunctionCfg(_) => {
            // reference to a function; ignore
            HashSet::new()
        }
        Expression::Undefined(expr_type) => {
            // If the variable is undefined, we can return the default value to optimize operations
            if let Some(default_expr) = expr_type.default(ns) {
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
                known_bits[0..bits].set_all(false);

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

fn highest_set_bit(bs: &[u8]) -> usize {
    for (i, b) in bs.iter().enumerate().rev() {
        if *b != 0 {
            return (i + 1) * 8 - bs[i].leading_zeros() as usize - 1;
        }
    }

    0
}

fn bigint_to_bitarr(v: &BigInt, bits: usize) -> BitArray<Lsb0, [u8; 32]> {
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
        ba[bits..256].set_all(false);
    }

    ba
}

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
    let ns = Namespace::new(crate::Target::Substrate {
        address_length: 32,
        value_length: 16,
    });
    let loc = crate::parser::pt::Loc(0, 0, 0);

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
    let expr = Expression::More(
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
    let expr = Expression::More(
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
    let expr = Expression::Less(
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

    /////////////
    // test: or
    /////////////
    let vars = HashMap::new();

    // true or unknown => true
    let res = expression_values(
        &Expression::Or(
            loc,
            Box::new(Expression::BoolLiteral(loc, true)),
            Box::new(Expression::FunctionArg(loc, Type::Bool, 0)),
        ),
        &vars,
        &ns,
    );

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(v.known_bits[0]);
    assert!(v.value[0]);

    // false or unknown => unknown
    let res = expression_values(
        &Expression::Or(
            loc,
            Box::new(Expression::BoolLiteral(loc, false)),
            Box::new(Expression::FunctionArg(loc, Type::Bool, 0)),
        ),
        &vars,
        &ns,
    );

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();

    assert!(!v.known_bits[0]);

    /////////////
    // test: and
    /////////////
    let vars = HashMap::new();

    // true and unknown => unknown
    let res = expression_values(
        &Expression::And(
            loc,
            Box::new(Expression::BoolLiteral(loc, true)),
            Box::new(Expression::FunctionArg(loc, Type::Bool, 0)),
        ),
        &vars,
        &ns,
    );

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();
    assert!(!v.known_bits[0]);

    // false and unknown => false
    let res = expression_values(
        &Expression::And(
            loc,
            Box::new(Expression::BoolLiteral(loc, false)),
            Box::new(Expression::FunctionArg(loc, Type::Bool, 0)),
        ),
        &vars,
        &ns,
    );

    assert_eq!(res.len(), 1);
    let v = res.iter().next().unwrap();
    assert!(v.known_bits[0]);
    assert!(!v.value[0]);
}
