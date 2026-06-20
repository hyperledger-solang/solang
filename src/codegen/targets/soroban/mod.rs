// SPDX-License-Identifier: Apache-2.0

pub(crate) mod dispatch;
pub(crate) mod encoding;
pub(crate) mod events;

use self::encoding::{soroban_decode, soroban_decode_arg, soroban_encode, soroban_encode_arg};
use self::events::SorobanEventEmitter;
use crate::codegen::cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy};
use crate::codegen::error::CodegenError;
use crate::codegen::expression::{expression, load_storage};
use crate::codegen::interface::{EventEmitter, TargetCodegen};
use crate::codegen::storage::{array_pop, storage_slots_array_push};
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::codegen::{Builtin, Expression, HostFunctions};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, StructType, Type};
use crate::sema::Recurse;
use crate::Target;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use solang_parser::helpers::CodeLocation;
use solang_parser::{diagnostics::Diagnostic, pt};
use std::collections::BTreeSet;

/// Codegen for the Soroban target. All Soroban-specific lowering lives under this module
/// (`dispatch`, `encoding`, `events`, plus the validation and storage helpers below).
pub(crate) struct SorobanTarget;

impl TargetCodegen for SorobanTarget {
    fn validate_contract(&self, contract_no: usize, ns: &mut Namespace) {
        validate_accessor_abi_types(contract_no, ns);
        if ns.diagnostics.any_errors() {
            return;
        }
        validate_event_abi_types(contract_no, ns);
    }

    fn validate_cfgs(&self, all_cfg: &[ControlFlowGraph], ns: &mut Namespace) {
        validate_abi_types(all_cfg, ns);
    }

    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        dispatch::function_dispatch(contract_no, all_cfg, ns, opt)
    }

    fn storage_array_length_is_inline(&self) -> bool {
        true
    }

    /// Soroban lazy decode path: if memory contains encoded handles, decode on demand.
    fn lower_load(
        &self,
        load: Expression,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        if let Expression::Load { ref expr, .. } = load {
            if let Type::Ref(inner) = expr.ty() {
                if matches!(inner.as_ref(), Type::SorobanHandle(_)) {
                    return soroban_decode_arg(load, cfg, vartab, ns, None);
                }
            }
        }
        load
    }

    fn prepare_storage_value(
        &self,
        value: Expression,
        dest: &Expression,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        // For Store to a non-SorobanHandle Ref, pass the value through unchanged.
        if let Type::Ref(inner) = dest.ty() {
            if !matches!(inner.as_ref(), Type::SorobanHandle(_)) {
                return value;
            }
        }
        // SetStorage or Store to a SorobanHandle: encode as ScVal.
        soroban_encode_arg(value, cfg, vartab, ns)
    }

    fn default_storage_value(
        &self,
        loc: &pt::Loc,
        ty: &Type,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Option<Expression> {
        match ty {
            Type::String | Type::DynamicBytes | Type::Slice(_) => {
                Some(soroban_vec_new(loc, ty, cfg, vartab))
            }
            Type::Array(elem_ty, dims)
                if dims.last() == Some(&ast::ArrayLength::Dynamic)
                    && !elem_ty.is_reference_type(ns) =>
            {
                Some(soroban_vec_new(loc, ty, cfg, vartab))
            }
            _ => None,
        }
    }

    fn abi_encode(
        &self,
        loc: &pt::Loc,
        args: Vec<Expression>,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        packed: bool,
    ) -> (Expression, Expression) {
        // Soroban encodes to ScVal handles; soroban_encode returns a 3-tuple, drop the spread.
        let (buffer, size, _) = soroban_encode(loc, args, ns, vartab, cfg, packed);
        (buffer, size)
    }

    fn abi_decode(
        &self,
        loc: &pt::Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size_expr: Option<Expression>,
    ) -> Vec<Expression> {
        soroban_decode(loc, buffer, types, ns, vartab, cfg, buffer_size_expr)
    }

    fn storage_array_push(
        &self,
        loc: &pt::Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        // Arrays whose elements are reference types use the shared hashed-slots path (the
        // entry offset and value encoding are routed back through this target); everything
        // else (scalars, `bytes`) goes through the dedicated host-vector push.
        let elem_is_ref = !args[0].ty().is_storage_bytes()
            && matches!(
                args[0].ty(),
                Type::StorageRef(_, inner)
                    if matches!(inner.deref_any(), Type::Array(elem_ty, _)
                        if elem_ty.is_reference_type(ns))
            );
        if elem_is_ref {
            storage_slots_array_push(loc, args, cfg, contract_no, func, ns, vartab, opt, self)
        } else {
            soroban_storage_push(loc, args, cfg, contract_no, func, ns, vartab, opt, self)
        }
    }

    fn storage_array_pop(
        &self,
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
        if args[0].ty().is_storage_bytes() {
            array_pop(
                loc,
                args,
                return_ty,
                cfg,
                contract_no,
                func,
                ns,
                vartab,
                opt,
                self,
            )
        } else {
            soroban_storage_pop(
                loc,
                args,
                return_ty,
                cfg,
                contract_no,
                func,
                ns,
                vartab,
                opt,
                self,
            )
        }
    }

    fn storage_array_entry_offset(
        &self,
        loc: &pt::Loc,
        var_expr: &Expression,
        index: Expression,
        elem_ty: &Type,
        _slot_ty: &Type,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        // Soroban indexes its host vector by an encoded key rather than a hashed slot.
        let index_encoded = soroban_encode_arg(index, cfg, vartab, ns);
        Expression::Subscript {
            loc: *loc,
            ty: elem_ty.clone(),
            array_ty: Type::StorageRef(false, Box::new(elem_ty.clone())),
            expr: Box::new(var_expr.clone()),
            index: Box::new(index_encoded),
        }
    }

    fn event_emitter<'a>(
        &self,
        _loc: &pt::Loc,
        event_no: usize,
        args: &'a [ast::Expression],
        ns: &'a Namespace,
    ) -> Box<dyn EventEmitter + 'a> {
        Box::new(SorobanEventEmitter { args, ns, event_no })
    }

    fn lower_builtin(
        &self,
        loc: &pt::Loc,
        builtin: ast::Builtin,
        _args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        _func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        _opt: &Options,
    ) -> Option<Expression> {
        match builtin {
            ast::Builtin::GetAddress => {
                // // In soroban, address is retrieved via a host function call
                if let Some(constant_id) = &ns.contracts[contract_no].program_id {
                    return Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Address(false),
                        value: BigInt::from_bytes_be(Sign::Plus, constant_id),
                    });
                }
                let address_var_no = vartab.temp_anonymous(&Type::Uint(64));
                let address_var = Expression::Variable {
                    loc: *loc,
                    ty: Type::Address(false),
                    var_no: address_var_no,
                };
                cfg.add(
                    vartab,
                    Instr::Call {
                        res: vec![address_var_no],
                        return_tys: vec![Type::Uint(64)],
                        call: InternalCallTy::HostFunction {
                            name: HostFunctions::GetCurrentContractAddress.name().to_string(),
                        },
                        args: vec![],
                    },
                );
                Some(address_var)
            }
            _ => None,
        }
    }

    fn lower_storage_struct_member(
        &self,
        loc: &pt::Loc,
        var_expr: Expression,
        struct_ty: &StructType,
        field_no: usize,
        ns: &Namespace,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
    ) -> Expression {
        let offset: BigInt = struct_ty.definition(ns).fields[..field_no]
            .iter()
            .filter(|field| !field.infinite_size)
            .map(|field| field.ty.storage_slots(ns))
            .sum();
        let offset_expr = Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(32),
            value: offset,
        };
        let offset_encoded = soroban_encode_arg(offset_expr, cfg, vartab, ns);
        let res = vartab.temp_name("vec_push_codegen", &Type::Uint(64));
        cfg.add(
            vartab,
            Instr::Call {
                res: vec![res],
                return_tys: vec![Type::Uint(64)],
                call: InternalCallTy::HostFunction {
                    name: HostFunctions::VecPushBack.name().to_string(),
                },
                args: vec![var_expr, offset_encoded],
            },
        );
        Expression::Variable {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            var_no: res,
        }
    }

    fn lower_load_storage(
        &self,
        value: Expression,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        soroban_decode_arg(value, cfg, vartab, ns, None)
    }

    fn post_process_program(&self, _ns: &mut Namespace, _opt: &Options) {}

    fn selector_hash_algorithm(&self) -> ast::Builtin {
        ast::Builtin::Keccak256
    }

    fn initial_storage_slot(&self) -> BigInt {
        BigInt::zero()
    }

    fn align_storage_slot(&self, slot: BigInt, _ty: &Type, _ns: &Namespace) -> BigInt {
        slot
    }

    fn default_gas_builtin(&self) -> BigInt {
        BigInt::zero()
    }

    fn lower_print_expr(&self, expr: Expression) -> Expression {
        expr
    }

    fn lower_mapping_subscript(
        &self,
        loc: &pt::Loc,
        elem_ty: &Type,
        array_ty: &Type,
        array: Expression,
        index: Expression,
    ) -> Expression {
        Expression::Subscript {
            loc: *loc,
            ty: elem_ty.clone(),
            array_ty: array_ty.clone(),
            expr: Box::new(array),
            index: Box::new(index),
        }
    }
}

pub(super) fn validate_accessor_abi_types(contract_no: usize, ns: &mut Namespace) {
    for variable in &ns.contracts[contract_no].variables {
        if !matches!(variable.visibility, pt::Visibility::Public(_)) {
            continue;
        }

        if let Some(unsupported_type) = unsupported_accessor_type(&variable.ty, ns) {
            ns.diagnostics.push(Diagnostic::error(
                variable.loc,
                format!(
                    "type '{unsupported_type}' is not supported as a Soroban public variable accessor return value"
                ),
            ));
        }
    }
}

pub(super) fn validate_event_abi_types(contract_no: usize, ns: &mut Namespace) {
    for event_no in ns.contracts[contract_no].emits_events.clone() {
        for field in &ns.events[event_no].fields {
            if let Some(unsupported_type) = unsupported_event_type(&field.ty, ns) {
                ns.diagnostics.push(Diagnostic::error(
                    field.ty_loc.unwrap_or(field.loc),
                    format!(
                        "type '{unsupported_type}' is not supported as a Soroban event parameter"
                    ),
                ));
            }
        }
    }
}

pub(super) fn validate_abi_types(all_cfg: &[ControlFlowGraph], ns: &mut Namespace) {
    for cfg in all_cfg {
        if !cfg.public {
            continue;
        }

        if is_public_accessor(cfg, ns) {
            continue;
        }

        if cfg.returns.len() > 1 {
            let loc = cfg.returns[1].ty_loc.unwrap_or(cfg.returns[1].loc);
            ns.diagnostics.push(Diagnostic::error(
                loc,
                "Soroban external functions can return at most one value".to_string(),
            ));
            continue;
        }

        for param in cfg.params.as_ref() {
            if let Some(unsupported_type) = unsupported_parameter_type(&param.ty, ns) {
                ns.diagnostics.push(Diagnostic::error(
                    param.ty_loc.unwrap_or(param.loc),
                    format!(
                        "type '{unsupported_type}' is not supported as a Soroban external function parameter"
                    ),
                ));
            }
        }

        for ret in cfg.returns.as_ref() {
            if let Some(unsupported_type) = unsupported_return_type(&ret.ty, ns) {
                ns.diagnostics.push(Diagnostic::error(
                    ret.ty_loc.unwrap_or(ret.loc),
                    format!(
                        "type '{unsupported_type}' is not supported as a Soroban external function return value"
                    ),
                ));
            }
        }
    }

    validate_string_handle_code_paths(all_cfg, ns);
    validate_unsupported_codegen_paths(all_cfg, ns);
}

fn unsupported_parameter_type(ty: &Type, ns: &Namespace) -> Option<String> {
    match ty {
        Type::DynamicBytes => Some("bytes memory".to_string()),
        Type::Bytes(n) => Some(format!("bytes{n}")),
        Type::Struct(_) => Some(format!("{} memory", ty.to_string(ns))),
        Type::Array(elem, _) if has_unsupported_soroban_array_element(elem.as_ref()) => {
            Some(format!("{} memory", ty.to_string(ns)))
        }
        _ => None,
    }
}

fn is_public_accessor(cfg: &ControlFlowGraph, ns: &Namespace) -> bool {
    match cfg.function_no {
        ASTFunction::SolidityFunction(function_no) => ns.functions[function_no].is_accessor,
        _ => false,
    }
}

fn unsupported_accessor_type(ty: &Type, ns: &Namespace) -> Option<String> {
    match ty {
        Type::DynamicBytes => Some("bytes".to_string()),
        Type::Bytes(n) => Some(format!("bytes{n}")),
        Type::Mapping(mapping) => unsupported_accessor_type(mapping.value.as_ref(), ns),
        Type::Array(elem, _) => unsupported_accessor_type(elem.as_ref(), ns),
        Type::Struct(struct_ty) => {
            let fields = &struct_ty.definition(ns).fields;

            if fields.len() > 1 {
                Some(ty.to_string(ns))
            } else {
                fields
                    .first()
                    .and_then(|field| unsupported_accessor_type(&field.ty, ns))
            }
        }
        _ => None,
    }
}

fn unsupported_event_type(ty: &Type, ns: &Namespace) -> Option<String> {
    match ty {
        Type::DynamicBytes => Some("bytes".to_string()),
        Type::Bytes(n) => Some(format!("bytes{n}")),
        Type::Struct(_) => Some(ty.to_string(ns)),
        _ => None,
    }
}

fn unsupported_return_type(ty: &Type, ns: &Namespace) -> Option<String> {
    match ty {
        Type::String => Some("string memory".to_string()),
        Type::DynamicBytes => Some("bytes memory".to_string()),
        Type::Bytes(n) => Some(format!("bytes{n}")),
        Type::Struct(_) => Some(format!("{} memory", ty.to_string(ns))),
        Type::Array(_, _) => Some(format!("{} memory", ty.to_string(ns))),
        _ => None,
    }
}

fn has_unsupported_soroban_array_element(ty: &Type) -> bool {
    match ty {
        Type::DynamicBytes | Type::Struct(_) => true,
        Type::Array(elem, _) => has_unsupported_soroban_array_element(elem.as_ref()),
        _ => false,
    }
}

fn validate_unsupported_codegen_paths(all_cfg: &[ControlFlowGraph], ns: &mut Namespace) {
    for cfg in all_cfg {
        for block in &cfg.blocks {
            for instr in &block.instr {
                validate_unsupported_codegen_instr(instr, ns);
            }
        }
    }
}

fn validate_unsupported_codegen_instr(instr: &Instr, ns: &mut Namespace) {
    validate_unsupported_codegen_instr_expressions(instr, ns);

    match instr {
        Instr::LoadStorage { ty, storage, .. } => {
            if let Some(unsupported_type) = unsupported_soroban_storage_type(ty, ns) {
                push_codegen_error(
                    ns,
                    CodegenError::unsupported_soroban_type(
                        storage.loc(),
                        "in storage load",
                        unsupported_type,
                    ),
                );
            }
        }
        Instr::SetStorage { ty, storage, .. } => {
            if let Some(unsupported_type) = unsupported_soroban_storage_type(ty, ns) {
                push_codegen_error(
                    ns,
                    CodegenError::unsupported_soroban_type(
                        storage.loc(),
                        "in storage store",
                        unsupported_type,
                    ),
                );
            }
        }
        Instr::SetStorageBytes { offset, .. } => {
            push_codegen_error(
                ns,
                CodegenError::unsupported_soroban_operation(
                    offset.loc(),
                    "storage bytes subscript assignment",
                ),
            );
        }
        Instr::PushStorage { ty, storage, .. } => {
            if let Some(unsupported_type) = unsupported_soroban_storage_type(ty, ns) {
                push_codegen_error(
                    ns,
                    CodegenError::unsupported_soroban_type(
                        storage.loc(),
                        "in storage push",
                        unsupported_type,
                    ),
                );
            }
        }
        Instr::PopStorage { ty, storage, .. } => {
            if let Some(unsupported_type) = unsupported_soroban_storage_type(ty, ns) {
                push_codegen_error(
                    ns,
                    CodegenError::unsupported_soroban_type(
                        storage.loc(),
                        "in storage pop",
                        unsupported_type,
                    ),
                );
            }
        }
        Instr::Constructor { loc, .. } => {
            push_codegen_error(
                ns,
                CodegenError::unsupported_soroban_operation(*loc, "contract construction"),
            );
        }
        Instr::ValueTransfer { address, .. } => {
            push_codegen_error(
                ns,
                CodegenError::unsupported_soroban_operation(address.loc(), "value transfer"),
            );
        }
        Instr::SelfDestruct { recipient } => {
            push_codegen_error(
                ns,
                CodegenError::unsupported_soroban_operation(recipient.loc(), "selfdestruct"),
            );
        }
        _ => (),
    }
}

fn validate_unsupported_codegen_instr_expressions(instr: &Instr, ns: &mut Namespace) {
    let mut cx = UnsupportedCodegenExprContext {
        diagnostics: Vec::new(),
    };

    instr.recurse_expressions(&mut cx, reject_unsupported_codegen_expr);

    for diagnostic in cx.diagnostics {
        push_codegen_error(ns, diagnostic);
    }
}

struct UnsupportedCodegenExprContext {
    diagnostics: Vec<CodegenError>,
}

fn reject_unsupported_codegen_expr(
    expr: &Expression,
    cx: &mut UnsupportedCodegenExprContext,
) -> bool {
    if let Expression::Subscript { loc, array_ty, .. } = expr {
        if array_ty.is_storage_bytes() {
            cx.diagnostics
                .push(CodegenError::unsupported_soroban_operation(
                    *loc,
                    "storage bytes subscript load",
                ));
        }
    }

    true
}

fn unsupported_soroban_storage_type(ty: &Type, ns: &Namespace) -> Option<String> {
    match ty.deref_any() {
        Type::ExternalFunction { .. } => Some(ty.to_string(ns)),
        Type::Array(elem, _) => unsupported_soroban_storage_type(elem.as_ref(), ns),
        Type::Mapping(mapping) => unsupported_soroban_storage_type(mapping.value.as_ref(), ns),
        Type::Struct(struct_ty) => struct_ty
            .definition(ns)
            .fields
            .iter()
            .find_map(|field| unsupported_soroban_storage_type(&field.ty, ns)),
        _ => None,
    }
}

fn push_codegen_error(ns: &mut Namespace, err: CodegenError) {
    if let Some(diagnostic) = err.diagnostic() {
        ns.diagnostics.push(diagnostic);
    }
}

fn validate_string_handle_code_paths(all_cfg: &[ControlFlowGraph], ns: &mut Namespace) {
    for cfg in all_cfg {
        let mut string_handle_vars = BTreeSet::new();

        for block in &cfg.blocks {
            for instr in &block.instr {
                validate_string_handle_uses(instr, &string_handle_vars, ns);

                match instr {
                    Instr::Set { res, expr, .. } => {
                        update_string_handle_var(*res, expr, &mut string_handle_vars);
                    }
                    Instr::Call {
                        res,
                        return_tys,
                        call: InternalCallTy::Static { cfg_no },
                        args,
                    } => {
                        validate_static_string_call_args(
                            &all_cfg[*cfg_no],
                            args,
                            &string_handle_vars,
                            ns,
                        );

                        for (res_no, ty) in res.iter().zip(return_tys.iter()) {
                            if matches!(ty, Type::String) {
                                string_handle_vars.insert(*res_no);
                            } else {
                                string_handle_vars.remove(res_no);
                            }
                        }
                    }
                    Instr::Call {
                        res, return_tys, ..
                    } => {
                        for (res_no, ty) in res.iter().zip(return_tys.iter()) {
                            if matches!(ty, Type::String) {
                                string_handle_vars.insert(*res_no);
                            } else {
                                string_handle_vars.remove(res_no);
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

fn update_string_handle_var(
    res: usize,
    expr: &Expression,
    string_handle_vars: &mut BTreeSet<usize>,
) {
    if is_soroban_string_handle_expr(expr, string_handle_vars) {
        string_handle_vars.insert(res);
    } else {
        string_handle_vars.remove(&res);
    }
}

fn validate_static_string_call_args(
    callee: &ControlFlowGraph,
    args: &[Expression],
    string_handle_vars: &BTreeSet<usize>,
    ns: &mut Namespace,
) {
    for (arg, param) in args.iter().zip(callee.params.iter()) {
        if matches!(param.ty, Type::String)
            && !is_soroban_string_handle_expr(arg, string_handle_vars)
        {
            ns.diagnostics.push(Diagnostic::error(
                arg.loc(),
                "passing string memory values to internal functions is not supported for target soroban"
                    .to_string(),
            ));
        }
    }
}

fn validate_string_handle_uses(
    instr: &Instr,
    string_handle_vars: &BTreeSet<usize>,
    ns: &mut Namespace,
) {
    let mut cx = StringHandleUseContext {
        string_handle_vars,
        diagnostics: Vec::new(),
    };

    match instr {
        Instr::Set { expr, .. } => {
            expr.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::Call { args, .. } | Instr::Return { value: args } => {
            for arg in args {
                arg.recurse(&mut cx, reject_string_handle_memory_use);
            }
        }
        Instr::BranchCond { cond, .. } | Instr::Print { expr: cond } => {
            cond.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::Store { dest, data } => {
            dest.recurse(&mut cx, reject_string_handle_memory_use);
            data.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::AssertFailure {
            encoded_args: Some(expr),
        } => {
            expr.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::LoadStorage { storage, .. } | Instr::ClearStorage { storage, .. } => {
            storage.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::SetStorage { value, storage, .. } => {
            value.recurse(&mut cx, reject_string_handle_memory_use);
            storage.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::SetStorageBytes {
            value,
            storage,
            offset,
        } => {
            value.recurse(&mut cx, reject_string_handle_memory_use);
            storage.recurse(&mut cx, reject_string_handle_memory_use);
            offset.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::PushStorage { value, storage, .. } => {
            if let Some(value) = value {
                value.recurse(&mut cx, reject_string_handle_memory_use);
            }

            storage.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::PopStorage { storage, .. } => {
            storage.recurse(&mut cx, reject_string_handle_memory_use);
        }
        Instr::PushMemory { array, value, .. } => {
            Expression::Variable {
                loc: pt::Loc::Codegen,
                ty: Type::String,
                var_no: *array,
            }
            .recurse(&mut cx, reject_string_handle_memory_use);
            value.recurse(&mut cx, reject_string_handle_memory_use);
        }
        _ => (),
    }

    for diagnostic in cx.diagnostics {
        ns.diagnostics.push(diagnostic);
    }
}

struct StringHandleUseContext<'a> {
    string_handle_vars: &'a BTreeSet<usize>,
    diagnostics: Vec<Diagnostic>,
}

fn reject_string_handle_memory_use(expr: &Expression, cx: &mut StringHandleUseContext<'_>) -> bool {
    if let Expression::Builtin {
        loc,
        kind: Builtin::ArrayLength,
        args,
        ..
    } = expr
    {
        if args.first().is_some_and(|arg| {
            matches!(arg.ty(), Type::String)
                && is_soroban_string_handle_expr(arg, cx.string_handle_vars)
        }) {
            cx.diagnostics.push(Diagnostic::error(
                *loc,
                "using string memory as bytes is not supported for target soroban".to_string(),
            ));
        }
    }

    true
}

fn is_soroban_string_handle_expr(expr: &Expression, string_handle_vars: &BTreeSet<usize>) -> bool {
    match expr {
        Expression::FunctionArg {
            ty: Type::String, ..
        } => true,
        Expression::Variable {
            ty: Type::String,
            var_no,
            ..
        } => string_handle_vars.contains(var_no),
        Expression::Cast {
            ty: Type::String,
            expr,
            ..
        } => is_soroban_string_handle_expr(expr, string_handle_vars),
        _ => false,
    }
}

fn soroban_vec_handle_ty(vec_ty: &Type) -> Type {
    let inner_ty = if let Type::StorageRef(_, inner) = vec_ty {
        inner.as_ref().clone()
    } else {
        vec_ty.clone()
    };

    Type::SorobanHandle(Box::new(inner_ty))
}

pub(crate) fn soroban_vec_new(
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
    // Storage wrapper: evaluate storage key/value and load vec object from storage.
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, target);
    let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt, target);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_storage(
        loc,
        &vec_ty,
        var_expr.clone(),
        cfg,
        vartab,
        None,
        ns,
        target,
    );
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
    // Storage wrapper: evaluate storage key and load vec object from storage.
    let var_expr = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, target);
    let vec_ty = args[0].ty();

    let old_vec_obj = load_storage(
        loc,
        &vec_ty,
        var_expr.clone(),
        cfg,
        vartab,
        None,
        ns,
        target,
    );
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
