use super::{cfg::ControlFlowGraph, Options};
use crate::{sema::ast::Namespace, Target};

pub(super) mod solana;
pub(super) mod substrate;

pub(super) fn function_dispatch(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> ControlFlowGraph {
    match &ns.target {
        Target::Solana => solana::function_dispatch(contract_no, all_cfg, ns, opt),
        Target::Substrate { .. } => substrate::function_dispatch(contract_no, all_cfg, ns, opt),
        _ => unimplemented!(),
    }
}
