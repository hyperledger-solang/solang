// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::buffer_validator::BufferValidator;
use crate::codegen::encoding::{
    allocate_array, allow_direct_copy, calculate_array_bytes_size,
    calculate_direct_copy_bytes_size, calculate_size_args, finish_array_loop, increment_four,
    load_array_item, load_struct_member, load_sub_array, retrieve_array_length, set_array_loop,
    AbiEncoding,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type};
use num_bigint::BigInt;
use num_traits::{One, Zero};
use solang_parser::pt::Loc;
use std::collections::HashMap;
use std::ops::{AddAssign, MulAssign};

/// This struct implements the trait Encoding for Borsh encoding
pub(super) struct BorshEncoding {
    /// The trait AbiEncoding has a 'cache_storage_loaded' function, which needs this HashMap to work.
    /// Encoding happens in two steps. First, we look at each argument to calculate their size. If an
    /// argument is a storage variable, we load it and save it to a local variable.
    ///
    /// During a second pass, we copy each argument to a buffer. To copy storage variables properly into
    /// the buffer, we must load them from storage and save them in a local variable. As we have
    /// already done this before, we can cache the Expression::Variable, containing the items we loaded before.
    /// In addition, loading from storage can be an expensive operation if it done with large structs
    /// or vectors. The has map contains (argument number, Expression::Variable)
    ///
    /// For more information, check the comment at function 'cache_storage_load' on encoding/mod.rs
    storage_cache: HashMap<usize, Expression>,

    packed_encoder: bool,
}

impl AbiEncoding for BorshEncoding {
    fn abi_encode(
        &mut self,
        loc: &Loc,
        args: Vec<Expression>,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let size = calculate_size_args(self, &args, ns, vartab, cfg);

        let encoded_bytes = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: encoded_bytes,
                expr: Expression::AllocDynamicArray(
                    *loc,
                    Type::DynamicBytes,
                    Box::new(size.clone()),
                    None,
                ),
            },
        );

        let mut offset = Expression::NumberLiteral(*loc, Type::Uint(32), BigInt::zero());
        let buffer = Expression::Variable(*loc, Type::DynamicBytes, encoded_bytes);

        for (arg_no, item) in args.iter().enumerate() {
            let advance = self.encode(item, &buffer, &offset, arg_no, ns, vartab, cfg);
            offset = Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(offset),
                Box::new(advance),
            );
        }

        (buffer, size)
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
        let buffer_size = vartab.temp_anonymous(&Type::Uint(32));
        if let Some(length_expression) = buffer_size_expr {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: buffer_size,
                    expr: length_expression,
                },
            );
        } else {
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: buffer_size,
                    expr: Expression::Builtin(
                        Loc::Codegen,
                        vec![Type::Uint(32)],
                        Builtin::ArrayLength,
                        vec![buffer.clone()],
                    ),
                },
            );
        }

        let mut validator = BufferValidator::new(buffer_size, types);

        let mut read_items: Vec<Expression> = vec![Expression::Poison; types.len()];
        let mut offset = Expression::NumberLiteral(*loc, Type::Uint(32), BigInt::zero());

        validator.initialize_validation(&offset, ns, vartab, cfg);

        for (item_no, item) in types.iter().enumerate() {
            validator.set_argument_number(item_no);
            validator.validate_buffer(&offset, ns, vartab, cfg);
            let (read_item, advance) =
                self.read_from_buffer(buffer, &offset, item, &mut validator, ns, vartab, cfg);
            read_items[item_no] = read_item;
            offset = Expression::Add(
                *loc,
                Type::Uint(32),
                false,
                Box::new(offset),
                Box::new(advance),
            );
        }

        validator.validate_all_bytes_read(offset, ns, vartab, cfg);

        read_items
    }

    fn cache_storage_loaded(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn get_encoding_size(&self, expr: &Expression, ty: &Type, ns: &Namespace) -> Expression {
        match ty {
            Type::Enum(_)
            | Type::Uint(_)
            | Type::Int(_)
            | Type::Contract(_)
            | Type::Bool
            | Type::Address(_)
            | Type::Bytes(_) => {
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

impl BorshEncoding {
    pub fn new(packed: bool) -> BorshEncoding {
        BorshEncoding {
            storage_cache: HashMap::new(),
            packed_encoder: packed,
        }
    }

    /// Encode expression to buffer. Returns the size in bytes of the encoded item.
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
        let expr_ty = expr.ty().unwrap_user_type(ns);

        match &expr_ty {
            Type::Contract(_) | Type::Address(_) => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );
                Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    BigInt::from(ns.address_length),
                )
            }

            Type::Bool => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(1u8))
            }

            Type::Uint(length) | Type::Int(length) => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(length / 8))
            }

            Type::Value => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );

                Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    BigInt::from(ns.value_length),
                )
            }

            Type::Bytes(length) => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(*length))
            }

            Type::String | Type::DynamicBytes => {
                let get_size = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![expr.clone()],
                );
                let array_length = vartab.temp_anonymous(&Type::Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Loc::Codegen,
                        res: array_length,
                        expr: get_size,
                    },
                );

                let var = Expression::Variable(Loc::Codegen, Type::Uint(32), array_length);

                let string_offset = if self.packed_encoder {
                    offset.clone()
                } else {
                    cfg.add(
                        vartab,
                        Instr::WriteBuffer {
                            buf: buffer.clone(),
                            offset: offset.clone(),
                            value: var.clone(),
                        },
                    );
                    increment_four(offset.clone())
                };

                // ptr + offset + size_of_integer
                let dest_address = Expression::AdvancePointer {
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(string_offset),
                };

                cfg.add(
                    vartab,
                    Instr::MemCopy {
                        source: expr.clone(),
                        destination: dest_address,
                        bytes: var.clone(),
                    },
                );

                if self.is_packed() {
                    var
                } else {
                    increment_four(var)
                }
            }

            Type::Enum(_) => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::one())
            }

            Type::Struct(struct_ty) => self.encode_struct(
                expr,
                buffer,
                offset.clone(),
                struct_ty,
                arg_no,
                ns,
                vartab,
                cfg,
            ),

            Type::Slice(ty) => {
                let dims = vec![ArrayLength::Dynamic];
                self.encode_array(
                    expr, &expr_ty, ty, &dims, arg_no, buffer, offset, ns, vartab, cfg,
                )
            }

            Type::Array(ty, dims) => self.encode_array(
                expr, &expr_ty, ty, dims, arg_no, buffer, offset, ns, vartab, cfg,
            ),

            Type::UserType(_) | Type::Unresolved | Type::Rational | Type::Unreachable => {
                unreachable!("Type should not exist in codegen")
            }

            Type::ExternalFunction { .. } => {
                let selector = expr.external_function_selector();

                let address = expr.external_function_address();

                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: selector,
                    },
                );

                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: increment_four(offset.clone()),
                        value: address,
                    },
                );

                let mut size = BigInt::from(4);
                size.add_assign(BigInt::from(ns.address_length));

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), size)
            }

            Type::InternalFunction { .. }
            | Type::Void
            | Type::BufferPointer
            | Type::Mapping(..) => unreachable!("This type cannot be encoded"),

            Type::Ref(r) => {
                if let Type::Struct(struct_ty) = &**r {
                    // Structs references should not be dereferenced
                    return self.encode_struct(
                        expr,
                        buffer,
                        offset.clone(),
                        struct_ty,
                        arg_no,
                        ns,
                        vartab,
                        cfg,
                    );
                }
                let loaded = Expression::Load(Loc::Codegen, *r.clone(), Box::new(expr.clone()));
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }

            Type::StorageRef(..) => {
                let loaded = self.storage_cache.remove(&arg_no).unwrap();
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
        }
    }

    /// Encode an array and return its size in bytes
    fn encode_array(
        &mut self,
        array: &Expression,
        array_ty: &Type,
        elem_ty: &Type,
        dims: &Vec<ArrayLength>,
        arg_no: usize,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = if dims.is_empty() {
            // Array has no dimension
            cfg.add(
                vartab,
                Instr::WriteBuffer {
                    buf: buffer.clone(),
                    offset: offset.clone(),
                    value: Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        BigInt::from(0u8),
                    ),
                },
            );

            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(4u8))
        } else if allow_direct_copy(array_ty, elem_ty, dims, ns) {
            // Calculate number of elements
            let (bytes_size, offset) = if matches!(dims.last(), Some(&ArrayLength::Fixed(_))) {
                let elem_no = calculate_direct_copy_bytes_size(dims, elem_ty, ns);
                (
                    Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), elem_no),
                    offset.clone(),
                )
            } else {
                let arr_size = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![array.clone()],
                );

                let size_temp = vartab.temp_anonymous(&Type::Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Loc::Codegen,
                        res: size_temp,
                        expr: arr_size,
                    },
                );

                let new_offset = if self.packed_encoder {
                    offset.clone()
                } else {
                    cfg.add(
                        vartab,
                        Instr::WriteBuffer {
                            buf: buffer.clone(),
                            offset: offset.clone(),
                            value: Expression::Variable(Loc::Codegen, Type::Uint(32), size_temp),
                        },
                    );
                    increment_four(offset.clone())
                };

                let size = calculate_array_bytes_size(size_temp, elem_ty, ns);

                (size, new_offset)
            };

            let dest_address = Expression::AdvancePointer {
                pointer: Box::new(buffer.clone()),
                bytes_offset: Box::new(offset),
            };

            cfg.add(
                vartab,
                Instr::MemCopy {
                    source: array.clone(),
                    destination: dest_address,
                    bytes: bytes_size.clone(),
                },
            );

            // If the array is dynamic, we have written into the buffer its size (a uint32)
            // and its elements
            let dyn_dims = dims.iter().filter(|d| **d == ArrayLength::Dynamic).count();
            if dyn_dims > 0 && !self.packed_encoder {
                Expression::Add(
                    Loc::Codegen,
                    Type::Uint(32),
                    false,
                    Box::new(bytes_size),
                    Box::new(Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        BigInt::from(4 * dyn_dims),
                    )),
                )
            } else {
                bytes_size
            }
        } else {
            // In all other cases, we must loop through the array
            let mut indexes: Vec<usize> = Vec::new();
            let offset_var = vartab.temp_anonymous(&Type::Uint(32));
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: offset.clone(),
                },
            );
            self.encode_complex_array(
                array,
                arg_no,
                dims,
                buffer,
                offset_var,
                dims.len() - 1,
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
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: Expression::Subtract(
                        Loc::Codegen,
                        Type::Uint(32),
                        false,
                        Box::new(Expression::Variable(
                            Loc::Codegen,
                            Type::Uint(32),
                            offset_var,
                        )),
                        Box::new(offset.clone()),
                    ),
                },
            );
            Expression::Variable(Loc::Codegen, Type::Uint(32), offset_var)
        };

        size
    }

    /// Encode a complex array.
    /// This function indexes an array from its outer dimension to its inner one
    fn encode_complex_array(
        &mut self,
        arr: &Expression,
        arg_no: usize,
        dims: &Vec<ArrayLength>,
        buffer: &Expression,
        offset_var: usize,
        dimension: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        indexes: &mut Vec<usize>,
    ) {
        // If this dimension is dynamic, we must save its length before all elements
        if dims[dimension] == ArrayLength::Dynamic && !self.packed_encoder {
            // TODO: This is wired up for the support of dynamic multidimensional arrays, like
            // TODO: 'int[3][][4] vec', but it needs testing, as soon as Solang works with them.
            // TODO: A discussion about this is under way here: https://github.com/hyperledger/solang/issues/932
            // We only support dynamic arrays whose non-constant length is the outer one.
            let (sub_array, _) = load_sub_array(
                arr.clone(),
                &dims[(dimension + 1)..dims.len()],
                indexes,
                true,
            );

            let size = Expression::Builtin(
                Loc::Codegen,
                vec![Type::Uint(32)],
                Builtin::ArrayLength,
                vec![sub_array],
            );

            let offset_expr = Expression::Variable(Loc::Codegen, Type::Uint(32), offset_var);
            cfg.add(
                vartab,
                Instr::WriteBuffer {
                    buf: buffer.clone(),
                    offset: offset_expr.clone(),
                    value: size,
                },
            );
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: increment_four(offset_expr),
                },
            );
        }
        let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if 0 == dimension {
            // If we are indexing the last dimension, we have an element, so we can encode it.
            let deref = load_array_item(arr, dims, indexes);
            let offset_expr = Expression::Variable(Loc::Codegen, Type::Uint(32), offset_var);
            let elem_size = self.encode(&deref, buffer, &offset_expr, arg_no, ns, vartab, cfg);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: Expression::Add(
                        Loc::Codegen,
                        Type::Uint(32),
                        false,
                        Box::new(elem_size),
                        Box::new(offset_expr),
                    ),
                },
            );
        } else {
            self.encode_complex_array(
                arr,
                arg_no,
                dims,
                buffer,
                offset_var,
                dimension - 1,
                ns,
                vartab,
                cfg,
                indexes,
            )
        }

        finish_array_loop(&for_loop, vartab, cfg);
    }

    /// Encode a struct
    fn encode_struct(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        mut offset: Expression,
        struct_ty: &StructType,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = if let Some(no_padding_size) = ns.calculate_struct_non_padded_size(struct_ty) {
            let padded_size = struct_ty.struct_padded_size(ns);
            // If the size without padding equals the size with padding, we
            // can memcpy this struct directly.
            if padded_size.eq(&no_padding_size) {
                let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), no_padding_size);
                let dest_address = Expression::AdvancePointer {
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(offset),
                };
                cfg.add(
                    vartab,
                    Instr::MemCopy {
                        source: expr.clone(),
                        destination: dest_address,
                        bytes: size.clone(),
                    },
                );
                return size;
            } else {
                // This struct has a fixed size, but we cannot memcpy it due to
                // its padding in memory
                Some(Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    no_padding_size,
                ))
            }
        } else {
            None
        };

        let qty = struct_ty.definition(ns).fields.len();
        let first_ty = struct_ty.definition(ns).fields[0].ty.clone();
        let loaded = load_struct_member(first_ty, expr.clone(), 0);

        let mut advance = self.encode(&loaded, buffer, &offset, arg_no, ns, vartab, cfg);
        let mut runtime_size = advance.clone();
        for i in 1..qty {
            let ith_type = struct_ty.definition(ns).fields[i].ty.clone();
            offset = Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(offset.clone()),
                Box::new(advance),
            );
            let loaded = load_struct_member(ith_type.clone(), expr.clone(), i);
            // After fetching the struct member, we can encode it
            advance = self.encode(&loaded, buffer, &offset, arg_no, ns, vartab, cfg);
            runtime_size = Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(runtime_size),
                Box::new(advance.clone()),
            );
        }

        size.unwrap_or(runtime_size)
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
            Type::Uint(_)
            | Type::Int(_)
            | Type::Bool
            | Type::Address(_)
            | Type::Contract(_)
            | Type::Enum(_)
            | Type::Value
            | Type::Bytes(_) => {
                let read_bytes = ty.memory_size_of(ns);

                let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), read_bytes);
                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let read_value = Expression::Builtin(
                    Loc::Codegen,
                    vec![ty.clone()],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), offset.clone()],
                );

                (read_value, size)
            }

            Type::DynamicBytes | Type::String => {
                // String and Dynamic bytes are encoded as size (uint32) + elements
                validator.validate_offset(increment_four(offset.clone()), ns, vartab, cfg);

                let array_length = retrieve_array_length(buffer, offset, vartab, cfg);

                let size = increment_four(Expression::Variable(
                    Loc::Codegen,
                    Type::Uint(32),
                    array_length,
                ));
                let offset_to_validate = Expression::Add(
                    Loc::Codegen,
                    Type::Uint(32),
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
                        destination: Expression::Variable(
                            Loc::Codegen,
                            ty.clone(),
                            allocated_array,
                        ),
                        bytes: Expression::Variable(Loc::Codegen, Type::Uint(32), array_length),
                    },
                );

                (
                    Expression::Variable(Loc::Codegen, ty.clone(), allocated_array),
                    size,
                )
            }

            Type::UserType(type_no) => {
                let usr_type = ns.user_types[*type_no].ty.clone();
                self.read_from_buffer(buffer, offset, &usr_type, validator, ns, vartab, cfg)
            }

            Type::ExternalFunction { .. } => {
                // Extneral function has selector + address
                let size = Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    BigInt::from(ns.address_length + 4),
                );

                validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);

                let selector = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Bytes(4)],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), offset.clone()],
                );

                let address = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Address(false)],
                    Builtin::ReadFromBuffer,
                    vec![buffer.clone(), increment_four(offset.clone())],
                );

                let external_func = Expression::Cast(
                    Loc::Codegen,
                    ty.clone(),
                    Box::new(Expression::StructLiteral(
                        Loc::Codegen,
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
                            loc: Loc::Codegen,
                            res: allocated_vector,
                            expr: Expression::ArrayLiteral(
                                Loc::Codegen,
                                array_ty.clone(),
                                vec![],
                                vec![],
                            ),
                        },
                    );

                    (
                        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), elem_no),
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

            let array_expr = Expression::Variable(Loc::Codegen, array_ty.clone(), var_no);
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
                        loc: Loc::Codegen,
                        res: array_var,
                        expr: Expression::ArrayLiteral(
                            Loc::Codegen,
                            array_ty.clone(),
                            vec![],
                            vec![],
                        ),
                    },
                );
            }

            let offset_var = vartab.temp_anonymous(&Type::Uint(32));
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: offset.clone(),
                },
            );
            let array_var_expr = Expression::Variable(Loc::Codegen, array_ty.clone(), array_var);
            let offset_expr = Expression::Variable(Loc::Codegen, Type::Uint(32), offset_var);
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
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: Expression::Subtract(
                        Loc::Codegen,
                        Type::Uint(32),
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
            let elems_size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), elems);
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
                    loc: Loc::Codegen,
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
                            loc: Loc::Codegen,
                            res: *var_no,
                            expr: Expression::Variable(
                                Loc::Codegen,
                                new_ty.clone(),
                                allocated_array,
                            ),
                        },
                    );
                } else {
                    unreachable!("array_var must be a variable");
                }
            } else {
                // TODO: This is wired up for multidimensional dynamic arrays, but they do no work yet
                // Check https://github.com/hyperledger/solang/issues/932 for more information
                let (sub_arr, _) = load_sub_array(
                    array_var.clone(),
                    &dims[(dimension + 1)..dims.len()],
                    indexes,
                    true,
                );
                cfg.add(
                    vartab,
                    Instr::Store {
                        dest: sub_arr,
                        data: Expression::Variable(Loc::Codegen, new_ty.clone(), allocated_array),
                    },
                );
            }
        }

        let for_loop = set_array_loop(array_var, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if 0 == dimension {
            let (read_expr, advance) =
                self.read_from_buffer(buffer, offset_expr, elem_ty, validator, ns, vartab, cfg);
            let ptr = load_array_item(array_var, dims, indexes);

            cfg.add(
                vartab,
                Instr::Store {
                    dest: ptr,
                    data: if matches!(read_expr.ty(), Type::Struct(_)) {
                        // Type::Struct is a pointer to a struct. If we are dealing with a vector
                        // of structs, we need to dereference the pointer before storing it at a
                        // given vector index.
                        Expression::Load(Loc::Codegen, read_expr.ty(), Box::new(read_expr))
                    } else {
                        read_expr
                    },
                },
            );
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: offset_var,
                    expr: Expression::Add(
                        Loc::Codegen,
                        Type::Uint(32),
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
                let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), no_padding_size);
                validator.validate_offset_plus_size(&offset, &size, ns, vartab, cfg);
                let source_address = Expression::AdvancePointer {
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(offset),
                };
                let allocated_struct = vartab.temp_anonymous(expr_ty);
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Loc::Codegen,
                        res: allocated_struct,
                        expr: Expression::StructLiteral(Loc::Codegen, expr_ty.clone(), vec![]),
                    },
                );
                let struct_var =
                    Expression::Variable(Loc::Codegen, expr_ty.clone(), allocated_struct);
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
                    Loc::Codegen,
                    Type::Uint(32),
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
                Loc::Codegen,
                Type::Uint(32),
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
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(runtime_size),
                Box::new(advance.clone()),
            );
        }

        let allocated_struct = vartab.temp_anonymous(expr_ty);
        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: allocated_struct,
                expr: Expression::StructLiteral(Loc::Codegen, expr_ty.clone(), read_items),
            },
        );

        let struct_var = Expression::Variable(Loc::Codegen, expr_ty.clone(), allocated_struct);
        (struct_var, size.unwrap_or(runtime_size))
    }
}
