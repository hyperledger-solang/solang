// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::create_encoder;
use crate::codegen::vartable::Vartable;
use crate::codegen::Expression;
use crate::codegen::HostFunctions;
use crate::sema::ast::{Namespace, RetrieveType, Type, Type::Uint};
use num_bigint::BigInt;
use num_traits::Zero;
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
) -> (Expression, Expression) {
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

    for (arg_no, item) in args.iter().enumerate() {
        println!("item {:?}", item);

        let obj = vartab.temp_name(format!("obj_{arg_no}").as_str(), &Type::Uint(64));

        let transformer = match item.ty() {
            Type::String => {
                let inp = Expression::VectorData {
                    pointer: Box::new(item.clone()),
                };

                let encoded = Expression::ShiftLeft {
                    loc: Loc::Codegen,
                    ty: Uint(64),
                    left: Box::new(inp),
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

                let len = if let Expression::AllocDynamicBytes { size, .. } = item {
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
                } else {
                    unreachable!()
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
            Type::Uint(64) => {
                let shift_left = Expression::ShiftLeft {
                    loc: *loc,
                    ty: Type::Uint(64),
                    left: Box::new(item.clone()),
                    right: Box::new(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(8),
                    }),
                };

                let added = Expression::Add {
                    loc: *loc,
                    ty: Type::Uint(64),
                    left: Box::new(shift_left),
                    right: Box::new(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(6),
                    }),
                    overflowing: false,
                };

                Instr::Set {
                    loc: *loc,
                    res: obj,
                    expr: added,
                }
            }
            Type::Address(_) => {
                // pass the address as is
                Instr::Set {
                    loc: *loc,
                    res: obj,
                    expr: item.clone(),
                }
            }
            // FIXME: Implement encoding/decoding for i128
            Type::Int(128) => Instr::Set {
                loc: *loc,
                res: obj,
                expr: item.clone(),
            },
            _ => todo!("Type not yet supported"),
        };

        let var = Expression::Variable {
            loc: *loc,
            ty: Type::Uint(64),
            var_no: obj,
        };

        cfg.add(vartab, transformer);

        let advance = encoder.encode(&var, &buffer, &offset, arg_no, ns, vartab, cfg);
        offset = Expression::Add {
            loc: *loc,
            ty: Uint(64),
            overflowing: false,
            left: offset.into(),
            right: advance.into(),
        };
    }

    (buffer, size_expr)
}

pub fn soroban_decode(
    loc: &Loc,
    buffer: &Expression,
    _types: &[Type],
    _ns: &Namespace,
    _vartab: &mut Vartable,
    _cfg: &mut ControlFlowGraph,
    _buffer_size_expr: Option<Expression>,
) -> Vec<Expression> {
    let mut returns = Vec::new();

    let loaded_val = Expression::Load {
        loc: Loc::Codegen,
        ty: Type::Uint(64),
        expr: Box::new(buffer.clone()),
    };

    let decoded_val = Expression::ShiftRight {
        loc: *loc,
        ty: Type::Uint(64),
        left: Box::new(loaded_val.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(64),
            value: BigInt::from(8),
        }),
        signed: false,
    };

    returns.push(decoded_val);

    returns
}
