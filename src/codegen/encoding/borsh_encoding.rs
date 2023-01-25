// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::buffer_validator::BufferValidator;
use crate::codegen::encoding::{
    allocate_array, allow_direct_copy, calculate_array_bytes_size,
    calculate_direct_copy_bytes_size, finish_array_loop, increment_four, retrieve_array_length,
    set_array_loop, AbiEncoding,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type, Type::Uint};
use num_bigint::BigInt;
use num_traits::{One, Zero};
use solang_parser::pt::{Loc, Loc::Codegen};
use std::collections::HashMap;
use std::ops::{Add, AddAssign, MulAssign};

use super::index_array;

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
        Expression::NumberLiteral(Codegen, Uint(32), 4.into())
    }

    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        self.encode_int(expr, buffer, offset, vartab, cfg, 32)
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
        let offset = Expression::Add(
            Codegen,
            Uint(32),
            false,
            offset.clone().into(),
            Expression::NumberLiteral(Codegen, Uint(32), size.clone()).into(),
        );
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                value: expr.external_function_address(),
                offset,
            },
        );
        size.add_assign(BigInt::from(ns.address_length));
        Expression::NumberLiteral(Codegen, Uint(32), size)
    }

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size_expr: Option<Expression>,
    ) -> Vec<Expression> {
        assert!(!self.packed_encoder);
        let buffer_size = vartab.temp_anonymous(&Uint(32));
        if let Some(length_expression) = buffer_size_expr {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: buffer_size,
                    expr: length_expression,
                },
            );
        } else {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: buffer_size,
                    expr: Expression::Builtin(
                        Codegen,
                        vec![Uint(32)],
                        Builtin::ArrayLength,
                        vec![buffer.clone()],
                    ),
                },
            );
        }

        let mut validator = BufferValidator::new(buffer_size, types);

        let mut read_items: Vec<Expression> = vec![Expression::Poison; types.len()];
        let mut offset = Expression::NumberLiteral(*loc, Uint(32), BigInt::zero());

        validator.initialize_validation(&offset, ns, vartab, cfg);

        for (item_no, item) in types.iter().enumerate() {
            validator.set_argument_number(item_no);
            validator.validate_buffer(&offset, ns, vartab, cfg);
            let (read_item, advance) =
                self.read_from_buffer(buffer, &offset, item, &mut validator, ns, vartab, cfg);
            read_items[item_no] = read_item;
            offset = Expression::Add(*loc, Uint(32), false, Box::new(offset), Box::new(advance));
        }

        validator.validate_all_bytes_read(offset, ns, vartab, cfg);

        read_items
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

    /// Read a value of type 'ty' from the buffer at a given offset. Returns an expression
    /// containing the read value and the number of bytes read.
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
        match ty {
            Type::Uint(width) | Type::Int(width) => {
                let encoding_size = width.next_power_of_two();

                let size = Expression::NumberLiteral(Codegen, Uint(32), (encoding_size / 8).into());
                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let read_value = Expression::Builtin(
                    Codegen,
                    vec![ty.clone()],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), offset.clone()],
                );
                let read_var = vartab.temp_anonymous(ty);

                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: read_var,
                        expr: if encoding_size == *width {
                            read_value
                        } else {
                            Expression::Trunc(Codegen, ty.clone(), Box::new(read_value))
                        },
                    },
                );

                let read_expr = Expression::Variable(Codegen, ty.clone(), read_var);
                (read_expr, size)
            }

            Type::Bool
            | Type::Address(_)
            | Type::Contract(_)
            | Type::Enum(_)
            | Type::Value
            | Type::Bytes(_) => {
                let read_bytes = ty.memory_size_of(ns);

                let size = Expression::NumberLiteral(Codegen, Uint(32), read_bytes);
                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let read_value = Expression::Builtin(
                    Codegen,
                    vec![ty.clone()],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), offset.clone()],
                );

                let read_var = vartab.temp_anonymous(ty);
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: read_var,
                        expr: read_value,
                    },
                );

                let read_expr = Expression::Variable(Codegen, ty.clone(), read_var);

                (read_expr, size)
            }

            Type::DynamicBytes | Type::String => {
                // String and Dynamic bytes are encoded as size (uint32) + elements
                validator.validate_offset(increment_four(offset.clone()), ns, vartab, cfg);

                let array_length = retrieve_array_length(buffer, offset, vartab, cfg);

                let size = increment_four(Expression::Variable(Codegen, Uint(32), array_length));
                let offset_to_validate = Expression::Add(
                    Codegen,
                    Uint(32),
                    false,
                    Box::new(size.clone()),
                    Box::new(offset.clone()),
                );

                validator.validate_offset(offset_to_validate, ns, vartab, cfg);
                let allocated_array = allocate_array(ty, array_length, vartab, cfg);

                let advanced_pointer = Expression::AdvancePointer {
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(increment_four(offset.clone())),
                };

                cfg.add(
                    vartab,
                    Instr::MemCopy {
                        source: advanced_pointer,
                        destination: Expression::Variable(Codegen, ty.clone(), allocated_array),
                        bytes: Expression::Variable(Codegen, Uint(32), array_length),
                    },
                );

                (
                    Expression::Variable(Codegen, ty.clone(), allocated_array),
                    size,
                )
            }

            Type::UserType(type_no) => {
                let usr_type = ns.user_types[*type_no].ty.clone();
                self.read_from_buffer(buffer, offset, &usr_type, validator, ns, vartab, cfg)
            }

            Type::ExternalFunction { .. } => {
                let selector_size = Type::FunctionSelector.memory_size_of(ns);
                // Extneral function has selector + address
                let size = Expression::NumberLiteral(
                    Codegen,
                    Uint(32),
                    BigInt::from(ns.address_length).add(&selector_size),
                );

                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let selector = Expression::Builtin(
                    Codegen,
                    vec![Type::FunctionSelector],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), offset.clone()],
                );

                let new_offset = Expression::Add(
                    Codegen,
                    Uint(32),
                    false,
                    offset.clone().into(),
                    Expression::NumberLiteral(Codegen, Uint(32), selector_size).into(),
                );

                let address = Expression::Builtin(
                    Codegen,
                    vec![Type::Address(false)],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), new_offset],
                );

                let external_func = Expression::Cast(
                    Codegen,
                    ty.clone(),
                    Box::new(Expression::StructLiteral(
                        Codegen,
                        Type::Struct(StructType::ExternalFunction),
                        vec![selector, address],
                    )),
                );

                (external_func, size)
            }

            Type::Array(elem_ty, dims) => self.decode_array(
                buffer, offset, ty, elem_ty, dims, validator, ns, vartab, cfg,
            ),

            Type::Slice(elem_ty) => {
                let dims = vec![ArrayLength::Dynamic];
                self.decode_array(
                    buffer, offset, ty, elem_ty, &dims, validator, ns, vartab, cfg,
                )
            }

            Type::Struct(struct_ty) => self.decode_struct(
                buffer,
                offset.clone(),
                ty,
                struct_ty,
                validator,
                ns,
                vartab,
                cfg,
            ),

            Type::Rational
            | Type::Ref(_)
            | Type::StorageRef(..)
            | Type::BufferPointer
            | Type::Unresolved
            | Type::InternalFunction { .. }
            | Type::Unreachable
            | Type::Void
            | Type::FunctionSelector
            | Type::Mapping(..) => unreachable!("Type should not appear on an encoded buffer"),
        }
    }

    /// Given the buffer and the offers, decode an array.
    /// The function returns an expression containing the array and the number of bytes read.
    fn decode_array(
        &self,
        buffer: &Expression,
        offset: &Expression,
        array_ty: &Type,
        elem_ty: &Type,
        dims: &[ArrayLength],
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        // Checks if we can memcpy the elements from the buffer directly to the allocated array
        if allow_direct_copy(array_ty, elem_ty, dims, ns) {
            // Calculate number of elements
            let (bytes_size, offset, var_no) =
                if matches!(dims.last(), Some(&ArrayLength::Fixed(_))) {
                    let elem_no = calculate_direct_copy_bytes_size(dims, elem_ty, ns);
                    let allocated_vector = vartab.temp_anonymous(array_ty);
                    cfg.add(
                        vartab,
                        Instr::Set {
                            loc: Codegen,
                            res: allocated_vector,
                            expr: Expression::ArrayLiteral(
                                Codegen,
                                array_ty.clone(),
                                vec![],
                                vec![],
                            ),
                        },
                    );

                    (
                        Expression::NumberLiteral(Codegen, Uint(32), elem_no),
                        offset.clone(),
                        allocated_vector,
                    )
                } else {
                    validator.validate_offset(increment_four(offset.clone()), ns, vartab, cfg);
                    let array_length = retrieve_array_length(buffer, offset, vartab, cfg);

                    let allocated_array = allocate_array(array_ty, array_length, vartab, cfg);

                    let size = calculate_array_bytes_size(array_length, elem_ty, ns);
                    (size, increment_four(offset.clone()), allocated_array)
                };

            validator.validate_offset_plus_size(&offset, &bytes_size, ns, vartab, cfg);

            let source_address = Expression::AdvancePointer {
                pointer: Box::new(buffer.clone()),
                bytes_offset: Box::new(offset),
            };

            let array_expr = Expression::Variable(Codegen, array_ty.clone(), var_no);
            cfg.add(
                vartab,
                Instr::MemCopy {
                    source: source_address,
                    destination: array_expr.clone(),
                    bytes: bytes_size.clone(),
                },
            );

            let bytes_size = if matches!(dims.last(), Some(ArrayLength::Dynamic)) {
                increment_four(bytes_size)
            } else {
                bytes_size
            };

            (array_expr, bytes_size)
        } else {
            let mut indexes: Vec<usize> = Vec::new();
            let array_var = vartab.temp_anonymous(array_ty);

            // The function decode_complex_array assumes that, if the dimension is fixed,
            // there is no need to allocate an array
            if matches!(dims.last(), Some(ArrayLength::Fixed(_))) {
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: array_var,
                        expr: Expression::ArrayLiteral(Codegen, array_ty.clone(), vec![], vec![]),
                    },
                );
            }

            let offset_var = vartab.temp_anonymous(&Uint(32));
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: offset.clone(),
                },
            );
            let array_var_expr = Expression::Variable(Codegen, array_ty.clone(), array_var);
            let offset_expr = Expression::Variable(Codegen, Uint(32), offset_var);
            self.decode_complex_array(
                &array_var_expr,
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
                &mut indexes,
            );
            // Subtract the original offset from
            // the offset variable to obtain the vector size in bytes
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: Expression::Subtract(
                        Codegen,
                        Uint(32),
                        false,
                        Box::new(offset_expr.clone()),
                        Box::new(offset.clone()),
                    ),
                },
            );
            (array_var_expr, offset_expr)
        }
    }

    /// Decodes a complex array from a borsh encoded buffer
    /// Complex arrays are either dynamic arrays or arrays of dynamic types, like structs.
    /// If this is an array of structs, whose representation in memory is padded, the array is
    /// also complex, because it cannot be memcpy'ed
    fn decode_complex_array(
        &self,
        array_var: &Expression,
        buffer: &Expression,
        offset_var: usize,
        offset_expr: &Expression,
        dimension: usize,
        elem_ty: &Type,
        dims: &[ArrayLength],
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        indexes: &mut Vec<usize>,
    ) {
        // If we have a 'int[3][4][] vec', we can only validate the buffer after we have
        // allocated the outer dimension, i.e., we are about to read a 'int[3][4]' item.
        // Arrays whose elements are dynamic cannot be verified.
        if validator.validation_necessary()
            && !dims[0..(dimension + 1)]
                .iter()
                .any(|d| *d == ArrayLength::Dynamic)
            && !elem_ty.is_dynamic(ns)
        {
            let mut elems = BigInt::one();
            for item in &dims[0..(dimension + 1)] {
                elems.mul_assign(item.array_length().unwrap());
            }
            elems.mul_assign(elem_ty.memory_size_of(ns));
            let elems_size = Expression::NumberLiteral(Codegen, Uint(32), elems);
            validator.validate_offset_plus_size(offset_expr, &elems_size, ns, vartab, cfg);
            validator.validate_array();
        }

        // Dynamic dimensions mean that the subarray we are processing must be allocated in memory.
        if dims[dimension] == ArrayLength::Dynamic {
            let offset_to_validate = increment_four(offset_expr.clone());
            validator.validate_offset(offset_to_validate, ns, vartab, cfg);
            let array_length = retrieve_array_length(buffer, offset_expr, vartab, cfg);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: increment_four(offset_expr.clone()),
                },
            );
            let new_ty = Type::Array(Box::new(elem_ty.clone()), dims[0..(dimension + 1)].to_vec());
            let allocated_array = allocate_array(&new_ty, array_length, vartab, cfg);

            if indexes.is_empty() {
                if let Expression::Variable(_, _, var_no) = array_var {
                    cfg.add(
                        vartab,
                        Instr::Set {
                            loc: Codegen,
                            res: *var_no,
                            expr: Expression::Variable(Codegen, new_ty.clone(), allocated_array),
                        },
                    );
                } else {
                    unreachable!("array_var must be a variable");
                }
            } else {
                // TODO: This is wired up for multidimensional dynamic arrays, but they do no work yet
                // Check https://github.com/hyperledger/solang/issues/932 for more information
                let sub_arr = index_array(array_var.clone(), dims, indexes, true);
                cfg.add(
                    vartab,
                    Instr::Store {
                        dest: sub_arr,
                        data: Expression::Variable(Codegen, new_ty.clone(), allocated_array),
                    },
                );
            }
        }

        let for_loop = set_array_loop(array_var, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if 0 == dimension {
            let (read_expr, advance) =
                self.read_from_buffer(buffer, offset_expr, elem_ty, validator, ns, vartab, cfg);
            let ptr = index_array(array_var.clone(), dims, indexes, true);

            cfg.add(
                vartab,
                Instr::Store {
                    dest: ptr,
                    data: if matches!(read_expr.ty(), Type::Struct(_)) {
                        // Type::Struct is a pointer to a struct. If we are dealing with a vector
                        // of structs, we need to dereference the pointer before storing it at a
                        // given vector index.
                        Expression::Load(Codegen, read_expr.ty(), Box::new(read_expr))
                    } else {
                        read_expr
                    },
                },
            );
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: Expression::Add(
                        Codegen,
                        Uint(32),
                        false,
                        Box::new(advance),
                        Box::new(offset_expr.clone()),
                    ),
                },
            );
        } else {
            self.decode_complex_array(
                array_var,
                buffer,
                offset_var,
                offset_expr,
                dimension - 1,
                elem_ty,
                dims,
                validator,
                ns,
                vartab,
                cfg,
                indexes,
            );
        }

        finish_array_loop(&for_loop, vartab, cfg);
    }

    /// Read a struct from the buffer
    fn decode_struct(
        &self,
        buffer: &Expression,
        mut offset: Expression,
        expr_ty: &Type,
        struct_ty: &StructType,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let size = if let Some(no_padding_size) = ns.calculate_struct_non_padded_size(struct_ty) {
            let padded_size = struct_ty.struct_padded_size(ns);
            // If the size without padding equals the size with padding,
            // we can memcpy this struct directly.
            if padded_size.eq(&no_padding_size) {
                let size = Expression::NumberLiteral(Codegen, Uint(32), no_padding_size);
                validator.validate_offset_plus_size(&offset, &size, ns, vartab, cfg);
                let source_address = Expression::AdvancePointer {
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(offset),
                };
                let allocated_struct = vartab.temp_anonymous(expr_ty);
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Codegen,
                        res: allocated_struct,
                        expr: Expression::StructLiteral(Codegen, expr_ty.clone(), vec![]),
                    },
                );
                let struct_var = Expression::Variable(Codegen, expr_ty.clone(), allocated_struct);
                cfg.add(
                    vartab,
                    Instr::MemCopy {
                        source: source_address,
                        destination: struct_var.clone(),
                        bytes: size.clone(),
                    },
                );
                return (struct_var, size);
            } else {
                // This struct has a fixed size, but we cannot memcpy it due to
                // its padding in memory
                Some(Expression::NumberLiteral(
                    Codegen,
                    Uint(32),
                    no_padding_size,
                ))
            }
        } else {
            None
        };

        let struct_tys = struct_ty
            .definition(ns)
            .fields
            .iter()
            .map(|item| item.ty.clone())
            .collect::<Vec<Type>>();

        // If it was not possible to validate the struct beforehand, we validate each field
        // during recursive calls to 'read_from_buffer'
        let mut struct_validator = validator.create_sub_validator(&struct_tys);

        let qty = struct_ty.definition(ns).fields.len();

        if validator.validation_necessary() {
            struct_validator.initialize_validation(&offset, ns, vartab, cfg);
        }

        let (mut read_expr, mut advance) = self.read_from_buffer(
            buffer,
            &offset,
            &struct_tys[0],
            &mut struct_validator,
            ns,
            vartab,
            cfg,
        );
        let mut runtime_size = advance.clone();

        let mut read_items = vec![Expression::Poison; qty];
        read_items[0] = read_expr;
        for i in 1..qty {
            struct_validator.set_argument_number(i);
            struct_validator.validate_buffer(&offset, ns, vartab, cfg);
            offset = Expression::Add(
                Codegen,
                Uint(32),
                false,
                Box::new(offset.clone()),
                Box::new(advance),
            );
            (read_expr, advance) = self.read_from_buffer(
                buffer,
                &offset,
                &struct_tys[i],
                &mut struct_validator,
                ns,
                vartab,
                cfg,
            );
            read_items[i] = read_expr;
            runtime_size = Expression::Add(
                Codegen,
                Uint(32),
                false,
                Box::new(runtime_size),
                Box::new(advance.clone()),
            );
        }

        let allocated_struct = vartab.temp_anonymous(expr_ty);
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: allocated_struct,
                expr: Expression::StructLiteral(Codegen, expr_ty.clone(), read_items),
            },
        );

        let struct_var = Expression::Variable(Codegen, expr_ty.clone(), allocated_struct);
        (struct_var, size.unwrap_or(runtime_size))
    }
}
