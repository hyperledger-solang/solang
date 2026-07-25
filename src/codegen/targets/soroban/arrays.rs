// SPDX-License-Identifier: Apache-2.0

use super::encoding::{soroban_decode_arg, soroban_storage_encode_arg};
use super::{load_raw_handle, soroban_vec_handle_ty};
use crate::codegen::cfg::{ControlFlowGraph, Instr, InternalCallTy};
use crate::codegen::expression::expression;
use crate::codegen::interface::TargetCodegen;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::codegen::{Expression, HostFunctions};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use solang_parser::pt;

fn soroban_vec_push_back(
    loc: &pt::Loc,
    vec_obj: Expression,
    vec_ty: &Type,
    value: Expression,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    let value_encoded = soroban_storage_encode_arg(value, cfg, vartab, ns);
    let handle_ty = soroban_vec_handle_ty(vec_ty);

    let new_vec_no = vartab.temp_name("soroban_vec_push", &handle_ty);

    let new_vec_var = Expression::Variable {
        loc: *loc,
        ty: handle_ty.clone(),
        var_no: new_vec_no,
    };

    let instr = Instr::Call {
        res: vec![new_vec_no],
        return_tys: vec![handle_ty],
        call: InternalCallTy::HostFunction {
            name: HostFunctions::VecPushBack.name().to_string(),
        },
        args: vec![vec_obj, value_encoded],
    };
    cfg.add(vartab, instr);
    new_vec_var
}

pub(crate) fn soroban_storage_push(
    loc: &pt::Loc,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    target: &dyn TargetCodegen,
) -> Expression {
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, target);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt, target);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_raw_handle(loc, var_expr.clone(), cfg, vartab);
    let new_vec_var = soroban_vec_push_back(loc, old_vec_obj, &vec_ty, value, cfg, ns, vartab);

    let store_instr = Instr::SetStorage {
        ty: vec_ty,
        value: new_vec_var.clone(),
        storage: var_expr.clone(),
        storage_type: None,
    };

    cfg.add(vartab, store_instr);

    var_expr
}

fn soroban_vec_pop_back(
    loc: &pt::Loc,
    vec_obj: Expression,
    vec_ty: &Type,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let handle_ty = soroban_vec_handle_ty(vec_ty);
    let new_vec_no = vartab.temp_name("soroban_vec_pop", &handle_ty);
    let new_vec_var = Expression::Variable {
        loc: *loc,
        ty: handle_ty.clone(),
        var_no: new_vec_no,
    };
    let instr = Instr::Call {
        res: vec![new_vec_no],
        return_tys: vec![handle_ty],
        call: InternalCallTy::HostFunction {
            name: HostFunctions::VecPopBack.name().to_string(),
        },
        args: vec![vec_obj],
    };
    cfg.add(vartab, instr);
    new_vec_var
}

pub(crate) fn soroban_storage_pop(
    loc: &pt::Loc,
    args: &[ast::Expression],
    return_ty: &Type,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    target: &dyn TargetCodegen,
) -> Expression {
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, target);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_raw_handle(loc, var_expr.clone(), cfg, vartab);
    let new_vec_var = soroban_vec_pop_back(loc, old_vec_obj, &vec_ty, cfg, vartab);

    let store_instr = Instr::SetStorage {
        ty: vec_ty,
        value: new_vec_var,
        storage: var_expr,
        storage_type: None,
    };
    cfg.add(vartab, store_instr);
    Expression::Undefined {
        ty: return_ty.clone(),
    }
}

pub(crate) fn soroban_storage_array_length(
    loc: &pt::Loc,
    ty: &Type,
    array: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    let vec_obj = load_raw_handle(loc, array, cfg, vartab);
    let len_no = vartab.temp_name("soroban_vec_len", &Type::Uint(64));
    let len_var = Expression::Variable {
        loc: *loc,
        ty: Type::Uint(64),
        var_no: len_no,
    };
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![len_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VecLen.name().to_string(),
            },
            args: vec![vec_obj],
        },
    );

    let len_u32 = soroban_decode_arg(len_var, cfg, vartab, ns, Some(Type::Uint(32)));
    len_u32.cast(ty, ns)
}
