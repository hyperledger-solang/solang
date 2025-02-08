// SPDX-License-Identifier: Apache-2.0

use crate::codegen::Builtin;
use crate::codegen::HostFunctions;
use crate::pt::Loc;
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{self, Function, Parameter};
use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy},
        vartable::Vartable,
        Expression, Options,
    },
    sema::ast::{Namespace, Type},
};
use num_bigint::BigInt;
use num_traits::Zero;
use sha2::digest::typenum::Exp;
use solang_parser::helpers::CodeLocation;
use solang_parser::pt::{self};
use std::f64::consts::E;
use std::ops::Deref;
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
        wrapper_cfg.returns = Arc::new(returns);

        println!("PARAMS: {:?}", wrapper_cfg.params);

        /*let return_type = if cfg.returns.len() == 1 {
            cfg.returns[0].clone()
        } else {
            ast::Parameter::new_default(Type::Void)
        };


        wrapper_cfg.returns = vec![return_type].into();*/
        wrapper_cfg.public = true;
        wrapper_cfg.function_no = cfg.function_no;

        //let mut vartab = Vartable::from_symbol_table(&function.symtable, ns.next_id);
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



        //let mut var_args_decoded = Vec::new();

        let decode_cfg = decode_arg_cfg(ns);

        ns.contracts[contract_no].cfg.push(decode_cfg.clone());



        //ns.contracts[contract_no].cfg.insert(0, decode_cfg);

        let decoder_cfg_no = ns.contracts[contract_no].cfg.len() - 1;

        let deb = &ns.contracts[contract_no].cfg[decoder_cfg_no];

        println!("DECODE CFG: {:?}", deb);


        //let cfg_expr = Expression::InternalFunctionCfg { ty: Type::InternalFunction { mutability: ast::Mutability::Pure(Loc::Codegen), params: vec![Type::Uint(64)], returns: vec![Type::Int(128)] }, cfg_no: decoder_cfg_no };
    

        let var_arg_decoded_no = vartab.temp_anonymous(&Type::Int(128));
        let var_arg_decoded = Expression::Variable {
            loc: pt::Loc::Codegen,
            ty: Type::Int(128),
            var_no: var_arg_decoded_no,
        };

        let decoded_arg = Instr::Call { res: vec![var_arg_decoded_no], return_tys: vec![Type::Int(128)], call: InternalCallTy::HostFunction { name: decode_cfg.clone().name.clone() }, args: vec![Expression::FunctionArg { loc: Loc::Codegen, ty: Type::Uint(64), arg_no: 0 }] };

        wrapper_cfg.add(&mut vartab, decoded_arg);

        let placeholder = Instr::Call {
            res: call_returns,
            call: InternalCallTy::Static { cfg_no },
            return_tys,
            args: vec![var_arg_decoded],
        };

        wrapper_cfg.add(&mut vartab, placeholder);

        println!("RETURNS: {:?}", value);

        let ret = encode_return(value, ns, &mut vartab, &mut wrapper_cfg);
        // TODO: support multiple returns
        /*if value.len() == 1 {
            /*// set the msb 8 bits of the return value to 6, the return value is 64 bits.
            // FIXME: this assumes that the solidity function always returns one value.


            wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![added] });*/

            let value = Expression::NumberLiteral { loc: pt::Loc::Codegen, ty: Type::Int(128), value: BigInt::zero() };
            wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![value] });
        } else {
            // Return 2 as numberliteral. 2 is the soroban Void type encoded.
            let two = Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(2_u64),
            };

            wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![two] });
        }*/


        /*let ret = Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(11_u64),
        };*/

        wrapper_cfg.add(&mut vartab, Instr::Return { value: vec![ret] });

        vartab.finalize(ns, &mut wrapper_cfg);
        cfg.public = false;
        wrapper_cfgs.push(wrapper_cfg);
        wrapper_cfgs.push(decode_cfg);
    }

    wrapper_cfgs
}

fn decode_arg_cfg(ns: &mut Namespace) -> ControlFlowGraph {

    let mut cfg = ControlFlowGraph::new("decode_arg".to_string(), ASTFunction::None);


    cfg.function_no = ASTFunction::None;

    cfg.public = true;
    
    let param = Parameter::new_default(Type::Uint(64));

    cfg.params = vec![param.clone()].into();

    let ret = Parameter::new_default(Type::Int(128));
    cfg.returns = vec![ret].into();

    let mut vartab = Vartable::new(ns.next_id);

    let ret_var = vartab.temp_anonymous(&Type::Int(128));

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Int(128),
        var_no: ret_var,
    };

    //vartab.set_dirty(ret_var);
    vartab.new_dirty_tracker();

    let arg = Expression::FunctionArg {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        arg_no: 0,
    };


    let tag = extract_tag(arg.clone());

    let val_in_host = cfg.new_basic_block("val_is_host".to_string());
    let val_in_obj = cfg.new_basic_block("val_is_obj".to_string());
    let return_block = cfg.new_basic_block("return".to_string());

    let is_in_obj = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: tag.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(11),
        }
        .into(),
    };

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: is_in_obj,
            true_block: val_in_obj,
            false_block: val_in_host,
        },
    );

    cfg.set_basic_block(val_in_obj);

    let value = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Int(64),
        left: arg.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Int(64),
            value: BigInt::from(8_u64),
        }
        .into(),
        signed: false,
    };

    let extend = Expression::ZeroExt { loc: Loc::Codegen, ty: Type::Int(128), expr: Box::new(value.clone()) };

    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: extend,
    };

    cfg.add(&mut vartab, set_instr);

    cfg.add(&mut vartab, Instr::Branch { block: return_block });

    cfg.set_basic_block(val_in_host);

    let value = Expression::NumberLiteral { loc: Loc::Codegen, ty: Type::Int(128), value: BigInt::zero() };

    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: value,
    };

    cfg.add(&mut vartab, set_instr);

    cfg.add(&mut vartab, Instr::Branch { block: return_block });

    cfg.set_basic_block(return_block);
    cfg.set_phis(return_block, vartab.pop_dirty_tracker());

    cfg.add(&mut vartab, Instr::Return { value: vec![ret] });
    



    vartab.finalize(ns, &mut cfg);



    cfg





}



/*fn decode_args(
    args: Arc<Vec<Parameter<Type>>>,
    _ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    var_args_encoded: &mut Vec<Expression>,
)  {
    //let mut decoded_args = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let ty = if let Type::Ref(ty) = &arg.ty {
            ty.deref()
        } else {
            unreachable!("Expected reference type");
        };

        println!("ARG: {:?}", ty);

    

        match ty {
            Type::Uint(64) => Expression::ShiftRight {
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
            },

            // TODO: implement encoding/decoding for Int 128
            Type::Int(128) => {
                let input = Expression::FunctionArg {
                    loc: arg.loc,
                    ty: arg.ty.clone(),
                    arg_no: i,
                };

                let lo_64 = vartab.temp_name("lo_64", &Type::Uint(64));
                let lo_var = Expression::Variable {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    var_no: lo_64,
                };

                let instr = Instr::Call {
                    res: vec![lo_64],
                    return_tys: vec![Type::Uint(64)],
                    call: InternalCallTy::HostFunction {
                        name: HostFunctions::ObjToI128Lo64.name().to_string(),
                    },
                    args: vec![input.clone()],
                };

                cfg.add(vartab, instr);

                let hi_64 = vartab.temp_name("hi_64", &Type::Uint(64));
                let hi_var = Expression::Variable {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    var_no: hi_64,
                };

                let instr = Instr::Call {
                    res: vec![hi_64],
                    return_tys: vec![Type::Uint(64)],
                    call: InternalCallTy::HostFunction {
                        name: HostFunctions::ObjToI128Hi64.name().to_string(),
                    },
                    args: vec![input],
                };

                cfg.add(vartab, instr);

                let size = Expression::NumberLiteral {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    value: BigInt::from(16),
                };

                let buf = Expression::AllocDynamicBytes {
                    loc: Loc::Codegen,
                    ty: Type::Bytes(16),
                    size: Box::new(size),
                    initializer: Some(vec![]),
                };

                let res_buf = vartab.temp_name("res_buf", &Type::Bytes(16));
                let res_buf_var = Expression::Variable {
                    loc: arg.loc,
                    ty: Type::Bytes(16),
                    var_no: res_buf,
                };

                let set = Instr::Set {
                    loc: arg.loc,
                    res: res_buf,
                    expr: buf,
                };

                cfg.add(vartab, set);

                let offset = Expression::NumberLiteral {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    value: BigInt::zero(),
                };

                let write_buf = Instr::WriteBuffer {
                    buf: res_buf_var.clone(),
                    offset: offset,
                    value: hi_var,
                };

                cfg.add(vartab, write_buf);

                let offset = Expression::NumberLiteral {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    value: BigInt::from(8),
                };

                let write_buf = Instr::WriteBuffer {
                    buf: res_buf_var.clone(),
                    offset: offset,
                    value: lo_var,
                };

                cfg.add(vartab, write_buf);

                let final_res = Expression::Load {
                    loc: Loc::Codegen,
                    ty: Type::Int(128),
                    expr: Box::new(res_buf_var),
                };

                //final_res

                //let sesa = 4_u56;
                Expression::NumberLiteral {
                    loc: arg.loc,
                    ty: Type::Uint(64),
                    value: BigInt::zero(),
                }

                //final_res
            }

            _ => unimplemented!(),
        };

        var_args_encoded.push(decoded_arg);
    }

    //decoded_args
}*/

fn extract_tag(arg: Expression) -> Expression {

    let bit_mask = Expression::NumberLiteral {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        value: BigInt::from(0xFF),
    };

    let tag = Expression::BitwiseAnd {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: arg.clone().into(),
        right: bit_mask.into(),
    };
    tag

    /*let val_in_host = cfg.new_basic_block("val_is_host".to_string());
    let val_in_obj = cfg.new_basic_block("val_is_obj".to_string());


    let is_in_obj = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: tag.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(11),
        }
        .into(),
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: is_in_obj,
            true_block: val_in_host,
            false_block: val_in_obj,
        },
    );

    cfg.set_basic_block(val_in_obj);

    let value = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: arg.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(8_u64),
        }
        .into(),
        signed: false,
    };

    if let Expression::Variable { loc, ty, var_no } = arg_var {
        let instr = Instr::Set {
            loc,
            res: var_no,
            expr: value,
        };

        cfg.add(vartab, instr);
        
    }
    else {
        unreachable!();
    }


    cfg.set_basic_block(val_in_host);*/

}

fn encode_return(
    returns: Vec<Expression>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let mut encoded_ret = Expression::NumberLiteral {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        value: BigInt::from(11),
    };

    let ret = returns[0].clone();

    encoded_ret = match ret.ty() {
        Type::Uint(64) => {
            let shifted = Expression::ShiftLeft {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                left: ret.into(),
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

        Type::Address(_) => ret,

        Type::Int(128) => {
            let low = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(ret.clone()),
            };

            let high = Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(128),
                left: Box::new(ret.clone()),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Int(128),
                    value: BigInt::from(64),
                }),
                signed: false,
            };

            let high = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(high),
            };

            let res = vartab.temp_name("res", &Type::Uint(64));
            let res_var = Expression::Variable {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                var_no: res,
            };

            let instr = Instr::Call {
                res: vec![res],
                return_tys: vec![Type::Uint(64)],
                call: InternalCallTy::HostFunction {
                    name: HostFunctions::ObjFromI128Pieces.name().to_string(),
                },
                args: vec![high, low],
            };

            cfg.add(vartab, instr);

            res_var
        }
        _ => unimplemented!(),
    };

    encoded_ret
}
