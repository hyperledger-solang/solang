// SPDX-License-Identifier: Apache-2.0

mod polkadot;
mod solana;

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::events::polkadot::PolkadotEventEmitter;
use crate::codegen::events::solana::SolanaEventEmitter;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace};
use crate::Target;
use solang_parser::pt;

/// This traits delineates the common behavior of event emission. As each target uses a different
/// encoding scheme, there must be an implementation of this trait for each.
pub(super) trait EventEmitter {
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
    );

    /// Generates the selector
    fn selector(&self, emitting_contract_no: usize) -> Vec<u8>;
}

/// Create a new event emitter based on the target blockchain
pub(super) fn new_event_emitter<'a>(
    loc: &pt::Loc,
    event_no: usize,
    args: &'a [ast::Expression],
    ns: &'a Namespace,
) -> Box<dyn EventEmitter + 'a> {
    match ns.target {
        Target::Polkadot { .. } | Target::EVM => {
            Box::new(PolkadotEventEmitter { args, ns, event_no })
        }

        Target::Solana => Box::new(SolanaEventEmitter {
            loc: *loc,
            args,
            ns,
            event_no,
        }),

        Target::Soroban => todo!(),
    }
}
