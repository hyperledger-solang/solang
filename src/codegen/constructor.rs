use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::expression::{default_gas, expression};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast;
use crate::sema::ast::{CallArgs, Function, Namespace, RetrieveType, Type};
use crate::Target;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use solang_parser::pt::Loc;

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
    let space = call_args
        .space
        .as_ref()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt));

    let mut tys: Vec<Type> = Vec::new();
    let mut packed: Vec<Expression> = Vec::new();
    let mut args: Vec<Expression> = Vec::new();
    if ns.target == Target::Solana {
        tys.resize(3, Type::Bool);
        tys[0] = Type::Uint(64);
        tys[1] = Type::Uint(32);
        tys[2] = Type::Bytes(1);

        let value_arg = value.clone().unwrap_or_else(|| {
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::zero())
        });
        let selector = ns.contracts[*contract_no].selector().to_be();
        let padding = Expression::NumberLiteral(*loc, Type::Bytes(1), BigInt::zero());

        packed.resize(3, Expression::Poison);
        packed[0] = value_arg;
        packed[1] = Expression::NumberLiteral(*loc, Type::Uint(32), BigInt::from(selector));
        packed[2] = padding;
    } else {
        let selector = match constructor_no {
            Some(func_no) => ns.functions[*func_no].selector(),
            None => ns.contracts[*contract_no]
                .default_constructor
                .as_ref()
                .unwrap()
                .0
                .selector(),
        };

        args.push(Expression::NumberLiteral(
            *loc,
            Type::Uint(32),
            BigInt::from_bytes_le(Sign::Plus, &selector),
        ));
        tys.push(Type::Uint(32));
    }

    let mut constructor_args = constructor_args
        .iter()
        .map(|e| expression(e, cfg, callee_contract_no, func, ns, vartab, opt))
        .collect::<Vec<Expression>>();
    let mut arg_types = constructor_args
        .iter()
        .map(|e| e.ty())
        .collect::<Vec<Type>>();
    args.append(&mut constructor_args);
    tys.append(&mut arg_types);

    let encoded_buffer = vartab.temp_anonymous(&Type::DynamicBytes);
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res: encoded_buffer,
            expr: Expression::AbiEncode {
                loc: *loc,
                tys,
                packed,
                args,
            },
        },
    );

    let encoded_args = Expression::Variable(*loc, Type::DynamicBytes, encoded_buffer);
    let encoded_args_len = Expression::Builtin(
        *loc,
        vec![Type::Uint(32)],
        Builtin::ArrayLength,
        vec![encoded_args.clone()],
    );

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
            space,
        },
    );
}
