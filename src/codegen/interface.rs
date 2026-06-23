// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Function, Namespace, StructType, Type};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::{self, Loc};

pub(crate) trait EventEmitter {
    fn emit(
        &self,
        contract_no: usize,
        func: &crate::sema::ast::Function,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        opt: &Options,
        target: &dyn TargetCodegen,
    );

    fn selector(&self, emitting_contract_no: usize) -> Vec<u8>;
}

pub(crate) trait TargetCodegen {
    /// Pre-CFG validation. Runs after storage layout, before any CFG is built.
    fn validate_contract(&self, _contract_no: usize, _ns: &mut Namespace) {}

    /// Post-CFG validation; needs the freshly built CFGs.
    fn validate_cfgs(&self, _all_cfg: &[ControlFlowGraph], _ns: &mut Namespace) {}

    /// Build the dispatcher CFG(s) appended after every function CFG is generated.
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph>;

    fn post_process_program(&self, _ns: &mut Namespace, _opt: &Options) {}

    fn selector_hash_algorithm(&self) -> ast::Builtin {
        ast::Builtin::Keccak256
    }

    fn lower_storage_array_length(
        &self,
        loc: &Loc,
        ty: &Type,
        array: Expression,
        elem_ty: &Type,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression;

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
        loc: &Loc,
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

    fn lower_builtin(
        &self,
        _loc: &Loc,
        _builtin: ast::Builtin,
        _args: &[ast::Expression],
        _cfg: &mut ControlFlowGraph,
        _contract_no: usize,
        _func: Option<&Function>,
        _ns: &Namespace,
        _vartab: &mut Vartable,
        _opt: &Options,
    ) -> Option<Expression> {
        None
    }

    fn lower_load(
        &self,
        load: Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression {
        load
    }

    fn prepare_storage_value(
        &self,
        value: Expression,
        _dest: &Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression {
        value
    }

    fn default_storage_value(
        &self,
        _loc: &Loc,
        _ty: &Type,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Option<Expression> {
        None
    }

    fn abi_encode(
        &self,
        loc: &Loc,
        args: Vec<Expression>,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        packed: bool,
    ) -> (Expression, Expression) {
        crate::codegen::encoding::abi_encode(loc, args, ns, vartab, cfg, packed)
    }

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size_expr: Option<Expression>,
    ) -> Vec<Expression> {
        crate::codegen::encoding::abi_decode(loc, buffer, types, ns, vartab, cfg, buffer_size_expr)
    }

    fn storage_array_push(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression;

    fn storage_array_pop(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        return_ty: &Type,
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression;

    fn event_emitter<'a>(
        &self,
        loc: &pt::Loc,
        event_no: usize,
        args: &'a [ast::Expression],
        ns: &'a Namespace,
    ) -> Box<dyn EventEmitter + 'a>;

    fn storage_array_entry_offset(
        &self,
        loc: &Loc,
        var_expr: &Expression,
        index: Expression,
        elem_ty: &Type,
        slot_ty: &Type,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        crate::codegen::storage::array_offset(
            loc,
            Expression::Keccak256 {
                loc: *loc,
                ty: slot_ty.clone(),
                exprs: vec![var_expr.clone()],
            },
            index,
            elem_ty.clone(),
            ns,
        )
    }

    fn lower_storage_struct_member(
        &self,
        loc: &Loc,
        var_expr: Expression,
        struct_ty: &StructType,
        field_no: usize,
        ns: &Namespace,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
    ) -> Expression;

    fn lower_load_storage(
        &self,
        value: Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression {
        value
    }
}
