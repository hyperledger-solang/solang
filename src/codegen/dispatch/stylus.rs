// SPDX-License-Identifier: Apache-2.0

use super::polkadot;
use crate::{
    codegen::{cfg::ControlFlowGraph, Options},
    sema::ast::Namespace,
};
use solang_parser::pt::FunctionTy;

pub(crate) fn function_dispatch(
    _contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> Vec<ControlFlowGraph> {
    vec![polkadot::Dispatch::new(all_cfg, ns, opt, FunctionTy::Function).build()]
}
