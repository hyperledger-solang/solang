// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Namespace, Type};
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
}
