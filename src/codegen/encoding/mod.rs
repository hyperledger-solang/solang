// SPDX-License-Identifier: Apache-2.0

mod borsh_encoding;
mod buffer_validator;
mod scale_encoding;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::borsh_encoding::BorshEncoding;
use crate::codegen::encoding::scale_encoding::ScaleEncoding;
use crate::codegen::expression::load_storage;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type};
use crate::Target;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};
use solang_parser::pt::Loc;
use std::ops::{AddAssign, MulAssign, Sub};

pub(super) fn abi_encode(
    loc: &Loc,
    args: Vec<Expression>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    packed: bool,
) -> (Expression, Expression) {
    if ns.target.is_substrate() {
        // TODO refactor into codegen
        return ScaleEncoding::new(packed).abi_encode(loc, args, ns, vartab, cfg);
    }
    let mut encoder = create_encoder(ns, packed);
    let size = calculate_size_args(&mut encoder, &args, ns, vartab, cfg);

    let encoded_bytes = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res: encoded_bytes,
            expr: Expression::AllocDynamicBytes(
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
        let advance = encoder.encode(item, &buffer, &offset, arg_no, ns, vartab, cfg);
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

/// This trait should be implemented by all encoding methods (ethabi, Scale and Borsh), so that
/// we have the same interface for creating encode and decode functions.
pub(super) trait AbiEncoding {
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
                self.encode_linear(expr, buffer, offset, vartab, cfg, ns.address_length.into())
            }
            Type::Bool => self.encode_linear(expr, buffer, offset, vartab, cfg, 1.into()),
            Type::Uint(width) | Type::Int(width) => {
                self.encode_int(expr, buffer, offset, vartab, cfg, *width)
            }
            Type::Value => {
                let size = ns.value_length.into();
                self.encode_linear(expr, buffer, offset, vartab, cfg, size)
            }
            Type::Bytes(length) => {
                let size = (*length).into();
                self.encode_linear(expr, buffer, offset, vartab, cfg, size)
            }
            Type::String | Type::DynamicBytes if self.is_packed() => {
                self.bytes_packed(expr, buffer, offset, vartab, cfg)
            }
            Type::String | Type::DynamicBytes => self.bytes(expr, buffer, offset, vartab, cfg),
            Type::Enum(_) => self.encode_linear(expr, buffer, offset, vartab, cfg, 1.into()),
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
                self.encode_external_function(expr, buffer, offset, ns, vartab, cfg)
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
                let loaded = self.storage_cache_remove(arg_no).unwrap();
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
        }
    }
    /// Receive the arguments and returns the variable containing a byte array and its size

    fn encode_linear(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        size: BigInt,
    ) -> Expression {
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: expr.clone(),
            },
        );
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), size)
    }

    fn encode_int(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        width: u16,
    ) -> Expression {
        let encoding_size = width.next_power_of_two();
        let expr = if encoding_size != width {
            if expr.ty().is_signed_int() {
                Expression::SignExt(
                    Loc::Codegen,
                    Type::Int(encoding_size),
                    Box::new(expr.clone()),
                )
            } else {
                Expression::ZeroExt(
                    Loc::Codegen,
                    Type::Uint(encoding_size),
                    Box::new(expr.clone()),
                )
            }
        } else {
            expr.clone()
        };

        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: expr,
            },
        );

        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), (encoding_size / 8).into())
    }

    fn encode_int_packed(&mut self) {
        todo!()
    }

    fn bytes_packed(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
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

        // ptr + offset + size_of_integer
        let dest_address = Expression::AdvancePointer {
            pointer: Box::new(buffer.clone()),
            bytes_offset: Box::new(offset.clone()),
        };

        cfg.add(
            vartab,
            Instr::MemCopy {
                source: expr.clone(),
                destination: dest_address,
                bytes: var.clone(),
            },
        );

        var
    }

    fn bytes(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let len_offset = offset.clone();
        let len = self.bytes_packed(expr, buffer, &increment_four(offset.clone()), vartab, cfg);
        self.encode_int(&len, buffer, &len_offset, vartab, cfg, 32);
        increment_four(len)
    }

    /// Encode a struct and return its size in bytes
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

                let new_offset = if self.is_packed() {
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
            if dyn_dims > 0 && !self.is_packed() {
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
        if dims[dimension] == ArrayLength::Dynamic && !self.is_packed() {
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

    fn encode_external_function(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
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

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size: Option<Expression>,
    ) -> Vec<Expression>;

    /// Cache items loaded from storage to reuse them later, so we avoid the expensive operation
    /// of loading from storage twice. We need the data in two different passes: first to
    /// calculate its size and then to copy it to the buffer.
    ///
    /// This function serves only to cache Expression::Variable, containing items loaded from storage.
    /// Nothing else should be stored here. For more information, check the comment at
    /// 'struct BorshEncoding' on borsh_encoding.rs
    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression);

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression>;

    /// Some types have sizes that are specific to each encoding scheme, so there is no way to generalize.
    fn get_encoding_size(&self, expr: &Expression, ty: &Type, ns: &Namespace) -> Expression;

    /// Returns if the we are packed encoding
    fn is_packed(&self) -> bool;
}

/// This function should return the correct encoder, given the target
pub(super) fn create_encoder(ns: &Namespace, packed: bool) -> Box<dyn AbiEncoding> {
    match &ns.target {
        Target::Solana => Box::new(BorshEncoding::new(packed)),
        // Solana utilizes Borsh encoding and Substrate, Scale encoding.
        // All other targets are using the Scale encoding, because we have tests for a
        // fake Ethereum target that checks the presence of Instr::AbiDecode and
        // Expression::AbiEncode.
        // If a new target is added, this piece of code needs to change.
        _ => Box::new(ScaleEncoding::new(packed)),
    }
}

/// Calculate the size of a set of arguments to encoding functions
fn calculate_size_args(
    encoder: &mut Box<dyn AbiEncoding>,
    args: &[Expression],
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let mut size = get_expr_size(encoder, 0, &args[0], ns, vartab, cfg);
    for (i, item) in args.iter().enumerate().skip(1) {
        size = Expression::Add(
            Loc::Codegen,
            Type::Uint(32),
            false,
            Box::new(size),
            Box::new(get_expr_size(encoder, i, item, ns, vartab, cfg)),
        );
    }

    size
}

/// Calculate the size of a single codegen::Expression
fn get_expr_size(
    encoder: &mut Box<dyn AbiEncoding>,
    arg_no: usize,
    expr: &Expression,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let ty = expr.ty().unwrap_user_type(ns);
    match &ty {
        Type::Value => {
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(ns.value_length))
        }

        Type::Struct(struct_ty) => {
            calculate_struct_size(encoder, arg_no, expr, struct_ty, ns, vartab, cfg)
        }

        Type::Slice(ty) => {
            let dims = vec![ArrayLength::Dynamic];
            calculate_array_size(encoder, expr, ty, &dims, arg_no, ns, vartab, cfg)
        }

        Type::Array(ty, dims) => {
            calculate_array_size(encoder, expr, ty, dims, arg_no, ns, vartab, cfg)
        }

        Type::UserType(_) | Type::Unresolved | Type::Rational => {
            unreachable!("Type should not exist in codegen")
        }

        Type::ExternalFunction { .. } => {
            let addr = Expression::Undefined(Type::Address(false));
            let address_size = encoder.get_encoding_size(&addr, &Type::Address(false), ns);
            if let Expression::NumberLiteral(_, _, mut number) = address_size {
                number.add_assign(BigInt::from(4u8));
                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), number)
            } else {
                increment_four(address_size)
            }
        }

        Type::InternalFunction { .. }
        | Type::Void
        | Type::Unreachable
        | Type::BufferPointer
        | Type::Mapping(..) => unreachable!("This type cannot be encoded"),

        Type::Ref(r) => {
            if let Type::Struct(struct_ty) = &**r {
                return calculate_struct_size(encoder, arg_no, expr, struct_ty, ns, vartab, cfg);
            }
            let loaded = Expression::Load(Loc::Codegen, *r.clone(), Box::new(expr.clone()));
            get_expr_size(encoder, arg_no, &loaded, ns, vartab, cfg)
        }

        Type::StorageRef(_, r) => {
            let var = load_storage(&Loc::Codegen, r, expr.clone(), cfg, vartab);
            let size = get_expr_size(encoder, arg_no, &var, ns, vartab, cfg);
            encoder.storage_cache_insert(arg_no, var.clone());
            size
        }

        _ => encoder.get_encoding_size(expr, &ty, ns),
    }
}

/// Calculate the size of an array
fn calculate_array_size(
    encoder: &mut Box<dyn AbiEncoding>,
    array: &Expression,
    elem_ty: &Type,
    dims: &Vec<ArrayLength>,
    arg_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let dyn_dims = dims.iter().filter(|d| **d == ArrayLength::Dynamic).count();

    // If the array does not have variable length elements,
    // we can calculate its size using a simple multiplication (direct_assessment)
    // i.e. 'uint8[3][] vec' has size vec.length*2*size_of(uint8)
    // In cases like 'uint [3][][2] v' this is not possible, as v[0] and v[1] have different sizes
    let direct_assessment =
        dyn_dims == 0 || (dyn_dims == 1 && dims.last() == Some(&ArrayLength::Dynamic));

    // Check if the array contains only fixed sized elements
    let primitive_size = if elem_ty.is_primitive() && direct_assessment {
        Some(elem_ty.memory_size_of(ns))
    } else if let Type::Struct(struct_ty) = elem_ty {
        if direct_assessment {
            ns.calculate_struct_non_padded_size(struct_ty)
        } else {
            None
        }
    } else {
        None
    };

    let size_var = if let Some(compile_type_size) = primitive_size {
        // If the array saves primitive-type elements, its size is sizeof(type)*vec.length
        let mut size = if let ArrayLength::Fixed(dim) = &dims.last().unwrap() {
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), dim.clone())
        } else {
            Expression::Builtin(
                Loc::Codegen,
                vec![Type::Uint(32)],
                Builtin::ArrayLength,
                vec![array.clone()],
            )
        };

        for item in dims.iter().take(dims.len() - 1) {
            let local_size = Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(32),
                item.array_length().unwrap().clone(),
            );
            size = Expression::Multiply(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(size),
                Box::new(local_size),
            );
        }

        let type_size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), compile_type_size);
        let size = Expression::Multiply(
            Loc::Codegen,
            Type::Uint(32),
            false,
            Box::new(size),
            Box::new(type_size),
        );
        let size_var = vartab.temp_anonymous(&Type::Uint(32));
        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: size_var,
                expr: size,
            },
        );

        size_var
    } else {
        let size_var = vartab.temp_name(
            format!("array_bytes_size_{}", arg_no).as_str(),
            &Type::Uint(32),
        );
        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: size_var,
                expr: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(0u8)),
            },
        );
        let mut index_vec: Vec<usize> = Vec::new();
        calculate_complex_array_size(
            encoder,
            arg_no,
            array,
            dims,
            dims.len() - 1,
            size_var,
            ns,
            &mut index_vec,
            vartab,
            cfg,
        );
        size_var
    };

    // Each dynamic dimension size occupies 4 bytes in the buffer
    let dyn_dims = dims.iter().filter(|d| **d == ArrayLength::Dynamic).count();

    if dyn_dims > 0 && !encoder.is_packed() {
        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: size_var,
                expr: Expression::Add(
                    Loc::Codegen,
                    Type::Uint(32),
                    false,
                    Box::new(Expression::Variable(Loc::Codegen, Type::Uint(32), size_var)),
                    Box::new(Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        BigInt::from(4 * dyn_dims),
                    )),
                ),
            },
        );
    }

    Expression::Variable(Loc::Codegen, Type::Uint(32), size_var)
}

/// Calculate the size of a complex array.
/// This function indexes an array from its outer dimension to its inner one
fn calculate_complex_array_size(
    encoder: &mut Box<dyn AbiEncoding>,
    arg_no: usize,
    arr: &Expression,
    dims: &Vec<ArrayLength>,
    dimension: usize,
    size_var_no: usize,
    ns: &Namespace,
    indexes: &mut Vec<usize>,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) {
    let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
    cfg.set_basic_block(for_loop.body_block);
    if 0 == dimension {
        let deref = load_array_item(arr, dims, indexes);
        let elem_size = get_expr_size(encoder, arg_no, &deref, ns, vartab, cfg);

        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: size_var_no,
                expr: Expression::Add(
                    Loc::Codegen,
                    Type::Uint(32),
                    false,
                    Box::new(Expression::Variable(
                        Loc::Codegen,
                        Type::Uint(32),
                        size_var_no,
                    )),
                    Box::new(elem_size),
                ),
            },
        );
    } else {
        calculate_complex_array_size(
            encoder,
            arg_no,
            arr,
            dims,
            dimension - 1,
            size_var_no,
            ns,
            indexes,
            vartab,
            cfg,
        );
    }

    finish_array_loop(&for_loop, vartab, cfg);
}

/// Get the array length at dimension 'index'
fn get_array_length(
    arr: &Expression,
    dims: &[ArrayLength],
    indexes: &[usize],
    dimension: usize,
) -> Expression {
    if let ArrayLength::Fixed(dim) = &dims[dimension] {
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), dim.clone())
    } else {
        let (sub_array, _) = load_sub_array(
            arr.clone(),
            &dims[(dimension + 1)..dims.len()],
            indexes,
            true,
        );

        Expression::Builtin(
            Loc::Codegen,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![sub_array],
        )
    }
}

/// Retrieves the size of a struct
fn calculate_struct_size(
    encoder: &mut Box<dyn AbiEncoding>,
    arg_no: usize,
    expr: &Expression,
    struct_ty: &StructType,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    if let Some(struct_size) = ns.calculate_struct_non_padded_size(struct_ty) {
        return Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), struct_size);
    }

    let first_type = struct_ty.definition(ns).fields[0].ty.clone();
    let first_field = load_struct_member(first_type, expr.clone(), 0);
    let mut size = get_expr_size(encoder, arg_no, &first_field, ns, vartab, cfg);
    for i in 1..struct_ty.definition(ns).fields.len() {
        let ty = struct_ty.definition(ns).fields[i].ty.clone();
        let field = load_struct_member(ty.clone(), expr.clone(), i);
        size = Expression::Add(
            Loc::Codegen,
            Type::Uint(32),
            false,
            Box::new(size.clone()),
            Box::new(get_expr_size(encoder, arg_no, &field, ns, vartab, cfg)),
        );
    }

    size
}

/// Loads an item from an array
fn load_array_item(arr: &Expression, dims: &[ArrayLength], indexes: &[usize]) -> Expression {
    let elem_ty = arr.ty().elem_ty();
    let (deref, ty) = load_sub_array(arr.clone(), dims, indexes, false);
    Expression::Subscript(
        Loc::Codegen,
        Type::Ref(Box::new(elem_ty)),
        ty,
        Box::new(deref),
        Box::new(Expression::Variable(
            Loc::Codegen,
            Type::Uint(32),
            *indexes.last().unwrap(),
        )),
    )
}

/// Dereferences a subarray. If we have 'int[3][][4] vec' and we need 'int[3][]',
/// this function returns so.
/// 'dims' should contain only the dimensions we want to index
/// 'index' is the list of indexes to use
/// 'index_first_dim' chooses whether to index the first dimension in dims
fn load_sub_array(
    mut arr: Expression,
    dims: &[ArrayLength],
    indexes: &[usize],
    index_first_dim: bool,
) -> (Expression, Type) {
    let mut ty = arr.ty();
    let elem_ty = ty.elem_ty();
    let start = !index_first_dim as usize;
    for i in (start..dims.len()).rev() {
        let local_ty = Type::Array(Box::new(elem_ty.clone()), dims[0..i].to_vec());
        arr = Expression::Subscript(
            Loc::Codegen,
            Type::Ref(Box::new(local_ty.clone())),
            ty,
            Box::new(arr),
            Box::new(Expression::Variable(
                Loc::Codegen,
                Type::Uint(32),
                indexes[indexes.len() - i - 1],
            )),
        );
        ty = local_ty;
    }

    (arr, ty)
}

/// This struct manages for-loops created when iterating over arrays
struct ForLoop {
    pub cond_block: usize,
    pub next_block: usize,
    pub body_block: usize,
    pub end_block: usize,
    pub index: usize,
}

/// Set up the loop to iterate over an array
fn set_array_loop(
    arr: &Expression,
    dims: &[ArrayLength],
    dimension: usize,
    indexes: &mut Vec<usize>,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> ForLoop {
    let index_temp = vartab.temp_name(format!("for_i_{}", dimension).as_str(), &Type::Uint(32));

    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: index_temp,
            expr: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(0u8)),
        },
    );

    indexes.push(index_temp);
    let cond_block = cfg.new_basic_block("cond".to_string());
    let next_block = cfg.new_basic_block("next".to_string());
    let body_block = cfg.new_basic_block("body".to_string());
    let end_block = cfg.new_basic_block("end_for".to_string());

    vartab.new_dirty_tracker();
    cfg.add(vartab, Instr::Branch { block: cond_block });
    cfg.set_basic_block(cond_block);
    let bound = get_array_length(arr, dims, indexes, dimension);
    let cond_expr = Expression::UnsignedLess(
        Loc::Codegen,
        Box::new(Expression::Variable(
            Loc::Codegen,
            Type::Uint(32),
            index_temp,
        )),
        Box::new(bound),
    );
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: cond_expr,
            true_block: body_block,
            false_block: end_block,
        },
    );

    ForLoop {
        cond_block,
        next_block,
        body_block,
        end_block,
        index: index_temp,
    }
}

/// Closes the for-loop when iterating over an array
fn finish_array_loop(for_loop: &ForLoop, vartab: &mut Vartable, cfg: &mut ControlFlowGraph) {
    cfg.add(
        vartab,
        Instr::Branch {
            block: for_loop.next_block,
        },
    );
    cfg.set_basic_block(for_loop.next_block);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: for_loop.index,
            expr: Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(Expression::Variable(
                    Loc::Codegen,
                    Type::Uint(32),
                    for_loop.index,
                )),
                Box::new(Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    BigInt::from(1u8),
                )),
            ),
        },
    );
    cfg.add(
        vartab,
        Instr::Branch {
            block: for_loop.cond_block,
        },
    );
    cfg.set_basic_block(for_loop.end_block);
    let phis = vartab.pop_dirty_tracker();
    cfg.set_phis(for_loop.next_block, phis.clone());
    cfg.set_phis(for_loop.end_block, phis.clone());
    cfg.set_phis(for_loop.cond_block, phis);
}

/// Loads a struct member
fn load_struct_member(ty: Type, expr: Expression, field: usize) -> Expression {
    if ty.is_fixed_reference_type() {
        // We should not dereference a struct or fixed array
        return Expression::StructMember(Loc::Codegen, ty, Box::new(expr), field);
    }

    Expression::Load(
        Loc::Codegen,
        ty.clone(),
        Box::new(Expression::StructMember(
            Loc::Codegen,
            Type::Ref(Box::new(ty)),
            Box::new(expr),
            field,
        )),
    )
}

/// Increment an expression by four. This is useful because we save array sizes as uint32, so we
/// need to increment the offset by four constantly.
fn increment_four(expr: Expression) -> Expression {
    Expression::Add(
        Loc::Codegen,
        Type::Uint(32),
        false,
        Box::new(expr),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            BigInt::from(4u8),
        )),
    )
}

/// Check if we can MemCpy elements of an array to/from a buffer
fn allow_direct_copy(
    array_ty: &Type,
    elem_ty: &Type,
    dims: &[ArrayLength],
    ns: &Namespace,
) -> bool {
    let type_direct_copy: bool = if let Type::Struct(struct_ty) = elem_ty {
        if let Some(no_padded_size) = ns.calculate_struct_non_padded_size(struct_ty) {
            let padded_size = struct_ty.struct_padded_size(ns);
            // This remainder tells us if padding is needed between the elements of an array
            let remainder = padded_size.mod_floor(&elem_ty.struct_elem_alignment(ns));

            no_padded_size.eq(&padded_size) && ns.target == Target::Solana && remainder.is_zero()
        } else {
            false
        }
    } else if let Type::Bytes(n) = elem_ty {
        // When n >=2, the bytes must be reversed
        *n < 2
    } else {
        elem_ty.is_primitive()
    };

    if array_ty.is_dynamic(ns) {
        // If this is a dynamic array, we can only MemCpy if its elements are of
        // any primitive type and we don't need to index it.
        dims.len() == 1 && type_direct_copy
    } else {
        // If the array is not dynamic, we can MemCpy elements if their are primitive.
        type_direct_copy
    }
}

/// Calculate the number of bytes needed to memcpy an entire vector
fn calculate_direct_copy_bytes_size(
    dims: &[ArrayLength],
    elem_ty: &Type,
    ns: &Namespace,
) -> BigInt {
    let mut elem_no = BigInt::one();
    for item in dims {
        debug_assert!(matches!(item, &ArrayLength::Fixed(_)));
        elem_no.mul_assign(item.array_length().unwrap());
    }
    let bytes = elem_ty.memory_size_of(ns);
    elem_no.mul_assign(bytes);

    elem_no
}

/// Calculate the size in bytes of a dynamic array, whose dynamic dimension is the outer.
/// It needs the variable saving the array's length.
fn calculate_array_bytes_size(
    length_variable: usize,
    elem_ty: &Type,
    ns: &Namespace,
) -> Expression {
    Expression::Multiply(
        Loc::Codegen,
        Type::Uint(32),
        false,
        Box::new(Expression::Variable(
            Loc::Codegen,
            Type::Uint(32),
            length_variable,
        )),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            elem_ty.memory_size_of(ns),
        )),
    )
}

/// Retrieve a dynamic array length from the encoded buffer. It returns the variable number in which
/// the length has been stored
fn retrieve_array_length(
    buffer: &Expression,
    offset: &Expression,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> usize {
    let array_length = vartab.temp_anonymous(&Type::Uint(32));
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: array_length,
            expr: Expression::Builtin(
                Loc::Codegen,
                vec![Type::Uint(32)],
                Builtin::ReadFromBuffer,
                vec![buffer.clone(), offset.clone()],
            ),
        },
    );

    array_length
}

/// Allocate an array in memory and return its variable number.
fn allocate_array(
    ty: &Type,
    length_variable: usize,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> usize {
    let array_var = vartab.temp_anonymous(ty);

    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: array_var,
            expr: Expression::AllocDynamicBytes(
                Loc::Codegen,
                ty.clone(),
                Box::new(Expression::Variable(
                    Loc::Codegen,
                    Type::Uint(32),
                    length_variable,
                )),
                None,
            ),
        },
    );

    array_var
}

impl StructType {
    /// Calculate a struct size in memory considering the padding, if necessary
    fn struct_padded_size(&self, ns: &Namespace) -> BigInt {
        let mut total = BigInt::zero();
        for item in &self.definition(ns).fields {
            let ty_align = item.ty.struct_elem_alignment(ns);
            let remainder = total.mod_floor(&ty_align);
            if !remainder.is_zero() {
                let padding = ty_align.sub(remainder);
                total.add_assign(padding);
            }
            total.add_assign(item.ty.memory_size_of(ns));
        }
        total
    }
}
