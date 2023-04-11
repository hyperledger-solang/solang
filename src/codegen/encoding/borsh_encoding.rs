// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::StructType;
use crate::sema::ast::{Namespace, Type, Type::Uint};
use num_bigint::BigInt;
use solang_parser::pt::Loc::Codegen;
use std::collections::HashMap;
use std::ops::AddAssign;

use super::buffer_validator::BufferValidator;

/// This struct implements the trait AbiEncoding for Borsh encoding
pub(super) struct BorshEncoding {
    storage_cache: HashMap<usize, Expression>,
    /// Are we packed encoding?
    packed_encoder: bool,
}

impl AbiEncoding for BorshEncoding {
    fn size_width(
        &self,
        _size: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: 4.into(),
        }
    }

    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        self.encode_int(expr, buffer, offset, ns, vartab, cfg, 32)
    }

    fn encode_external_function(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: expr.external_function_selector(),
            },
        );
        let mut size = Type::FunctionSelector.memory_size_of(ns);
        let offset = Expression::Add {
            loc: Codegen,
            ty: Uint(32),
            overflowing: false,
            left: offset.clone().into(),
            right: Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: size.clone(),
            }
            .into(),
        };
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                value: expr.external_function_address(),
                offset,
            },
        );
        size.add_assign(BigInt::from(ns.address_length));
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: size,
        }
    }

    fn retrieve_array_length(
        &self,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (usize, Expression) {
        let array_length = vartab.temp_anonymous(&Uint(32));
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: array_length,
                expr: Expression::Builtin {
                    loc: Codegen,
                    tys: vec![Uint(32)],
                    kind: Builtin::ReadFromBuffer,
                    args: vec![buffer.clone(), offset.clone()],
                },
            },
        );
        (
            array_length,
            Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 4.into(),
            },
        )
    }

    fn decode_external_function(
        &self,
        buffer: &Expression,
        offset: &Expression,
        ty: &Type,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let selector_size = Type::FunctionSelector.memory_size_of(ns);
        // External function has selector + address
        let size = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: BigInt::from(ns.address_length) + &selector_size,
        };
        validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

        let selector = Expression::Builtin {
            loc: Codegen,
            tys: vec![Type::FunctionSelector],
            kind: Builtin::ReadFromBuffer,
            args: vec![buffer.clone(), offset.clone()],
        };

        let new_offset = offset.clone().add_u32(Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: selector_size,
        });

        let address = Expression::Builtin {
            loc: Codegen,
            tys: vec![Type::Address(false)],
            kind: Builtin::ReadFromBuffer,
            args: vec![buffer.clone(), new_offset],
        };

        let external_func = Expression::Cast {
            loc: Codegen,
            ty: ty.clone(),
            expr: Box::new(Expression::StructLiteral {
                loc: Codegen,
                ty: Type::Struct(StructType::ExternalFunction),
                values: vec![selector, address],
            }),
        };

        (external_func, size)
    }

    fn calculate_string_size(
        &self,
        expr: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        // When encoding a variable length array, the total size is "length (u32)" + elements
        let length = Expression::Builtin {
            loc: Codegen,
            tys: vec![Uint(32)],
            kind: Builtin::ArrayLength,
            args: vec![expr.clone()],
        };

        if self.is_packed() {
            length
        } else {
            length.add_u32(Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 4.into(),
            })
        }
    }

    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression> {
        self.storage_cache.remove(&arg_no)
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }
}

impl BorshEncoding {
    pub fn new(packed: bool) -> BorshEncoding {
        BorshEncoding {
            storage_cache: HashMap::new(),
            packed_encoder: packed,
        }
    }
}
