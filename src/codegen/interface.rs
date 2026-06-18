// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::Options;
use crate::sema::ast::Namespace;

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
}
