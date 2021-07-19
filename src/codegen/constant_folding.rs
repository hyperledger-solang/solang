use super::cfg::{ControlFlowGraph, Instr};
use super::reaching_definitions;
use crate::parser::pt::Loc;
use crate::sema::ast::{Builtin, Diagnostic, Expression, Namespace, StringLocation, Type};
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use ripemd160::Ripemd160;
use sha2::{Digest, Sha256};
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub};
use tiny_keccak::{Hasher, Keccak};

/// Constant folding pass on the given cfg. During constant folding, we may find issues
/// like divide by zero, so this function returns a list of diagnostics which should
/// be added to the namespace.
pub fn constant_folding(cfg: &mut ControlFlowGraph, ns: &mut Namespace) {
    // for each block, instruction
    for block_no in 0..cfg.blocks.len() {
        let mut vars = cfg.blocks[block_no].defs.clone();

        for instr_no in 0..cfg.blocks[block_no].instr.len() {
            let cur = reaching_definitions::Def { block_no, instr_no };

            match &cfg.blocks[block_no].instr[instr_no] {
                Instr::Set { loc, res, expr, .. } => {
                    let (expr, expr_constant) = expression(expr, Some(&vars), &cur, cfg, ns);

                    if expr_constant {
                        ns.var_constants.insert(*loc, expr.clone());
                    }

                    cfg.blocks[block_no].instr[instr_no] = Instr::Set {
                        loc: *loc,
                        res: *res,
                        expr,
                    };
                }
                Instr::Call {
                    res,
                    call,
                    args,
                    return_tys,
                } => {
                    let args = args
                        .iter()
                        .map(|e| expression(e, Some(&vars), &cur, cfg, ns).0)
                        .collect();

                    cfg.blocks[block_no].instr[instr_no] = Instr::Call {
                        res: res.clone(),
                        call: call.clone(),
                        args,
                        return_tys: return_tys.clone(),
                    };
                }
                Instr::Return { value } => {
                    let value = value
                        .iter()
                        .map(|e| expression(e, Some(&vars), &cur, cfg, ns).0)
                        .collect();

                    cfg.blocks[block_no].instr[instr_no] = Instr::Return { value };
                }
                Instr::BranchCond {
                    cond,
                    true_block,
                    false_block,
                } => {
                    let (cond, _) = expression(cond, Some(&vars), &cur, cfg, ns);

                    if let Expression::BoolLiteral(_, cond) = cond {
                        cfg.blocks[block_no].instr[instr_no] = Instr::Branch {
                            block: if cond { *true_block } else { *false_block },
                        };
                    } else {
                        cfg.blocks[block_no].instr[instr_no] = Instr::BranchCond {
                            cond,
                            true_block: *true_block,
                            false_block: *false_block,
                        };
                    }
                }
                Instr::Store { dest, pos } => {
                    let (dest, _) = expression(dest, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::Store { dest, pos: *pos };
                }
                Instr::AssertFailure { expr: Some(expr) } => {
                    let (expr, _) = expression(expr, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] =
                        Instr::AssertFailure { expr: Some(expr) };
                }
                Instr::Print { expr } => {
                    let (expr, _) = expression(expr, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::Print { expr };
                }
                Instr::ClearStorage { ty, storage } => {
                    let (storage, _) = expression(storage, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::ClearStorage {
                        ty: ty.clone(),
                        storage,
                    };
                }
                Instr::SetStorage { ty, storage, value } => {
                    let (storage, _) = expression(storage, Some(&vars), &cur, cfg, ns);
                    let (value, _) = expression(value, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::SetStorage {
                        ty: ty.clone(),
                        storage,
                        value,
                    };
                }
                Instr::SetStorageBytes {
                    storage,
                    value,
                    offset,
                } => {
                    let (storage, _) = expression(storage, Some(&vars), &cur, cfg, ns);
                    let (value, _) = expression(value, Some(&vars), &cur, cfg, ns);
                    let (offset, _) = expression(offset, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::SetStorageBytes {
                        storage,
                        value,
                        offset,
                    };
                }
                Instr::PushStorage {
                    res,
                    storage,
                    value,
                } => {
                    let (storage, _) = expression(storage, Some(&vars), &cur, cfg, ns);
                    let (value, _) = expression(value, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::PushStorage {
                        res: *res,
                        storage,
                        value,
                    };
                }
                Instr::PopStorage { res, ty, storage } => {
                    let (storage, _) = expression(storage, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::PopStorage {
                        res: *res,
                        ty: ty.clone(),
                        storage,
                    };
                }
                Instr::PushMemory {
                    res,
                    ty,
                    array,
                    value,
                } => {
                    let (value, _) = expression(value, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::PushMemory {
                        res: *res,
                        ty: ty.clone(),
                        array: *array,
                        value: Box::new(value),
                    };
                }
                Instr::Constructor {
                    success,
                    res,
                    contract_no,
                    constructor_no,
                    args,
                    value,
                    gas,
                    salt,
                    space,
                } => {
                    let args = args
                        .iter()
                        .map(|e| expression(e, Some(&vars), &cur, cfg, ns).0)
                        .collect();
                    let value = value
                        .as_ref()
                        .map(|expr| expression(expr, Some(&vars), &cur, cfg, ns).0);
                    let gas = expression(gas, Some(&vars), &cur, cfg, ns).0;
                    let salt = salt
                        .as_ref()
                        .map(|expr| expression(expr, Some(&vars), &cur, cfg, ns).0);
                    let space = space
                        .as_ref()
                        .map(|expr| expression(expr, Some(&vars), &cur, cfg, ns).0);

                    cfg.blocks[block_no].instr[instr_no] = Instr::Constructor {
                        success: *success,
                        res: *res,
                        contract_no: *contract_no,
                        constructor_no: *constructor_no,
                        args,
                        value,
                        gas,
                        salt,
                        space,
                    };
                }
                Instr::ExternalCall {
                    success,
                    address,
                    payload,
                    value,
                    gas,
                    callty,
                } => {
                    let value = expression(value, Some(&vars), &cur, cfg, ns).0;
                    let gas = expression(gas, Some(&vars), &cur, cfg, ns).0;
                    let payload = expression(payload, Some(&vars), &cur, cfg, ns).0;
                    let address = address
                        .as_ref()
                        .map(|expr| expression(expr, Some(&vars), &cur, cfg, ns).0);

                    cfg.blocks[block_no].instr[instr_no] = Instr::ExternalCall {
                        success: *success,
                        address,
                        payload,
                        value,
                        gas,
                        callty: callty.clone(),
                    };
                }
                Instr::AbiDecode {
                    res,
                    selector,
                    exception_block,
                    tys,
                    data,
                } => {
                    let (data, _) = expression(data, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::AbiDecode {
                        res: res.clone(),
                        selector: *selector,
                        exception_block: *exception_block,
                        tys: tys.clone(),
                        data,
                    }
                }
                Instr::SelfDestruct { recipient } => {
                    let (recipient, _) = expression(recipient, Some(&vars), &cur, cfg, ns);

                    cfg.blocks[block_no].instr[instr_no] = Instr::SelfDestruct { recipient };
                }
                Instr::EmitEvent {
                    event_no,
                    data,
                    data_tys,
                    topics,
                    topic_tys,
                } => {
                    let data = data
                        .iter()
                        .map(|e| expression(e, Some(&vars), &cur, cfg, ns).0)
                        .collect();

                    let topics = topics
                        .iter()
                        .map(|e| expression(e, Some(&vars), &cur, cfg, ns).0)
                        .collect();

                    cfg.blocks[block_no].instr[instr_no] = Instr::EmitEvent {
                        event_no: *event_no,
                        data,
                        data_tys: data_tys.clone(),
                        topics,
                        topic_tys: topic_tys.clone(),
                    }
                }
                _ => (),
            }

            reaching_definitions::apply_transfers(
                &cfg.blocks[block_no].transfers[instr_no],
                &mut vars,
            );
        }
    }
}

/// Recursively walk the expression and fold any constant expressions or variables. This function returns the
/// constant folded expression, and a boolean which is true if the value is "pure", the value does not depend
/// on context. This is used for constant folding, so that e.g. an external function call is not constant
/// folded (and moved/copied as a result).
fn expression(
    expr: &Expression,
    vars: Option<&reaching_definitions::VarDefs>,
    pos: &reaching_definitions::Def,
    cfg: &ControlFlowGraph,
    ns: &mut Namespace,
) -> (Expression, bool) {
    match expr {
        Expression::Add(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.add(right))
            } else {
                (
                    Expression::Add(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::Subtract(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.sub(right))
            } else {
                (
                    Expression::Subtract(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::Multiply(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.mul(right))
            } else {
                (
                    Expression::Multiply(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::BitwiseAnd(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.bitand(right))
            } else {
                (
                    Expression::BitwiseAnd(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::BitwiseOr(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.bitor(right))
            } else {
                (
                    Expression::BitwiseOr(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::BitwiseXor(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                bigint_to_expression(loc, ty, left.bitxor(right))
            } else {
                (
                    Expression::BitwiseXor(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                    left.1 && right.1,
                )
            }
        }
        Expression::ShiftLeft(loc, ty, left_expr, right_expr) => {
            let left = expression(left_expr, vars, pos, cfg, ns);
            let right = expression(right_expr, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                if right.sign() == Sign::Minus || right >= &BigInt::from(left_expr.ty().bits(ns)) {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("left shift by {} is not possible", right),
                    ));
                } else {
                    let right: u64 = right.to_u64().unwrap();

                    return bigint_to_expression(loc, ty, left.shl(&right));
                }
            }
            (
                Expression::ShiftLeft(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                left.1 && right.1,
            )
        }
        Expression::ShiftRight(loc, ty, left_expr, right_expr, signed) => {
            let left = expression(left_expr, vars, pos, cfg, ns);
            let right = expression(right_expr, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                if right.sign() == Sign::Minus || right >= &BigInt::from(left_expr.ty().bits(ns)) {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("right shift by {} is not possible", right),
                    ));
                } else {
                    let right: u64 = right.to_u64().unwrap();

                    return bigint_to_expression(loc, ty, left.shr(&right));
                }
            }

            (
                Expression::ShiftRight(
                    *loc,
                    ty.clone(),
                    Box::new(left.0),
                    Box::new(right.0),
                    *signed,
                ),
                left.1 && right.1,
            )
        }
        Expression::Power(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left.0, &right.0)
            {
                if right.sign() == Sign::Minus || right >= &BigInt::from(u32::MAX) {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("power {} not possible", right),
                    ));
                } else {
                    let right: u32 = right.to_u32().unwrap();

                    return bigint_to_expression(loc, ty, left.pow(right));
                }
            }

            (
                Expression::Power(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                left.1 && right.1,
            )
        }
        Expression::Divide(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let Expression::NumberLiteral(_, _, right) = &right.0 {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, String::from("divide by zero")));
                } else if let Expression::NumberLiteral(_, _, left) = &left.0 {
                    return bigint_to_expression(loc, ty, left.div(right));
                }
            }

            (
                Expression::Divide(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                left.1 && right.1,
            )
        }
        Expression::Modulo(loc, ty, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            if let Expression::NumberLiteral(_, _, right) = &right.0 {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, String::from("divide by zero")));
                } else if let Expression::NumberLiteral(_, _, left) = &left.0 {
                    return bigint_to_expression(loc, ty, left.rem(right));
                }
            }

            (
                Expression::Modulo(*loc, ty.clone(), Box::new(left.0), Box::new(right.0)),
                left.1 && right.1,
            )
        }
        Expression::ZeroExt(loc, ty, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);
            if let Expression::NumberLiteral(_, _, n) = expr.0 {
                (Expression::NumberLiteral(*loc, ty.clone(), n), true)
            } else {
                (
                    Expression::ZeroExt(*loc, ty.clone(), Box::new(expr.0)),
                    expr.1,
                )
            }
        }
        Expression::SignExt(loc, ty, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);
            if let Expression::NumberLiteral(_, _, n) = expr.0 {
                (Expression::NumberLiteral(*loc, ty.clone(), n), true)
            } else {
                (
                    Expression::SignExt(*loc, ty.clone(), Box::new(expr.0)),
                    expr.1,
                )
            }
        }
        Expression::Trunc(loc, ty, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);
            if let Expression::NumberLiteral(_, _, n) = expr.0 {
                bigint_to_expression(loc, ty, n)
            } else {
                (
                    Expression::Trunc(*loc, ty.clone(), Box::new(expr.0)),
                    expr.1,
                )
            }
        }
        Expression::Complement(loc, ty, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);
            if let Expression::NumberLiteral(_, _, n) = expr.0 {
                bigint_to_expression(loc, ty, !n)
            } else {
                (
                    Expression::Complement(*loc, ty.clone(), Box::new(expr.0)),
                    expr.1,
                )
            }
        }
        Expression::UnaryMinus(loc, ty, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);
            if let Expression::NumberLiteral(_, _, n) = expr.0 {
                bigint_to_expression(loc, ty, -n)
            } else {
                (
                    Expression::UnaryMinus(*loc, ty.clone(), Box::new(expr.0)),
                    expr.1,
                )
            }
        }
        Expression::Variable(loc, ty, var) => {
            if !matches!(ty, Type::Ref(_) | Type::StorageRef(_, _)) {
                if let Some(vars) = vars {
                    if let Some(defs) = vars.get(var) {
                        // There must be at least one definition, and all should evaluate to the same value
                        let mut v = None;

                        for def in defs.keys() {
                            if let Some(expr) = get_definition(def, cfg) {
                                let expr = expression(expr, None, pos, cfg, ns);

                                if expr.1 {
                                    if let Some(last) = &v {
                                        if !constants_equal(last, &expr.0) {
                                            v = None;
                                            break;
                                        }
                                    }

                                    v = Some(expr.0);
                                } else {
                                    v = None;
                                    break;
                                }
                            } else {
                                v = None;
                                break;
                            }
                        }

                        if let Some(expr) = v {
                            if *loc != Loc(0, 0, 0) {
                                ns.var_constants.insert(*loc, expr.clone());
                            }
                            return (expr, true);
                        }
                    }
                }
            }

            (expr.clone(), false)
        }
        Expression::Builtin(loc, tys, Builtin::Keccak256, args) => {
            let arg = expression(&args[0], vars, pos, cfg, ns);

            if let Expression::AllocDynamicArray(_, _, _, Some(bs)) = arg.0 {
                let mut hasher = Keccak::v256();
                hasher.update(&bs);
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);

                (
                    Expression::BytesLiteral(*loc, tys[0].clone(), hash.to_vec()),
                    true,
                )
            } else {
                (
                    Expression::Builtin(*loc, tys.clone(), Builtin::Keccak256, vec![arg.0]),
                    false,
                )
            }
        }
        Expression::Builtin(loc, tys, Builtin::Ripemd160, args) => {
            let arg = expression(&args[0], vars, pos, cfg, ns);

            if let Expression::AllocDynamicArray(_, _, _, Some(bs)) = arg.0 {
                let mut hasher = Ripemd160::new();
                hasher.update(&bs);
                let result = hasher.finalize();

                (
                    Expression::BytesLiteral(*loc, tys[0].clone(), result[..].to_vec()),
                    true,
                )
            } else {
                (
                    Expression::Builtin(*loc, tys.clone(), Builtin::Ripemd160, vec![arg.0]),
                    false,
                )
            }
        }
        Expression::Builtin(loc, tys, Builtin::Blake2_256, args) => {
            let arg = expression(&args[0], vars, pos, cfg, ns);

            if let Expression::AllocDynamicArray(_, _, _, Some(bs)) = arg.0 {
                let hash = blake2_rfc::blake2b::blake2b(32, &[], &bs);

                (
                    Expression::BytesLiteral(*loc, tys[0].clone(), hash.as_bytes().to_vec()),
                    true,
                )
            } else {
                (
                    Expression::Builtin(*loc, tys.clone(), Builtin::Blake2_256, vec![arg.0]),
                    false,
                )
            }
        }
        Expression::Builtin(loc, tys, Builtin::Blake2_128, args) => {
            let arg = expression(&args[0], vars, pos, cfg, ns);

            if let Expression::AllocDynamicArray(_, _, _, Some(bs)) = arg.0 {
                let hash = blake2_rfc::blake2b::blake2b(16, &[], &bs);

                (
                    Expression::BytesLiteral(*loc, tys[0].clone(), hash.as_bytes().to_vec()),
                    true,
                )
            } else {
                (
                    Expression::Builtin(*loc, tys.clone(), Builtin::Blake2_128, vec![arg.0]),
                    false,
                )
            }
        }
        Expression::Builtin(loc, tys, Builtin::Sha256, args) => {
            let arg = expression(&args[0], vars, pos, cfg, ns);

            if let Expression::AllocDynamicArray(_, _, _, Some(bs)) = arg.0 {
                let mut hasher = Sha256::new();

                // write input message
                hasher.update(&bs);

                // read hash digest and consume hasher
                let result = hasher.finalize();

                (
                    Expression::BytesLiteral(*loc, tys[0].clone(), result[..].to_vec()),
                    true,
                )
            } else {
                (
                    Expression::Builtin(*loc, tys.clone(), Builtin::Sha256, vec![arg.0]),
                    false,
                )
            }
        }
        Expression::Keccak256(loc, ty, args) => {
            let mut all_constant = true;
            let mut hasher = Keccak::v256();

            let args = args
                .iter()
                .map(|expr| {
                    let (expr, _) = expression(expr, vars, pos, cfg, ns);

                    if all_constant {
                        match &expr {
                            Expression::AllocDynamicArray(_, _, _, Some(bs))
                            | Expression::BytesLiteral(_, _, bs) => {
                                hasher.update(&bs);
                            }
                            Expression::NumberLiteral(_, ty, n) => {
                                let (sign, mut bs) = n.to_bytes_le();

                                match ty {
                                    Type::Uint(bits) => bs.resize(*bits as usize / 8, 0),
                                    Type::Int(bits) => {
                                        let v = if sign == Sign::Minus { 0xffu8 } else { 0 };

                                        bs.resize(*bits as usize / 8, v);
                                    }
                                    Type::Bytes(n) => {
                                        while (*n as usize) < bs.len() {
                                            bs.insert(0, 0);
                                        }
                                    }
                                    _ => unreachable!(),
                                }

                                hasher.update(&bs);
                            }
                            _ => {
                                all_constant = false;
                            }
                        }
                    }

                    expr
                })
                .collect();

            if all_constant {
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                let mut hash = hash.to_vec();
                hash.reverse();

                (Expression::BytesLiteral(*loc, ty.clone(), hash), true)
            } else {
                (Expression::Keccak256(*loc, ty.clone(), args), false)
            }
        }
        // The rest is simply for recursing; no constant expansion should be done
        Expression::StructLiteral(loc, ty, args) => {
            let args = args
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();

            (Expression::StructLiteral(*loc, ty.clone(), args), false)
        }
        Expression::ArrayLiteral(loc, ty, lengths, args) => {
            let args = args
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();

            (
                Expression::ArrayLiteral(*loc, ty.clone(), lengths.clone(), args),
                false,
            )
        }
        Expression::ConstArrayLiteral(loc, ty, lengths, args) => {
            let args = args
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();

            (
                Expression::ConstArrayLiteral(*loc, ty.clone(), lengths.clone(), args),
                false,
            )
        }
        Expression::Load(loc, ty, expr) => {
            let (expr, _) = expression(expr, vars, pos, cfg, ns);

            (Expression::Load(*loc, ty.clone(), Box::new(expr)), false)
        }
        Expression::StorageLoad(loc, ty, expr) => {
            let (expr, _) = expression(expr, vars, pos, cfg, ns);

            (
                Expression::StorageLoad(*loc, ty.clone(), Box::new(expr)),
                false,
            )
        }
        Expression::Cast(loc, ty, expr) => {
            let (expr, _) = expression(expr, vars, pos, cfg, ns);

            (Expression::Cast(*loc, ty.clone(), Box::new(expr)), false)
        }
        Expression::BytesCast(loc, from, to, expr) => {
            let (expr, _) = expression(expr, vars, pos, cfg, ns);

            (
                Expression::BytesCast(*loc, from.clone(), to.clone(), Box::new(expr)),
                false,
            )
        }
        Expression::More(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::More(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::Less(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::Less(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::MoreEqual(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::MoreEqual(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::LessEqual(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::LessEqual(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::Equal(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::Equal(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::NotEqual(loc, left, right) => {
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::NotEqual(*loc, Box::new(left.0), Box::new(right.0)),
                false,
            )
        }
        Expression::Ternary(loc, ty, cond, left, right) => {
            let cond = expression(cond, vars, pos, cfg, ns);
            let left = expression(left, vars, pos, cfg, ns);
            let right = expression(right, vars, pos, cfg, ns);

            (
                Expression::Ternary(
                    *loc,
                    ty.clone(),
                    Box::new(cond.0),
                    Box::new(left.0),
                    Box::new(right.0),
                ),
                false,
            )
        }
        Expression::Not(loc, expr) => {
            let expr = expression(expr, vars, pos, cfg, ns);

            (Expression::Not(*loc, Box::new(expr.0)), expr.1)
        }
        Expression::Subscript(loc, ty, array, index) => {
            let array = expression(array, vars, pos, cfg, ns);
            let index = expression(index, vars, pos, cfg, ns);

            (
                Expression::Subscript(*loc, ty.clone(), Box::new(array.0), Box::new(index.0)),
                false,
            )
        }
        Expression::StructMember(loc, ty, strct, member) => {
            let strct = expression(strct, vars, pos, cfg, ns);

            (
                Expression::StructMember(*loc, ty.clone(), Box::new(strct.0), *member),
                false,
            )
        }
        Expression::DynamicArrayLength(loc, array) => {
            let array = expression(array, vars, pos, cfg, ns);

            (
                Expression::DynamicArrayLength(*loc, Box::new(array.0)),
                false,
            )
        }
        Expression::DynamicArraySubscript(loc, ty, array, index) => {
            let array = expression(array, vars, pos, cfg, ns);
            let index = expression(index, vars, pos, cfg, ns);

            (
                Expression::DynamicArraySubscript(
                    *loc,
                    ty.clone(),
                    Box::new(array.0),
                    Box::new(index.0),
                ),
                false,
            )
        }
        Expression::StorageBytesSubscript(loc, array, index) => {
            let array = expression(array, vars, pos, cfg, ns);
            let index = expression(index, vars, pos, cfg, ns);

            (
                Expression::StorageBytesSubscript(*loc, Box::new(array.0), Box::new(index.0)),
                false,
            )
        }
        Expression::StorageArrayLength {
            loc,
            ty,
            array,
            elem_ty,
        } => {
            let array = expression(array, vars, pos, cfg, ns);

            (
                Expression::StorageArrayLength {
                    loc: *loc,
                    ty: ty.clone(),
                    array: Box::new(array.0),
                    elem_ty: elem_ty.clone(),
                },
                false,
            )
        }
        Expression::StringCompare(loc, left, right) => {
            if let (StringLocation::CompileTime(left), StringLocation::CompileTime(right)) =
                (left, right)
            {
                (Expression::BoolLiteral(*loc, left == right), true)
            } else {
                let left = if let StringLocation::RunTime(left) = left {
                    StringLocation::RunTime(Box::new(expression(left, vars, pos, cfg, ns).0))
                } else {
                    left.clone()
                };

                let right = if let StringLocation::RunTime(right) = right {
                    StringLocation::RunTime(Box::new(expression(right, vars, pos, cfg, ns).0))
                } else {
                    right.clone()
                };

                (Expression::StringCompare(*loc, left, right), false)
            }
        }
        Expression::StringConcat(loc, ty, left, right) => {
            if let (StringLocation::CompileTime(left), StringLocation::CompileTime(right)) =
                (left, right)
            {
                let mut bs = Vec::with_capacity(left.len() + right.len());

                bs.extend(left);
                bs.extend(right);

                (Expression::BytesLiteral(*loc, ty.clone(), bs), true)
            } else {
                let left = if let StringLocation::RunTime(left) = left {
                    StringLocation::RunTime(Box::new(expression(left, vars, pos, cfg, ns).0))
                } else {
                    left.clone()
                };

                let right = if let StringLocation::RunTime(right) = right {
                    StringLocation::RunTime(Box::new(expression(right, vars, pos, cfg, ns).0))
                } else {
                    right.clone()
                };

                (
                    Expression::StringConcat(*loc, ty.clone(), left, right),
                    false,
                )
            }
        }
        Expression::Builtin(loc, tys, builtin, args) => {
            let args = args
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();

            (
                Expression::Builtin(*loc, tys.clone(), *builtin, args),
                false,
            )
        }
        Expression::ExternalFunction {
            loc,
            ty,
            address,
            function_no,
        } => {
            let address = expression(address, vars, pos, cfg, ns);

            (
                Expression::ExternalFunction {
                    loc: *loc,
                    ty: ty.clone(),
                    address: Box::new(address.0),
                    function_no: *function_no,
                },
                address.1,
            )
        }
        Expression::AbiEncode {
            loc,
            tys,
            packed,
            args,
        } => {
            let packed = packed
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();
            let args = args
                .iter()
                .map(|expr| expression(expr, vars, pos, cfg, ns).0)
                .collect();

            (
                Expression::AbiEncode {
                    loc: *loc,
                    tys: tys.clone(),
                    packed,
                    args,
                },
                false,
            )
        }
        Expression::NumberLiteral(_, _, _)
        | Expression::BoolLiteral(_, _)
        | Expression::BytesLiteral(_, _, _)
        | Expression::CodeLiteral(_, _, _)
        | Expression::FunctionArg(_, _, _) => (expr.clone(), true),
        Expression::AllocDynamicArray(_, _, _, _)
        | Expression::ReturnData(_)
        | Expression::FormatString { .. }
        | Expression::InternalFunctionCfg(_) => (expr.clone(), false),
        // nothing else is permitted in cfg
        _ => panic!("expr should not be in cfg: {:?}", expr),
    }
}

fn bigint_to_expression(loc: &Loc, ty: &Type, n: BigInt) -> (Expression, bool) {
    let n = match ty {
        Type::Uint(bits) => {
            if n.bits() > *bits as u64 {
                let (_, mut bs) = n.to_bytes_le();
                bs.truncate(*bits as usize / 8);

                BigInt::from_bytes_le(Sign::Plus, &bs)
            } else {
                n
            }
        }
        Type::Int(bits) => {
            if n.bits() > *bits as u64 {
                let mut bs = n.to_signed_bytes_le();
                bs.truncate(*bits as usize / 8);

                BigInt::from_signed_bytes_le(&bs)
            } else {
                n
            }
        }
        Type::StorageRef(_, _) => n,
        _ => unreachable!(),
    };

    (Expression::NumberLiteral(*loc, ty.clone(), n), true)
}

fn get_definition<'a>(
    def: &reaching_definitions::Def,
    cfg: &'a ControlFlowGraph,
) -> Option<&'a Expression> {
    if let Instr::Set { expr, .. } = &cfg.blocks[def.block_no].instr[def.instr_no] {
        Some(expr)
    } else {
        None
    }
}

/// Are these two expressions the same constant-folded value?
fn constants_equal(left: &Expression, right: &Expression) -> bool {
    match left {
        Expression::NumberLiteral(_, _, left) => match right {
            Expression::NumberLiteral(_, _, right) => left == right,
            _ => false,
        },
        Expression::BytesLiteral(_, _, left)
        | Expression::AllocDynamicArray(_, _, _, Some(left)) => match right {
            Expression::BytesLiteral(_, _, right)
            | Expression::AllocDynamicArray(_, _, _, Some(right)) => left == right,
            _ => false,
        },
        _ => false,
    }
}
