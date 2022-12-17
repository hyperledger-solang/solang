// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Namespace, Parameter, RetrieveType, Type};
use num_bigint::BigInt;
use solang_parser::pt::Loc;

use super::increment_four;

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
                    increment_four(length)
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
