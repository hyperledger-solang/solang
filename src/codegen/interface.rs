// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Function, Namespace, StructType, Type};
use num_bigint::BigInt;
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
    fn validate_contract(&self, _contract_no: usize, _ns: &mut Namespace);

    /// Post-CFG validation; needs the freshly built CFGs.
    fn validate_cfgs(&self, _all_cfg: &[ControlFlowGraph], _ns: &mut Namespace);

    /// Build the dispatcher CFG(s) appended after every function CFG is generated.
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph>;

    fn post_process_program(&self, _ns: &mut Namespace, _opt: &Options);

    fn selector_hash_algorithm(&self) -> ast::Builtin;

    fn storage_array_length_is_inline(&self) -> bool;

    fn initial_storage_slot(&self) -> BigInt;

    fn align_storage_slot(&self, slot: BigInt, _ty: &Type, _ns: &Namespace) -> BigInt;

    fn default_gas_builtin(&self) -> BigInt;

    fn lower_print_expr(&self, expr: Expression) -> Expression;

    fn lower_mapping_subscript(
        &self,
        loc: &Loc,
        elem_ty: &Type,
        array_ty: &Type,
        array: Expression,
        index: Expression,
    ) -> Expression;

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
    ) -> Option<Expression>;

    fn lower_load(
        &self,
        load: Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression;

    fn prepare_storage_value(
        &self,
        value: Expression,
        _dest: &Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression;

    fn default_storage_value(
        &self,
        _loc: &Loc,
        _ty: &Type,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Option<Expression>;

    fn abi_encode(
        &self,
        loc: &Loc,
        args: Vec<Expression>,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        packed: bool,
    ) -> (Expression, Expression);

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size_expr: Option<Expression>,
    ) -> Vec<Expression>;

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
    ) -> Expression;

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
    ) -> Expression;
}
