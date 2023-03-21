use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr},
        encoding::abi_decode,
        vartable::Vartable,
        Builtin, Expression, Options,
    },
    sema::ast::{Namespace, Type, Type::Uint},
};
use num_bigint::{BigInt, Sign};
use solang_parser::pt::{FunctionTy, Loc::Codegen};

/// The dispatching algorithm consists of these steps:
/// 1. If the input is less than the expected selector length (default 4 bytes), fallback or receive.
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
        signed: false,
        left: Expression::NumberLiteral(Codegen, Uint(32), ns.target.selector_length().into())
            .into(),
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
    let selector_ty = Uint(8 * ns.target.selector_length() as u16);
    let cond = Expression::Builtin(
        Codegen,
        vec![selector_ty.clone()],
        Builtin::ReadFromBuffer,
        vec![
            input.clone(),
            Expression::NumberLiteral(Codegen, selector_ty.clone(), 0.into()),
        ],
    );
    let switch_block = cfg.new_basic_block("switch".into());
    let cases = all_cfg
        .iter()
        .enumerate()
        .filter(|(_, msg_cfg)| {
            msg_cfg.public && matches!(msg_cfg.ty, FunctionTy::Function | FunctionTy::Constructor)
        })
        .map(|(msg_no, msg_cfg)| {
            let selector = BigInt::from_bytes_le(Sign::Plus, &msg_cfg.selector);
            let expr = Expression::NumberLiteral(Codegen, selector_ty.clone(), selector);
            let case = dispatch_case(contract_no, all_cfg, msg_no, &mut cfg, ns, vartab);
            (expr, case)
        })
        .collect::<Vec<_>>();
    let switch = Instr::Switch {
        cond,
        cases,
        default,
    };
    cfg.add(vartab, switch);

    // Handle fallback or receive case
    cfg.set_basic_block(default);
    fallback_or_receive(&mut cfg);

    cfg
}

fn dispatch_case(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    msg_no: usize,
    cfg: &mut ControlFlowGraph,
    ns: &mut Namespace,
    vartab: &mut Vartable,
) -> usize {
    let case = cfg.new_basic_block(format!("dispatch_case_{msg_no}"));
    cfg.set_basic_block(case);
    abort_if_value_transfer(msg_no, all_cfg, cfg, ns, vartab);

    case
}

fn fallback_or_receive(cfg: &mut ControlFlowGraph) {}

/// Insert a trap into the cfg, if the message `msg_no` is not payable but received value anyways.
fn abort_if_value_transfer(
    msg_no: usize,
    all_cfg: &[ControlFlowGraph],
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    vartab: &mut Vartable,
) {
    if !all_cfg[msg_no].nonpayable {
        return;
    }

    // Read transferred value from args
    let value_ty = Uint(ns.value_length as u16);
    let input = Expression::FunctionArg(Codegen, Type::DynamicBytes, 1);
    let value_var = vartab.temp_name("value", &value_ty);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: value_var,
            expr: input,
        },
    );

    // Abort if we have transferred value
    let cond = Expression::More {
        loc: Codegen,
        signed: false,
        left: Expression::NumberLiteral(Codegen, value_ty.clone(), 0.into()).into(),
        right: Expression::Variable(Codegen, value_ty, value_var).into(),
    };
    let abort_block = cfg.new_basic_block("has_value".into());
    let next_bb = cfg.new_basic_block("no_value".into());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: abort_block,
            false_block: next_bb,
        },
    );
    cfg.set_basic_block(abort_block);
    cfg.add(vartab, Instr::AssertFailure { encoded_args: None });
    cfg.set_basic_block(next_bb);
}
