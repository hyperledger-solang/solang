// SPDX-License-Identifier: Apache-2.0

//! Ethereum ABI encoding for the RISC-V target.
//!
//! Only static types are supported: value types take one 32-byte word, and
//! fixed-size arrays and structs are their elements laid out inline, so the
//! size of every encoded value is known at compile time.
//!
//! `Instr::WriteBuffer` and `Builtin::ReadFromBuffer` byte-swap values typed
//! `Type::Bytes(n)`, so values are widened to 256 bits and accessed as
//! `Type::Bytes(32)` to get big-endian words.

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::interface::TargetCodegen;
use crate::codegen::revert::{assert_failure, SolidityError};
use crate::codegen::targets::abi::{
    buffer_validator::BufferValidator, load_struct_member, AbiEncoding,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type, Type::Uint};
use num_bigint::BigInt;
use solang_parser::pt::Loc::Codegen;
use std::collections::HashMap;

const WORD: u64 = 32;

/// The encoded size of a static type.
pub(crate) fn static_size(ty: &Type, ns: &Namespace) -> BigInt {
    match ty {
        Type::Ref(ty) | Type::StorageRef(_, ty) => static_size(ty, ns),
        Type::UserType(_) => static_size(&ty.clone().unwrap_user_type(ns), ns),
        Type::Uint(_)
        | Type::Int(_)
        | Type::Bool
        | Type::Enum(_)
        | Type::Value
        | Type::Address(_)
        | Type::Contract(_)
        | Type::Bytes(_) => WORD.into(),
        Type::Array(elem_ty, dims) => {
            let elems: BigInt = dims
                .iter()
                .map(|dim| {
                    dim.array_length()
                        .cloned()
                        .unwrap_or_else(|| unsupported(ty, ns))
                })
                .product();
            elems * static_size(elem_ty, ns)
        }
        Type::Struct(struct_ty) => struct_ty
            .definition(ns)
            .fields
            .iter()
            .map(|field| static_size(&field.ty, ns))
            .sum(),
        _ => unsupported(ty, ns),
    }
}

fn unsupported(ty: &Type, ns: &Namespace) -> ! {
    unimplemented!(
        "the RISC-V target cannot ABI encode `{}` yet: only static types are supported",
        ty.to_string(ns)
    )
}

fn number(value: BigInt) -> Expression {
    Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value,
    }
}

fn is_fixed_array(dims: &[ArrayLength]) -> bool {
    dims.iter().all(|dim| matches!(dim, ArrayLength::Fixed(_)))
}

pub(crate) struct EthAbiEncoding {
    storage_cache: HashMap<usize, Expression>,
}

impl EthAbiEncoding {
    pub fn new() -> Self {
        Self {
            storage_cache: HashMap::new(),
        }
    }

    /// Widen a value type to the `Uint(256)` holding its ABI word.
    fn widen(expr: &Expression, ns: &Namespace) -> Expression {
        let ty = expr.ty().unwrap_user_type(ns);

        let widened = match &ty {
            // The cast converts the big-endian byte array to a number.
            Type::Address(_) | Type::Contract(_) => {
                return Expression::Cast {
                    loc: Codegen,
                    ty: Uint(256),
                    expr: expr.clone().into(),
                }
            }
            Type::Int(256) | Type::Uint(256) | Type::Bytes(32) | Type::Value => expr.clone(),
            Type::Int(_) => Expression::SignExt {
                loc: Codegen,
                ty: Type::Int(256),
                expr: expr.clone().into(),
            },
            // bytesN is left-aligned in its word.
            Type::Bytes(n) => Expression::ShiftLeft {
                loc: Codegen,
                ty: Uint(256),
                left: Expression::ZeroExt {
                    loc: Codegen,
                    ty: Uint(256),
                    expr: expr.clone().into(),
                }
                .into(),
                right: Expression::NumberLiteral {
                    loc: Codegen,
                    ty: Uint(256),
                    value: BigInt::from((WORD - *n as u64) * 8),
                }
                .into(),
            },
            _ => Expression::ZeroExt {
                loc: Codegen,
                ty: Uint(256),
                expr: expr.clone().into(),
            },
        };

        if widened.ty() == Uint(256) {
            widened
        } else {
            Expression::Cast {
                loc: Codegen,
                ty: Uint(256),
                expr: widened.into(),
            }
        }
    }

    fn encode_struct_fields(
        &mut self,
        expr: &Expression,
        struct_ty: &StructType,
        buffer: &Expression,
        offset: &Expression,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let mut field_offset = offset.clone();
        for (i, field) in struct_ty.definition(ns).fields.iter().enumerate() {
            let member = load_struct_member(field.ty.clone(), expr.clone(), i, ns);
            let size = self.encode(&member, buffer, &field_offset, arg_no, ns, vartab, cfg);
            field_offset = field_offset.add_u32(size);
        }
        number(static_size(&Type::Struct(*struct_ty), ns))
    }

    /// Revert unless `cond` holds. Solidity rejects calldata whose words are
    /// not the canonical encoding of their type in the same way.
    fn require(
        cond: Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        let invalid = cfg.new_basic_block("abi_invalid_value".into());
        let valid = cfg.new_basic_block("abi_valid_value".into());
        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: valid,
                false_block: invalid,
            },
        );
        cfg.set_basic_block(invalid);
        assert_failure(&Codegen, SolidityError::Empty, ns, cfg, vartab);
        cfg.set_basic_block(valid);
    }

    fn read_value_type(
        &self,
        buffer: &Expression,
        offset: &Expression,
        ty: &Type,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = number(WORD.into());
        validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

        let word_var = vartab.temp_anonymous(&Uint(256));
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: word_var,
                expr: Expression::Cast {
                    loc: Codegen,
                    ty: Uint(256),
                    expr: Expression::Builtin {
                        loc: Codegen,
                        tys: vec![Type::Bytes(32)],
                        kind: Builtin::ReadFromBuffer,
                        args: vec![buffer.clone(), offset.clone()],
                    }
                    .into(),
                },
            },
        );
        let word = Expression::Variable {
            loc: Codegen,
            ty: Uint(256),
            var_no: word_var,
        };

        let value = match ty {
            Type::Uint(256) | Type::Int(256) => word.clone(),
            Type::Bytes(n) if *n < 32 => Expression::Trunc {
                loc: Codegen,
                ty: ty.clone(),
                expr: Expression::ShiftRight {
                    loc: Codegen,
                    ty: Uint(256),
                    left: word.clone().into(),
                    right: Expression::NumberLiteral {
                        loc: Codegen,
                        ty: Uint(256),
                        value: BigInt::from((WORD - *n as u64) * 8),
                    }
                    .into(),
                    signed: false,
                }
                .into(),
            },
            Type::Address(_) | Type::Contract(_) | Type::Bytes(_) | Type::Value => {
                Expression::Cast {
                    loc: Codegen,
                    ty: ty.clone(),
                    expr: word.clone().into(),
                }
            }
            _ => Expression::Trunc {
                loc: Codegen,
                ty: ty.clone(),
                expr: word.clone().into(),
            },
        };

        let read_var = vartab.temp_anonymous(ty);
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: read_var,
                expr: value,
            },
        );
        let read = Expression::Variable {
            loc: Codegen,
            ty: ty.clone(),
            var_no: read_var,
        };

        match ty {
            Type::Uint(256) | Type::Int(256) | Type::Bytes(32) | Type::Value => (),
            Type::Enum(no) => Self::require(
                Expression::Less {
                    loc: Codegen,
                    signed: false,
                    left: word.into(),
                    right: Expression::NumberLiteral {
                        loc: Codegen,
                        ty: Uint(256),
                        value: ns.enums[*no].values.len().into(),
                    }
                    .into(),
                },
                ns,
                vartab,
                cfg,
            ),
            // The word must be what encoding the decoded value gives back.
            _ => Self::require(
                Expression::Equal {
                    loc: Codegen,
                    left: word.into(),
                    right: Self::widen(&read, ns).into(),
                },
                ns,
                vartab,
                cfg,
            ),
        }

        read
    }
}

impl AbiEncoding for EthAbiEncoding {
    fn size_width(
        &self,
        _size: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        unimplemented!("dynamically sized types are not supported on RISC-V yet")
    }

    fn encode(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let ty = expr.ty().unwrap_user_type(ns);

        match &ty {
            Type::Uint(_)
            | Type::Int(_)
            | Type::Bool
            | Type::Enum(_)
            | Type::Value
            | Type::Address(_)
            | Type::Contract(_)
            | Type::Bytes(_) => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: Expression::Cast {
                            loc: Codegen,
                            ty: Type::Bytes(32),
                            expr: Self::widen(expr, ns).into(),
                        },
                    },
                );
                number(WORD.into())
            }
            Type::Struct(struct_ty) => {
                self.encode_struct_fields(expr, struct_ty, buffer, offset, arg_no, ns, vartab, cfg)
            }
            // A struct reference is a pointer to the struct, not something to load.
            Type::Ref(inner) if matches!(**inner, Type::Struct(_)) => {
                let Type::Struct(struct_ty) = &**inner else {
                    unreachable!()
                };
                self.encode_struct_fields(expr, struct_ty, buffer, offset, arg_no, ns, vartab, cfg)
            }
            // Element by element, as the shared `encode_array` would memcpy
            // the in-memory representation.
            Type::Array(_, dims) if is_fixed_array(dims) => {
                let offset_var = vartab.temp_anonymous(&Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: offset_var,
                        expr: offset.clone(),
                    },
                );
                self.encode_complex_array(
                    expr,
                    arg_no,
                    dims,
                    buffer,
                    offset_var,
                    dims.len() - 1,
                    ns,
                    vartab,
                    cfg,
                    &mut Vec::new(),
                );
                number(static_size(&ty, ns))
            }
            Type::Ref(inner) => {
                let loaded = Expression::Load {
                    loc: Codegen,
                    ty: *inner.clone(),
                    expr: expr.clone().into(),
                };
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
            Type::StorageRef(..) => {
                let loaded = self.storage_cache_remove(arg_no).unwrap();
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
            _ => unsupported(&ty, ns),
        }
    }

    fn encode_size(
        &mut self,
        _expr: &Expression,
        _buffer: &Expression,
        _offset: &Expression,
        _ns: &Namespace,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        unimplemented!("dynamically sized types are not supported on RISC-V yet")
    }

    fn read_from_buffer(
        &self,
        buffer: &Expression,
        offset: &Expression,
        ty: &Type,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let ty = ty.clone().unwrap_user_type(ns);
        let size = number(static_size(&ty, ns));

        let value = match &ty {
            Type::Struct(struct_ty) => {
                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let mut field_offset = offset.clone();
                let values = struct_ty
                    .definition(ns)
                    .fields
                    .iter()
                    .map(|field| {
                        let (value, advance) = self.read_from_buffer(
                            buffer,
                            &field_offset,
                            &field.ty,
                            validator,
                            ns,
                            vartab,
                            cfg,
                        );
                        field_offset = field_offset.clone().add_u32(advance);
                        value
                    })
                    .collect();

                let struct_var = vartab.temp_anonymous(&ty);
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: struct_var,
                        expr: Expression::StructLiteral {
                            loc: Codegen,
                            ty: ty.clone(),
                            values,
                        },
                    },
                );
                Expression::Variable {
                    loc: Codegen,
                    ty: ty.clone(),
                    var_no: struct_var,
                }
            }
            // Element by element, as the shared `decode_array` would memcpy
            // into the in-memory representation.
            Type::Array(elem_ty, dims) if is_fixed_array(dims) => {
                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let array_var = vartab.temp_anonymous(&ty);
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: array_var,
                        expr: Expression::ArrayLiteral {
                            loc: Codegen,
                            ty: ty.clone(),
                            dimensions: vec![],
                            values: vec![],
                        },
                    },
                );
                let array = Expression::Variable {
                    loc: Codegen,
                    ty: ty.clone(),
                    var_no: array_var,
                };

                let offset_var = vartab.temp_anonymous(&Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: offset_var,
                        expr: offset.clone(),
                    },
                );
                let offset_expr = Expression::Variable {
                    loc: Codegen,
                    ty: Uint(32),
                    var_no: offset_var,
                };

                self.decode_complex_array(
                    &array,
                    buffer,
                    offset_var,
                    &offset_expr,
                    dims.len() - 1,
                    elem_ty,
                    dims,
                    validator,
                    ns,
                    vartab,
                    cfg,
                    &mut Vec::new(),
                );
                array
            }
            Type::Uint(_)
            | Type::Int(_)
            | Type::Bool
            | Type::Enum(_)
            | Type::Value
            | Type::Address(_)
            | Type::Contract(_)
            | Type::Bytes(_) => {
                self.read_value_type(buffer, offset, &ty, validator, ns, vartab, cfg)
            }
            _ => unsupported(&ty, ns),
        };

        (value, size)
    }

    fn get_expr_size(
        &mut self,
        _arg_no: usize,
        expr: &Expression,
        ns: &Namespace,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
        _target: &dyn TargetCodegen,
    ) -> Expression {
        number(static_size(&expr.ty(), ns))
    }

    fn calculate_struct_size(
        &mut self,
        _arg_no: usize,
        _expr: &Expression,
        struct_ty: &StructType,
        ns: &Namespace,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
        _target: &dyn TargetCodegen,
    ) -> Expression {
        number(static_size(&Type::Struct(*struct_ty), ns))
    }

    fn encode_external_function(
        &mut self,
        _expr: &Expression,
        _buffer: &Expression,
        _offset: &Expression,
        _ns: &Namespace,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        unimplemented!("the RISC-V target cannot ABI encode external functions yet")
    }

    fn decode_external_function(
        &self,
        _buffer: &Expression,
        _offset: &Expression,
        _ty: &Type,
        _validator: &mut BufferValidator,
        _ns: &Namespace,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        unimplemented!("the RISC-V target cannot ABI decode external functions yet")
    }

    fn retrieve_array_length(
        &self,
        _buffer: &Expression,
        _offset: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> (usize, Expression) {
        unimplemented!("dynamically sized arrays are not supported on RISC-V yet")
    }

    fn calculate_string_size(
        &self,
        _expr: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        unimplemented!("dynamically sized types are not supported on RISC-V yet")
    }

    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression> {
        self.storage_cache.remove(&arg_no)
    }

    fn is_packed(&self) -> bool {
        false
    }

    /// Constant-fold revert data such as `Panic(uint256)`: a 4 byte selector
    /// followed by 32-byte words.
    fn const_encode(&self, args: &[Expression]) -> Option<Vec<u8>> {
        let mut result = Vec::new();

        for arg in args {
            match arg {
                Expression::NumberLiteral { ty, value, .. } => {
                    let width = match ty {
                        Type::Bytes(n) => *n as usize,
                        Type::Uint(_) | Type::Int(_) => WORD as usize,
                        _ => return None,
                    };

                    let (_, bytes) = value.to_bytes_be();
                    if bytes.len() > width {
                        return None;
                    }

                    result.extend(std::iter::repeat_n(0, width - bytes.len()));
                    result.extend_from_slice(&bytes);
                }
                _ => return None,
            }
        }

        Some(result)
    }
}
