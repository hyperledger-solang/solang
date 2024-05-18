use solang_parser::pt::Identifier;

use crate::codegen::statements::LoopScopes;
use crate::sema;
use crate::sema::ast::{Function, FunctionAttributes};
use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy, ReturnCode},
        encoding::{abi_decode, abi_encode},
        revert::log_runtime_error,
        vartable::Vartable,
        Builtin, Expression, Options,
    },
    sema::ast::{Namespace, Parameter, Type, Type::Uint},
};

use crate::codegen::expression;

pub fn function_dispatch(
    contract_no: usize,
    all_cfg: &mut [ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> Vec<ControlFlowGraph> {
    // For each function in all_cfg, we will generate a wrapper function that will call the function
    // The wrapper function will call abi_encode to encode the arguments, and then call the function

    let mut wrapper_cfgs = Vec::new();

    for cfg in all_cfg.iter_mut() {
        if cfg.function_no == ASTFunction::None {
            continue;
        }


        

        println!(
            "generating wrapper for function {} with number {:?}",
            cfg.name, cfg.function_no
        );

        let function = match &cfg.function_no {
            ASTFunction::SolidityFunction(no) | ASTFunction::YulFunction(no) => &ns.functions[*no],
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
                cfg.name.clone()
            }
        };

        let mut wrapper_cfg = ControlFlowGraph::new(wrapper_name.to_string(), ASTFunction::None);

        wrapper_cfg.params = function.params.clone();
        wrapper_cfg.returns = function.returns.clone();
        wrapper_cfg.public = true;

        let mut vartab = Vartable::from_symbol_table(&function.symtable, ns.next_id);

        for (i, arg) in function.symtable.arguments.iter().enumerate() {
            if let Some(pos) = arg {
                let var = &function.symtable.vars[pos];
                wrapper_cfg.add(
                    &mut vartab,
                    Instr::Set {
                        loc: var.id.loc,
                        res: *pos,
                        expr: Expression::FunctionArg {
                            loc: var.id.loc,
                            ty: var.ty.clone(),
                            arg_no: i,
                        },
                    },
                );
            }
        }

        let mut value = Vec::new();
        let mut return_tys = Vec::new();

        for (i, arg) in function.returns.iter().enumerate() {
            value.push(Expression::Variable {
                loc: arg.loc,
                ty: arg.ty.clone(),
                var_no: function.symtable.returns[i],
            });
            return_tys.push(arg.ty.clone());
        }

        let return_instr = Instr::Return { value };

        let cfg_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => no,
            _ => 0,
        };
        let placeholder = Instr::Call {
            res: function.symtable.returns.clone(),
            call: InternalCallTy::Static { cfg_no },
            return_tys,
            args: function
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| Expression::FunctionArg {
                    loc: p.loc,
                    ty: p.ty.clone(),
                    arg_no: i,
                })
                .collect(),
        };

        wrapper_cfg.add(&mut vartab, placeholder);

        wrapper_cfg.add(&mut vartab, return_instr);

        vartab.finalize(ns, &mut wrapper_cfg);

        // If we emit a wrapper for a function, there is no need to make the function itself as public
        cfg.public = false;

        wrapper_cfgs.push(wrapper_cfg);
    }

    wrapper_cfgs
}
