// SPDX-License-Identifier: Apache-2.0

use anchor_syn::Ty;
use num_bigint::BigInt;
use solang_parser::pt::{self};

use crate::sema::ast::{self, Function};
use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy},
        vartable::Vartable,
        Expression, Options,
    },
    sema::ast::{Namespace, Type},
};

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

        wrapper_cfg.params = function.params.clone();

        let return_type = if cfg.returns.len() == 1 {
            cfg.returns[0].clone()
        } else {
            ast::Parameter::new_default(Type::Void)
        };

        wrapper_cfg.returns = vec![return_type].into();
        wrapper_cfg.public = true;
        wrapper_cfg.function_no = cfg.function_no.clone();

        let mut vartab = Vartable::from_symbol_table(&function.symtable, ns.next_id);

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
        let placeholder = Instr::Call {
            res: call_returns,
            call: InternalCallTy::Static { cfg_no },
            return_tys,
            args: decode_args(function, ns),
        };

        wrapper_cfg.add(&mut vartab, placeholder);

        // TODO: support multiple returns
        if value.len() == 1 {
            let added = encode_return(function, value[0].clone());
            wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![added] });
        } else {
            // Return 2 as numberliteral. 2 is the soroban Void type encoded.
            let two = Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(2_u64),
            };

            wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![two] });
        }

        vartab.finalize(ns, &mut wrapper_cfg);
        cfg.public = false;
        wrapper_cfgs.push(wrapper_cfg);
    }

    wrapper_cfgs
}

fn decode_args(function: &Function, ns: &Namespace) -> Vec<Expression> {
    let mut args = Vec::new();

    for (i, arg) in function.params.iter().enumerate() {
        println!("ARGUMENTS {:?}", arg);

        let arg = match arg.ty {
            Type::Uint(64) | Type::Uint(32) => Expression::ShiftRight {
                loc: arg.loc,
                ty: Type::Uint(64),
                left: Box::new(Expression::FunctionArg {
                    loc: arg.loc,
                    ty: arg.ty.clone(),
                    arg_no: i,
                }),
                right: Box::new(Expression::NumberLiteral {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    value: BigInt::from(8_u64),
                }),
                signed: false,
            },
            Type::Address(_) => Expression::FunctionArg {
                loc: arg.loc,
                ty: arg.ty.clone(),
                arg_no: i,
            }
            .cast(&Type::Address(false), ns),

            // FIXME: Should properly decode the value instead of just passing it
            Type::Uint(128) | Type::Int(128) => Expression::FunctionArg {
                loc: arg.loc,
                ty: arg.ty.clone(),
                arg_no: i,
            },

            _ => unimplemented!(),
        };

        args.push(arg);
    }

    args
}

// TODO: Implement for UINT32
fn encode_return(function: &Function, value: Expression) -> Expression {
    let returned = function.returns[0].clone();
    match returned.ty {
        Type::Uint(64) => {
            let shifted = Expression::ShiftLeft {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                left: value.clone().into(),
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

            let added = Expression::Add {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                overflowing: true,
                left: shifted.into(),
                right: tag.into(),
            };

            added
        }

        // TODO: support UINT32 here
        Type::Uint(32) => {
            let shifted = Expression::ShiftLeft {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                left: value.clone().into(),
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
                value: BigInt::from(4_u64),
            };

            let added = Expression::Add {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                overflowing: true,
                left: shifted.into(),
                right: tag.into(),
            };

            added
        }
        _ => unimplemented!(),
    }
}
