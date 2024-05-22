use bitvec::view::AsBits;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToBytes};
use solang_parser::pt::{self, Identifier};

use crate::codegen::statements::LoopScopes;
use crate::sema::ast::{Function, FunctionAttributes};
use crate::sema::{self, ast};
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
        //wrapper_cfg.returns = function.returns.clone();

        //let returns = Vec::new();
        let param = ast::Parameter::new_default(Type::Uint(64));
        wrapper_cfg.returns = vec![param].into();
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

        let mut call_returns = Vec::new();
        for (i, arg) in function.returns.iter().enumerate() {
            let new = vartab.temp_anonymous(&arg.ty);
            value.push(Expression::Variable {
                loc: arg.loc,
                ty: arg.ty.clone(),
                var_no: new,
            });
            return_tys.push(arg.ty.clone());
            call_returns.push(new);
        }

        //let return_instr = Instr::Return { value };

        let cfg_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => no,
            _ => 0,
        };
        let placeholder = Instr::Call {
            res: call_returns,
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

        let number_literal = Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(3_u64),
        };

        
        // set the msb 8 bits of the return value to 6, the return value is 64 bits.

        let shifted = Expression::ShiftLeft {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            left: number_literal.clone().into(),
            right: Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(8_u64),
            }
            .into(),
        };

        let tag = Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(6_u64),
        };

        let added = Expression::Add { loc: pt::Loc::Codegen, ty: Type::Uint(64), overflowing: false, left: shifted.into(), right: tag.into() };

        wrapper_cfg.add(
            &mut vartab,
            Instr::Return {
                value: vec![added],
            },
        );

        vartab.finalize(ns, &mut wrapper_cfg);

        println!(" PRINTING WRAPPER CFG{:?} ", wrapper_cfg.returns);
        //wrapper_cfg.vars = vartab.vars.clone();
        cfg.public = false;
        wrapper_cfgs.push(wrapper_cfg);
    }

    wrapper_cfgs
}
