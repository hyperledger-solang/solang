use super::cfg::{ControlFlowGraph, HashTy, Instr, InternalCallTy, Vartable};
use super::storage::{array_offset, array_pop, array_push, bytes_pop, bytes_push};
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use parser::pt;
use sema::ast::{Builtin, CallTy, Expression, Namespace, Parameter, StringLocation, Type};
use sema::eval::eval_const_number;
use sema::expression::{cast_shift_arg, try_bigint_to_expression, try_cast};
use std::collections::HashSet;
use std::ops::Mul;
use Target;

pub fn expression(
    expr: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    match expr {
        Expression::StorageVariable(_, _, var_contract_no, var_no) => {
            // base storage variables should precede contract variables, not overlap
            ns.contracts[contract_no].get_storage_slot(*var_contract_no, *var_no, ns)
        }
        Expression::StorageLoad(loc, ty, expr) => Expression::StorageLoad(
            *loc,
            ty.clone(),
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::Add(loc, ty, left, right) => Expression::Add(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Subtract(loc, ty, left, right) => Expression::Subtract(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Multiply(loc, ty, left, right) => Expression::Multiply(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Divide(loc, ty, left, right) => Expression::Divide(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Modulo(loc, ty, left, right) => Expression::Modulo(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Power(loc, ty, left, right) => Expression::Power(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::BitwiseOr(loc, ty, left, right) => Expression::BitwiseOr(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::BitwiseAnd(loc, ty, left, right) => Expression::BitwiseAnd(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::BitwiseXor(loc, ty, left, right) => Expression::BitwiseXor(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::ShiftLeft(loc, ty, left, right) => Expression::ShiftLeft(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::ShiftRight(loc, ty, left, right, sign) => Expression::ShiftRight(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
            *sign,
        ),
        Expression::Equal(loc, left, right) => Expression::Equal(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::NotEqual(loc, left, right) => Expression::NotEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::More(loc, left, right) => Expression::More(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::MoreEqual(loc, left, right) => Expression::MoreEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::Less(loc, left, right) => Expression::Less(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::LessEqual(loc, left, right) => Expression::LessEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, ns, vartab)),
            Box::new(expression(right, cfg, contract_no, ns, vartab)),
        ),
        Expression::ConstantVariable(_, _, var_contract_no, var_no) => expression(
            ns.contracts[*var_contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap(),
            cfg,
            contract_no,
            ns,
            vartab,
        ),
        Expression::Not(loc, expr) => Expression::Not(
            *loc,
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::Complement(loc, ty, expr) => Expression::Complement(
            *loc,
            ty.clone(),
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::UnaryMinus(loc, ty, expr) => Expression::UnaryMinus(
            *loc,
            ty.clone(),
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::StructLiteral(loc, ty, exprs) => Expression::StructLiteral(
            *loc,
            ty.clone(),
            exprs
                .iter()
                .map(|e| expression(e, cfg, contract_no, ns, vartab))
                .collect(),
        ),
        Expression::Assign(_, _, left, right) => {
            assign_single(left, right, cfg, contract_no, ns, vartab)
        }
        Expression::PreDecrement(loc, ty, var) | Expression::PreIncrement(loc, ty, var) => {
            let res = vartab.temp_anonymous(ty);
            let v = expression(var, cfg, contract_no, ns, vartab);
            let v = match var.ty() {
                Type::Ref(ty) => Expression::Load(var.loc(), ty.as_ref().clone(), Box::new(v)),
                Type::StorageRef(ty) => {
                    Expression::StorageLoad(var.loc(), ty.as_ref().clone(), Box::new(v))
                }
                _ => v,
            };

            let one = Box::new(Expression::NumberLiteral(*loc, ty.clone(), BigInt::one()));
            let expr = match expr {
                Expression::PreDecrement(_, _, _) => {
                    Expression::Subtract(*loc, ty.clone(), Box::new(v), one)
                }
                Expression::PreIncrement(_, _, _) => {
                    Expression::Add(*loc, ty.clone(), Box::new(v), one)
                }
                _ => unreachable!(),
            };

            cfg.add(vartab, Instr::Set { res, expr });

            match var.as_ref() {
                Expression::Variable(_, _, pos) => {
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: *pos,
                            expr: Expression::Variable(*loc, ty.clone(), res),
                        },
                    );
                }
                _ => {
                    let dest = expression(var, cfg, contract_no, ns, vartab);

                    match var.ty() {
                        Type::StorageRef(_) => {
                            cfg.add(
                                vartab,
                                Instr::SetStorage {
                                    local: res,
                                    ty: ty.clone(),
                                    storage: dest,
                                },
                            );
                        }
                        Type::Ref(_) => {
                            cfg.add(vartab, Instr::Store { pos: res, dest });
                        }
                        _ => unreachable!(),
                    }
                }
            }

            Expression::Variable(*loc, ty.clone(), res)
        }
        Expression::PostDecrement(loc, ty, var) | Expression::PostIncrement(loc, ty, var) => {
            let res = vartab.temp_anonymous(ty);
            let v = expression(var, cfg, contract_no, ns, vartab);
            let v = match var.ty() {
                Type::Ref(ty) => Expression::Load(var.loc(), ty.as_ref().clone(), Box::new(v)),
                Type::StorageRef(ty) => {
                    Expression::StorageLoad(var.loc(), ty.as_ref().clone(), Box::new(v))
                }
                _ => v,
            };

            cfg.add(vartab, Instr::Set { res, expr: v });

            let one = Box::new(Expression::NumberLiteral(*loc, ty.clone(), BigInt::one()));
            let expr = match expr {
                Expression::PostDecrement(_, _, _) => Expression::Subtract(
                    *loc,
                    ty.clone(),
                    Box::new(Expression::Variable(*loc, ty.clone(), res)),
                    one,
                ),
                Expression::PostIncrement(_, _, _) => Expression::Add(
                    *loc,
                    ty.clone(),
                    Box::new(Expression::Variable(*loc, ty.clone(), res)),
                    one,
                ),
                _ => unreachable!(),
            };

            match var.as_ref() {
                Expression::Variable(_, _, pos) => {
                    cfg.add(vartab, Instr::Set { res: *pos, expr });
                }
                _ => {
                    let dest = expression(var, cfg, contract_no, ns, vartab);
                    let res = vartab.temp_anonymous(ty);
                    cfg.add(vartab, Instr::Set { res, expr });

                    match var.ty() {
                        Type::StorageRef(_) => {
                            cfg.add(
                                vartab,
                                Instr::SetStorage {
                                    local: res,
                                    ty: ty.clone(),
                                    storage: dest,
                                },
                            );
                        }
                        Type::Ref(_) => {
                            cfg.add(vartab, Instr::Store { pos: res, dest });
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Expression::Variable(*loc, ty.clone(), res)
        }
        Expression::Constructor {
            loc,
            contract_no,
            constructor_no,
            args,
            value,
            gas,
            salt,
        } => {
            let address_res = vartab.temp_anonymous(&Type::Contract(*contract_no));

            let args = args
                .iter()
                .map(|v| expression(&v, cfg, *contract_no, ns, vartab))
                .collect();
            let gas = expression(gas, cfg, *contract_no, ns, vartab);
            let value = match value {
                Some(value) => Some(expression(&value, cfg, *contract_no, ns, vartab)),
                None => None,
            };
            let salt = match salt {
                Some(salt) => Some(expression(&salt, cfg, *contract_no, ns, vartab)),
                None => None,
            };

            cfg.add(
                vartab,
                Instr::Constructor {
                    success: None,
                    res: address_res,
                    contract_no: *contract_no,
                    constructor_no: *constructor_no,
                    args,
                    value,
                    gas,
                    salt,
                },
            );

            Expression::Variable(*loc, Type::Contract(*contract_no), address_res)
        }
        Expression::InternalFunction {
            contract_no: base_contract_no,
            function_no,
            signature,
            ..
        } => {
            let (base_contract_no, function_no) = if let Some(signature) = signature {
                ns.contracts[contract_no].virtual_functions[signature]
            } else {
                (*base_contract_no, *function_no)
            };

            Expression::InternalFunctionCfg(
                ns.contracts[contract_no].all_functions[&(base_contract_no, function_no)],
            )
        }
        Expression::Builtin(loc, returns, Builtin::ExternalFunctionAddress, func) => {
            if let Expression::ExternalFunction { address, .. } = &func[0] {
                expression(address, cfg, contract_no, ns, vartab)
            } else {
                let func = expression(&func[0], cfg, contract_no, ns, vartab);

                Expression::Builtin(
                    *loc,
                    returns.clone(),
                    Builtin::ExternalFunctionAddress,
                    vec![func],
                )
            }
        }
        Expression::Builtin(loc, returns, Builtin::ExternalFunctionSelector, func) => {
            if let Expression::ExternalFunction {
                contract_no: base_contract_no,
                function_no,
                ..
            } = &func[0]
            {
                let selector = ns.contracts[*base_contract_no].functions[*function_no].selector();

                Expression::NumberLiteral(*loc, Type::Uint(32), BigInt::from(selector))
            } else {
                let func = expression(&func[0], cfg, contract_no, ns, vartab);

                Expression::Builtin(
                    *loc,
                    returns.clone(),
                    Builtin::ExternalFunctionSelector,
                    vec![func],
                )
            }
        }
        Expression::InternalFunctionCall { .. }
        | Expression::ExternalFunctionCall { .. }
        | Expression::Builtin(_, _, Builtin::AbiDecode, _) => {
            let mut returns = emit_function_call(expr, contract_no, cfg, ns, vartab);

            assert_eq!(returns.len(), 1);

            returns.remove(0)
        }
        Expression::ExternalFunction {
            loc,
            ty,
            address,
            contract_no: base_contract_no,
            function_no,
        } => {
            let address = expression(address, cfg, contract_no, ns, vartab);

            Expression::ExternalFunction {
                loc: *loc,
                ty: ty.clone(),
                address: Box::new(address),
                contract_no: *base_contract_no,
                function_no: *function_no,
            }
        }
        Expression::ArraySubscript(loc, ty, array, index) => {
            array_subscript(loc, ty, array, index, cfg, contract_no, ns, vartab)
        }
        Expression::StructMember(loc, ty, var, member) => Expression::StructMember(
            *loc,
            ty.clone(),
            Box::new(expression(var, cfg, contract_no, ns, vartab)),
            *member,
        ),
        Expression::StorageBytesSubscript(loc, var, index) => Expression::StorageBytesSubscript(
            *loc,
            Box::new(expression(var, cfg, contract_no, ns, vartab)),
            Box::new(expression(index, cfg, contract_no, ns, vartab)),
        ),
        Expression::StringCompare(loc, left, right) => Expression::StringCompare(
            *loc,
            string_location(left, cfg, contract_no, ns, vartab),
            string_location(right, cfg, contract_no, ns, vartab),
        ),
        Expression::StringConcat(loc, ty, left, right) => Expression::StringConcat(
            *loc,
            ty.clone(),
            string_location(left, cfg, contract_no, ns, vartab),
            string_location(right, cfg, contract_no, ns, vartab),
        ),
        Expression::DynamicArrayLength(loc, expr) => Expression::DynamicArrayLength(
            *loc,
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::DynamicArrayPush(loc, array, ty, value) => {
            let elem_ty = match ty {
                Type::Array(..) => match ty.array_elem() {
                    elem @ Type::Struct(..) => Type::Ref(Box::new(elem)),
                    elem => elem,
                },
                Type::DynamicBytes => Type::Uint(8),
                _ => unreachable!(),
            };
            let address_res = vartab.temp_anonymous(&elem_ty);

            let address_arr = match expression(array, cfg, contract_no, ns, vartab) {
                Expression::Variable(_, _, pos) => pos,
                _ => unreachable!(),
            };

            cfg.add(
                vartab,
                Instr::PushMemory {
                    res: address_res,
                    ty: ty.clone(),
                    array: address_arr,
                    value: value.clone(),
                },
            );

            Expression::Variable(*loc, elem_ty, address_res)
        }
        Expression::DynamicArrayPop(loc, array, ty) => {
            let elem_ty = match ty {
                Type::Array(..) => match ty.array_elem() {
                    elem @ Type::Struct(..) => Type::Ref(Box::new(elem)),
                    elem => elem,
                },
                Type::DynamicBytes => Type::Uint(8),
                _ => unreachable!(),
            };
            let address_res = vartab.temp_anonymous(&elem_ty);

            let address_arr = match expression(array, cfg, contract_no, ns, vartab) {
                Expression::Variable(_, _, pos) => pos,
                _ => unreachable!(),
            };

            cfg.add(
                vartab,
                Instr::PopMemory {
                    res: address_res,
                    ty: ty.clone(),
                    array: address_arr,
                },
            );

            Expression::Variable(*loc, elem_ty, address_res)
        }
        Expression::StorageBytesLength(loc, expr) => Expression::StorageBytesLength(
            *loc,
            Box::new(expression(expr, cfg, contract_no, ns, vartab)),
        ),
        Expression::Or(loc, left, right) => {
            let boolty = Type::Bool;
            let l = expression(left, cfg, contract_no, ns, vartab);

            let pos = vartab.temp(
                &pt::Identifier {
                    name: "or".to_owned(),
                    loc: *loc,
                },
                &Type::Bool,
            );

            let right_side = cfg.new_basic_block("or_right_side".to_string());
            let end_or = cfg.new_basic_block("or_end".to_string());

            cfg.add(
                vartab,
                Instr::Set {
                    res: pos,
                    expr: Expression::BoolLiteral(*loc, true),
                },
            );
            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: l,
                    true_: end_or,
                    false_: right_side,
                },
            );
            cfg.set_basic_block(right_side);

            let r = expression(right, cfg, contract_no, ns, vartab);

            cfg.add(vartab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_or, phis);

            cfg.add(vartab, Instr::Branch { bb: end_or });

            cfg.set_basic_block(end_or);

            Expression::Variable(*loc, boolty, pos)
        }
        Expression::And(loc, left, right) => {
            let boolty = Type::Bool;
            let l = expression(left, cfg, contract_no, ns, vartab);

            let pos = vartab.temp(
                &pt::Identifier {
                    name: "and".to_owned(),
                    loc: *loc,
                },
                &Type::Bool,
            );

            let right_side = cfg.new_basic_block("and_right_side".to_string());
            let end_and = cfg.new_basic_block("and_end".to_string());

            cfg.add(
                vartab,
                Instr::Set {
                    res: pos,
                    expr: Expression::BoolLiteral(*loc, false),
                },
            );
            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: l,
                    true_: right_side,
                    false_: end_and,
                },
            );
            cfg.set_basic_block(right_side);

            let r = expression(right, cfg, contract_no, ns, vartab);

            cfg.add(vartab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_and, phis);

            cfg.add(vartab, Instr::Branch { bb: end_and });

            cfg.set_basic_block(end_and);

            Expression::Variable(*loc, boolty, pos)
        }
        Expression::Trunc(loc, ty, e) => Expression::Trunc(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, ns, vartab)),
        ),
        Expression::ZeroExt(loc, ty, e) => Expression::ZeroExt(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, ns, vartab)),
        ),
        Expression::SignExt(loc, ty, e) => Expression::SignExt(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, ns, vartab)),
        ),
        Expression::Cast(loc, ty, e) => Expression::Cast(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, ns, vartab)),
        ),
        Expression::Load(loc, ty, e) => Expression::Load(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, ns, vartab)),
        ),
        // for some built-ins, we have to inline special case code
        Expression::Builtin(loc, _, Builtin::ArrayPush, args) => {
            array_push(loc, args, cfg, contract_no, ns, vartab)
        }
        Expression::Builtin(loc, _, Builtin::ArrayPop, args) => {
            array_pop(loc, args, cfg, contract_no, ns, vartab)
        }
        Expression::Builtin(loc, _, Builtin::BytesPush, args) => {
            bytes_push(loc, args, cfg, contract_no, ns, vartab)
        }
        Expression::Builtin(loc, _, Builtin::BytesPop, args) => {
            bytes_pop(loc, args, cfg, contract_no, ns, vartab)
        }
        Expression::Builtin(_, _, Builtin::Assert, args) => {
            let true_ = cfg.new_basic_block("noassert".to_owned());
            let false_ = cfg.new_basic_block("doassert".to_owned());

            let cond = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond,
                    true_,
                    false_,
                },
            );

            cfg.set_basic_block(false_);
            cfg.add(vartab, Instr::AssertFailure { expr: None });

            cfg.set_basic_block(true_);

            Expression::Poison
        }
        Expression::Builtin(_, _, Builtin::Print, args) => {
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(vartab, Instr::Print { expr });

            Expression::Poison
        }
        Expression::Builtin(_, _, Builtin::Require, args) => {
            let true_ = cfg.new_basic_block("noassert".to_owned());
            let false_ = cfg.new_basic_block("doassert".to_owned());

            let cond = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond,
                    true_,
                    false_,
                },
            );

            cfg.set_basic_block(false_);

            let expr = args
                .get(1)
                .map(|s| expression(s, cfg, contract_no, ns, vartab));

            cfg.add(vartab, Instr::AssertFailure { expr });

            cfg.set_basic_block(true_);

            Expression::Poison
        }
        Expression::Builtin(_, _, Builtin::Revert, args) => {
            let expr = args
                .get(0)
                .map(|s| expression(s, cfg, contract_no, ns, vartab));

            cfg.add(vartab, Instr::AssertFailure { expr });

            Expression::Poison
        }
        Expression::Builtin(_, _, Builtin::SelfDestruct, args) => {
            let recipient = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(vartab, Instr::SelfDestruct { recipient });

            Expression::Poison
        }
        Expression::Builtin(loc, tys, Builtin::Keccak256, args) => {
            let res = vartab.temp_anonymous(&tys[0]);
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Hash {
                    res,
                    hash: HashTy::Keccak256,
                    expr,
                },
            );

            Expression::Variable(*loc, tys[0].clone(), res)
        }
        Expression::Builtin(loc, tys, Builtin::Ripemd160, args) => {
            let res = vartab.temp_anonymous(&tys[0]);
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Hash {
                    res,
                    hash: HashTy::Ripemd160,
                    expr,
                },
            );

            Expression::Variable(*loc, tys[0].clone(), res)
        }
        Expression::Builtin(loc, tys, Builtin::Sha256, args) => {
            let res = vartab.temp_anonymous(&tys[0]);
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Hash {
                    res,
                    hash: HashTy::Sha256,
                    expr,
                },
            );

            Expression::Variable(*loc, tys[0].clone(), res)
        }
        Expression::Builtin(loc, tys, Builtin::Blake2_128, args) => {
            let res = vartab.temp_anonymous(&tys[0]);
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Hash {
                    res,
                    hash: HashTy::Blake2_128,
                    expr,
                },
            );

            Expression::Variable(*loc, tys[0].clone(), res)
        }
        Expression::Builtin(loc, tys, Builtin::Blake2_256, args) => {
            let res = vartab.temp_anonymous(&tys[0]);
            let expr = expression(&args[0], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Hash {
                    res,
                    hash: HashTy::Blake2_256,
                    expr,
                },
            );

            Expression::Variable(*loc, tys[0].clone(), res)
        }
        Expression::Builtin(loc, _, Builtin::PayableSend, args) => {
            let address = expression(&args[0], cfg, contract_no, ns, vartab);
            let value = expression(&args[1], cfg, contract_no, ns, vartab);

            let success = vartab.temp(
                &pt::Identifier {
                    loc: *loc,
                    name: "success".to_owned(),
                },
                &Type::Bool,
            );

            cfg.add(
                vartab,
                Instr::ExternalCall {
                    success: Some(success),
                    address: Some(address),
                    payload: Expression::BytesLiteral(*loc, Type::DynamicBytes, vec![]),
                    args: Vec::new(),
                    value,
                    gas: Expression::NumberLiteral(*loc, Type::Uint(64), BigInt::zero()),
                    callty: CallTy::Regular,
                },
            );

            Expression::Variable(*loc, Type::Bool, success)
        }
        Expression::Builtin(loc, _, Builtin::PayableTransfer, args) => {
            let address = expression(&args[0], cfg, contract_no, ns, vartab);
            let value = expression(&args[1], cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::ExternalCall {
                    success: None,
                    address: Some(address),
                    payload: Expression::BytesLiteral(*loc, Type::DynamicBytes, vec![]),
                    args: Vec::new(),
                    value,
                    gas: Expression::NumberLiteral(*loc, Type::Uint(64), BigInt::zero()),
                    callty: CallTy::Regular,
                },
            );

            Expression::Poison
        }
        Expression::Builtin(loc, _, Builtin::AbiEncode, args) => {
            let tys = args.iter().map(|a| a.ty()).collect();
            let args = args
                .iter()
                .map(|v| expression(&v, cfg, contract_no, ns, vartab))
                .collect();

            let res = vartab.temp(
                &pt::Identifier {
                    loc: *loc,
                    name: "encoded".to_owned(),
                },
                &Type::DynamicBytes,
            );

            cfg.add(
                vartab,
                Instr::AbiEncodeVector {
                    res,
                    tys,
                    selector: None,
                    packed: false,
                    args,
                },
            );

            Expression::Variable(*loc, Type::DynamicBytes, res)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodePacked, args) => {
            let tys = args.iter().map(|a| a.ty()).collect();
            let args = args
                .iter()
                .map(|v| expression(&v, cfg, contract_no, ns, vartab))
                .collect();

            let res = vartab.temp(
                &pt::Identifier {
                    loc: *loc,
                    name: "encoded".to_owned(),
                },
                &Type::DynamicBytes,
            );

            cfg.add(
                vartab,
                Instr::AbiEncodeVector {
                    res,
                    tys,
                    selector: None,
                    packed: true,
                    args,
                },
            );

            Expression::Variable(*loc, Type::DynamicBytes, res)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodeWithSelector, args) => {
            let tys = args.iter().skip(1).map(|a| a.ty()).collect();
            // first argument is selector
            let mut args_iter = args.iter();
            let selector = expression(&args_iter.next().unwrap(), cfg, contract_no, ns, vartab);
            let args = args_iter
                .map(|v| expression(&v, cfg, contract_no, ns, vartab))
                .collect();

            let res = vartab.temp(
                &pt::Identifier {
                    loc: *loc,
                    name: "encoded".to_owned(),
                },
                &Type::DynamicBytes,
            );

            cfg.add(
                vartab,
                Instr::AbiEncodeVector {
                    res,
                    tys,
                    selector: Some(selector),
                    packed: false,
                    args,
                },
            );

            Expression::Variable(*loc, Type::DynamicBytes, res)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodeWithSignature, args) => {
            let tys = args.iter().skip(1).map(|a| a.ty()).collect();
            // first argument is signature which needs hashing and shifting
            let mut args_iter = args.iter();
            let hash = Expression::Builtin(
                *loc,
                vec![Type::Bytes(32)],
                Builtin::Keccak256,
                vec![args_iter.next().unwrap().clone()],
            );
            let hash = expression(&hash, cfg, contract_no, ns, vartab);
            let selector = try_cast(loc, hash, &Type::Bytes(4), false, ns).unwrap();
            let args = args_iter
                .map(|v| expression(&v, cfg, contract_no, ns, vartab))
                .collect();

            let res = vartab.temp(
                &pt::Identifier {
                    loc: *loc,
                    name: "encoded".to_owned(),
                },
                &Type::DynamicBytes,
            );

            cfg.add(
                vartab,
                Instr::AbiEncodeVector {
                    res,
                    tys,
                    selector: Some(selector),
                    packed: false,
                    args,
                },
            );

            Expression::Variable(*loc, Type::DynamicBytes, res)
        }
        // The Substrate gas price builtin takes an argument; the others do not
        Expression::Builtin(loc, _, Builtin::Gasprice, expr)
            if expr.len() == 1 && ns.target != Target::Substrate =>
        {
            let ty = Type::Value;
            let gasprice = Expression::Builtin(*loc, vec![ty.clone()], Builtin::Gasprice, vec![]);
            let units = expression(&expr[0], cfg, contract_no, ns, vartab);

            Expression::Multiply(*loc, ty, Box::new(units), Box::new(gasprice))
        }
        _ => expr.clone(),
    }
}

pub fn assign_single(
    left: &Expression,
    right: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    match left {
        Expression::Variable(_, _, pos) => {
            let expr = expression(right, cfg, contract_no, ns, vartab);
            cfg.add(vartab, Instr::Set { res: *pos, expr });

            left.clone()
        }
        _ => {
            let left_ty = left.ty();
            let ty = left_ty.deref_memory();

            let pos = vartab.temp_anonymous(&ty);

            let dest = expression(left, cfg, contract_no, ns, vartab);
            let right = expression(right, cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::Set {
                    res: pos,
                    expr: right,
                },
            );

            match left_ty {
                Type::StorageRef(_) => {
                    if let Expression::StorageBytesSubscript(_, array, index) = dest {
                        // Set a byte in a byte array
                        cfg.add(
                            vartab,
                            Instr::SetStorageBytes {
                                local: pos,
                                storage: array,
                                offset: index,
                            },
                        );
                    } else {
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                local: pos,
                                ty: ty.deref_any().clone(),
                                storage: dest,
                            },
                        );
                    }
                }
                Type::Ref(_) => {
                    cfg.add(vartab, Instr::Store { pos, dest });
                }
                _ => unreachable!(),
            }

            Expression::Variable(left.loc(), ty.clone(), pos)
        }
    }
}

/// Convert a function call expression to CFG in expression context
pub fn emit_function_call(
    expr: &Expression,
    callee_contract_no: usize,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Vec<Expression> {
    match expr {
        Expression::InternalFunctionCall { function, args, .. } => {
            if let Expression::InternalFunction {
                contract_no,
                function_no,
                signature,
                ..
            } = function.as_ref()
            {
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                    .collect();

                let (base_contract_no, function_no) = if let Some(signature) = signature {
                    ns.contracts[callee_contract_no].virtual_functions[signature]
                } else {
                    (*contract_no, *function_no)
                };

                let cfg_no = ns.contracts[callee_contract_no].all_functions
                    [&(base_contract_no, function_no)];

                let ftype = &ns.contracts[base_contract_no].functions[function_no];

                if !ftype.returns.is_empty() {
                    let mut res = Vec::new();
                    let mut returns = Vec::new();

                    for ret in &ftype.returns {
                        let id = pt::Identifier {
                            loc: ret.loc,
                            name: ret.name.to_owned(),
                        };

                        let temp_pos = vartab.temp(&id, &ret.ty);
                        res.push(temp_pos);
                        returns.push(Expression::Variable(id.loc, ret.ty.clone(), temp_pos));
                    }

                    cfg.add(
                        vartab,
                        Instr::Call {
                            res,
                            call: InternalCallTy::Static(cfg_no),
                            args,
                        },
                    );

                    returns
                } else {
                    cfg.add(
                        vartab,
                        Instr::Call {
                            res: Vec::new(),
                            call: InternalCallTy::Static(cfg_no),
                            args,
                        },
                    );

                    vec![Expression::Poison]
                }
            } else if let Type::InternalFunction { returns, .. } = function.ty().deref_any() {
                let cfg_expr = expression(function, cfg, callee_contract_no, ns, vartab);

                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                    .collect();

                if !returns.is_empty() {
                    let mut res = Vec::new();
                    let mut return_values = Vec::new();

                    for ty in returns {
                        let id = pt::Identifier {
                            loc: pt::Loc(0, 0, 0),
                            name: String::new(),
                        };

                        let temp_pos = vartab.temp(&id, &ty);
                        res.push(temp_pos);
                        return_values.push(Expression::Variable(id.loc, ty.clone(), temp_pos));
                    }

                    cfg.add(
                        vartab,
                        Instr::Call {
                            res,
                            call: InternalCallTy::Dynamic(cfg_expr),
                            args,
                        },
                    );

                    return_values
                } else {
                    cfg.add(
                        vartab,
                        Instr::Call {
                            res: Vec::new(),
                            call: InternalCallTy::Dynamic(cfg_expr),
                            args,
                        },
                    );

                    vec![Expression::Poison]
                }
            } else {
                unreachable!();
            }
        }
        Expression::ExternalFunctionCallRaw {
            loc,
            address,
            args,
            value,
            gas,
            ty,
        } => {
            let args = expression(args, cfg, callee_contract_no, ns, vartab);
            let address = expression(address, cfg, callee_contract_no, ns, vartab);
            let gas = expression(gas, cfg, callee_contract_no, ns, vartab);
            let value = expression(value, cfg, callee_contract_no, ns, vartab);

            let success = vartab.temp_name("success", &Type::Bool);

            cfg.add(
                vartab,
                Instr::ExternalCall {
                    success: Some(success),
                    address: Some(address),
                    payload: args,
                    args: vec![],
                    value,
                    gas,
                    callty: ty.clone(),
                },
            );

            vec![
                Expression::Variable(*loc, Type::Bool, success),
                Expression::ReturnData(*loc),
            ]
        }
        Expression::ExternalFunctionCall {
            loc,
            function,
            args,
            value,
            gas,
            ..
        } => {
            if let Expression::ExternalFunction {
                contract_no,
                function_no,
                address,
                ty,
                ..
            } = function.as_ref()
            {
                let ftype = &ns.contracts[*contract_no].functions[*function_no];
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                    .collect();
                let address = expression(address, cfg, callee_contract_no, ns, vartab);
                let gas = expression(gas, cfg, callee_contract_no, ns, vartab);
                let value = expression(value, cfg, callee_contract_no, ns, vartab);

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: None,
                        address: None,
                        payload: Expression::ExternalFunction {
                            contract_no: *contract_no,
                            function_no: *function_no,
                            address: Box::new(address),
                            loc: *loc,
                            ty: ty.clone(),
                        },
                        args,
                        value,
                        gas,
                        callty: CallTy::Regular,
                    },
                );

                if !ftype.returns.is_empty() {
                    let mut returns = Vec::new();
                    let mut res = Vec::new();

                    for ret in &ftype.returns {
                        let id = pt::Identifier {
                            loc: ret.loc,
                            name: ret.name.to_owned(),
                        };
                        let temp_pos = vartab.temp(&id, &ret.ty);
                        res.push(temp_pos);
                        returns.push(Expression::Variable(id.loc, ret.ty.clone(), temp_pos));
                    }

                    cfg.add(
                        vartab,
                        Instr::AbiDecode {
                            res,
                            selector: None,
                            exception: None,
                            tys: ftype.returns.clone(),
                            data: Expression::ReturnData(*loc),
                        },
                    );

                    returns
                } else {
                    vec![Expression::Poison]
                }
            } else if let Type::ExternalFunction {
                returns: func_returns,
                ..
            } = function.ty()
            {
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                    .collect();
                let payload = expression(function, cfg, callee_contract_no, ns, vartab);
                let gas = expression(gas, cfg, callee_contract_no, ns, vartab);
                let value = expression(value, cfg, callee_contract_no, ns, vartab);

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: None,
                        address: None,
                        payload,
                        args,
                        value,
                        gas,
                        callty: CallTy::Regular,
                    },
                );

                if !func_returns.is_empty() {
                    let mut returns = Vec::new();
                    let mut res = Vec::new();
                    let mut tys = Vec::new();

                    for ty in func_returns {
                        let temp_pos = vartab.temp_anonymous(&ty);
                        res.push(temp_pos);
                        returns.push(Expression::Variable(pt::Loc(0, 0, 0), ty.clone(), temp_pos));

                        tys.push(Parameter {
                            loc: pt::Loc(0, 0, 0),
                            ty,
                            ty_loc: pt::Loc(0, 0, 0),
                            name: String::new(),
                            name_loc: None,
                            indexed: false,
                        });
                    }

                    cfg.add(
                        vartab,
                        Instr::AbiDecode {
                            res,
                            selector: None,
                            exception: None,
                            tys,
                            data: Expression::ReturnData(*loc),
                        },
                    );

                    returns
                } else {
                    vec![Expression::Poison]
                }
            } else {
                unreachable!();
            }
        }
        Expression::Builtin(loc, tys, Builtin::AbiDecode, args) => {
            let data = expression(&args[0], cfg, callee_contract_no, ns, vartab);

            let mut returns = Vec::new();
            let mut res = Vec::new();

            for ret in tys {
                let temp_pos = vartab.temp_anonymous(&ret);
                res.push(temp_pos);
                returns.push(Expression::Variable(*loc, ret.clone(), temp_pos));
            }

            cfg.add(
                vartab,
                Instr::AbiDecode {
                    res,
                    selector: None,
                    exception: None,
                    tys: tys
                        .iter()
                        .map(|ty| Parameter {
                            name: "".to_owned(),
                            name_loc: None,
                            loc: *loc,
                            ty: ty.clone(),
                            ty_loc: *loc,
                            indexed: false,
                        })
                        .collect(),
                    data,
                },
            );

            returns
        }
        _ => unreachable!(),
    }
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &pt::Loc,
    ty: &Type,
    array: &Expression,
    index: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    if array.ty().is_mapping() {
        let array = expression(array, cfg, contract_no, ns, vartab);
        let index = expression(index, cfg, contract_no, ns, vartab);
        return Expression::Keccak256(*loc, ty.clone(), vec![array, index]);
    }

    let array_ty = array.ty();
    let mut array = expression(array, cfg, contract_no, ns, vartab);
    let index_ty = index.ty();
    let index = expression(index, cfg, contract_no, ns, vartab);
    let index_loc = index.loc();

    let index_width = index_ty.bits(ns);

    let array_length = match array_ty.deref_any() {
        Type::Bytes(n) => try_bigint_to_expression(&array.loc(), &BigInt::from(*n)).unwrap(),
        Type::Array(_, _) => match array_ty.array_length() {
            None => {
                if let Type::StorageRef(_) = array_ty {
                    let array_length =
                        Expression::StorageLoad(*loc, Type::Uint(256), Box::new(array.clone()));

                    array = Expression::Keccak256(*loc, Type::Uint(256), vec![array]);

                    array_length
                } else {
                    Expression::DynamicArrayLength(*loc, Box::new(array.clone()))
                }
            }
            Some(l) => try_bigint_to_expression(loc, l).unwrap(),
        },
        Type::DynamicBytes => Expression::DynamicArrayLength(*loc, Box::new(array.clone())),
        _ => {
            unreachable!();
        }
    };

    let array_width = array_length.ty().bits(ns);
    let width = std::cmp::max(array_width, index_width);
    let coerced_ty = Type::Uint(width);

    let pos = vartab.temp(
        &pt::Identifier {
            name: "index".to_owned(),
            loc: *loc,
        },
        &coerced_ty,
    );

    cfg.add(
        vartab,
        Instr::Set {
            res: pos,
            expr: try_cast(&index.loc(), index, &coerced_ty, false, ns).unwrap(),
        },
    );

    // If the array is fixed length and the index also constant, the
    // branch will be optimized away.
    let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
    let in_bounds = cfg.new_basic_block("in_bounds".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: Expression::MoreEqual(
                *loc,
                Box::new(Expression::Variable(index_loc, coerced_ty.clone(), pos)),
                Box::new(
                    try_cast(&array.loc(), array_length.clone(), &coerced_ty, false, ns).unwrap(),
                ),
            ),
            true_: out_of_bounds,
            false_: in_bounds,
        },
    );

    cfg.set_basic_block(out_of_bounds);
    cfg.add(vartab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(in_bounds);

    if let Type::StorageRef(ty) = array_ty {
        let elem_ty = ty.storage_array_elem();
        let elem_size = elem_ty.storage_slots(ns);
        let slot_ty = Type::Uint(256);

        if let Ok(array_length) = eval_const_number(&array_length, Some(contract_no), ns) {
            if array_length.1.mul(elem_size.clone()).to_u64().is_some() {
                // we need to calculate the storage offset. If this can be done with 64 bit
                // arithmetic it will be much more efficient on wasm
                return Expression::Add(
                    *loc,
                    elem_ty,
                    Box::new(array),
                    Box::new(Expression::ZeroExt(
                        *loc,
                        slot_ty,
                        Box::new(Expression::Multiply(
                            *loc,
                            Type::Uint(64),
                            Box::new(
                                try_cast(
                                    &index_loc,
                                    Expression::Variable(index_loc, coerced_ty, pos),
                                    &Type::Uint(64),
                                    false,
                                    ns,
                                )
                                .unwrap(),
                            ),
                            Box::new(Expression::NumberLiteral(*loc, Type::Uint(64), elem_size)),
                        )),
                    )),
                );
            }
        }

        array_offset(
            loc,
            array,
            try_cast(
                &index_loc,
                Expression::Variable(index_loc, coerced_ty, pos),
                &ns.storage_type(),
                false,
                ns,
            )
            .unwrap(),
            elem_ty,
            ns,
        )
    } else {
        match array_ty.deref_memory() {
            Type::Bytes(array_length) => {
                let res_ty = Type::Bytes(1);
                let from_ty = Type::Bytes(*array_length);

                Expression::Trunc(
                    *loc,
                    res_ty,
                    Box::new(Expression::ShiftRight(
                        *loc,
                        from_ty.clone(),
                        Box::new(array),
                        // shift by (array_length - 1 - index) * 8
                        Box::new(Expression::ShiftLeft(
                            *loc,
                            from_ty.clone(),
                            Box::new(Expression::Subtract(
                                *loc,
                                from_ty.clone(),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    from_ty.clone(),
                                    BigInt::from_u8(array_length - 1).unwrap(),
                                )),
                                Box::new(cast_shift_arg(
                                    loc,
                                    Expression::Variable(index_loc, coerced_ty, pos),
                                    index_width,
                                    &array_ty,
                                    ns,
                                )),
                            )),
                            Box::new(Expression::NumberLiteral(
                                *loc,
                                from_ty,
                                BigInt::from_u8(3).unwrap(),
                            )),
                        )),
                        false,
                    )),
                )
            }
            Type::Array(_, dim) if dim.last().unwrap().is_some() => Expression::ArraySubscript(
                *loc,
                array_ty.array_deref(),
                Box::new(array),
                Box::new(Expression::Variable(index_loc, coerced_ty, pos)),
            ),
            Type::DynamicBytes | Type::Array(_, _) => Expression::DynamicArraySubscript(
                *loc,
                array_ty.array_deref(),
                Box::new(array),
                Box::new(Expression::Variable(index_loc, coerced_ty, pos)),
            ),
            _ => {
                // should not happen as type-checking already done
                unreachable!();
            }
        }
    }
}

fn string_location(
    loc: &StringLocation,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> StringLocation {
    match loc {
        StringLocation::RunTime(s) => {
            StringLocation::RunTime(Box::new(expression(s, cfg, contract_no, ns, vartab)))
        }
        _ => loc.clone(),
    }
}
