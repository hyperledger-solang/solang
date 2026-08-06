// SPDX-License-Identifier: Apache-2.0

use super::encoding::soroban_encode_arg;
use super::soroban_field_index_val;
use crate::codegen::cfg::{ControlFlowGraph, Instr, InternalCallTy};
use crate::codegen::expression::expression;
use crate::codegen::interface::TargetCodegen;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::codegen::{Expression, HostFunctions};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use solang_parser::pt;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Idx {
    Field(usize),
    Array(Box<Expression>),
}

#[derive(Debug, Clone)]
pub(crate) struct Loc {
    pub root_key: Expression,
    pub idxs: Vec<Idx>,
}

pub(crate) fn is_array_descent(array_ty: &Type) -> bool {
    match array_ty {
        Type::StorageRef(_, inner) => matches!(inner.as_ref(), Type::Array(..) | Type::Slice(_)),
        _ => false,
    }
}

pub(crate) fn is_descent_storage_expr(e: &ast::Expression) -> bool {
    match e {
        ast::Expression::StructMember { ty, .. } => ty.is_contract_storage(),
        ast::Expression::Subscript { array_ty, .. } => is_array_descent(array_ty),
        _ => false,
    }
}

pub(crate) fn root_storage_type(expr: &ast::Expression, ns: &Namespace) -> Option<pt::StorageType> {
    match expr {
        ast::Expression::StorageVariable {
            var_no,
            contract_no,
            ..
        } => ns.contracts[*contract_no]
            .variables
            .get(*var_no)
            .and_then(|v| v.storage_type.clone()),
        ast::Expression::StructMember { expr: inner, .. }
        | ast::Expression::Subscript { array: inner, .. } => root_storage_type(inner, ns),
        _ => None,
    }
}

pub(crate) fn lower_storage_lvalue(
    left: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    target: &dyn TargetCodegen,
) -> Expression {
    match left {
        ast::Expression::StructMember {
            loc,
            ty,
            expr: var,
            field,
        } if ty.is_contract_storage() => {
            let inner = lower_storage_lvalue(var, cfg, contract_no, func, ns, vartab, opt, target);
            Expression::StructMember {
                loc: *loc,
                ty: ty.clone(),
                expr: Box::new(inner),
                member: *field,
            }
        }
        ast::Expression::Subscript {
            loc,
            ty,
            array_ty,
            array,
            index,
        } if is_array_descent(array_ty) => {
            let inner =
                lower_storage_lvalue(array, cfg, contract_no, func, ns, vartab, opt, target);
            let idx = expression(index, cfg, contract_no, func, ns, vartab, opt, target)
                .cast(&Type::Uint(64), ns);
            Expression::Subscript {
                loc: *loc,
                ty: ty.clone(),
                array_ty: array_ty.clone(),
                expr: Box::new(inner),
                index: Box::new(idx),
            }
        }
        _ => expression(left, cfg, contract_no, func, ns, vartab, opt, target),
    }
}

pub(crate) fn lower_storage_path(
    container: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    target: &dyn TargetCodegen,
) -> (Expression, Loc, Option<pt::StorageType>) {
    let storage_type = root_storage_type(container, ns);
    let dest = lower_storage_lvalue(container, cfg, contract_no, func, ns, vartab, opt, target);
    let mut path = peel(&dest);
    hoist_indices(&mut path, cfg, vartab);
    (dest, path, storage_type)
}

pub(crate) fn peel(expr: &Expression) -> Loc {
    let mut idxs_rev: Vec<Idx> = Vec::new();
    let mut cur = expr;

    loop {
        match cur {
            Expression::StructMember {
                expr: inner,
                member,
                ty,
                ..
            } if ty.is_contract_storage() => {
                idxs_rev.push(Idx::Field(*member));
                cur = inner;
            }
            Expression::Subscript {
                expr: inner,
                index,
                array_ty,
                ..
            } if is_array_descent(array_ty) => {
                idxs_rev.push(Idx::Array(index.clone()));
                cur = inner;
            }
            _ => break,
        }
    }
    idxs_rev.reverse();
    Loc {
        root_key: cur.clone(),
        idxs: idxs_rev,
    }
}

pub(crate) fn hoist_indices(loc: &mut Loc, cfg: &mut ControlFlowGraph, vartab: &mut Vartable) {
    for idx in &mut loc.idxs {
        let Idx::Array(expr) = idx else { continue };
        if matches!(
            **expr,
            Expression::Variable { .. } | Expression::NumberLiteral { .. }
        ) {
            continue;
        }
        let ty = expr.ty();
        let var_no = vartab.temp_name("storage_idx", &ty);
        cfg.add(
            vartab,
            Instr::Set {
                loc: pt::Loc::Codegen,
                res: var_no,
                expr: (**expr).clone(),
            },
        );
        **expr = Expression::Variable {
            loc: pt::Loc::Codegen,
            ty,
            var_no,
        };
    }
}

pub(crate) fn path_load(
    loc: &Loc,
    storage_type: &Option<pt::StorageType>,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    let ploc = pt::Loc::Codegen;
    let mut handle = load_root(&ploc, loc.root_key.clone(), storage_type, cfg, vartab);
    for idx in &loc.idxs {
        let idx_val = encode_index(&ploc, idx, cfg, vartab, ns);
        handle = vec_get(&ploc, handle, idx_val, cfg, vartab);
    }
    handle
}

fn load_root(
    loc: &pt::Loc,
    root_key: Expression,
    storage_type: &Option<pt::StorageType>,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let handle_no = vartab.temp_name("storage_handle", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::LoadStorage {
            res: handle_no,
            ty: Type::Uint(64),
            storage: root_key,
            storage_type: storage_type.clone(),
        },
    );
    Expression::Variable {
        loc: *loc,
        ty: Type::Uint(64),
        var_no: handle_no,
    }
}

fn encode_index(
    loc: &pt::Loc,
    idx: &Idx,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    match idx {
        Idx::Field(field_no) => soroban_field_index_val(loc, *field_no, cfg, vartab, ns),
        Idx::Array(index) => {
            soroban_encode_arg((**index).clone().cast(&Type::Uint(32), ns), cfg, vartab, ns)
        }
    }
}

fn vec_get(
    loc: &pt::Loc,
    handle: Expression,
    idx_val: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let elem_no = vartab.temp_name("path_vec_get", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![elem_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VecGet.name().to_string(),
            },
            args: vec![handle, idx_val],
        },
    );
    Expression::Variable {
        loc: *loc,
        ty: Type::Uint(64),
        var_no: elem_no,
    }
}

fn vec_put(
    loc: &pt::Loc,
    handle: Expression,
    idx_val: Expression,
    value: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let new_no = vartab.temp_name("path_vec_put", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![new_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VecPut.name().to_string(),
            },
            args: vec![handle, idx_val, value],
        },
    );
    Expression::Variable {
        loc: *loc,
        ty: Type::Uint(64),
        var_no: new_no,
    }
}

pub(crate) fn path_store(
    loc: &Loc,
    value: Expression,
    storage_type: &Option<pt::StorageType>,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) {
    let ploc = pt::Loc::Codegen;
    let n = loc.idxs.len();

    let mut new_root = value;
    if n > 0 {
        let encoded: Vec<Expression> = loc
            .idxs
            .iter()
            .map(|idx| encode_index(&ploc, idx, cfg, vartab, ns))
            .collect();

        let mut handles = Vec::with_capacity(n);
        handles.push(load_root(
            &ploc,
            loc.root_key.clone(),
            storage_type,
            cfg,
            vartab,
        ));
        for k in 1..n {
            let h = vec_get(
                &ploc,
                handles[k - 1].clone(),
                encoded[k - 1].clone(),
                cfg,
                vartab,
            );
            handles.push(h);
        }

        for k in (0..n).rev() {
            new_root = vec_put(
                &ploc,
                handles[k].clone(),
                encoded[k].clone(),
                new_root,
                cfg,
                vartab,
            );
        }
    }

    cfg.add(
        vartab,
        Instr::SetStorage {
            ty: Type::Uint(64),
            value: new_root,
            storage: loc.root_key.clone(),
            storage_type: storage_type.clone(),
        },
    );
}
