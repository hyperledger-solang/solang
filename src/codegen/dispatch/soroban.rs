// SPDX-License-Identifier: Apache-2.0

use crate::pt::Loc;
use crate::sema::ast;
use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy},
        encoding::soroban_encoding::{soroban_decode_arg, soroban_encode_arg},
        vartable::Vartable,
        Expression, Options,
    },
    sema::ast::{Namespace, Type},
};
use num_bigint::BigInt;
use solang_parser::pt;
use std::sync::Arc;

pub fn function_dispatch(
    contract_no: usize,
    all_cfg: &mut [ControlFlowGraph],
    ns: &mut Namespace,
    _opt: &Options,
) -> Vec<ControlFlowGraph> {
    // For each function in all_cfg, we will generate a wrapper function that will call the function
    // The wrapper function will call abi_encode to encode the arguments, and then call the function

    let mut wrapper_cfgs = Vec::new();

    for cfg in all_cfg.iter_mut() {
        let function = match &cfg.function_no {
            ASTFunction::SolidityFunction(no) => &ns.functions[*no],
            _ => continue,
        };

        let wrapper_name = {
            if cfg.public {
                if function.mangled_name_contracts.contains(&contract_no) {
                    function.mangled_name.clone()
                } else {
                    function.id.name.clone()
                }
            } else {
                continue;
            }
        };

        let mut wrapper_cfg = ControlFlowGraph::new(wrapper_name.to_string(), ASTFunction::None);

        let mut params = Vec::new();
        for p in function.params.as_ref() {
            let type_ref = Type::Ref(Box::new(p.ty.clone()));
            let mut param = ast::Parameter::new_default(type_ref);
            param.id = p.id.clone();
            params.push(param);
        }

        let mut returns = Vec::new();
        for ret in function.returns.as_ref() {
            let type_ref = Type::Ref(Box::new(ret.ty.clone()));
            let ret = ast::Parameter::new_default(type_ref);
            returns.push(ret);
        }

        wrapper_cfg.params = Arc::new(params);

        if returns.is_empty() {
            returns.push(ast::Parameter::new_default(Type::Ref(Box::new(Type::Void))));
        }
        wrapper_cfg.returns = Arc::new(returns);

        wrapper_cfg.public = true;
        wrapper_cfg.function_no = cfg.function_no;

        let mut vartab = Vartable::new(ns.next_id);

        let mut value = Vec::new();
        let mut return_tys = Vec::new();

        let mut call_returns = Vec::new();
        for arg in function.returns.iter() {
            let new = vartab.temp_anonymous(&arg.ty);
            value.push(Expression::Variable {
                loc: arg.loc,
                ty: arg.ty.clone(),
                var_no: new,
            });
            return_tys.push(arg.ty.clone());
            call_returns.push(new);
        }

        let cfg_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => no,
            _ => 0,
        };

        let decoded = decode_args(&mut wrapper_cfg, &mut vartab);

        let placeholder = Instr::Call {
            res: call_returns,
            call: InternalCallTy::Static { cfg_no },
            return_tys,
            args: decoded,
        };

        wrapper_cfg.add(&mut vartab, placeholder);

        let ret = encode_return(value, ns, &mut vartab, &mut wrapper_cfg);
        wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![ret] });

        vartab.finalize(ns, &mut wrapper_cfg);
        cfg.public = false;
        wrapper_cfgs.push(wrapper_cfg);
    }

    wrapper_cfgs
}

fn decode_args(wrapper_cfg: &mut ControlFlowGraph, vartab: &mut Vartable) -> Vec<Expression> {
    let mut args = Vec::new();

    let params = wrapper_cfg.params.clone();

    for (i, arg) in params.iter().enumerate() {
        let arg = Expression::FunctionArg {
            loc: pt::Loc::Codegen,
            ty: arg.ty.clone(),
            arg_no: i,
        };

        let decoded = soroban_decode_arg(arg.clone(), wrapper_cfg, vartab);

        args.push(decoded);
    }

    args
}

fn encode_return(
    returns: Vec<Expression>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    if returns.len() == 1 {
        soroban_encode_arg(returns[0].clone(), cfg, vartab, ns)
    } else {
        Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(2),
        }
    }
}
