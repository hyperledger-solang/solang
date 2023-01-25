// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::expression::{default_gas, expression};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::{
    ast,
    ast::{CallArgs, Function, Namespace, Type},
};
use solang_parser::pt::Loc;

use super::encoding::abi_encode;

/// This function encodes the constructor arguments and place an instruction in the CFG to
/// call the constructor of a contract.
pub(super) fn call_constructor(
    loc: &Loc,
    contract_no: &usize,
    callee_contract_no: usize,
    constructor_no: &Option<usize>,
    constructor_args: &[ast::Expression],
    call_args: &CallArgs,
    address_res: usize,
    success: Option<usize>,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) {
    let value = call_args
        .value
        .as_ref()
        .map(|v| expression(v, cfg, callee_contract_no, func, ns, vartab, opt));

    let gas = if let Some(gas) = &call_args.gas {
        expression(gas, cfg, callee_contract_no, func, ns, vartab, opt)
    } else {
        default_gas(ns)
    };

    let salt = call_args
        .salt
        .as_ref()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt));
    let address = call_args
        .address
        .as_ref()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt));
    let seeds = call_args
        .seeds
        .as_ref()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt));

    let mut constructor_args = constructor_args
        .iter()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt))
        .collect::<Vec<Expression>>();

    let selector = match constructor_no {
        Some(func_no) => ns.functions[*func_no].selector(ns, contract_no),
        None => ns.contracts[*contract_no]
            .default_constructor
            .as_ref()
            .unwrap()
            .0
            .selector(ns, contract_no),
    };

    let mut args = vec![Expression::BytesLiteral(
        *loc,
        Type::FunctionSelector,
        selector,
    )];

    args.append(&mut constructor_args);

    let (encoded_args, encoded_args_len) = abi_encode(loc, args, ns, vartab, cfg, false);

    cfg.add(
        vartab,
        Instr::Constructor {
            success,
            res: address_res,
            contract_no: *contract_no,
            encoded_args,
            encoded_args_len,
            value,
            gas,
            salt,
            address,
            seeds,
        },
    );
}
