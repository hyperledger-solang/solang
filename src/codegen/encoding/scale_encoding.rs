// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Namespace, Parameter, RetrieveType, Type};
use num_bigint::BigInt;
use solang_parser::pt::Loc;

use super::{increment_by, increment_four};

/// This struct implements the trait AbiEncoding for Parity's Scale encoding
pub(super) struct ScaleEncoding {
    /// Are we pakced encoding?
    packed_encoder: bool,
}

impl ScaleEncoding {
    pub fn new(packed: bool) -> ScaleEncoding {
        ScaleEncoding {
            packed_encoder: packed,
        }
    }
}

impl AbiEncoding for ScaleEncoding {
    /// SALE encoding uses "compact" integer for sizes.
    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let loc = Loc::Codegen;

        let small = cfg.new_basic_block("small".into());
        let medium = cfg.new_basic_block("medium".into());
        let medium_or_big = cfg.new_basic_block("medium_or_big".into());
        let big = cfg.new_basic_block("big".into());
        let done = cfg.new_basic_block("done".into());
        let fail = cfg.new_basic_block("fail".into());
        let prepare = cfg.new_basic_block("prepare".into());

        let compare = Expression::UnsignedMore(
            loc,
            expr.clone().into(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0x40000000.into()).into(),
        );

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond: compare,
                true_block: fail,
                false_block: prepare,
            },
        );

        cfg.set_basic_block(fail);
        cfg.add(vartab, Instr::AssertFailure { encoded_args: None });

        cfg.set_basic_block(prepare);
        let compare = Expression::UnsignedMore(
            Loc::Codegen,
            expr.clone().into(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0x40.into()).into(),
        );
        cfg.add(
            vartab,
            Instr::BranchCond {
                cond: compare,
                true_block: medium_or_big,
                false_block: small,
            },
        );

        cfg.set_basic_block(medium_or_big);
        let compare = Expression::UnsignedMore(
            Loc::Codegen,
            expr.clone().into(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0x4000.into()).into(),
        );
        cfg.add(
            vartab,
            Instr::BranchCond {
                cond: compare,
                true_block: big,
                false_block: medium,
            },
        );

        vartab.new_dirty_tracker();
        let size_variable = vartab.temp_anonymous(&Type::Uint(32));

        let mul = Expression::Multiply(
            Loc::Codegen,
            Type::Uint(32),
            false,
            expr.clone().into(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 4.into()).into(),
        );

        cfg.set_basic_block(small);
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul.clone(),
            },
        );
        cfg.add(
            vartab,
            Instr::Set {
                loc,
                res: size_variable,
                expr: Expression::NumberLiteral(loc, Type::Uint(32), 1.into()),
            },
        );
        cfg.add(vartab, Instr::Branch { block: done });

        cfg.set_basic_block(medium);
        let mul2 = Expression::BitwiseOr(
            loc,
            Type::Uint(32),
            mul.clone().into(),
            Expression::NumberLiteral(loc, Type::Uint(32), 1.into()).into(),
        );
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul2,
            },
        );
        cfg.add(
            vartab,
            Instr::Set {
                loc,
                res: size_variable,
                expr: Expression::NumberLiteral(loc, Type::Uint(32), 2.into()),
            },
        );
        cfg.add(vartab, Instr::Branch { block: done });

        cfg.set_basic_block(big);
        let mul2 = Expression::BitwiseOr(
            loc,
            Type::Uint(32),
            mul.clone().into(),
            Expression::NumberLiteral(loc, Type::Uint(32), 2.into()).into(),
        );
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul2,
            },
        );
        cfg.add(
            vartab,
            Instr::Set {
                loc,
                res: size_variable,
                expr: Expression::NumberLiteral(loc, Type::Uint(32), 4.into()),
            },
        );
        cfg.add(vartab, Instr::Branch { block: done });

        cfg.set_basic_block(done);
        cfg.set_phis(done, vartab.pop_dirty_tracker());
        Expression::Variable(loc, Type::Uint(32), size_variable)
    }

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        _ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size: Option<Expression>,
    ) -> Vec<Expression> {
        assert!(!self.packed_encoder);
        let mut returns: Vec<Expression> = Vec::with_capacity(types.len());
        let mut var_nos: Vec<usize> = Vec::with_capacity(types.len());
        let mut decode_params: Vec<Parameter> = Vec::with_capacity(types.len());

        for item in types {
            let var_no = vartab.temp_anonymous(item);
            var_nos.push(var_no);
            returns.push(Expression::Variable(*loc, item.clone(), var_no));
            decode_params.push(Parameter {
                loc: Loc::Codegen,
                id: None,
                ty: item.clone(),
                ty_loc: None,
                indexed: false,
                readonly: false,
                recursive: false,
            });
        }

        cfg.add(
            vartab,
            Instr::AbiDecode {
                res: var_nos,
                selector: None,
                exception_block: None,
                tys: decode_params,
                data: buffer.clone(),
                data_len: buffer_size,
            },
        );

        returns
    }

    fn storage_cache_insert(&mut self, _arg_no: usize, _expr: Expression) {
        unreachable!("This function is not needed for Scale encoding");
    }

    fn storage_cache_remove(&mut self, _arg_no: usize) -> Option<Expression> {
        todo!()
    }

    fn get_encoding_size(&self, expr: &Expression, ty: &Type, ns: &Namespace) -> Expression {
        match ty {
            Type::Uint(n) | Type::Int(n) => Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(32),
                BigInt::from(n.next_power_of_two() / 8),
            ),

            Type::Enum(_) | Type::Contract(_) | Type::Bool | Type::Address(_) | Type::Bytes(_) => {
                let size = ty.memory_size_of(ns);
                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), size)
            }

            Type::String | Type::DynamicBytes => {
                // When encoding a variable length array, the total size is "length (u32)" + elements
                let length = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![expr.clone()],
                );

                if self.is_packed() {
                    length
                } else {
                    increment_by(
                        length,
                        Expression::NumberLiteral(Loc::Builtin, Type::Uint(32), 1.into()),
                    )
                }
            }

            _ => unreachable!("Type should have the same size for all encoding schemes"),
        }
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }
}

impl ScaleEncoding {
    pub fn abi_encode(
        &mut self,
        loc: &Loc,
        mut args: Vec<Expression>,
        _ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let tys = args.iter().map(|e| e.ty()).collect::<Vec<Type>>();

        let encoded_buffer = vartab.temp_anonymous(&Type::DynamicBytes);
        let mut packed: Vec<Expression> = Vec::new();
        if self.packed_encoder {
            std::mem::swap(&mut packed, &mut args);
        }

        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: encoded_buffer,
                expr: Expression::AbiEncode {
                    loc: *loc,
                    packed,
                    args,
                    tys,
                },
            },
        );

        let encoded_expr = Expression::Variable(*loc, Type::DynamicBytes, encoded_buffer);
        let buffer_len = Expression::Builtin(
            *loc,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![encoded_expr.clone()],
        );

        (encoded_expr, buffer_len)
    }
}
