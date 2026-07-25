// SPDX-License-Identifier: Apache-2.0

//! Codegen for arrays held in Soroban contract storage.
//!
//! A storage array is one host `VecObject`; array index `i` maps to vec index `i`.
//! Every operation is a read-modify-write on the stored vec handle: load the handle
//! from storage, call the matching host function, store the (possibly new) handle
//! back. Elements are always encoded/decoded with the STORAGE form
//! (`soroban_storage_encode_arg` / `soroban_storage_decode_arg`) so composite
//! elements (e.g. structs) are stored as VecObjects, not ABI ScMaps.

use super::encoding::soroban_storage_encode_arg;
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

/// `vec_push_back(vec, encoded_value) -> new_vec`. Encodes `value` in the storage
/// form and returns the new vec handle.
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

/// Storage `arr.push(value)` on Soroban: load the vec handle, push the encoded
/// element, store the new handle back to the same slot.
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
    // Storage wrapper: evaluate the slot and value once.
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, target);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt, target);
    let vec_ty = args[0].ty();

    // Load the raw vec handle (no type decode — mirrors the struct storage path,
    // which loads the raw handle and decodes explicitly only where needed).
    let old_vec_obj = load_raw_handle(loc, var_expr.clone(), cfg, vartab);
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
