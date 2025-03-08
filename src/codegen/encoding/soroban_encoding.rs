// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::InternalCallTy;
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::create_encoder;
use crate::codegen::vartable::Vartable;
use crate::codegen::Expression;
use crate::codegen::HostFunctions;
use crate::sema::ast::{Namespace, RetrieveType, Type, Type::Uint};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::helpers::CodeLocation;
use solang_parser::pt;
use solang_parser::pt::Loc;

/// Soroban encoder works a little differently than the other encoders.
/// For an external call, Soroban first needs to convert values into Soroban ScVals.
/// Each ScVal is 64 bits long, and encoded either via a host function or shifting bits.
/// For this reason, the soroban encoder is implemented as separate from the other encoders.
pub fn soroban_encode(
    loc: &Loc,
    args: Vec<Expression>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    packed: bool,
) -> (Expression, Expression, Vec<Expression>) {
    let mut encoder = create_encoder(ns, packed);

    let size = 8 * args.len(); // 8 bytes per argument

    let size_expr = Expression::NumberLiteral {
        loc: *loc,
        ty: Uint(64),
        value: size.into(),
    };
    let encoded_bytes = vartab.temp_name("abi_encoded", &Type::Bytes(size as u8));

    let expr = Expression::AllocDynamicBytes {
        loc: *loc,
        ty: Type::Bytes(size as u8),
        size: size_expr.clone().into(),
        initializer: Some(vec![]),
    };

    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res: encoded_bytes,
            expr,
        },
    );

    let mut offset = Expression::NumberLiteral {
        loc: *loc,
        ty: Uint(64),
        value: BigInt::zero(),
    };

    let buffer = Expression::Variable {
        loc: *loc,
        ty: Type::Bytes(size as u8),
        var_no: encoded_bytes,
    };

    let mut encoded_items = Vec::new();

    for (arg_no, item) in args.iter().enumerate() {
        let var = soroban_encode_arg(item.clone(), cfg, vartab, ns);

        encoded_items.push(var.clone());

        let advance = encoder.encode(&var, &buffer, &offset, arg_no, ns, vartab, cfg);
        offset = Expression::Add {
            loc: *loc,
            ty: Uint(64),
            overflowing: false,
            left: offset.into(),
            right: advance.into(),
        };
    }

    (buffer, size_expr, encoded_items)
}

pub fn soroban_decode(
    _loc: &Loc,
    buffer: &Expression,
    _types: &[Type],
    _ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    _buffer_size_expr: Option<Expression>,
) -> Vec<Expression> {
    let mut returns = Vec::new();

    let loaded_val = Expression::Load {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        expr: Box::new(buffer.clone()),
    };

    let decoded_val = soroban_decode_arg(loaded_val, cfg, vartab);

    returns.push(decoded_val);

    returns
}

pub fn soroban_decode_arg(
    arg: Expression,
    wrapper_cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let ty = if let Type::Ref(inner_ty) = arg.ty() {
        *inner_ty
    } else {
        arg.ty()
    };

    match ty {
        Type::Uint(64) => Expression::ShiftRight {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            left: arg.into(),
            right: Box::new(Expression::NumberLiteral {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(8_u64),
            }),
            signed: false,
        },

        Type::Address(_) => arg.clone(),

        Type::Int(128) | Type::Uint(128) => decode_i128(wrapper_cfg, vartab, arg),
        _ => unimplemented!(),
    }
}

pub fn soroban_encode_arg(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    let obj = vartab.temp_name("obj_".to_string().as_str(), &Type::Uint(64));

    let ret = match item.ty() {
        Type::String => {
            let inp = Expression::VectorData {
                pointer: Box::new(item.clone()),
            };

            let inp_extend = Expression::ZeroExt {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                expr: Box::new(inp),
            };

            let encoded = Expression::ShiftLeft {
                loc: Loc::Codegen,
                ty: Uint(64),
                left: Box::new(inp_extend),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Uint(64),
                    value: BigInt::from(32),
                }),
            };

            let encoded = Expression::Add {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                overflowing: true,
                left: Box::new(encoded),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Uint(64),
                    value: BigInt::from(4),
                }),
            };

            let len = match item {
                Expression::AllocDynamicBytes { size, .. } => {
                    let sesa = Expression::ShiftLeft {
                        loc: Loc::Codegen,
                        ty: Uint(64),
                        left: Box::new(size.clone().cast(&Type::Uint(64), ns)),
                        right: Box::new(Expression::NumberLiteral {
                            loc: Loc::Codegen,
                            ty: Type::Uint(64),
                            value: BigInt::from(32),
                        }),
                    };

                    Expression::Add {
                        loc: Loc::Codegen,
                        ty: Type::Uint(64),
                        overflowing: true,
                        left: Box::new(sesa),
                        right: Box::new(Expression::NumberLiteral {
                            loc: Loc::Codegen,
                            ty: Type::Uint(64),
                            value: BigInt::from(4),
                        }),
                    }
                }
                Expression::BytesLiteral { loc, ty: _, value } => {
                    let len = Expression::NumberLiteral {
                        loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(value.len() as u64),
                    };

                    let len = Expression::ShiftLeft {
                        loc,
                        ty: Type::Uint(64),
                        left: Box::new(len),
                        right: Box::new(Expression::NumberLiteral {
                            loc,
                            ty: Type::Uint(64),
                            value: BigInt::from(32),
                        }),
                    };

                    Expression::Add {
                        loc,
                        ty: Type::Uint(64),
                        left: Box::new(len),
                        right: Box::new(Expression::NumberLiteral {
                            loc,
                            ty: Type::Uint(64),
                            value: BigInt::from(4),
                        }),
                        overflowing: false,
                    }
                }
                _ => unreachable!(),
            };

            Instr::Call {
                res: vec![obj],
                return_tys: vec![Type::Uint(64)],
                call: crate::codegen::cfg::InternalCallTy::HostFunction {
                    name: HostFunctions::SymbolNewFromLinearMemory.name().to_string(),
                },
                args: vec![encoded, len],
            }
        }
        Type::Uint(64) | Type::Int(64) => {
            let shift_left = Expression::ShiftLeft {
                loc: item.loc(),
                ty: Type::Uint(64),
                left: Box::new(item.clone()),
                right: Box::new(Expression::NumberLiteral {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    value: BigInt::from(8),
                }),
            };

            let tag = match item.ty() {
                Type::Uint(64) => 6,
                Type::Int(64) => 7,
                _ => unreachable!(),
            };

            let added = Expression::Add {
                loc: item.loc(),
                ty: Type::Uint(64),
                left: Box::new(shift_left),
                right: Box::new(Expression::NumberLiteral {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    value: BigInt::from(tag),
                }),
                overflowing: false,
            };

            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: added,
            }
        }
        Type::Address(_) => {
            let instr = if let Expression::Cast { loc, ty: _, expr } = item {
                let address_literal = expr;

                let pointer = Expression::VectorData {
                    pointer: address_literal.clone(),
                };

                let pointer_extend = Expression::ZeroExt {
                    loc,
                    ty: Type::Uint(64),
                    expr: Box::new(pointer),
                };

                let encoded = Expression::ShiftLeft {
                    loc,
                    ty: Uint(64),
                    left: Box::new(pointer_extend),
                    right: Box::new(Expression::NumberLiteral {
                        loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(32),
                    }),
                };

                let encoded = Expression::Add {
                    loc,
                    ty: Type::Uint(64),
                    overflowing: true,
                    left: Box::new(encoded),
                    right: Box::new(Expression::NumberLiteral {
                        loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(4),
                    }),
                };

                let len = if let Expression::BytesLiteral { loc, ty: _, value } =
                    *address_literal.clone()
                {
                    let len = Expression::NumberLiteral {
                        loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(value.len() as u64),
                    };

                    let len = Expression::ShiftLeft {
                        loc,
                        ty: Type::Uint(64),
                        left: Box::new(len),
                        right: Box::new(Expression::NumberLiteral {
                            loc,
                            ty: Type::Uint(64),
                            value: BigInt::from(32),
                        }),
                    };

                    Expression::Add {
                        loc,
                        ty: Type::Uint(64),
                        left: Box::new(len),
                        right: Box::new(Expression::NumberLiteral {
                            loc,
                            ty: Type::Uint(64),
                            value: BigInt::from(4),
                        }),
                        overflowing: false,
                    }
                } else {
                    todo!()
                };

                let str_key_temp = vartab.temp_name("str_key", &Type::Uint(64));
                let str_key_var = Expression::Variable {
                    loc,
                    ty: Type::Uint(64),
                    var_no: str_key_temp,
                };

                let soroban_str_key = Instr::Call {
                    res: vec![str_key_temp],
                    return_tys: vec![Type::Uint(64)],
                    call: crate::codegen::cfg::InternalCallTy::HostFunction {
                        name: HostFunctions::StringNewFromLinearMemory.name().to_string(),
                    },
                    args: vec![encoded.clone(), len.clone()],
                };

                cfg.add(vartab, soroban_str_key);

                let address_object = Instr::Call {
                    res: vec![obj],
                    return_tys: vec![Type::Uint(64)],
                    call: crate::codegen::cfg::InternalCallTy::HostFunction {
                        name: HostFunctions::StrKeyToAddr.name().to_string(),
                    },
                    args: vec![str_key_var],
                };

                address_object
            } else {
                Instr::Set {
                    loc: Loc::Codegen,
                    res: obj,
                    expr: item.clone(),
                }
            };

            instr
        }
        Type::Int(128) | Type::Uint(128) => {
            let low = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(item.clone()),
            };

            let high = Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(128),
                left: Box::new(item.clone()),
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

            let encoded = encode_i128(cfg, vartab, low, high, item.ty());
            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: encoded,
            }
        }
        _ => todo!("Type not yet supported"),
    };

    cfg.add(vartab, ret);

    Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: obj,
    }
}

fn encode_i128(
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    lo: Expression,
    high: Expression,
    int128_ty: Type,
) -> Expression {
    let ret_var = vartab.temp_anonymous(&lo.ty());

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: lo.ty().clone(),
        var_no: ret_var,
    };

    vartab.new_dirty_tracker();
    let fits_in_56_bits = cfg.new_basic_block("fits_in_56_bits".to_string());
    let should_be_in_host = cfg.new_basic_block("should_be_in_host".to_string());
    let return_block = cfg.new_basic_block("finish".to_string());

    let high_8_bits = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: lo.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(56_u64),
        }
        .into(),
        signed: false,
    };

    let cond = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: high_8_bits.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(0_u64),
        }
        .into(),
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: fits_in_56_bits,
            false_block: should_be_in_host,
        },
    );

    cfg.set_basic_block(fits_in_56_bits);

    let to_return = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        left: Box::new(lo.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(8_u64),
        }),
    };
    let tag = match int128_ty {
        Type::Int(128) => 11,
        Type::Uint(128) => 10,
        _ => unreachable!(),
    };

    let to_return = Expression::Add {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        left: to_return.into(),
        right: Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(tag),
        }
        .into(),
        overflowing: false,
    };

    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: to_return,
    };
    cfg.add(vartab, set_instr);

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(should_be_in_host);

    let instr = match int128_ty {
        Type::Int(128) => Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjFromI128Pieces.name().to_string(),
            },
            args: vec![high, lo],
        },
        Type::Uint(128) => Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjFromU128Pieces.name().to_string(),
            },
            args: vec![high, lo],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, instr);

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(return_block);
    cfg.set_phis(return_block, vartab.pop_dirty_tracker());

    ret
}

fn decode_i128(cfg: &mut ControlFlowGraph, vartab: &mut Vartable, arg: Expression) -> Expression {
    let ty = if let Type::Ref(inner_ty) = arg.ty() {
        *inner_ty.clone()
    } else {
        arg.ty()
    };

    let ret_var = vartab.temp_anonymous(&ty);

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: ty.clone(),
        var_no: ret_var,
    };

    vartab.new_dirty_tracker();

    let tag = extract_tag(arg.clone());

    let val_in_host = cfg.new_basic_block("val_is_host".to_string());
    let val_in_obj = cfg.new_basic_block("val_is_obj".to_string());
    let return_block = cfg.new_basic_block("finish".to_string());

    let predicate = match ty {
        Type::Int(128) => 11,
        Type::Uint(128) => 10,
        _ => unreachable!(),
    };
    let is_in_obj = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: tag.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(predicate),
        }
        .into(),
    };

    cfg.add(
        vartab,
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

    let extend = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(value.clone()),
    };

    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: extend,
    };

    cfg.add(vartab, set_instr);

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(val_in_host);

    let low_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let low_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: low_var_no,
    };

    let get_lo_instr = match ty {
        Type::Int(128) => Instr::Call {
            res: vec![low_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI128Lo64.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(128) => Instr::Call {
            res: vec![low_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU128Lo64.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_lo_instr);

    let low_var = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(low_var),
    };

    let high_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let high_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: high_var_no,
    };

    let get_hi_instr = match ty {
        Type::Int(128) => Instr::Call {
            res: vec![high_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI128Hi64.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(128) => Instr::Call {
            res: vec![high_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU128Hi64.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_hi_instr);

    let total = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(high_var),
    };

    let total = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: ty.clone(),
        left: Box::new(total),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: ty.clone(),
            value: BigInt::from(64),
        }),
    };

    let total = Expression::Add {
        loc: Loc::Codegen,
        ty: ty.clone(),
        overflowing: false,
        left: total.into(),
        right: low_var.into(),
    };

    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: total,
    };

    cfg.add(vartab, set_instr);

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(return_block);
    cfg.set_phis(return_block, vartab.pop_dirty_tracker());

    ret
}

fn extract_tag(arg: Expression) -> Expression {
    let bit_mask = Expression::NumberLiteral {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        value: BigInt::from(0xFF),
    };

    Expression::BitwiseAnd {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: arg.clone().into(),
        right: bit_mask.into(),
    }
}
