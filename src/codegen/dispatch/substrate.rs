use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr},
        encoding::abi_decode,
        vartable::Vartable,
        Builtin, Expression, Options,
    },
    sema::ast::{Namespace, Type, Type::Uint},
};
use solang_parser::pt::Loc::Codegen;

/// The dispatching algorithm consists of these steps:
/// 1. If the input is less than 4 bytes, fallback or receive.
/// 2. Match the function selector
///     - If no selector matches, fallback or receive.
///     - If the function is non-payable but the call features endowment, revert.
/// 3. ABI decode the arguments.
/// 4. Call the matching function.
/// 5. Return the result:
///     - On success, ABI encode the result (if any) and return.
///     - On failure, trap the contract.
///
/// We distinguish between fallback and receive:
/// - If there is no endowment, dispatch to fallback
/// - If there is endowment, dispatch to receive
pub(crate) fn function_dispatch(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> ControlFlowGraph {
    let vartab = &mut Vartable::new(ns.next_id);
    let mut cfg = ControlFlowGraph::new("solang_dispatch".into(), ASTFunction::None);

    // Read input lengt
    let input = Expression::FunctionArg(Codegen, Type::DynamicBytes, 0);
    let input_len_var = vartab.temp_name("input_len", &Uint(32));
    let expr = Expression::Builtin(
        Codegen,
        vec![Uint(32)],
        Builtin::ArrayLength,
        vec![input.clone()],
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: input_len_var,
            expr,
        },
    );
    let default = cfg.new_basic_block("fb_or_recv".to_string());
    let start_dispatch_block = cfg.new_basic_block("start_dispatch".to_string());
    let cond = Expression::MoreEqual {
        loc: Codegen,
        signed: true,
        left: Expression::NumberLiteral(Codegen, Uint(32), 4.into()).into(),
        right: Expression::Variable(Codegen, Uint(32), input_len_var).into(),
    };
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: start_dispatch_block,
            false_block: default,
        },
    );

    // Read selector
    cfg.set_basic_block(start_dispatch_block);
    let cond = Expression::Builtin(
        Codegen,
        vec![Uint(32)],
        Builtin::ReadFromBuffer,
        vec![
            input.clone(),
            Expression::NumberLiteral(Codegen, Uint(32), 0.into()),
        ],
    );
    let switch_block = cfg.new_basic_block("switch".into());
    let mut cases = vec![];
    let switch = Instr::Switch {
        cond,
        cases,
        default,
    };
    cfg.add(vartab, switch);

    // Handle fallback or receive case
    cfg.set_basic_block(default);

    todo!()
}
