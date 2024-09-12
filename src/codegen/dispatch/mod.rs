// SPDX-License-Identifier: Apache-2.0

use super::{cfg::ControlFlowGraph, Options};
use crate::{sema::ast::Namespace, Target};

pub(crate) mod polkadot;
pub(super) mod solana;
pub(super) mod soroban;

pub(super) fn function_dispatch(
    contract_no: usize,
    all_cfg: &mut [ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> Vec<ControlFlowGraph> {
    match &ns.target {
        Target::Solana => vec![solana::function_dispatch(contract_no, all_cfg, ns, opt)],
        Target::Polkadot { .. } | Target::EVM => {
            polkadot::function_dispatch(contract_no, all_cfg, ns, opt)
        }
        Target::Soroban => soroban::function_dispatch(contract_no, all_cfg, ns, opt),
    }
}
