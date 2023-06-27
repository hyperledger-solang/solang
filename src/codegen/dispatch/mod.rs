// SPDX-License-Identifier: Apache-2.0

use super::{cfg::ControlFlowGraph, Options};
use crate::{sema::ast::Namespace, Target};

pub(super) mod solana;
pub(super) mod substrate;

pub(super) fn function_dispatch(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> Vec<ControlFlowGraph> {
    match &ns.target {
        Target::Solana => vec![solana::function_dispatch(contract_no, all_cfg, ns, opt)],
        Target::Substrate { .. } | Target::EVM => {
            substrate::function_dispatch(contract_no, all_cfg, ns, opt)
        }
    }
}
