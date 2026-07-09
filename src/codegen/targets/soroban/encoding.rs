// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::InternalCallTy;
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::create_encoder;
use crate::codegen::error::CodegenError;
use crate::codegen::vartable::Vartable;
use crate::codegen::HostFunctions;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type, Type::Uint};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::helpers::CodeLocation;
use solang_parser::pt;
use solang_parser::pt::Loc;

#[allow(dead_code)]
pub(super) mod tags {
    // Inline / small-value tags (CAP-0046-01 §ScVal bit layout, bits 0-7)
    pub const FALSE: u64 = 0;
    pub const TRUE: u64 = 1;
    pub const VOID: u64 = 2;
    pub const ERROR: u64 = 3;
    pub const U32: u64 = 4;
    pub const I32: u64 = 5;
    pub const U64_SML: u64 = 6;
    pub const I64_SML: u64 = 7;
    pub const U128_SML: u64 = 10;
    pub const I128_SML: u64 = 11;
    pub const U256_SML: u64 = 12;
    pub const I256_SML: u64 = 13;

    // Object-handle tags (host allocates; handle stored in bits 32-63)
    pub const U64_OBJ: u64 = 64;
    pub const I64_OBJ: u64 = 65;
    pub const U128_OBJ: u64 = 68;
    pub const I128_OBJ: u64 = 69;
    pub const U256_OBJ: u64 = 70;
    pub const I256_OBJ: u64 = 71;
    pub const BYTES_OBJ: u64 = 72;
    pub const STRING_OBJ: u64 = 73;
    pub const SYMBOL_OBJ: u64 = 74;
    pub const VEC_OBJ: u64 = 75;
    pub const MAP_OBJ: u64 = 76;
    pub const ADDR_OBJ: u64 = 77;
}

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
        ty: Uint(32),
        value: size.into(),
    };
    let encoded_bytes = vartab.temp_name("abi_encoded", &Type::Bytes(size as u8));

    let expr = Expression::AllocDynamicBytes {
        loc: *loc,
        ty: Type::Bytes(size as u8),
        size: size_expr.clone().into(),
        initializer: None,
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
        let var = if matches!(
            item,
            Expression::AllocDynamicBytes { .. } | Expression::BytesLiteral { .. }
        ) {
            encode_as_symbol(item.clone(), cfg, vartab, ns)
        } else {
            soroban_encode_arg(item.clone(), cfg, vartab, ns)
        };

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
    ns: &Namespace,
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

    let decoded_val = soroban_decode_arg(loaded_val, cfg, vartab, ns, None);

    returns.push(decoded_val);

    returns
}

pub fn soroban_decode_arg(
    arg: Expression,
    wrapper_cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
    decode_as: Option<Type>,
) -> Expression {
    let ty = match decode_as {
        Some(ty) => ty,
        None => {
            if let Type::Ref(inner_ty) = arg.ty() {
                *inner_ty
            } else if let Type::StorageRef(_, inner) = arg.ty() {
                *inner
            } else if let Type::SorobanHandle(inner) = arg.ty() {
                *inner
            } else {
                arg.ty()
            }
        }
    };

    match ty {
        Type::Bool => Expression::NotEqual {
            loc: Loc::Codegen,
            left: arg.into(),
            right: Box::new(Expression::NumberLiteral {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                value: 0u64.into(),
            }),
        },
        Type::Uint(64) => decode_u64(wrapper_cfg, vartab, arg),

        Type::Address(_) => arg.clone(),
        Type::String => decode_string(arg, wrapper_cfg, vartab),
        Type::DynamicBytes => decode_bytes(arg, wrapper_cfg, vartab),
        Type::Bytes(n) => {
            let as_dyn = decode_bytes(arg, wrapper_cfg, vartab);
            Expression::BytesCast {
                loc: Loc::Codegen,
                ty: Type::Bytes(n),
                from: Type::DynamicBytes,
                expr: Box::new(as_dyn),
            }
        }

        Type::Enum(enum_no) => {
            let decoded = soroban_decode_arg(arg, wrapper_cfg, vartab, ns, Some(Type::Uint(32)));
            decoded.cast(&Type::Enum(enum_no), ns)
        }

        Type::Int(128) | Type::Uint(128) => decode_i128(wrapper_cfg, vartab, arg, &ty),

        Type::Int(256) | Type::Uint(256) => decode_i256(wrapper_cfg, vartab, arg, &ty),

        Type::Uint(32) => {
            // get payload out of major bits then truncate to 32‑bit
            Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                expr: Box::new(Expression::ShiftRight {
                    loc: Loc::Codegen,
                    ty: Type::Uint(64),
                    left: arg.into(),
                    right: Box::new(Expression::NumberLiteral {
                        loc: Loc::Codegen,
                        ty: Type::Uint(64),
                        value: 32u64.into(),
                    }),
                    signed: false,
                }),
            }
        }

        Type::Int(32) => Expression::Trunc {
            loc: Loc::Codegen,
            ty: Type::Int(32),
            expr: Box::new(Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                left: arg.into(),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Uint(64),
                    value: 32u64.into(),
                }),
                signed: true,
            }),
        },
        Type::Int(64) => Expression::ShiftRight {
            loc: Loc::Codegen,
            ty: Type::Int(64),
            left: arg.into(),
            right: Box::new(Expression::NumberLiteral {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(8u64),
            }),
            signed: true,
        },
        Type::Struct(StructType::UserDefined(n)) => {
            decode_struct_map(arg, wrapper_cfg, vartab, n, ns, ty)
        }
        Type::Array(elem_ty, _) => {
            if let Type::StorageRef(_, _) = arg.ty() {
                arg.clone()
            } else {
                decode_vector(arg, &elem_ty, ns, wrapper_cfg, vartab)
            }
        }

        _ => panic!(
            "{}",
            CodegenError::unsupported_soroban_type(
                arg.loc(),
                "by the Soroban decoder",
                ty.to_string(ns),
            )
        ),
    }
}

pub fn soroban_storage_decode_arg(
    arg: Expression,
    wrapper_cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
    decode_as: Option<Type>,
) -> Expression {
    let ty = match &decode_as {
        Some(ty) => ty.clone(),
        None => match arg.ty() {
            Type::Ref(inner) => *inner,
            Type::StorageRef(_, inner) => *inner,
            Type::SorobanHandle(inner) => *inner,
            other => other,
        },
    };

    match ty {
        Type::Struct(StructType::UserDefined(n)) => {
            decode_struct_storage(arg, wrapper_cfg, vartab, n, ns, ty)
        }
        _ => soroban_decode_arg(arg, wrapper_cfg, vartab, ns, decode_as),
    }
}

pub fn soroban_storage_encode_arg(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    match item.ty() {
        Type::Struct(StructType::UserDefined(n)) => encode_struct_storage(item, cfg, vartab, ns, n),
        _ => soroban_encode_arg(item, cfg, vartab, ns),
    }
}

pub fn soroban_encode_arg(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    if let Type::Bytes(n) = item.ty() {
        let as_dyn = Expression::BytesCast {
            loc: item.loc(),
            ty: Type::DynamicBytes,
            from: Type::Bytes(n),
            expr: Box::new(item),
        };
        return soroban_encode_arg(as_dyn, cfg, vartab, ns);
    }

    let obj = vartab.temp_name("obj_".to_string().as_str(), &Type::Uint(64));

    let ret = match item.ty() {
        Type::Bool => {
            let encoded = match item {
                Expression::BoolLiteral { value, .. } => Expression::NumberLiteral {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    value: if value { 1u64.into() } else { 0u64.into() },
                },
                _ => item.cast(&Type::Uint(64), ns),
            };

            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: encoded,
            }
        }
        Type::String | Type::DynamicBytes => {
            let loc = item.loc();

            let item_var = if matches!(item, Expression::Variable { .. }) {
                item.clone()
            } else {
                let tmp = vartab.temp_name("vec_tmp", &item.ty());
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc,
                        res: tmp,
                        expr: item.clone(),
                    },
                );
                Expression::Variable {
                    loc,
                    ty: item.ty(),
                    var_no: tmp,
                }
            };

            let ptr_u32val = encode_object(
                loc,
                Expression::VectorData {
                    pointer: Box::new(item_var.clone()),
                },
                32,
                tags::U32,
            );
            let len_u32val = encode_object(
                loc,
                Expression::Builtin {
                    loc,
                    tys: vec![Type::Uint(32)],
                    kind: Builtin::ArrayLength,
                    args: vec![item_var],
                },
                32,
                tags::U32,
            );

            let host_fn = if matches!(item.ty(), Type::String) {
                HostFunctions::StringNewFromLinearMemory // b.i
            } else {
                HostFunctions::BytesNewFromLinearMemory // b.3
            };

            Instr::Call {
                res: vec![obj],
                return_tys: vec![Type::Uint(64)],
                call: InternalCallTy::HostFunction {
                    name: host_fn.name().to_string(),
                },
                args: vec![ptr_u32val, len_u32val],
            }
        }
        Type::Uint(32) | Type::Int(32) => {
            // widen to 64 bits so we can shift
            let widened = match item.ty() {
                Type::Uint(32) => Expression::ZeroExt {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    expr: Box::new(item.clone()),
                },
                Type::Int(32) => Expression::SignExt {
                    loc: item.loc(),
                    ty: Type::Int(64),
                    expr: Box::new(item.clone()),
                },
                _ => unreachable!(),
            };

            // the value goes into the major bits of the 64 bit value
            let shifted = Expression::ShiftLeft {
                loc: item.loc(),
                ty: Type::Uint(64),
                left: Box::new(widened.cast(&Type::Uint(64), ns)),
                right: Box::new(Expression::NumberLiteral {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    value: 32u64.into(), // 24 (minor) + 8 (tag)
                }),
            };

            let tag = if matches!(item.ty(), Type::Uint(32)) {
                4
            } else {
                5
            };
            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: Expression::Add {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    left: Box::new(shifted),
                    right: Box::new(Expression::NumberLiteral {
                        loc: item.loc(),
                        ty: Type::Uint(64),
                        value: tag.into(),
                    }),
                    overflowing: false,
                },
            }
        }
        Type::Enum(_) => {
            let widened = Expression::ZeroExt {
                loc: item.loc(),
                ty: Type::Uint(64),
                expr: Box::new(item.cast(&Type::Uint(32), ns)),
            };

            let shifted = Expression::ShiftLeft {
                loc: item.loc(),
                ty: Type::Uint(64),
                left: Box::new(widened),
                right: Box::new(Expression::NumberLiteral {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    value: 32u64.into(),
                }),
            };

            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: Expression::Add {
                    loc: item.loc(),
                    ty: Type::Uint(64),
                    left: Box::new(shifted),
                    right: Box::new(Expression::NumberLiteral {
                        loc: item.loc(),
                        ty: Type::Uint(64),
                        value: tags::I32.into(),
                    }),
                    overflowing: false,
                },
            }
        }
        Type::Uint(64) => {
            let encoded = encode_u64(cfg, vartab, item.clone());
            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: encoded,
            }
        }
        Type::Int(64) => {
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

            let tag = tags::I64_SML;

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
            let instr = if let Expression::Cast {
                loc: _,
                ty: _,
                expr,
            } = item.clone()
            {
                if let Expression::BytesLiteral { loc, ty: _, value } = *expr.clone() {
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
                            value: BigInt::from(tags::U32),
                        }),
                    };

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

                    let len = Expression::Add {
                        loc,
                        ty: Type::Uint(64),
                        left: Box::new(len),
                        right: Box::new(Expression::NumberLiteral {
                            loc,
                            ty: Type::Uint(64),
                            value: BigInt::from(tags::U32),
                        }),
                        overflowing: false,
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

                    Instr::Call {
                        res: vec![obj],
                        return_tys: vec![Type::Uint(64)],
                        call: crate::codegen::cfg::InternalCallTy::HostFunction {
                            name: HostFunctions::StrKeyToAddr.name().to_string(),
                        },
                        args: vec![str_key_var],
                    }
                } else {
                    Instr::Set {
                        loc: Loc::Codegen,
                        res: obj,
                        expr: item.clone(),
                    }
                }
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
        Type::Int(256) | Type::Uint(256) => {
            // For 256-bit integers, we need to split into four 64-bit pieces
            // lo_lo: bits 0-63
            // lo_hi: bits 64-127
            // hi_lo: bits 128-191
            // hi_hi: bits 192-255

            let is_signed = matches!(item.ty(), Type::Int(256));

            // Extract lo_lo (bits 0-63)
            let lo_lo = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(item.clone()),
            };

            // Extract lo_hi (bits 64-127)
            let lo_hi_shift = Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(256),
                left: Box::new(item.clone()),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Int(256),
                    value: BigInt::from(64),
                }),
                signed: is_signed,
            };

            let lo_hi = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(lo_hi_shift),
            };

            // Extract hi_lo (bits 128-191)
            let hi_lo_shift = Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(256),
                left: Box::new(item.clone()),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Int(256),
                    value: BigInt::from(128),
                }),
                signed: is_signed,
            };

            let hi_lo = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(hi_lo_shift),
            };

            // Extract hi_hi (bits 192-255)
            let hi_hi_shift = Expression::ShiftRight {
                loc: Loc::Codegen,
                ty: Type::Int(256),
                left: Box::new(item.clone()),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Int(256),
                    value: BigInt::from(192),
                }),
                signed: is_signed,
            };

            let hi_hi = Expression::Trunc {
                loc: Loc::Codegen,
                ty: Type::Int(64),
                expr: Box::new(hi_hi_shift),
            };

            let encoded = encode_i256(cfg, vartab, lo_lo, lo_hi, hi_lo, hi_hi, item.ty());
            Instr::Set {
                loc: item.loc(),
                res: obj,
                expr: encoded,
            }
        }
        Type::Struct(StructType::UserDefined(n)) => {
            let map = encode_struct_map(item.clone(), cfg, vartab, ns, n);
            Instr::Set {
                loc: Loc::Codegen,
                res: obj,
                expr: map,
            }
        }
        Type::SorobanHandle(_) => Instr::Set {
            loc: Loc::Codegen,
            res: obj,
            expr: item.clone(),
        },
        Type::Array(_, _) => Instr::Set {
            loc: Loc::Codegen,
            res: obj,
            expr: encode_vector(item.clone(), cfg, vartab),
        },

        _ => panic!(
            "{}",
            CodegenError::unsupported_soroban_type(
                item.loc(),
                "by the Soroban encoder",
                item.ty().to_string(ns),
            )
        ),
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

    let check_lo = cfg.new_basic_block("check_lo".to_string());
    let fits_in_56_bits = cfg.new_basic_block("fits_in_56_bits".to_string());
    let should_be_in_host = cfg.new_basic_block("should_be_in_host".to_string());
    let return_block = cfg.new_basic_block("finish".to_string());

    let high_is_zero = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: high.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: high.ty(),
            value: BigInt::from(0_u64),
        }
        .into(),
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: high_is_zero,
            true_block: check_lo,
            false_block: should_be_in_host,
        },
    );

    cfg.set_basic_block(check_lo);

    // check if the low limb fits within the small representation limit
    // signed positive must stay under 55 bits to avoid sign-extension confusion.
    // unsigned can use up to 56 bits
    let shift_amount = match int128_ty {
        Type::Uint(128) => 56_u64,
        Type::Int(128) => 55_u64,
        _ => unreachable!(),
    };

    let lo_shifted = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: lo.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(shift_amount),
        }
        .into(),
        signed: false,
    };

    let lo_is_small = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: lo_shifted.into(),
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
            cond: lo_is_small,
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
        Type::Int(128) => tags::I128_SML,
        Type::Uint(128) => tags::U128_SML,
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

fn encode_u64(cfg: &mut ControlFlowGraph, vartab: &mut Vartable, value: Expression) -> Expression {
    let ret_var = vartab.temp_anonymous(&Type::Uint(64));

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: ret_var,
    };

    vartab.new_dirty_tracker();

    let fits_in_56_bits = cfg.new_basic_block("u64_fits_in_56_bits".to_string());
    let should_be_in_host = cfg.new_basic_block("u64_should_be_in_host".to_string());
    let return_block = cfg.new_basic_block("u64_finish".to_string());

    let high_8_bits = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: value.clone().into(),
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
        left: high_8_bits.into(),
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

    let small_value = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        left: Box::new(value.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(8_u64),
        }),
    };

    let small_value = Expression::Add {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        left: small_value.into(),
        right: Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(tags::U64_SML),
        }
        .into(),
        overflowing: false,
    };

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: ret_var,
            expr: small_value,
        },
    );

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(should_be_in_host);

    cfg.add(
        vartab,
        Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjFromU64.name().to_string(),
            },
            args: vec![value],
        },
    );

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

/// Encodes a 256-bit integer (signed or unsigned) into a Soroban ScVal.
/// This function handles both Int256 and Uint256 types by splitting them into
/// four 64-bit pieces and using the appropriate host functions.
fn encode_i256(
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    lo_lo: Expression,
    lo_hi: Expression,
    hi_lo: Expression,
    hi_hi: Expression,
    int256_ty: Type,
) -> Expression {
    let ret_var = vartab.temp_anonymous(&lo_lo.ty());

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: lo_lo.ty().clone(),
        var_no: ret_var,
    };

    // For 256-bit integers, we always use the host functions since they can't fit in a 64-bit ScVal
    let instr = match int256_ty {
        Type::Int(256) => Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjFromI256Pieces.name().to_string(),
            },
            args: vec![hi_hi, hi_lo, lo_hi, lo_lo],
        },
        Type::Uint(256) => Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjFromU256Pieces.name().to_string(),
            },
            args: vec![hi_hi, hi_lo, lo_hi, lo_lo],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, instr);

    ret
}

fn decode_i128(
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    arg: Expression,
    ty: &Type,
) -> Expression {
    let ty: Type = ty.clone();

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
        Type::Int(128) => tags::I128_SML,
        Type::Uint(128) => tags::U128_SML,
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

    let is_signed = matches!(ty, Type::Int(128));

    let value = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: if is_signed {
            Type::Int(64)
        } else {
            Type::Uint(64)
        },
        left: arg.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(8_u64),
        }
        .into(),
        signed: is_signed,
    };

    let extend = match ty {
        Type::Int(128) => Expression::SignExt {
            loc: Loc::Codegen,
            ty: ty.clone(),
            expr: Box::new(value.clone()),
        },
        Type::Uint(128) => Expression::ZeroExt {
            loc: Loc::Codegen,
            ty: ty.clone(),
            expr: Box::new(value.clone()),
        },
        _ => unreachable!(),
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

/// Decodes a 256-bit integer (signed or unsigned) from a Soroban ScVal.
/// This function handles both Int256 and Uint256 types by retrieving
/// the four 64-bit pieces from the host object.
fn decode_i256(
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    arg: Expression,
    ty: &Type,
) -> Expression {
    let ty: Type = ty.clone();

    let ret_var = vartab.temp_anonymous(&ty);

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: ty.clone(),
        var_no: ret_var,
    };

    // For 256-bit integers, we need to extract all four 64-bit pieces
    // lo_lo: bits 0-63
    // lo_hi: bits 64-127
    // hi_lo: bits 128-191
    // hi_hi: bits 192-255

    // Extract lo_lo (bits 0-63)
    let lo_lo_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let lo_lo_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: lo_lo_var_no,
    };

    let get_lo_lo_instr = match ty {
        Type::Int(256) => Instr::Call {
            res: vec![lo_lo_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI256LoLo.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(256) => Instr::Call {
            res: vec![lo_lo_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU256LoLo.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_lo_lo_instr);

    // Extract lo_hi (bits 64-127)
    let lo_hi_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let lo_hi_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: lo_hi_var_no,
    };

    let get_lo_hi_instr = match ty {
        Type::Int(256) => Instr::Call {
            res: vec![lo_hi_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI256LoHi.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(256) => Instr::Call {
            res: vec![lo_hi_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU256LoHi.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_lo_hi_instr);

    // Extract hi_lo (bits 128-191)
    let hi_lo_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let hi_lo_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: hi_lo_var_no,
    };

    let get_hi_lo_instr = match ty {
        Type::Int(256) => Instr::Call {
            res: vec![hi_lo_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI256HiLo.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(256) => Instr::Call {
            res: vec![hi_lo_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU256HiLo.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_hi_lo_instr);

    // Extract hi_hi (bits 192-255)
    let hi_hi_var_no = vartab.temp_anonymous(&Type::Uint(64));
    let hi_hi_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: hi_hi_var_no,
    };

    let get_hi_hi_instr = match ty {
        Type::Int(256) => Instr::Call {
            res: vec![hi_hi_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToI256HiHi.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        Type::Uint(256) => Instr::Call {
            res: vec![hi_hi_var_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU256HiHi.name().to_string(),
            },
            args: vec![arg.clone()],
        },
        _ => unreachable!(),
    };

    cfg.add(vartab, get_hi_hi_instr);

    // Now combine all pieces to form the 256-bit value
    // Start with hi_hi (bits 192-255)
    let mut combined = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(hi_hi_var),
    };

    // Shift left by 64 and add hi_lo (bits 128-191)
    combined = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: ty.clone(),
        left: Box::new(combined),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: ty.clone(),
            value: BigInt::from(64),
        }),
    };

    let hi_lo_extended = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(hi_lo_var),
    };

    combined = Expression::Add {
        loc: Loc::Codegen,
        ty: ty.clone(),
        overflowing: false,
        left: Box::new(combined),
        right: Box::new(hi_lo_extended),
    };

    // Shift left by 64 and add lo_hi (bits 64-127)
    combined = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: ty.clone(),
        left: Box::new(combined),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: ty.clone(),
            value: BigInt::from(64),
        }),
    };

    let lo_hi_extended = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(lo_hi_var),
    };

    combined = Expression::Add {
        loc: Loc::Codegen,
        ty: ty.clone(),
        overflowing: false,
        left: Box::new(combined),
        right: Box::new(lo_hi_extended),
    };

    // Shift left by 64 and add lo_lo (bits 0-63)
    combined = Expression::ShiftLeft {
        loc: Loc::Codegen,
        ty: ty.clone(),
        left: Box::new(combined),
        right: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: ty.clone(),
            value: BigInt::from(64),
        }),
    };

    let lo_lo_extended = Expression::ZeroExt {
        loc: Loc::Codegen,
        ty: ty.clone(),
        expr: Box::new(lo_lo_var),
    };

    combined = Expression::Add {
        loc: Loc::Codegen,
        ty: ty.clone(),
        overflowing: false,
        left: Box::new(combined),
        right: Box::new(lo_lo_extended),
    };

    // Set the final combined value
    let set_instr = Instr::Set {
        loc: pt::Loc::Codegen,
        res: ret_var,
        expr: combined,
    };

    cfg.add(vartab, set_instr);

    ret
}

fn decode_u64(cfg: &mut ControlFlowGraph, vartab: &mut Vartable, arg: Expression) -> Expression {
    let ty = match arg.ty() {
        Type::Ref(inner_ty) => *inner_ty.clone(),
        Type::SorobanHandle(inner_ty) => *inner_ty.clone(),
        _ => arg.ty(),
    };

    let ret_var = vartab.temp_anonymous(&ty);

    let ret = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: ty.clone(),
        var_no: ret_var,
    };

    vartab.new_dirty_tracker();

    let tag = extract_tag(arg.clone());

    let val_is_u64_small = cfg.new_basic_block("u64_val_is_u64_small".to_string());
    let val_is_u32_small = cfg.new_basic_block("u64_val_is_u32_small".to_string());
    let val_in_host = cfg.new_basic_block("u64_val_is_host".to_string());
    let val_not_u64_small = cfg.new_basic_block("u64_val_not_u64_small".to_string());
    let return_block = cfg.new_basic_block("u64_finish".to_string());

    let is_u64_small = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: tag.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(tags::U64_SML),
        }
        .into(),
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: is_u64_small,
            true_block: val_is_u64_small,
            false_block: val_not_u64_small,
        },
    );

    cfg.set_basic_block(val_is_u64_small);

    let u64_small_value = Expression::ShiftRight {
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

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: ret_var,
            expr: u64_small_value,
        },
    );

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(val_not_u64_small);

    // Some host paths (for example VecLen) produce U32Val. Allow widening it
    // when decoding to uint64.
    let is_u32_small = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: tag.into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(tags::U32),
        }
        .into(),
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: is_u32_small,
            true_block: val_is_u32_small,
            false_block: val_in_host,
        },
    );

    cfg.set_basic_block(val_is_u32_small);

    let u32_small_value = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: arg.clone().into(),
        right: Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(32_u64),
        }
        .into(),
        signed: false,
    };

    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: ret_var,
            expr: u32_small_value,
        },
    );

    cfg.add(
        vartab,
        Instr::Branch {
            block: return_block,
        },
    );

    cfg.set_basic_block(val_in_host);

    cfg.add(
        vartab,
        Instr::Call {
            res: vec![ret_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::ObjToU64.name().to_string(),
            },
            args: vec![arg],
        },
    );

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

#[allow(dead_code)]
fn struct_field_key(
    name: &str,
    loc: pt::Loc,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    encode_as_symbol(
        Expression::BytesLiteral {
            loc,
            ty: Type::String,
            value: name.as_bytes().to_vec(),
        },
        cfg,
        vartab,
        ns,
    )
}

fn encode_struct_map(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
    struct_no: usize,
) -> Expression {
    encode_struct_storage(item, cfg, vartab, ns, struct_no)
}

fn encode_struct_storage(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
    struct_no: usize,
) -> Expression {
    let loc = item.loc();
    let field_tys: Vec<Type> = ns.structs[struct_no]
        .fields
        .iter()
        .map(|f| f.ty.clone())
        .collect();

    let mut vec_no = vartab.temp_name("struct_vec", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![vec_no],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VectorNew.name().to_string(),
            },
            args: vec![],
        },
    );

    for (index, field_ty) in field_tys.iter().enumerate() {
        let member = Expression::StructMember {
            loc,
            ty: field_ty.clone(),
            expr: Box::new(item.clone()),
            member: index,
        };
        let loaded = Expression::Load {
            loc: Loc::Codegen,
            ty: field_ty.clone(),
            expr: Box::new(member),
        };
        let encoded = soroban_encode_arg(loaded, cfg, vartab, ns);

        let prev_vec = Expression::Variable {
            loc,
            ty: Type::Uint(64),
            var_no: vec_no,
        };
        let next_vec = vartab.temp_name("struct_vec", &Type::Uint(64));
        cfg.add(
            vartab,
            Instr::Call {
                res: vec![next_vec],
                return_tys: vec![Type::Uint(64)],
                call: InternalCallTy::HostFunction {
                    name: HostFunctions::VecPushBack.name().to_string(),
                },
                args: vec![prev_vec, encoded],
            },
        );
        vec_no = next_vec;
    }

    Expression::Variable {
        loc,
        ty: Type::Uint(64),
        var_no: vec_no,
    }
}

fn encode_vector(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let len = Expression::Builtin {
        loc: item.loc(),
        tys: vec![Type::Uint(32)],
        kind: Builtin::ArrayLength,
        args: vec![item.clone()],
    };

    let data_ptr = Expression::VectorData {
        pointer: Box::new(item.clone()),
    };

    // VectorNewFromLinearMemory expects (ptr_u32val, len_u32val).
    let encoded_ptr = encode_object(item.loc(), data_ptr, 32, tags::U32);
    let encoded_len = encode_object(item.loc(), len, 32, tags::U32);

    let obj = vartab.temp_name("vec_obj", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![obj],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::VectorNewFromLinearMemory.name().to_string(),
            },
            args: vec![encoded_ptr, encoded_len],
        },
    );

    Expression::Variable {
        loc: item.loc(),
        ty: Type::Uint(64),
        var_no: obj,
    }
}

fn decode_struct_storage(
    vec_object: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    struct_no: usize,
    ns: &Namespace,
    struct_ty: Type,
) -> Expression {
    let field_tys: Vec<Type> = ns.structs[struct_no]
        .fields
        .iter()
        .map(|f| f.ty.clone())
        .collect();

    let mut members = Vec::new();

    for (index, ty) in field_tys.iter().enumerate() {
        let idx_val = encode_object(
            Loc::Codegen,
            Expression::NumberLiteral {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                value: BigInt::from(index),
            },
            32,
            tags::U32,
        );

        let elem_no = vartab.temp_name("struct_field_val", &Type::Uint(64));
        cfg.add(
            vartab,
            Instr::Call {
                res: vec![elem_no],
                return_tys: vec![Type::Uint(64)],
                call: InternalCallTy::HostFunction {
                    name: HostFunctions::VecGet.name().to_string(),
                },
                args: vec![vec_object.clone(), idx_val],
            },
        );
        let elem = Expression::Variable {
            loc: Loc::Codegen,
            ty: Type::Uint(64),
            var_no: elem_no,
        };

        let decoded = soroban_decode_arg(elem, cfg, vartab, ns, Some(ty.clone()));
        members.push(decoded);
    }

    Expression::StructLiteral {
        loc: Loc::Codegen,
        ty: struct_ty,
        values: members,
    }
}

fn decode_struct_map(
    arg: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    struct_no: usize,
    ns: &Namespace,
    struct_ty: Type,
) -> Expression {
    decode_struct_storage(arg, cfg, vartab, struct_no, ns, struct_ty)
}

pub(crate) fn encode_as_symbol(
    item: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) -> Expression {
    let loc = item.loc();

    let (ptr_u32val, len_u32val) = match &item {
        Expression::BytesLiteral { value, .. } => {
            let ptr = encode_object(
                loc,
                Expression::VectorData {
                    pointer: Box::new(item.clone()),
                },
                32,
                tags::U32,
            );
            let len = encode_object(
                loc,
                Expression::NumberLiteral {
                    loc,
                    ty: Type::Uint(32),
                    value: BigInt::from(value.len()),
                },
                32,
                tags::U32,
            );
            (ptr, len)
        }
        Expression::AllocDynamicBytes { size, .. } => {
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

            let len = Expression::Add {
                loc: Loc::Codegen,
                ty: Type::Uint(64),
                overflowing: true,
                left: Box::new(sesa),
                right: Box::new(Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Uint(64),
                    value: BigInt::from(4),
                }),
            };
            (encoded, len)
        }
        _ => {
            unreachable!(
                "encode_as_symbol only accepts BytesLiteral :- {:?}",
                item.clone()
            );
        }
    };

    let sym_var = vartab.temp_name("symbol_obj", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![sym_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::SymbolNewFromLinearMemory.name().to_string(),
            },
            args: vec![ptr_u32val, len_u32val],
        },
    );

    Expression::Variable {
        loc,
        ty: Type::Uint(64),
        var_no: sym_var,
    }
}

pub(crate) fn encode_object(loc: pt::Loc, value: Expression, shift: u64, tag: u64) -> Expression {
    let shifted = Expression::ShiftLeft {
        loc,
        ty: Type::Uint(64),
        left: Box::new(Expression::ZeroExt {
            loc,
            ty: Type::Uint(64),
            expr: Box::new(value),
        }),
        right: Box::new(Expression::NumberLiteral {
            loc,
            ty: Type::Uint(64),
            value: BigInt::from(shift),
        }),
    };

    Expression::Add {
        loc,
        ty: Type::Uint(64),
        left: Box::new(shifted),
        right: Box::new(Expression::NumberLiteral {
            loc,
            ty: Type::Uint(64),
            value: BigInt::from(tag),
        }),
        overflowing: false,
    }
}

pub(crate) fn decode_object(loc: pt::Loc, tagged: Expression, shift: u64) -> Expression {
    Expression::Trunc {
        loc,
        ty: Type::Uint(32),
        expr: Box::new(Expression::ShiftRight {
            loc,
            ty: Type::Uint(64),
            left: Box::new(tagged),
            right: Box::new(Expression::NumberLiteral {
                loc,
                ty: Type::Uint(64),
                value: BigInt::from(shift),
            }),
            signed: false,
        }),
    }
}

pub(crate) fn decode_string(
    handle: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let loc = Loc::Codegen;

    let raw_len_var = vartab.temp_name("str_len_raw", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![raw_len_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::StringLen.name().to_string(),
            },
            args: vec![handle.clone()],
        },
    );
    let raw_len = Expression::Variable {
        loc,
        ty: Type::Uint(64),
        var_no: raw_len_var,
    };

    let len_u32 = decode_object(loc, raw_len, 32);

    let buf_var = vartab.temp_name("str_buf", &Type::String);
    cfg.add(
        vartab,
        Instr::Set {
            loc,
            res: buf_var,
            expr: Expression::AllocDynamicBytes {
                loc,
                ty: Type::String,
                size: Box::new(len_u32.clone()),
                initializer: None,
            },
        },
    );
    let buf = Expression::Variable {
        loc,
        ty: Type::String,
        var_no: buf_var,
    };

    let lm_pos = encode_object(
        loc,
        Expression::VectorData {
            pointer: Box::new(buf.clone()),
        },
        32,
        tags::U32,
    );

    let src_pos = Expression::NumberLiteral {
        loc,
        ty: Type::Uint(64),
        value: BigInt::from(tags::U32),
    };

    let len_u32val = encode_object(loc, len_u32, 32, tags::U32);

    let unused = vartab.temp_name("str_copy_ret", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![unused],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::StringCopyToLinearMemory.name().to_string(),
            },
            args: vec![handle, src_pos, lm_pos, len_u32val],
        },
    );

    buf
}

pub(crate) fn decode_bytes(
    handle: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let loc = Loc::Codegen;

    let raw_len_var = vartab.temp_name("bytes_len_raw", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![raw_len_var],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::BytesLen.name().to_string(),
            },
            args: vec![handle.clone()],
        },
    );
    let raw_len = Expression::Variable {
        loc,
        ty: Type::Uint(64),
        var_no: raw_len_var,
    };

    let len_u32 = decode_object(loc, raw_len, 32);

    let buf_var = vartab.temp_name("bytes_buf", &Type::DynamicBytes);
    cfg.add(
        vartab,
        Instr::Set {
            loc,
            res: buf_var,
            expr: Expression::AllocDynamicBytes {
                loc,
                ty: Type::DynamicBytes,
                size: Box::new(len_u32.clone()),
                initializer: None,
            },
        },
    );
    let buf = Expression::Variable {
        loc,
        ty: Type::DynamicBytes,
        var_no: buf_var,
    };

    let lm_pos = encode_object(
        loc,
        Expression::VectorData {
            pointer: Box::new(buf.clone()),
        },
        32,
        tags::U32,
    );
    let src_pos = Expression::NumberLiteral {
        loc,
        ty: Type::Uint(64),
        value: BigInt::from(tags::U32), // U32Val(0)
    };
    let len_u32val = encode_object(loc, len_u32, 32, tags::U32);

    let unused = vartab.temp_name("bytes_copy_ret", &Type::Uint(64));
    cfg.add(
        vartab,
        Instr::Call {
            res: vec![unused],
            return_tys: vec![Type::Uint(64)],
            call: InternalCallTy::HostFunction {
                name: HostFunctions::BytesCopyToLinearMemory.name().to_string(),
            },
            args: vec![handle, src_pos, lm_pos, len_u32val],
        },
    );

    buf
}

fn decode_vector(
    vec_object: Expression,
    elem_ty: &Type,
    _ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let vec_len = vartab.temp_name("vec_len", &Type::Uint(64));

    let get_len_instr = Instr::Call {
        res: vec![vec_len],
        return_tys: vec![Type::Uint(64)],
        call: crate::codegen::cfg::InternalCallTy::HostFunction {
            name: HostFunctions::VecLen.name().to_string(),
        },
        args: vec![vec_object.clone()],
    };

    cfg.add(vartab, get_len_instr);

    let len_var = Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        var_no: vec_len,
    };

    let decoded_len_u64 = Expression::ShiftRight {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(64),
        left: Box::new(len_var.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(64),
            value: BigInt::from(32),
        }),
        signed: false,
    };

    let decoded_len_u32 = Expression::Trunc {
        loc: Loc::Codegen,
        ty: Type::Uint(32),
        expr: Box::new(decoded_len_u64.clone()),
    };

    let decoded_array_ty = Type::Array(
        Box::new(Type::SorobanHandle(Box::new(elem_ty.clone()))),
        vec![ArrayLength::Dynamic],
    );
    let decoded_buffer_var = vartab.temp_name("vector_data_decoded", &decoded_array_ty);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: decoded_buffer_var,
            expr: Expression::AllocDynamicBytes {
                loc: Loc::Codegen,
                ty: decoded_array_ty.clone(),
                size: Box::new(decoded_len_u32.clone()),
                initializer: None,
            },
        },
    );

    let decoded_buffer = Expression::Variable {
        loc: Loc::Codegen,
        ty: decoded_array_ty,
        var_no: decoded_buffer_var,
    };

    let data_location = Expression::VectorData {
        pointer: decoded_buffer.clone().into(),
    };

    let data_location = encode_object(Loc::Codegen, data_location, 32, tags::U32);
    let unused = vartab.temp_name("unused_void_return", &Type::Uint(64));
    let unpack_instr = Instr::Call {
        res: vec![unused],
        return_tys: vec![Type::Uint(64)],
        call: crate::codegen::cfg::InternalCallTy::HostFunction {
            name: HostFunctions::VecUnpackToLinearMemory.name().to_string(),
        },
        args: vec![vec_object.clone(), data_location, len_var],
    };

    cfg.add(vartab, unpack_instr);

    decoded_buffer
}
