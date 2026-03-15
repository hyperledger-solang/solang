// SPDX-License-Identifier: Apache-2.0

use super::cfg::{ControlFlowGraph, Instr, InternalCallTy};
use super::encoding::soroban_encoding::soroban_encode_arg;
use super::expression::{expression, load_storage};
use super::vartable::Vartable;
use super::Options;
use crate::codegen::{Expression, HostFunctions};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use solang_parser::pt;

fn soroban_vec_handle_ty(vec_ty: &Type) -> Type {
    let inner_ty = if let Type::StorageRef(_, inner) = vec_ty {
        inner.as_ref().clone()
    } else {
        vec_ty.clone()
    };

    Type::SorobanHandle(Box::new(inner_ty))
}

pub(super) fn soroban_vec_new(
    loc: &pt::Loc,
    vec_ty: &Type,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let handle_ty = soroban_vec_handle_ty(vec_ty);
    let empty_vec_no = vartab.temp_name("soroban_vec_new", &handle_ty);

    let empty_vec_var = Expression::Variable {
        loc: *loc,
        ty: handle_ty.clone(),
        var_no: empty_vec_no,
    };

    cfg.add(
        vartab,
        Instr::Call {
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VectorNew.name().to_string(),
            },
            args: vec![],
            return_tys: vec![handle_ty],
            res: vec![empty_vec_no],
        },
    );

    empty_vec_var
}

fn soroban_vec_push_back(
    loc: &pt::Loc,
    vec_obj: Expression,
    vec_ty: &Type,
    value: Expression,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    let value_encoded = soroban_encode_arg(value, cfg, vartab, ns);
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

pub(super) fn soroban_storage_push(
    loc: &pt::Loc,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {

    // Storage wrapper: evaluate storage key/value and load vec object from storage.
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_storage(loc, &vec_ty, var_expr.clone(), cfg, vartab, None, ns);
    let new_vec_var = soroban_vec_push_back(loc, old_vec_obj, &vec_ty, value, cfg, ns, vartab);

    // Storage wrapper: store updated vec object.
    let store_instr = Instr::SetStorage {
        ty: vec_ty,
        value: new_vec_var.clone(),
        storage: var_expr.clone(),
        storage_type: None,
    };

    cfg.add(vartab, store_instr);

    var_expr
}

pub(super) fn soroban_storage_pop(
    loc: &pt::Loc,
    args: &[ast::Expression],
    return_ty: &Type,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    // Storage wrapper: evaluate storage key and load vec object from storage.
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_storage(loc, &vec_ty, var_expr.clone(), cfg, vartab, None, ns);
    let new_vec_var = soroban_vec_pop_back(loc, old_vec_obj, &vec_ty, cfg, vartab);
    let new_vec_no = match &new_vec_var {
        Expression::Variable { var_no, .. } => *var_no,
        _ => unreachable!(),
    };

    // Storage wrapper: store updated vec object.
    let store_instr = Instr::SetStorage {
        ty: vec_ty,
        value: new_vec_var.clone(),
        storage: var_expr.clone(),
        storage_type: None,
    };

    cfg.add(vartab, store_instr);

    Expression::Variable {
        loc: *loc,
        ty: return_ty.clone(),
        var_no: new_vec_no,
    }
}
