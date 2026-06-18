// SPDX-License-Identifier: Apache-2.0

pub(crate) mod polkadot;
pub(crate) mod solana;

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::interface::TargetCodegen;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::sema::ast::Function;

/// This traits delineates the common behavior of event emission. As each target uses a different
/// encoding scheme, there must be an implementation of this trait for each.
///
/// Re-exported from [`crate::codegen::interface`] as part of the target-codegen boundary; it is
/// `pub(crate)` so it can be named there.
pub(crate) trait EventEmitter {
    /// Generate the CFG instructions for emitting an event.
    /// All necessary code analysis should have been done during parsing and 'sema';
    /// If code generation does not work here, there is a bug in the compiler.
    fn emit(
        &self,
        contract_no: usize,
        func: &Function,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        opt: &Options,
        target: &dyn TargetCodegen,
    );

    /// Generates the selector
    fn selector(&self, emitting_contract_no: usize) -> Vec<u8>;
}
