use super::storage::{
    array_offset, array_pop, array_push, storage_slots_array_pop, storage_slots_array_push,
};
use super::Options;
use super::{
    cfg::{ControlFlowGraph, Instr, InternalCallTy},
    vartable::Vartable,
};
use crate::codegen::unused_variable::should_remove_assignment;
use crate::parser::pt;
use crate::parser::pt::CodeLocation;
use crate::sema::ast::{
    Builtin, CallTy, Expression, FormatArg, Function, Namespace, Parameter, StringLocation, Type,
};
use crate::sema::eval::{eval_const_number, eval_const_rational};
use crate::sema::expression::{bigint_to_expression, cast, cast_shift_arg, ResolveTo};
use crate::Target;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use std::ops::Mul;

pub fn expression(
    expr: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    match expr {
        Expression::StorageVariable(_, _, var_contract_no, var_no) => {
            // base storage variables should precede contract variables, not overlap
            ns.contracts[contract_no].get_storage_slot(*var_contract_no, *var_no, ns)
        }
        Expression::StorageLoad(loc, ty, expr) => {
            let storage = expression(expr, cfg, contract_no, func, ns, vartab, opt);

            load_storage(loc, ty, storage, cfg, vartab, opt)
        }
        Expression::Add(loc, ty, unchecked, left, right) => add(
            loc,
            ty,
            unchecked,
            left,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            right,
            opt,
        ),
        Expression::Subtract(loc, ty, unchecked, left, right) => substract(
            loc,
            ty,
            unchecked,
            left,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            right,
            opt,
        ),
        Expression::Multiply(loc, ty, unchecked, left, right) => {
            if ty.is_rational() {
                let (_, r) = eval_const_rational(expr, Some(contract_no), ns).unwrap();

                Expression::NumberLiteral(*loc, ty.clone(), r.to_integer())
            } else {
                Expression::Multiply(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
                    Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
                )
            }
        }
        Expression::Divide(loc, ty, left, right) => Expression::Divide(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Modulo(loc, ty, left, right) => Expression::Modulo(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Power(loc, ty, unchecked, left, right) => Expression::Power(
            *loc,
            ty.clone(),
            *unchecked,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::BitwiseOr(loc, ty, left, right) => Expression::BitwiseOr(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::BitwiseAnd(loc, ty, left, right) => Expression::BitwiseAnd(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::BitwiseXor(loc, ty, left, right) => Expression::BitwiseXor(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::ShiftLeft(loc, ty, left, right) => Expression::ShiftLeft(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::ShiftRight(loc, ty, left, right, sign) => Expression::ShiftRight(
            *loc,
            ty.clone(),
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
            *sign,
        ),
        Expression::Equal(loc, left, right) => Expression::Equal(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::NotEqual(loc, left, right) => Expression::NotEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::More(loc, left, right) => Expression::More(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::MoreEqual(loc, left, right) => Expression::MoreEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Less(loc, left, right) => Expression::Less(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::LessEqual(loc, left, right) => Expression::LessEqual(
            *loc,
            Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::ConstantVariable(_, _, Some(var_contract_no), var_no) => expression(
            ns.contracts[*var_contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap(),
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
        ),
        Expression::ConstantVariable(_, _, None, var_no) => expression(
            ns.constants[*var_no].initializer.as_ref().unwrap(),
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
        ),
        Expression::Not(loc, expr) => Expression::Not(
            *loc,
            Box::new(expression(expr, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Complement(loc, ty, expr) => Expression::Complement(
            *loc,
            ty.clone(),
            Box::new(expression(expr, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::UnaryMinus(loc, ty, expr) => Expression::UnaryMinus(
            *loc,
            ty.clone(),
            Box::new(expression(expr, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::StructLiteral(loc, ty, exprs) => Expression::StructLiteral(
            *loc,
            ty.clone(),
            exprs
                .iter()
                .map(|e| expression(e, cfg, contract_no, func, ns, vartab, opt))
                .collect(),
        ),
        Expression::ArrayLiteral(loc, ty, lengths, args) => Expression::ArrayLiteral(
            *loc,
            ty.clone(),
            lengths.clone(),
            args.iter()
                .map(|e| expression(e, cfg, contract_no, func, ns, vartab, opt))
                .collect(),
        ),
        Expression::ConstArrayLiteral(loc, ty, lengths, args) => Expression::ConstArrayLiteral(
            *loc,
            ty.clone(),
            lengths.clone(),
            args.iter()
                .map(|e| expression(e, cfg, contract_no, func, ns, vartab, opt))
                .collect(),
        ),
        Expression::Assign(_, _, left, right) => {
            // If we reach this condition, the assignment is inside an expression.
            if let Some(function) = func {
                if should_remove_assignment(ns, left, function, opt) {
                    return expression(right, cfg, contract_no, func, ns, vartab, opt);
                }
            }

            assign_single(left, right, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::PreDecrement(loc, ty, unchecked, var)
        | Expression::PreIncrement(loc, ty, unchecked, var) => pre_incdec(
            vartab,
            ty,
            var,
            cfg,
            contract_no,
            func,
            ns,
            loc,
            expr,
            unchecked,
            opt,
        ),
        Expression::PostDecrement(loc, ty, unchecked, var)
        | Expression::PostIncrement(loc, ty, unchecked, var) => post_incdec(
            vartab,
            ty,
            var,
            cfg,
            contract_no,
            func,
            ns,
            loc,
            expr,
            unchecked,
            opt,
        ),
        Expression::Constructor {
            loc,
            contract_no,
            constructor_no,
            args,
            value,
            gas,
            salt,
            space,
        } => {
            let address_res = vartab.temp_anonymous(&Type::Contract(*contract_no));

            let args = args
                .iter()
                .map(|v| expression(v, cfg, *contract_no, func, ns, vartab, opt))
                .collect();
            let gas = if let Some(gas) = gas {
                expression(gas, cfg, *contract_no, func, ns, vartab, opt)
            } else {
                default_gas(ns)
            };
            let value = value
                .as_ref()
                .map(|value| expression(value, cfg, *contract_no, func, ns, vartab, opt));
            let salt = salt
                .as_ref()
                .map(|salt| expression(salt, cfg, *contract_no, func, ns, vartab, opt));
            let space = space
                .as_ref()
                .map(|space| expression(space, cfg, *contract_no, func, ns, vartab, opt));

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
                    space,
                },
            );

            Expression::Variable(*loc, Type::Contract(*contract_no), address_res)
        }
        Expression::InternalFunction {
            function_no,
            signature,
            ..
        } => {
            let function_no = if let Some(signature) = signature {
                &ns.contracts[contract_no].virtual_functions[signature]
            } else {
                function_no
            };

            Expression::InternalFunctionCfg(ns.contracts[contract_no].all_functions[function_no])
        }
        Expression::StorageArrayLength {
            loc,
            ty,
            array,
            elem_ty,
        } => {
            let array_ty = array.ty().deref_into();
            let array = expression(array, cfg, contract_no, func, ns, vartab, opt);

            match array_ty {
                Type::Bytes(length) => bigint_to_expression(
                    loc,
                    &BigInt::from_u8(length).unwrap(),
                    ns,
                    &mut Vec::new(),
                    ResolveTo::Type(ty),
                )
                .unwrap(),
                Type::DynamicBytes => Expression::StorageArrayLength {
                    loc: *loc,
                    ty: ty.clone(),
                    array: Box::new(array),
                    elem_ty: elem_ty.clone(),
                },
                Type::Array(_, dim) => match dim.last().unwrap() {
                    None => {
                        if ns.target == Target::Solana {
                            Expression::StorageArrayLength {
                                loc: *loc,
                                ty: ty.clone(),
                                array: Box::new(array),
                                elem_ty: elem_ty.clone(),
                            }
                        } else {
                            load_storage(loc, &ns.storage_type(), array, cfg, vartab, opt)
                        }
                    }
                    Some(length) => {
                        bigint_to_expression(loc, length, ns, &mut Vec::new(), ResolveTo::Type(ty))
                            .unwrap()
                    }
                },
                _ => unreachable!(),
            }
        }
        Expression::Builtin(loc, returns, Builtin::ExternalFunctionAddress, func_expr) => {
            if let Expression::ExternalFunction { address, .. } = &func_expr[0] {
                expression(address, cfg, contract_no, func, ns, vartab, opt)
            } else {
                let func_expr = expression(&func_expr[0], cfg, contract_no, func, ns, vartab, opt);

                Expression::Builtin(
                    *loc,
                    returns.clone(),
                    Builtin::ExternalFunctionAddress,
                    vec![func_expr],
                )
            }
        }
        Expression::Builtin(loc, returns, Builtin::FunctionSelector, func_expr) => match &func_expr
            [0]
        {
            Expression::ExternalFunction { function_no, .. }
            | Expression::InternalFunction { function_no, .. } => {
                let selector = ns.functions[*function_no].selector();

                Expression::NumberLiteral(*loc, Type::Bytes(4), BigInt::from(selector))
            }
            _ => {
                let func_expr = expression(&func_expr[0], cfg, contract_no, func, ns, vartab, opt);

                Expression::Builtin(
                    *loc,
                    returns.clone(),
                    Builtin::FunctionSelector,
                    vec![func_expr],
                )
            }
        },
        Expression::InternalFunctionCall { .. }
        | Expression::ExternalFunctionCall { .. }
        | Expression::Builtin(_, _, Builtin::AbiDecode, _) => {
            let mut returns = emit_function_call(expr, contract_no, cfg, func, ns, vartab, opt);
            assert_eq!(returns.len(), 1);

            returns.remove(0)
        }
        Expression::ExternalFunction {
            loc,
            ty,
            address,
            function_no,
        } => {
            let address = expression(address, cfg, contract_no, func, ns, vartab, opt);

            Expression::ExternalFunction {
                loc: *loc,
                ty: ty.clone(),
                address: Box::new(address),
                function_no: *function_no,
            }
        }
        Expression::Subscript(loc, elem_ty, array_ty, array, index) => array_subscript(
            loc,
            elem_ty,
            array_ty,
            array,
            index,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
        ),
        Expression::StructMember(loc, ty, var, field_no) if ty.is_contract_storage() => {
            if let Type::Struct(struct_no) = var.ty().deref_any() {
                let offset = if ns.target == Target::Solana {
                    ns.structs[*struct_no].storage_offsets[*field_no].clone()
                } else {
                    ns.structs[*struct_no].fields[..*field_no]
                        .iter()
                        .map(|field| field.ty.storage_slots(ns))
                        .sum()
                };

                Expression::Add(
                    *loc,
                    ty.clone(),
                    true,
                    Box::new(expression(var, cfg, contract_no, func, ns, vartab, opt)),
                    Box::new(Expression::NumberLiteral(*loc, ns.storage_type(), offset)),
                )
            } else {
                unreachable!();
            }
        }
        Expression::StructMember(loc, ty, var, member) => Expression::StructMember(
            *loc,
            ty.clone(),
            Box::new(expression(var, cfg, contract_no, func, ns, vartab, opt)),
            *member,
        ),
        Expression::StringCompare(loc, left, right) => Expression::StringCompare(
            *loc,
            string_location(left, cfg, contract_no, func, ns, vartab, opt),
            string_location(right, cfg, contract_no, func, ns, vartab, opt),
        ),
        Expression::StringConcat(loc, ty, left, right) => Expression::StringConcat(
            *loc,
            ty.clone(),
            string_location(left, cfg, contract_no, func, ns, vartab, opt),
            string_location(right, cfg, contract_no, func, ns, vartab, opt),
        ),
        Expression::Or(loc, left, right) => {
            expr_or(left, cfg, contract_no, func, ns, vartab, loc, right, opt)
        }
        Expression::And(loc, left, right) => {
            and(left, cfg, contract_no, func, ns, vartab, loc, right, opt)
        }
        Expression::CheckingTrunc(loc, ty, e) => {
            checking_trunc(loc, e, ty, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::Trunc(loc, ty, e) => Expression::Trunc(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::ZeroExt(loc, ty, e) => Expression::ZeroExt(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::SignExt(loc, ty, e) => Expression::SignExt(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Cast(loc, ty @ Type::Address(_), e) => {
            if let Ok((_, address)) = eval_const_number(e, Some(contract_no), ns) {
                Expression::NumberLiteral(*loc, ty.clone(), address)
            } else {
                Expression::Cast(
                    *loc,
                    ty.clone(),
                    Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
                )
            }
        }
        Expression::Cast(loc, ty, e) => {
            if matches!(ty, Type::String | Type::DynamicBytes)
                && matches!(expr.ty(), Type::String | Type::DynamicBytes)
            {
                expression(e, cfg, contract_no, func, ns, vartab, opt)
            } else {
                Expression::Cast(
                    *loc,
                    ty.clone(),
                    Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
                )
            }
        }
        Expression::BytesCast(loc, ty, from, e) => Expression::BytesCast(
            *loc,
            ty.clone(),
            from.clone(),
            Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
        ),
        Expression::Load(loc, ty, e) => Expression::Load(
            *loc,
            ty.clone(),
            Box::new(expression(e, cfg, contract_no, func, ns, vartab, opt)),
        ),
        // for some built-ins, we have to inline special case code
        Expression::Builtin(loc, ty, Builtin::ArrayPush, args) => {
            if args[0].ty().is_contract_storage() {
                if ns.target == Target::Solana || args[0].ty().is_storage_bytes() {
                    array_push(loc, args, cfg, contract_no, func, ns, vartab, opt)
                } else {
                    storage_slots_array_push(loc, args, cfg, contract_no, func, ns, vartab, opt)
                }
            } else {
                memory_array_push(
                    &ty[0],
                    vartab,
                    &args[0],
                    cfg,
                    contract_no,
                    func,
                    ns,
                    &args[1],
                    loc,
                    opt,
                )
            }
        }
        Expression::Builtin(loc, ty, Builtin::ArrayPop, args) => {
            if args[0].ty().is_contract_storage() {
                if ns.target == Target::Solana || args[0].ty().is_storage_bytes() {
                    array_pop(loc, args, &ty[0], cfg, contract_no, func, ns, vartab, opt)
                } else {
                    storage_slots_array_pop(
                        loc,
                        args,
                        &ty[0],
                        cfg,
                        contract_no,
                        func,
                        ns,
                        vartab,
                        opt,
                    )
                }
            } else {
                let address_res = vartab.temp_anonymous(&ty[0]);

                let address_arr =
                    match expression(&args[0], cfg, contract_no, func, ns, vartab, opt) {
                        Expression::Variable(_, _, pos) => pos,
                        _ => unreachable!(),
                    };

                cfg.add(
                    vartab,
                    Instr::PopMemory {
                        res: address_res,
                        ty: args[0].ty(),
                        array: address_arr,
                    },
                );

                Expression::Variable(*loc, ty[0].clone(), address_res)
            }
        }
        Expression::Builtin(_, _, Builtin::Assert, args) => {
            expr_assert(cfg, &args[0], contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(_, _, Builtin::Print, args) => {
            let expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);

            cfg.add(vartab, Instr::Print { expr });

            Expression::Poison
        }
        Expression::Builtin(_, _, Builtin::Require, args) => {
            require(cfg, args, contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(_, _, Builtin::Revert, args) => {
            revert(args, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(_, _, Builtin::SelfDestruct, args) => {
            self_destruct(args, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(loc, _, Builtin::PayableSend, args) => {
            payable_send(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::Builtin(loc, _, Builtin::PayableTransfer, args) => {
            payable_transfer(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncode, args) => {
            abi_encode(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodePacked, args) => {
            abi_encode_packed(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodeWithSelector, args) => {
            abi_encode_with_selector(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodeWithSignature, args) => {
            abi_encode_with_signature(args, loc, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(loc, _, Builtin::AbiEncodeCall, args) => {
            abi_encode_call(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        // The Substrate gas price builtin takes an argument; the others do not
        Expression::Builtin(loc, _, Builtin::Gasprice, expr)
            if expr.len() == 1 && ns.target == Target::Ewasm =>
        {
            builtin_ewasm_gasprice(loc, expr, cfg, contract_no, func, ns, vartab, opt)
        }
        Expression::Builtin(loc, tys, builtin, args) => expr_builtin(
            args,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            loc,
            tys,
            builtin,
            opt,
        ),
        Expression::FormatString(loc, args) => {
            format_string(args, cfg, contract_no, func, ns, vartab, loc, opt)
        }
        Expression::AllocDynamicArray(loc, ty, size, init) => {
            alloc_dynamic_array(size, cfg, contract_no, func, ns, vartab, loc, ty, init, opt)
        }
        Expression::Ternary(loc, ty, cond, left, right) => ternary(
            loc,
            ty,
            cond,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            left,
            right,
            opt,
        ),
        Expression::InterfaceId(loc, contract_no) => interfaceid(ns, contract_no, loc),
        _ => expr.clone(),
    }
}

fn memory_array_push(
    ty: &Type,
    vartab: &mut Vartable,
    array: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    value: &Expression,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let address_res = vartab.temp_anonymous(ty);
    let address_arr = match expression(array, cfg, contract_no, func, ns, vartab, opt) {
        Expression::Variable(_, _, pos) => pos,
        _ => unreachable!(),
    };
    let value = expression(value, cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::PushMemory {
            res: address_res,
            ty: array.ty(),
            array: address_arr,
            value: Box::new(value),
        },
    );
    Expression::Variable(*loc, ty.clone(), address_res)
}

fn post_incdec(
    vartab: &mut Vartable,
    ty: &Type,
    var: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    loc: &pt::Loc,
    expr: &Expression,
    unchecked: &bool,
    opt: &Options,
) -> Expression {
    let res = vartab.temp_anonymous(ty);
    let v = expression(var, cfg, contract_no, func, ns, vartab, opt);
    let v = match var.ty() {
        Type::Ref(ty) => Expression::Load(var.loc(), ty.as_ref().clone(), Box::new(v)),
        Type::StorageRef(_, ty) => load_storage(&var.loc(), ty.as_ref(), v, cfg, vartab, opt),
        _ => v,
    };
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res,
            expr: v,
        },
    );
    let one = Box::new(Expression::NumberLiteral(*loc, ty.clone(), BigInt::one()));
    let expr = match expr {
        Expression::PostDecrement(..) => Expression::Subtract(
            *loc,
            ty.clone(),
            *unchecked,
            Box::new(Expression::Variable(*loc, ty.clone(), res)),
            one,
        ),
        Expression::PostIncrement(..) => Expression::Add(
            *loc,
            ty.clone(),
            *unchecked,
            Box::new(Expression::Variable(*loc, ty.clone(), res)),
            one,
        ),
        _ => unreachable!(),
    };
    match var {
        Expression::Variable(loc, _, pos) => {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr,
                },
            );
        }
        _ => {
            let dest = expression(var, cfg, contract_no, func, ns, vartab, opt);
            let res = vartab.temp_anonymous(ty);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: pt::Loc::Codegen,
                    res,
                    expr,
                },
            );

            match var.ty() {
                Type::StorageRef(..) => {
                    cfg.add(
                        vartab,
                        Instr::SetStorage {
                            value: Expression::Variable(*loc, ty.clone(), res),
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

fn pre_incdec(
    vartab: &mut Vartable,
    ty: &Type,
    var: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    loc: &pt::Loc,
    expr: &Expression,
    unchecked: &bool,
    opt: &Options,
) -> Expression {
    let res = vartab.temp_anonymous(ty);
    let v = expression(var, cfg, contract_no, func, ns, vartab, opt);
    let v = match var.ty() {
        Type::Ref(ty) => Expression::Load(var.loc(), ty.as_ref().clone(), Box::new(v)),
        Type::StorageRef(_, ty) => load_storage(&var.loc(), ty.as_ref(), v, cfg, vartab, opt),
        _ => v,
    };
    let one = Box::new(Expression::NumberLiteral(*loc, ty.clone(), BigInt::one()));
    let expr = match expr {
        Expression::PreDecrement(..) => {
            Expression::Subtract(*loc, ty.clone(), *unchecked, Box::new(v), one)
        }
        Expression::PreIncrement(..) => {
            Expression::Add(*loc, ty.clone(), *unchecked, Box::new(v), one)
        }
        _ => unreachable!(),
    };
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res,
            expr,
        },
    );
    match var {
        Expression::Variable(loc, _, pos) => {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr: Expression::Variable(*loc, ty.clone(), res),
                },
            );
        }
        _ => {
            let dest = expression(var, cfg, contract_no, func, ns, vartab, opt);

            match var.ty() {
                Type::StorageRef(..) => {
                    cfg.add(
                        vartab,
                        Instr::SetStorage {
                            value: Expression::Variable(*loc, ty.clone(), res),
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

fn expr_or(
    left: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    right: &Expression,
    opt: &Options,
) -> Expression {
    let boolty = Type::Bool;
    let l = expression(left, cfg, contract_no, func, ns, vartab, opt);
    vartab.new_dirty_tracker(ns.next_id);
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
            loc: pt::Loc::Codegen,
            res: pos,
            expr: Expression::BoolLiteral(*loc, true),
        },
    );
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: l,
            true_block: end_or,
            false_block: right_side,
        },
    );
    cfg.set_basic_block(right_side);
    let r = expression(right, cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: pos,
            expr: r,
        },
    );
    cfg.add(vartab, Instr::Branch { block: end_or });
    cfg.set_basic_block(end_or);
    let mut phis = vartab.pop_dirty_tracker();
    phis.insert(pos);
    cfg.set_phis(end_or, phis);
    Expression::Variable(*loc, boolty, pos)
}

fn and(
    left: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    right: &Expression,
    opt: &Options,
) -> Expression {
    let boolty = Type::Bool;
    let l = expression(left, cfg, contract_no, func, ns, vartab, opt);
    vartab.new_dirty_tracker(ns.next_id);
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
            loc: pt::Loc::Codegen,
            res: pos,
            expr: Expression::BoolLiteral(*loc, false),
        },
    );
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: l,
            true_block: right_side,
            false_block: end_and,
        },
    );
    cfg.set_basic_block(right_side);
    let r = expression(right, cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: pos,
            expr: r,
        },
    );
    cfg.add(vartab, Instr::Branch { block: end_and });
    cfg.set_basic_block(end_and);
    let mut phis = vartab.pop_dirty_tracker();
    phis.insert(pos);
    cfg.set_phis(end_and, phis);
    Expression::Variable(*loc, boolty, pos)
}

fn expr_assert(
    cfg: &mut ControlFlowGraph,
    args: &Expression,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());
    let cond = expression(args, cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: true_,
            false_block: false_,
        },
    );
    cfg.set_basic_block(false_);
    cfg.add(vartab, Instr::AssertFailure { expr: None });
    cfg.set_basic_block(true_);
    Expression::Poison
}

fn require(
    cfg: &mut ControlFlowGraph,
    args: &[Expression],
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());
    let cond = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: true_,
            false_block: false_,
        },
    );
    cfg.set_basic_block(false_);
    let expr = args
        .get(1)
        .map(|s| expression(s, cfg, contract_no, func, ns, vartab, opt));
    if ns.target == Target::Solana {
        // On Solana, print the reason, do not abi encoding it
        if let Some(expr) = expr {
            cfg.add(vartab, Instr::Print { expr });
        }
        cfg.add(vartab, Instr::AssertFailure { expr: None });
    } else {
        cfg.add(vartab, Instr::AssertFailure { expr });
    }
    cfg.set_basic_block(true_);
    Expression::Poison
}

fn revert(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let expr = args
        .get(0)
        .map(|s| expression(s, cfg, contract_no, func, ns, vartab, opt));
    cfg.add(vartab, Instr::AssertFailure { expr });
    Expression::Poison
}

fn self_destruct(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let recipient = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    cfg.add(vartab, Instr::SelfDestruct { recipient });
    Expression::Poison
}

fn payable_send(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let address = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt);
    let success = vartab.temp(
        &pt::Identifier {
            loc: *loc,
            name: "success".to_owned(),
        },
        &Type::Bool,
    );
    if ns.target != Target::Ewasm {
        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: Some(success),
                address,
                value,
            },
        );
    } else {
        // Ethereum can only transfer via external call
        cfg.add(
            vartab,
            Instr::ExternalCall {
                success: Some(success),
                address: Some(address),
                payload: Expression::AllocDynamicArray(
                    *loc,
                    Type::DynamicBytes,
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(32),
                        BigInt::from(0),
                    )),
                    Some(vec![]),
                ),
                value,
                gas: Expression::NumberLiteral(*loc, Type::Uint(64), BigInt::from(i64::MAX)),
                callty: CallTy::Regular,
            },
        );
    }
    Expression::Variable(*loc, Type::Bool, success)
}

fn payable_transfer(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let address = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt);
    if ns.target != Target::Ewasm {
        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: None,
                address,
                value,
            },
        );
    } else {
        // Ethereum can only transfer via external call
        cfg.add(
            vartab,
            Instr::ExternalCall {
                success: None,
                address: Some(address),
                payload: Expression::AllocDynamicArray(
                    *loc,
                    Type::DynamicBytes,
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(32),
                        BigInt::from(0),
                    )),
                    Some(vec![]),
                ),
                value,
                gas: Expression::NumberLiteral(*loc, Type::Uint(64), BigInt::from(i64::MAX)),
                callty: CallTy::Regular,
            },
        );
    }
    Expression::Poison
}

fn abi_encode(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let tys = args.iter().map(|a| a.ty()).collect();
    let args = args
        .iter()
        .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
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
        Instr::Set {
            loc: *loc,
            res,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed: vec![],
                args,
            },
        },
    );
    Expression::Variable(*loc, Type::DynamicBytes, res)
}

fn abi_encode_packed(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let tys = args.iter().map(|a| a.ty()).collect();
    let packed = args
        .iter()
        .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
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
        Instr::Set {
            loc: *loc,
            res,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed,
                args: vec![],
            },
        },
    );
    Expression::Variable(*loc, Type::DynamicBytes, res)
}

fn abi_encode_with_selector(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let mut tys: Vec<Type> = args.iter().skip(1).map(|a| a.ty()).collect();
    let mut args_iter = args.iter();
    let selector = expression(
        args_iter.next().unwrap(),
        cfg,
        contract_no,
        func,
        ns,
        vartab,
        opt,
    );
    let args = args_iter
        .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
        .collect();
    let res = vartab.temp(
        &pt::Identifier {
            loc: *loc,
            name: "encoded".to_owned(),
        },
        &Type::DynamicBytes,
    );
    tys.insert(0, Type::Bytes(4));
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed: vec![selector],
                args,
            },
        },
    );
    Expression::Variable(*loc, Type::DynamicBytes, res)
}

fn abi_encode_with_signature(
    args: &[Expression],
    loc: &pt::Loc,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let mut tys: Vec<Type> = args.iter().skip(1).map(|a| a.ty()).collect();
    let mut args_iter = args.iter();
    let hash = Expression::Builtin(
        *loc,
        vec![Type::Bytes(32)],
        Builtin::Keccak256,
        vec![args_iter.next().unwrap().clone()],
    );
    let hash = expression(&hash, cfg, contract_no, func, ns, vartab, opt);
    let selector = cast(loc, hash, &Type::Bytes(4), false, ns, &mut Vec::new()).unwrap();
    let args = args_iter
        .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
        .collect();
    let res = vartab.temp(
        &pt::Identifier {
            loc: *loc,
            name: "encoded".to_owned(),
        },
        &Type::DynamicBytes,
    );
    tys.insert(0, Type::Bytes(4));
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed: vec![selector],
                args,
            },
        },
    );
    Expression::Variable(*loc, Type::DynamicBytes, res)
}

fn abi_encode_call(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let mut tys: Vec<Type> = args.iter().skip(1).map(|a| a.ty()).collect();
    let mut args_iter = args.iter();
    let selector = expression(
        &Expression::Builtin(
            *loc,
            vec![Type::Bytes(4)],
            Builtin::FunctionSelector,
            vec![args_iter.next().unwrap().clone()],
        ),
        cfg,
        contract_no,
        func,
        ns,
        vartab,
        opt,
    );
    let args = args_iter
        .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
        .collect();
    let res = vartab.temp(
        &pt::Identifier {
            loc: *loc,
            name: "encoded".to_owned(),
        },
        &Type::DynamicBytes,
    );
    tys.insert(0, Type::Bytes(4));
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed: vec![selector],
                args,
            },
        },
    );
    Expression::Variable(*loc, Type::DynamicBytes, res)
}

fn builtin_ewasm_gasprice(
    loc: &pt::Loc,
    expr: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let ty = Type::Value;
    let gasprice = Expression::Builtin(*loc, vec![ty.clone()], Builtin::Gasprice, vec![]);
    let units = expression(&expr[0], cfg, contract_no, func, ns, vartab, opt);
    Expression::Multiply(*loc, ty, true, Box::new(units), Box::new(gasprice))
}

fn expr_builtin(
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    tys: &[Type],
    builtin: &Builtin,
    opt: &Options,
) -> Expression {
    match builtin {
        Builtin::WriteInt8
        | Builtin::WriteInt16LE
        | Builtin::WriteInt32LE
        | Builtin::WriteInt64LE
        | Builtin::WriteInt128LE
        | Builtin::WriteInt256LE
        | Builtin::WriteAddress
        | Builtin::WriteUint16LE
        | Builtin::WriteUint32LE
        | Builtin::WriteUint64LE
        | Builtin::WriteUint128LE
        | Builtin::WriteUint256LE => {
            let buf = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
            let offset = expression(&args[2], cfg, contract_no, func, ns, vartab, opt);

            // range check
            let cond = Expression::LessEqual(
                *loc,
                Box::new(Expression::Add(
                    *loc,
                    Type::Uint(32),
                    false,
                    Box::new(offset.clone()),
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(32),
                        BigInt::from(args[1].ty().bits(ns) / 8),
                    )),
                )),
                Box::new(Expression::Builtin(
                    *loc,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![buf.clone()],
                )),
            );

            let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
            let in_bounds = cfg.new_basic_block("in_bounds".to_string());

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond,
                    true_block: in_bounds,
                    false_block: out_of_bounds,
                },
            );

            cfg.set_basic_block(out_of_bounds);
            cfg.add(vartab, Instr::AssertFailure { expr: None });

            cfg.set_basic_block(in_bounds);

            let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt);
            cfg.add(vartab, Instr::WriteBuffer { buf, value, offset });

            Expression::Undefined(tys[0].clone())
        }
        Builtin::ReadInt8
        | Builtin::ReadInt16LE
        | Builtin::ReadInt32LE
        | Builtin::ReadInt64LE
        | Builtin::ReadInt128LE
        | Builtin::ReadInt256LE
        | Builtin::ReadAddress
        | Builtin::ReadUint16LE
        | Builtin::ReadUint32LE
        | Builtin::ReadUint64LE
        | Builtin::ReadUint128LE
        | Builtin::ReadUint256LE => {
            let buf = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
            let offset = expression(&args[1], cfg, contract_no, func, ns, vartab, opt);

            // range check
            let cond = Expression::LessEqual(
                *loc,
                Box::new(Expression::Add(
                    *loc,
                    Type::Uint(32),
                    false,
                    Box::new(offset.clone()),
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(32),
                        BigInt::from(tys[0].bits(ns) / 8),
                    )),
                )),
                Box::new(Expression::Builtin(
                    *loc,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![buf.clone()],
                )),
            );

            let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
            let in_bounds = cfg.new_basic_block("in_bounds".to_string());

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond,
                    true_block: in_bounds,
                    false_block: out_of_bounds,
                },
            );

            cfg.set_basic_block(out_of_bounds);
            cfg.add(vartab, Instr::AssertFailure { expr: None });

            cfg.set_basic_block(in_bounds);

            Expression::Builtin(*loc, tys.to_vec(), *builtin, vec![buf, offset])
        }
        _ => {
            let args = args
                .iter()
                .map(|v| expression(v, cfg, contract_no, func, ns, vartab, opt))
                .collect();

            Expression::Builtin(*loc, tys.to_vec(), *builtin, args)
        }
    }
}

fn checking_trunc(
    loc: &pt::Loc,
    expr: &Expression,
    ty: &Type,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let bits = match ty {
        Type::Uint(bits) => *bits as u32,
        Type::Value => (ns.value_length as u32 * 8),
        _ => unreachable!(),
    };

    let source_ty = expr.ty();

    let overflow = Expression::NumberLiteral(*loc, source_ty.clone(), BigInt::from(2u32).pow(bits));

    let pos = vartab.temp(
        &pt::Identifier {
            name: "value".to_owned(),
            loc: *loc,
        },
        &source_ty,
    );

    let expr = expression(expr, cfg, contract_no, func, ns, vartab, opt);

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: pos,
            expr,
        },
    );

    let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
    let in_bounds = cfg.new_basic_block("in_bounds".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: Expression::MoreEqual(
                *loc,
                Box::new(Expression::Variable(*loc, source_ty.clone(), pos)),
                Box::new(overflow),
            ),
            true_block: out_of_bounds,
            false_block: in_bounds,
        },
    );

    cfg.set_basic_block(out_of_bounds);
    cfg.add(vartab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(in_bounds);

    Expression::Trunc(
        *loc,
        ty.clone(),
        Box::new(Expression::Variable(*loc, source_ty, pos)),
    )
}

fn format_string(
    args: &[(FormatArg, Expression)],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    opt: &Options,
) -> Expression {
    let args = args
        .iter()
        .map(|(spec, arg)| {
            (
                *spec,
                expression(arg, cfg, contract_no, func, ns, vartab, opt),
            )
        })
        .collect();
    Expression::FormatString(*loc, args)
}

fn alloc_dynamic_array(
    size: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    loc: &pt::Loc,
    ty: &Type,
    init: &Option<Vec<u8>>,
    opt: &Options,
) -> Expression {
    let size = expression(size, cfg, contract_no, func, ns, vartab, opt);
    Expression::AllocDynamicArray(*loc, ty.clone(), Box::new(size), init.clone())
}

fn ternary(
    loc: &pt::Loc,
    ty: &Type,
    cond: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    left: &Expression,
    right: &Expression,
    opt: &Options,
) -> Expression {
    let cond = expression(cond, cfg, contract_no, func, ns, vartab, opt);

    vartab.new_dirty_tracker(ns.next_id);

    let pos = vartab.temp(
        &pt::Identifier {
            name: "ternary_result".to_owned(),
            loc: *loc,
        },
        ty,
    );

    let left_block = cfg.new_basic_block("left_value".to_string());
    let right_block = cfg.new_basic_block("right_value".to_string());
    let done_block = cfg.new_basic_block("ternary_done".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: left_block,
            false_block: right_block,
        },
    );

    cfg.set_basic_block(left_block);

    let expr = expression(left, cfg, contract_no, func, ns, vartab, opt);

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: pos,
            expr,
        },
    );

    cfg.add(vartab, Instr::Branch { block: done_block });

    cfg.set_basic_block(right_block);

    let expr = expression(right, cfg, contract_no, func, ns, vartab, opt);

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: pos,
            expr,
        },
    );

    cfg.add(vartab, Instr::Branch { block: done_block });

    cfg.set_basic_block(done_block);

    let mut phis = vartab.pop_dirty_tracker();
    phis.insert(pos);
    cfg.set_phis(done_block, phis);

    Expression::Variable(*loc, ty.clone(), pos)
}

fn interfaceid(ns: &Namespace, contract_no: &usize, loc: &pt::Loc) -> Expression {
    let mut id: u32 = 0;
    for func_no in &ns.contracts[*contract_no].functions {
        let func = &ns.functions[*func_no];

        if func.ty == pt::FunctionTy::Function {
            id ^= func.selector();
        }
    }
    Expression::NumberLiteral(*loc, Type::Bytes(4), BigInt::from(id))
}

fn add(
    loc: &pt::Loc,
    ty: &Type,
    unchecked: &bool,
    left: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    right: &Expression,
    opt: &Options,
) -> Expression {
    Expression::Add(
        *loc,
        ty.clone(),
        *unchecked,
        Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
        Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
    )
}

fn substract(
    loc: &pt::Loc,
    ty: &Type,
    unchecked: &bool,
    left: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    right: &Expression,
    opt: &Options,
) -> Expression {
    Expression::Subtract(
        *loc,
        ty.clone(),
        *unchecked,
        Box::new(expression(left, cfg, contract_no, func, ns, vartab, opt)),
        Box::new(expression(right, cfg, contract_no, func, ns, vartab, opt)),
    )
}

pub fn assign_single(
    left: &Expression,
    right: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    match left {
        Expression::Variable(loc, _, pos) => {
            let expr = expression(right, cfg, contract_no, func, ns, vartab, opt);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr,
                },
            );

            left.clone()
        }
        _ => {
            let left_ty = left.ty();
            let ty = left_ty.deref_memory();

            let pos = vartab.temp_anonymous(ty);

            // Set a subscript in storage bytes needs special handling
            let set_storage_bytes = if let Expression::Subscript(_, _, array_ty, _, _) = &left {
                array_ty.is_storage_bytes()
            } else {
                false
            };

            let dest = expression(left, cfg, contract_no, func, ns, vartab, opt);
            let right = expression(right, cfg, contract_no, func, ns, vartab, opt);

            cfg.add(
                vartab,
                Instr::Set {
                    loc: pt::Loc::Codegen,
                    res: pos,
                    expr: right,
                },
            );

            match left_ty {
                Type::StorageRef(..) if set_storage_bytes => {
                    if let Expression::Subscript(_, _, _, array, index) = dest {
                        // Set a byte in a byte array
                        cfg.add(
                            vartab,
                            Instr::SetStorageBytes {
                                value: Expression::Variable(left.loc(), ty.clone(), pos),
                                storage: *array,
                                offset: *index,
                            },
                        );
                    } else {
                        unreachable!();
                    }
                }
                Type::StorageRef(..) => {
                    cfg.add(
                        vartab,
                        Instr::SetStorage {
                            value: Expression::Variable(left.loc(), ty.clone(), pos),
                            ty: ty.deref_any().clone(),
                            storage: dest,
                        },
                    );
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
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Vec<Expression> {
    match expr {
        Expression::InternalFunctionCall { function, args, .. } => {
            if let Expression::InternalFunction {
                function_no,
                signature,
                ..
            } = function.as_ref()
            {
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, func, ns, vartab, opt))
                    .collect();

                let function_no = if let Some(signature) = signature {
                    &ns.contracts[callee_contract_no].virtual_functions[signature]
                } else {
                    function_no
                };

                let cfg_no = ns.contracts[callee_contract_no].all_functions[function_no];

                let ftype = &ns.functions[*function_no];

                if !ftype.returns.is_empty() {
                    let mut res = Vec::new();
                    let mut returns = Vec::new();
                    let mut return_tys = Vec::new();

                    for ret in &ftype.returns {
                        let id = pt::Identifier {
                            loc: ret.loc,
                            name: ret.name_as_str().to_owned(),
                        };

                        let temp_pos = vartab.temp(&id, &ret.ty);
                        return_tys.push(ret.ty.clone());
                        res.push(temp_pos);
                        returns.push(Expression::Variable(id.loc, ret.ty.clone(), temp_pos));
                    }

                    cfg.add(
                        vartab,
                        Instr::Call {
                            res,
                            call: InternalCallTy::Static(cfg_no),
                            args,
                            return_tys,
                        },
                    );

                    returns
                } else {
                    cfg.add(
                        vartab,
                        Instr::Call {
                            res: Vec::new(),
                            return_tys: Vec::new(),
                            call: InternalCallTy::Static(cfg_no),
                            args,
                        },
                    );

                    vec![Expression::Poison]
                }
            } else if let Type::InternalFunction { returns, .. } = function.ty().deref_any() {
                let cfg_expr = expression(function, cfg, callee_contract_no, func, ns, vartab, opt);

                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, func, ns, vartab, opt))
                    .collect();

                if !returns.is_empty() {
                    let mut res = Vec::new();
                    let mut return_values = Vec::new();
                    let mut return_tys = Vec::new();

                    for ty in returns {
                        let id = pt::Identifier {
                            loc: pt::Loc::Codegen,
                            name: String::new(),
                        };

                        let temp_pos = vartab.temp(&id, ty);
                        res.push(temp_pos);
                        return_tys.push(ty.clone());
                        return_values.push(Expression::Variable(id.loc, ty.clone(), temp_pos));
                    }

                    cfg.add(
                        vartab,
                        Instr::Call {
                            res,
                            call: InternalCallTy::Dynamic(cfg_expr),
                            return_tys,
                            args,
                        },
                    );

                    return_values
                } else {
                    cfg.add(
                        vartab,
                        Instr::Call {
                            res: Vec::new(),
                            return_tys: Vec::new(),
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
            let args = expression(args, cfg, callee_contract_no, func, ns, vartab, opt);
            let address = expression(address, cfg, callee_contract_no, func, ns, vartab, opt);
            let gas = if let Some(gas) = gas {
                expression(gas, cfg, callee_contract_no, func, ns, vartab, opt)
            } else {
                default_gas(ns)
            };
            let value = if let Some(value) = value {
                expression(value, cfg, callee_contract_no, func, ns, vartab, opt)
            } else {
                Expression::NumberLiteral(pt::Loc::Codegen, Type::Value, BigInt::zero())
            };

            let success = vartab.temp_name("success", &Type::Bool);

            let (payload, address) = if ns.target == Target::Solana {
                (
                    Expression::AbiEncode {
                        loc: *loc,
                        packed: vec![
                            address,
                            Expression::Builtin(
                                *loc,
                                vec![Type::Address(false)],
                                Builtin::GetAddress,
                                Vec::new(),
                            ),
                            value.clone(),
                            Expression::NumberLiteral(*loc, Type::Bytes(4), BigInt::zero()),
                            Expression::NumberLiteral(*loc, Type::Bytes(1), BigInt::zero()),
                            args,
                        ],
                        args: Vec::new(),
                        tys: vec![
                            Type::Address(false),
                            Type::Address(false),
                            Type::Uint(64),
                            Type::Bytes(4),
                            Type::Bytes(1),
                            Type::DynamicBytes,
                        ],
                    },
                    None,
                )
            } else {
                (args, Some(address))
            };

            cfg.add(
                vartab,
                Instr::ExternalCall {
                    success: Some(success),
                    address,
                    payload,
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
                function_no,
                address,
                ..
            } = function.as_ref()
            {
                let ftype = &ns.functions[*function_no];
                let mut tys: Vec<Type> = args.iter().map(|a| a.ty()).collect();
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, func, ns, vartab, opt))
                    .collect();
                let address = expression(address, cfg, callee_contract_no, func, ns, vartab, opt);
                let gas = if let Some(gas) = gas {
                    expression(gas, cfg, callee_contract_no, func, ns, vartab, opt)
                } else {
                    default_gas(ns)
                };
                let value = if let Some(value) = value {
                    expression(value, cfg, callee_contract_no, func, ns, vartab, opt)
                } else {
                    Expression::NumberLiteral(pt::Loc::Codegen, Type::Value, BigInt::zero())
                };

                let dest_func = &ns.functions[*function_no];

                tys.insert(0, Type::Bytes(4));

                let (payload, address) = if ns.target == Target::Solana {
                    tys.insert(0, Type::Address(false));
                    tys.insert(1, Type::Address(false));
                    tys.insert(2, Type::Uint(64));
                    tys.insert(3, Type::Bytes(4));
                    tys.insert(4, Type::Bytes(1));

                    (
                        Expression::AbiEncode {
                            loc: *loc,
                            tys,
                            packed: vec![
                                address,
                                Expression::Builtin(
                                    *loc,
                                    vec![Type::Address(false)],
                                    Builtin::GetAddress,
                                    Vec::new(),
                                ),
                                value.clone(),
                                Expression::NumberLiteral(*loc, Type::Bytes(4), BigInt::zero()),
                                Expression::NumberLiteral(*loc, Type::Bytes(1), BigInt::zero()),
                                Expression::NumberLiteral(
                                    *loc,
                                    Type::Bytes(4),
                                    BigInt::from(dest_func.selector()),
                                ),
                            ],
                            args,
                        },
                        None,
                    )
                } else {
                    (
                        Expression::AbiEncode {
                            loc: *loc,
                            tys,
                            packed: vec![Expression::NumberLiteral(
                                *loc,
                                Type::Bytes(4),
                                BigInt::from(dest_func.selector()),
                            )],
                            args,
                        },
                        Some(address),
                    )
                };

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: None,
                        address,
                        payload,
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
                            name: ret.name_as_str().to_owned(),
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
                            exception_block: None,
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
                let mut tys: Vec<Type> = args.iter().map(|a| a.ty()).collect();
                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, func, ns, vartab, opt))
                    .collect();
                let function = expression(function, cfg, callee_contract_no, func, ns, vartab, opt);
                let gas = if let Some(gas) = gas {
                    expression(gas, cfg, callee_contract_no, func, ns, vartab, opt)
                } else {
                    default_gas(ns)
                };
                let value = if let Some(value) = value {
                    expression(value, cfg, callee_contract_no, func, ns, vartab, opt)
                } else {
                    Expression::NumberLiteral(pt::Loc::Codegen, Type::Value, BigInt::zero())
                };

                let selector = Expression::Builtin(
                    *loc,
                    vec![Type::Bytes(4)],
                    Builtin::FunctionSelector,
                    vec![function.clone()],
                );
                let address = Expression::Builtin(
                    *loc,
                    vec![Type::Address(false)],
                    Builtin::ExternalFunctionAddress,
                    vec![function],
                );

                tys.insert(0, Type::Bytes(4));

                let payload = Expression::AbiEncode {
                    loc: *loc,
                    tys,
                    packed: vec![selector],
                    args,
                };

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: None,
                        address: Some(address),
                        payload,
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
                        returns.push(Expression::Variable(pt::Loc::Codegen, ty.clone(), temp_pos));

                        tys.push(Parameter {
                            loc: pt::Loc::Codegen,
                            ty,
                            ty_loc: pt::Loc::Codegen,
                            name: None,
                            indexed: false,
                            readonly: false,
                        });
                    }

                    cfg.add(
                        vartab,
                        Instr::AbiDecode {
                            res,
                            selector: None,
                            exception_block: None,
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
            let data = expression(&args[0], cfg, callee_contract_no, func, ns, vartab, opt);

            let mut returns = Vec::new();
            let mut res = Vec::new();

            for ret in tys {
                let temp_pos = vartab.temp_anonymous(ret);
                res.push(temp_pos);
                returns.push(Expression::Variable(*loc, ret.clone(), temp_pos));
            }

            cfg.add(
                vartab,
                Instr::AbiDecode {
                    res,
                    selector: None,
                    exception_block: None,
                    tys: tys
                        .iter()
                        .map(|ty| Parameter {
                            name: None,
                            loc: *loc,
                            ty: ty.clone(),
                            ty_loc: *loc,
                            indexed: false,
                            readonly: false,
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

pub fn default_gas(ns: &Namespace) -> Expression {
    Expression::NumberLiteral(
        pt::Loc::Codegen,
        Type::Uint(64),
        // See EIP150
        if ns.target == Target::Ewasm {
            BigInt::from(i64::MAX)
        } else {
            BigInt::zero()
        },
    )
}

/// Codegen for an array subscript expression
fn array_subscript(
    loc: &pt::Loc,
    elem_ty: &Type,
    array_ty: &Type,
    array: &Expression,
    index: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    if array_ty.is_storage_bytes() {
        return Expression::Subscript(
            *loc,
            elem_ty.clone(),
            array_ty.clone(),
            Box::new(expression(array, cfg, contract_no, func, ns, vartab, opt)),
            Box::new(expression(index, cfg, contract_no, func, ns, vartab, opt)),
        );
    }

    if array_ty.is_mapping() {
        let array = expression(array, cfg, contract_no, func, ns, vartab, opt);
        let index = expression(index, cfg, contract_no, func, ns, vartab, opt);

        return if ns.target == Target::Solana {
            Expression::Subscript(
                *loc,
                elem_ty.clone(),
                array_ty.clone(),
                Box::new(array),
                Box::new(index),
            )
        } else {
            Expression::Keccak256(*loc, array_ty.clone(), vec![array, index])
        };
    }

    let mut array = expression(array, cfg, contract_no, func, ns, vartab, opt);
    let index_ty = index.ty();
    let index = expression(index, cfg, contract_no, func, ns, vartab, opt);
    let index_loc = index.loc();

    let index_width = index_ty.bits(ns);

    let array_length = match array_ty.deref_any() {
        Type::Bytes(n) => bigint_to_expression(
            &array.loc(),
            &BigInt::from(*n),
            ns,
            &mut Vec::new(),
            ResolveTo::Unknown,
        )
        .unwrap(),
        Type::Array(..) => match array_ty.array_length() {
            None => {
                if let Type::StorageRef(..) = array_ty {
                    if ns.target == Target::Solana {
                        Expression::StorageArrayLength {
                            loc: *loc,
                            ty: ns.storage_type(),
                            array: Box::new(array.clone()),
                            elem_ty: array_ty.storage_array_elem().deref_into(),
                        }
                    } else {
                        let array_length =
                            load_storage(loc, &Type::Uint(256), array.clone(), cfg, vartab, opt);

                        array = Expression::Keccak256(*loc, Type::Uint(256), vec![array]);

                        array_length
                    }
                } else {
                    Expression::Builtin(
                        *loc,
                        vec![Type::Uint(32)],
                        Builtin::ArrayLength,
                        vec![array.clone()],
                    )
                }
            }
            Some(l) => {
                bigint_to_expression(loc, l, ns, &mut Vec::new(), ResolveTo::Unknown).unwrap()
            }
        },
        Type::DynamicBytes => Expression::Builtin(
            *loc,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![array.clone()],
        ),
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
            loc: pt::Loc::Codegen,
            res: pos,
            expr: cast(&index.loc(), index, &coerced_ty, false, ns, &mut Vec::new()).unwrap(),
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
                    cast(
                        &array.loc(),
                        array_length.clone(),
                        &coerced_ty,
                        false,
                        ns,
                        &mut Vec::new(),
                    )
                    .unwrap(),
                ),
            ),
            true_block: out_of_bounds,
            false_block: in_bounds,
        },
    );

    cfg.set_basic_block(out_of_bounds);
    cfg.add(vartab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(in_bounds);

    if let Type::StorageRef(_, ty) = &array_ty {
        let elem_ty = ty.storage_array_elem();
        let slot_ty = ns.storage_type();

        if ns.target == Target::Solana {
            if ty.array_length().is_some() && ty.is_sparse_solana(ns) {
                let index = cast(
                    &index_loc,
                    Expression::Variable(index_loc, coerced_ty, pos),
                    &Type::Uint(256),
                    false,
                    ns,
                    &mut Vec::new(),
                )
                .unwrap();

                Expression::Subscript(
                    *loc,
                    elem_ty,
                    array_ty.clone(),
                    Box::new(array),
                    Box::new(index),
                )
            } else {
                let index = cast(
                    &index_loc,
                    Expression::Variable(index_loc, coerced_ty, pos),
                    &slot_ty,
                    false,
                    ns,
                    &mut Vec::new(),
                )
                .unwrap();

                if ty.array_length().is_some() {
                    // fixed length array
                    let elem_size = elem_ty.deref_any().size_of(ns);

                    Expression::Add(
                        *loc,
                        elem_ty,
                        true,
                        Box::new(array),
                        Box::new(Expression::Multiply(
                            *loc,
                            slot_ty.clone(),
                            true,
                            Box::new(index),
                            Box::new(Expression::NumberLiteral(*loc, slot_ty, elem_size)),
                        )),
                    )
                } else {
                    Expression::Subscript(
                        *loc,
                        elem_ty,
                        array_ty.clone(),
                        Box::new(array),
                        Box::new(index),
                    )
                }
            }
        } else {
            let elem_size = elem_ty.storage_slots(ns);

            if let Ok(array_length) = eval_const_number(&array_length, Some(contract_no), ns) {
                if array_length.1.mul(elem_size.clone()).to_u64().is_some() {
                    // we need to calculate the storage offset. If this can be done with 64 bit
                    // arithmetic it will be much more efficient on wasm
                    return Expression::Add(
                        *loc,
                        elem_ty,
                        true,
                        Box::new(array),
                        Box::new(Expression::ZeroExt(
                            *loc,
                            slot_ty,
                            Box::new(Expression::Multiply(
                                *loc,
                                Type::Uint(64),
                                true,
                                Box::new(
                                    cast(
                                        &index_loc,
                                        Expression::Variable(index_loc, coerced_ty, pos),
                                        &Type::Uint(64),
                                        false,
                                        ns,
                                        &mut Vec::new(),
                                    )
                                    .unwrap(),
                                ),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    Type::Uint(64),
                                    elem_size,
                                )),
                            )),
                        )),
                    );
                }
            }

            array_offset(
                loc,
                array,
                cast(
                    &index_loc,
                    Expression::Variable(index_loc, coerced_ty, pos),
                    &ns.storage_type(),
                    false,
                    ns,
                    &mut Vec::new(),
                )
                .unwrap(),
                elem_ty,
                ns,
            )
        }
    } else {
        match array_ty.deref_memory() {
            Type::Bytes(array_length) => {
                let res_ty = Type::Bytes(1);
                let from_ty = Type::Bytes(*array_length);
                let index_ty = Type::Uint(*array_length as u16 * 8);

                Expression::Trunc(
                    *loc,
                    res_ty,
                    Box::new(Expression::ShiftRight(
                        *loc,
                        from_ty,
                        Box::new(array),
                        // shift by (array_length - 1 - index) * 8
                        Box::new(Expression::ShiftLeft(
                            *loc,
                            index_ty.clone(),
                            Box::new(Expression::Subtract(
                                *loc,
                                index_ty.clone(),
                                true,
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    index_ty.clone(),
                                    BigInt::from_u8(array_length - 1).unwrap(),
                                )),
                                Box::new(cast_shift_arg(
                                    loc,
                                    Expression::Variable(index_loc, coerced_ty, pos),
                                    index_width,
                                    array_ty,
                                    ns,
                                )),
                            )),
                            Box::new(Expression::NumberLiteral(
                                *loc,
                                index_ty,
                                BigInt::from_u8(3).unwrap(),
                            )),
                        )),
                        false,
                    )),
                )
            }
            Type::Array(_, dim) if dim.last().unwrap().is_some() => Expression::Subscript(
                *loc,
                elem_ty.clone(),
                array_ty.clone(),
                Box::new(array),
                Box::new(Expression::Variable(index_loc, coerced_ty, pos)),
            ),
            Type::DynamicBytes | Type::Array(..) => Expression::Subscript(
                *loc,
                elem_ty.clone(),
                array_ty.clone(),
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
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> StringLocation {
    match loc {
        StringLocation::RunTime(s) => StringLocation::RunTime(Box::new(expression(
            s,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
        ))),
        _ => loc.clone(),
    }
}

// Generate a load from storage instruction
pub fn load_storage(
    loc: &pt::Loc,
    ty: &Type,
    storage: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    _opt: &Options,
) -> Expression {
    let res = vartab.temp_anonymous(ty);

    cfg.add(
        vartab,
        Instr::LoadStorage {
            res,
            ty: ty.clone(),
            storage,
        },
    );

    Expression::Variable(*loc, ty.clone(), res)
}
