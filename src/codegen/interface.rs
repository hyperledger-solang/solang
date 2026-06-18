// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Function, Namespace, Type};
use solang_parser::pt::Loc;

/// The per-event emission strategy, produced by a target. Defined in
/// [`crate::codegen::events`]; re-exported here so both boundary traits have a
/// single home. `TargetCodegen::event_emitter` is wired to return it in a later phase,
/// hence the allow until the first in-crate use lands.
#[allow(unused_imports)]
pub(crate) use crate::codegen::events::EventEmitter;

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

    /// Whole-program post-processing, called once after every contract's CFGs.
    fn post_process_program(&self, _ns: &mut Namespace, _opt: &Options) {}

    /// Hash algorithm used for function selector computation.
    /// Keccak256 everywhere except Solana (Sha256).
    fn selector_hash_algorithm(&self) -> ast::Builtin {
        ast::Builtin::Keccak256
    }

    /// Whether dynamic storage arrays store their length inline in the value (Solana/Soroban)
    /// or in a separate storage slot (Polkadot).
    fn storage_array_length_is_inline(&self) -> bool {
        false
    }

    /// Optionally rewrite a freshly-built `Load` expression.
    /// Soroban decodes handles on load; other targets pass through unchanged.
    fn lower_load(
        &self,
        load: Expression,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression {
        load
    }

    /// Transform a value just before it is written to storage or a storage-backed ref.
    /// Soroban encodes values to ScVal handles; other targets pass through unchanged.
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

    /// Default value for an uninitialised storage variable; `None` means "skip the variable".
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

    /// Encode `args` into the target's wire format, returning `(buffer, length)`.
    /// The default drives the shared buffer encoder (Borsh / SCALE); Soroban encodes to
    /// ScVal handles instead.
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

    /// Decode `types` out of `buffer`. The default drives the shared buffer decoder
    /// (Borsh / SCALE); Soroban decodes ScVal handles instead.
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

    /// Lower `arr.push(value)` on a contract-storage array. Each target owns its storage
    /// representation (Solana flat slots, Polkadot hashed slots, Soroban host vectors), so
    /// there is no shared default.
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

    /// Lower `arr.pop()` on a contract-storage array. See [`Self::storage_array_push`].
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

    /// Compute the storage slot of array element `index` for the hashed-slots push path.
    /// The default derives it from `keccak256(array_slot)`; Soroban indexes its host vector
    /// with an encoded key instead.
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
}
