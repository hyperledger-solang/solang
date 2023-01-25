// SPDX-License-Identifier: Apache-2.0

/// Any supported encoding scheme should be implemented here.
/// The module is organized as follows:
///
/// - `fn abi_encode()` and `fn abi_decode()` are entry point for whereever there is
/// something to be encoded or decoded.
/// - The `AbiEncoding` defines the encoding and decoding API and must be implemented by all schemes.
/// - There are some helper functions to work with more complex types.
///   Any such helper function should work fine regardless of the encoding scheme being used.
mod borsh_encoding;
mod buffer_validator;
mod scale_encoding;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::borsh_encoding::BorshEncoding;
use crate::codegen::encoding::scale_encoding::ScaleEncoding;
use crate::codegen::expression::load_storage;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type, Type::Uint};
use crate::Target;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};
use solang_parser::pt::{Loc, Loc::Codegen};
use std::ops::{AddAssign, MulAssign, Sub};

/// Insert encoding instructions into the `cfg` for any `Expression` in `args`.
/// Returns a pointer to the encoded data and the size as a 32bit integer.
pub(super) fn abi_encode(
    loc: &Loc,
    args: Vec<Expression>,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    packed: bool,
) -> (Expression, Expression) {
    let mut encoder = create_encoder(ns, packed);
    let size = calculate_size_args(&mut encoder, &args, ns, vartab, cfg);
    let encoded_bytes = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
    let expr = Expression::AllocDynamicBytes(*loc, Type::DynamicBytes, size.clone().into(), None);
    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res: encoded_bytes,
            expr,
        },
    );

    let mut offset = Expression::NumberLiteral(*loc, Uint(32), BigInt::zero());
    let buffer = Expression::Variable(*loc, Type::DynamicBytes, encoded_bytes);
    for (arg_no, item) in args.iter().enumerate() {
        let advance = encoder.encode(item, &buffer, &offset, arg_no, ns, vartab, cfg);
        offset = Expression::Add(*loc, Uint(32), false, offset.into(), advance.into());
    }
    (buffer, size)
}

/// Calculate the size of a set of arguments to encoding functions
fn calculate_size_args(
    encoder: &mut Box<dyn AbiEncoding>,
    args: &[Expression],
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let mut size = encoder.get_expr_size(0, &args[0], ns, vartab, cfg);
    for (i, item) in args.iter().enumerate().skip(1) {
        let additional = encoder.get_expr_size(i, item, ns, vartab, cfg);
        size = Expression::Add(Codegen, Uint(32), false, size.into(), additional.into());
    }
    size
}

/// This trait should be implemented by all encoding methods (ethabi, SCALE and Borsh), so that
/// we have the same interface for creating encode and decode functions.
///
/// Note: This trait mostly reflects the situation around SCALE and Borsh encoding schemes.
/// These two encoding schemes share only minor differences. We provide default implementations
/// for many methods, which properly work for SCALE and Borsh encoding.
///
/// However, this might be less suitable for schemas vastly different than SCALE or Borsh.
/// In the worst case scenario, you need to provide your own implementation of `fn encode(..)`,
/// which effectively means implementing the encoding logic for any given sema `Type` on your own.
pub(super) trait AbiEncoding {
    /// The width (in bits) used in size hints for dynamic size types.
    fn size_width(
        &self,
        size: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression;

    /// Provide generic encoding for any given `expr` into `buffer`, depending on its `Type`.
    /// Relies on the methods encoding individual expressions (`encode_*`) to return the encoded size.
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
        let expr_ty = &expr.ty().unwrap_user_type(ns);
        match expr_ty {
            Type::Contract(_) | Type::Address(_) => {
                self.encode_directly(expr, buffer, offset, vartab, cfg, ns.address_length.into())
            }
            Type::Bool => self.encode_directly(expr, buffer, offset, vartab, cfg, 1.into()),
            Type::Uint(width) | Type::Int(width) => {
                self.encode_int(expr, buffer, offset, vartab, cfg, *width)
            }
            Type::Value => {
                let size = ns.value_length.into();
                self.encode_directly(expr, buffer, offset, vartab, cfg, size)
            }
            Type::Bytes(length) => {
                self.encode_directly(expr, buffer, offset, vartab, cfg, (*length).into())
            }
            Type::String | Type::DynamicBytes => {
                self.encode_bytes(expr, buffer, offset, vartab, cfg)
            }
            Type::Enum(_) => self.encode_directly(expr, buffer, offset, vartab, cfg, 1.into()),
            Type::Struct(ty) => {
                self.encode_struct(expr, buffer, offset.clone(), ty, arg_no, ns, vartab, cfg)
            }
            Type::Slice(ty) => {
                let dims = &[ArrayLength::Dynamic];
                self.encode_array(
                    expr, expr_ty, ty, dims, arg_no, buffer, offset, ns, vartab, cfg,
                )
            }
            Type::Array(ty, dims) => self.encode_array(
                expr, expr_ty, ty, dims, arg_no, buffer, offset, ns, vartab, cfg,
            ),
            Type::ExternalFunction { .. } => {
                self.encode_external_function(expr, buffer, offset, ns, vartab, cfg)
            }
            Type::FunctionSelector => {
                let size = ns.target.selector_length().into();
                self.encode_directly(expr, buffer, offset, vartab, cfg, size)
            }
            Type::Ref(r) => {
                if let Type::Struct(ty) = &**r {
                    // Structs references should not be dereferenced
                    return self.encode_struct(
                        expr,
                        buffer,
                        offset.clone(),
                        ty,
                        arg_no,
                        ns,
                        vartab,
                        cfg,
                    );
                }
                let loaded = Expression::Load(Codegen, *r.clone(), expr.clone().into());
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
            Type::StorageRef(..) => {
                let loaded = self.storage_cache_remove(arg_no).unwrap();
                self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg)
            }
            Type::UserType(_) | Type::Unresolved | Type::Rational | Type::Unreachable => {
                unreachable!("Type should not exist in codegen")
            }
            Type::InternalFunction { .. }
            | Type::Void
            | Type::BufferPointer
            | Type::Mapping(..) => unreachable!("This type cannot be encoded"),
        }
    }

    /// Write whatever is inside the given `expr` into `buffer` without any modification.
    fn encode_directly(
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
        Expression::NumberLiteral(Codegen, Uint(32), size)
    }

    /// Encode `expr` into `buffer` as an integer.
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
                Expression::SignExt(Codegen, Type::Int(encoding_size), expr.clone().into())
            } else {
                Expression::ZeroExt(Codegen, Type::Uint(encoding_size), expr.clone().into())
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

        Expression::NumberLiteral(Codegen, Uint(32), (encoding_size / 8).into())
    }

    /// Encode `expr` into `buffer` as size hint for dynamically sized datastructures.
    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression;

    /// Encode `expr` into `buffer` as bytes.
    fn encode_bytes(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let len = array_outer_length(expr, vartab, cfg);
        let (data_offset, size) = if self.is_packed() {
            (offset.clone(), None)
        } else {
            let size = self.encode_size(&len, buffer, offset, vartab, cfg);
            (increment_by(offset.clone(), size.clone()), Some(size))
        };
        // ptr + offset + size_of_integer
        let dest_address = Expression::AdvancePointer {
            pointer: buffer.clone().into(),
            bytes_offset: data_offset.into(),
        };
        cfg.add(
            vartab,
            Instr::MemCopy {
                source: expr.clone(),
                destination: dest_address,
                bytes: len.clone(),
            },
        );
        if let Some(size) = size {
            increment_by(len, size)
        } else {
            len
        }
    }

    /// Encode `expr` into `buffer` as a struct type.
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
            // If the size without padding equals the size with padding, memcpy this struct directly.
            if padded_size.eq(&no_padding_size) {
                let size = Expression::NumberLiteral(Codegen, Uint(32), no_padding_size);
                let dest_address = Expression::AdvancePointer {
                    pointer: buffer.clone().into(),
                    bytes_offset: offset.into(),
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
                // This struct has a fixed size, but we can't memcpy it due to its padding in memory.
                Some(Expression::NumberLiteral(
                    Codegen,
                    Uint(32),
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
                Codegen,
                Uint(32),
                false,
                offset.clone().into(),
                advance.into(),
            );
            let loaded = load_struct_member(ith_type.clone(), expr.clone(), i);
            // After fetching the struct member, we can encode it
            advance = self.encode(&loaded, buffer, &offset, arg_no, ns, vartab, cfg);
            runtime_size = Expression::Add(
                Codegen,
                Uint(32),
                false,
                runtime_size.into(),
                advance.clone().into(),
            );
        }

        size.unwrap_or(runtime_size)
    }

    /// Encode `expr` into `buffer` as an array.
    fn encode_array(
        &mut self,
        array: &Expression,
        array_ty: &Type,
        elem_ty: &Type,
        dims: &[ArrayLength],
        arg_no: usize,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        assert!(!dims.is_empty());

        if allow_direct_copy(array_ty, elem_ty, dims, ns) {
            // Calculate number of elements
            let (bytes_size, offset, size_length) =
                if matches!(dims.last(), Some(&ArrayLength::Fixed(_))) {
                    let elem_no = calculate_direct_copy_bytes_size(dims, elem_ty, ns);
                    (
                        Expression::NumberLiteral(Codegen, Uint(32), elem_no),
                        offset.clone(),
                        None,
                    )
                } else {
                    let value = array_outer_length(array, vartab, cfg);

                    let (new_offset, size_length) = if self.is_packed() {
                        (offset.clone(), None)
                    } else {
                        let encoded_size = self.encode_size(&value, buffer, offset, vartab, cfg);
                        let new_offset = increment_by(offset.clone(), encoded_size.clone());
                        (new_offset, Some(encoded_size))
                    };

                    if let Expression::Variable(_, _, size_temp) = value {
                        let size = calculate_array_bytes_size(size_temp, elem_ty, ns);
                        (size, new_offset, size_length)
                    } else {
                        unreachable!()
                    }
                };

            let dest_address = Expression::AdvancePointer {
                pointer: buffer.clone().into(),
                bytes_offset: offset.into(),
            };

            cfg.add(
                vartab,
                Instr::MemCopy {
                    source: array.clone(),
                    destination: dest_address,
                    bytes: bytes_size.clone(),
                },
            );

            // If the array is dynamic, we have written into the buffer its size and its elements
            return match (size_length, self.is_packed()) {
                (Some(len), false) => {
                    Expression::Add(Codegen, Uint(32), false, bytes_size.into(), len.into())
                }
                _ => bytes_size,
            };
        }

        // In all other cases, we must loop through the array
        let mut indexes: Vec<usize> = Vec::new();
        let offset_var_no = vartab.temp_anonymous(&Uint(32));
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: offset_var_no,
                expr: offset.clone(),
            },
        );
        self.encode_complex_array(
            array,
            arg_no,
            dims,
            buffer,
            offset_var_no,
            dims.len() - 1,
            ns,
            vartab,
            cfg,
            &mut indexes,
        );

        // The offset variable minus the original offset obtains the vector size in bytes
        let offset_var = Expression::Variable(Codegen, Uint(32), offset_var_no).into();
        let sub = Expression::Subtract(Codegen, Uint(32), false, offset_var, offset.clone().into());
        cfg.add(
            vartab,
            Instr::Set {
                loc: Codegen,
                res: offset_var_no,
                expr: sub,
            },
        );
        Expression::Variable(Codegen, Uint(32), offset_var_no)
    }

    /// Encode `expr` into `buffer` as a complex array.
    /// This function indexes an array from its outer dimension to its inner one.
    ///
    /// Note: In the default implementation, `encode_array` decides when to use this method for you.
    fn encode_complex_array(
        &mut self,
        arr: &Expression,
        arg_no: usize,
        dims: &[ArrayLength],
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
            let sub_array = index_array(arr.clone(), dims, indexes, false);

            let size = Expression::Builtin(
                Codegen,
                vec![Uint(32)],
                Builtin::ArrayLength,
                vec![sub_array],
            );

            let offset_expr = Expression::Variable(Codegen, Uint(32), offset_var);
            let encoded_size = self.encode_size(&size, buffer, &offset_expr, vartab, cfg);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: increment_by(offset_expr, encoded_size),
                },
            );
        }
        let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if 0 == dimension {
            // If we are indexing the last dimension, we have an element, so we can encode it.
            let deref = index_array(arr.clone(), dims, indexes, false);
            let offset_expr = Expression::Variable(Codegen, Uint(32), offset_var);
            let elem_size = self.encode(&deref, buffer, &offset_expr, arg_no, ns, vartab, cfg);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: offset_var,
                    expr: Expression::Add(
                        Codegen,
                        Uint(32),
                        false,
                        elem_size.into(),
                        offset_expr.into(),
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

    /// Encode `expr` into `buffer` as an external function pointer.
    fn encode_external_function(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression;

    /// Calculate the size of a single codegen::Expression
    fn get_expr_size(
        &mut self,
        arg_no: usize,
        expr: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let ty = expr.ty().unwrap_user_type(ns);
        match &ty {
            Type::Value => {
                Expression::NumberLiteral(Codegen, Uint(32), BigInt::from(ns.value_length))
            }
            Type::Uint(n) | Type::Int(n) => Expression::NumberLiteral(
                Codegen,
                Uint(32),
                BigInt::from(n.next_power_of_two() / 8),
            ),
            Type::Enum(_) | Type::Contract(_) | Type::Bool | Type::Address(_) | Type::Bytes(_) => {
                Expression::NumberLiteral(Codegen, Uint(32), ty.memory_size_of(ns))
            }
            Type::FunctionSelector => Expression::NumberLiteral(
                Codegen,
                Uint(32),
                BigInt::from(ns.target.selector_length()),
            ),
            Type::Struct(struct_ty) => {
                self.calculate_struct_size(arg_no, expr, struct_ty, ns, vartab, cfg)
            }
            Type::Slice(ty) => {
                let dims = vec![ArrayLength::Dynamic];
                self.calculate_array_size(expr, ty, &dims, arg_no, ns, vartab, cfg)
            }
            Type::Array(ty, dims) => {
                self.calculate_array_size(expr, ty, dims, arg_no, ns, vartab, cfg)
            }
            Type::ExternalFunction { .. } => {
                let selector_len: BigInt = ns.target.selector_length().into();
                let mut address_size = Type::Address(false).memory_size_of(ns);
                address_size.add_assign(selector_len);
                Expression::NumberLiteral(Codegen, Uint(32), address_size)
            }
            Type::Ref(r) => {
                if let Type::Struct(struct_ty) = &**r {
                    return self.calculate_struct_size(arg_no, expr, struct_ty, ns, vartab, cfg);
                }
                let loaded = Expression::Load(Codegen, *r.clone(), expr.clone().into());
                self.get_expr_size(arg_no, &loaded, ns, vartab, cfg)
            }
            Type::StorageRef(_, r) => {
                let var = load_storage(&Codegen, r, expr.clone(), cfg, vartab);
                let size = self.get_expr_size(arg_no, &var, ns, vartab, cfg);
                self.storage_cache_insert(arg_no, var.clone());
                size
            }
            Type::String | Type::DynamicBytes => self.calculate_string_size(expr, vartab, cfg),
            Type::InternalFunction { .. }
            | Type::Void
            | Type::Unreachable
            | Type::BufferPointer
            | Type::Mapping(..) => unreachable!("This type cannot be encoded"),
            Type::UserType(_) | Type::Unresolved | Type::Rational => {
                unreachable!("Type should not exist in codegen")
            }
        }
    }

    /// Calculate the size of an array
    fn calculate_array_size(
        &mut self,
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

        if let Some(compile_type_size) = primitive_size {
            // If the array saves primitive-type elements, its size is sizeof(type)*vec.length
            let mut size = if let ArrayLength::Fixed(dim) = &dims.last().unwrap() {
                Expression::NumberLiteral(Codegen, Uint(32), dim.clone())
            } else {
                Expression::Builtin(
                    Codegen,
                    vec![Uint(32)],
                    Builtin::ArrayLength,
                    vec![array.clone()],
                )
            };

            for item in dims.iter().take(dims.len() - 1) {
                let local_size = Expression::NumberLiteral(
                    Codegen,
                    Uint(32),
                    item.array_length().unwrap().clone(),
                );
                size = Expression::Multiply(
                    Codegen,
                    Uint(32),
                    false,
                    size.into(),
                    local_size.clone().into(),
                );
            }

            let type_size = Expression::NumberLiteral(Codegen, Uint(32), compile_type_size);
            let size =
                Expression::Multiply(Codegen, Uint(32), false, size.into(), type_size.into());
            let size_var = vartab.temp_anonymous(&Uint(32));
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: size_var,
                    expr: size,
                },
            );
            let size_var = Expression::Variable(Codegen, Uint(32), size_var);
            if self.is_packed() || !matches!(&dims.last().unwrap(), ArrayLength::Dynamic) {
                return size_var;
            }
            let size_width = self.size_width(&size_var, vartab, cfg);
            Expression::Add(Codegen, Uint(32), false, size_var.into(), size_width.into())
        } else {
            let size_var =
                vartab.temp_name(format!("array_bytes_size_{arg_no}").as_str(), &Uint(32));
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: size_var,
                    expr: Expression::NumberLiteral(Codegen, Uint(32), BigInt::from(0u8)),
                },
            );
            let mut index_vec: Vec<usize> = Vec::new();
            self.calculate_complex_array_size(
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
            Expression::Variable(Codegen, Uint(32), size_var)
        }
    }

    /// Calculate the size of a complex array.
    /// This function indexes an array from its outer dimension to its inner one and
    /// accounts for the encoded length size for dynamic dimensions.
    fn calculate_complex_array_size(
        &mut self,
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
        // If this dimension is dynamic, account for the encoded vector length variable.
        if !self.is_packed() && dims[dimension] == ArrayLength::Dynamic {
            let arr = index_array(arr.clone(), dims, indexes, false);
            let size =
                Expression::Builtin(Codegen, vec![Uint(32)], Builtin::ArrayLength, vec![arr]);
            let size_width = self.size_width(&size, vartab, cfg);
            let size_var = Expression::Variable(Codegen, Uint(32), size_var_no);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: size_var_no,
                    expr: Expression::Add(
                        Codegen,
                        Uint(32),
                        false,
                        size_var.into(),
                        size_width.into(),
                    ),
                },
            );
        }

        let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if 0 == dimension {
            let deref = index_array(arr.clone(), dims, indexes, false);
            let elem_size = self.get_expr_size(arg_no, &deref, ns, vartab, cfg);
            let size_var = Expression::Variable(Codegen, Uint(32), size_var_no);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: Codegen,
                    res: size_var_no,
                    expr: Expression::Add(
                        Codegen,
                        Uint(32),
                        false,
                        size_var.into(),
                        elem_size.into(),
                    ),
                },
            );
        } else {
            self.calculate_complex_array_size(
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

    /// Retrieves the size of a struct
    fn calculate_struct_size(
        &mut self,
        arg_no: usize,
        expr: &Expression,
        struct_ty: &StructType,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        if let Some(struct_size) = ns.calculate_struct_non_padded_size(struct_ty) {
            return Expression::NumberLiteral(Codegen, Uint(32), struct_size);
        }
        let first_type = struct_ty.definition(ns).fields[0].ty.clone();
        let first_field = load_struct_member(first_type, expr.clone(), 0);
        let mut size = self.get_expr_size(arg_no, &first_field, ns, vartab, cfg);
        for i in 1..struct_ty.definition(ns).fields.len() {
            let ty = struct_ty.definition(ns).fields[i].ty.clone();
            let field = load_struct_member(ty.clone(), expr.clone(), i);
            let expr_size = self.get_expr_size(arg_no, &field, ns, vartab, cfg).into();
            size = Expression::Add(Codegen, Uint(32), false, size.clone().into(), expr_size);
        }
        size
    }

    fn calculate_string_size(
        &self,
        expr: &Expression,
        _vartab: &mut Vartable,
        _cfg: &mut ControlFlowGraph,
    ) -> Expression {
        // When encoding a variable length array, the total size is "length (u32)" + elements
        let length = Expression::Builtin(
            Codegen,
            vec![Uint(32)],
            Builtin::ArrayLength,
            vec![expr.clone()],
        );

        if self.is_packed() {
            length
        } else {
            increment_four(length)
        }
    }

    /// Insert decoding routines into the `cfg` for the `Expression`s in `args`.
    /// Returns a list containing the encoded data.
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

    /// Encoding happens in two steps. First, we look at each argument to calculate its size. If an
    /// argument is a storage variable, we load it and save it to a local variable.
    ///
    /// During a second pass, we copy each argument to a buffer. To copy storage variables properly into
    /// the buffer, we must load them from storage and save them in a local variable. As we have
    /// already done this, we can cache the Expression::Variable, containing the items we loaded before.
    /// In addition, loading from storage can be an expensive operation if it's done with large structs
    /// or vectors.
    ///
    /// This function serves only to cache Expression::Variable, containing items loaded from storage.
    /// Nothing else should be stored here.
    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression);

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression>;

    /// Returns if the we are packed encoding
    fn is_packed(&self) -> bool;
}

/// This function should return the correct encoder, given the target
pub(super) fn create_encoder(ns: &Namespace, packed: bool) -> Box<dyn AbiEncoding> {
    match &ns.target {
        Target::Solana => Box::new(BorshEncoding::new(packed)),
        // Solana utilizes Borsh encoding and Substrate, SCALE encoding.
        // All other targets are using the SCALE encoding, because we have tests for a
        // fake Ethereum target that checks the presence of Instr::AbiDecode and
        // Expression::AbiEncode.
        // If a new target is added, this piece of code needs to change.
        _ => Box::new(ScaleEncoding::new(packed)),
    }
}

/// Indexes an array. If we have 'int[3][][4] vec' and we need 'int[3][]',
/// 'int[3]' or 'int' (the array element) this function returns so.
///
/// * `arr` - The expression that represents the array
/// * `dims` - is the vector containing the array's dimensions
/// * `index` - is the list of indexes to use for each dimension
/// * `coerce_pointer_return` - forces the return of a pointer in this function.
///
/// When applying Expression::Subscript to a fixed-sized array, like 'int[3][4] vec', we
/// have a pointer to a 'int[3]'. If we would like to index it again, there is no need to load,
/// because a pointer to 'int[3]' is what the LLVM GEP instruction requires.
///
/// The underlying representation of a dynamic array is a C struct called 'struct vector'. In this
/// sense, a vector like 'uint16[][] vec is a 'struct vector', whose buffer elements are all pointers to
/// other 'struct vector's. When we index the first dimension of 'vec', we have a pointer to a pointer
/// to a 'struct vector', which is not compatible with LLVM GEP instruction for further indexing.
/// Therefore, we need an Expression::Load to obtain a pointer to 'struct vector' to be able to index
/// it again.
///
/// Even though all the types this function returns are pointers in the LLVM IR representation,
/// the argument `coerce_pointer_return` must be true when we are dealing with dynamic arrays that are
/// going to be the destination address of a store instruction.
///
/// In a case like this,
///
/// uint16[][] vec;
/// uint16[] vec1;
/// vec[0] = vec1;
///
/// 'vec[0]' must be a pointer to a pointer to a 'struct vector' so that the LLVM Store instruction
/// can be executed properly, as the value we are trying to store there is a pointer to a 'struct vector'.
/// In this case, we must coerce the return of a pointer. Everywhere else, the load is necessary.
///
/// `coerce_pointer_return` has not effect for fixed sized arrays.
fn index_array(
    mut arr: Expression,
    dims: &[ArrayLength],
    indexes: &[usize],
    coerce_pointer_return: bool,
) -> Expression {
    let mut ty = arr.ty();
    let elem_ty = ty.elem_ty();
    let begin = dims.len() - indexes.len();

    for i in (begin..dims.len()).rev() {
        // If we are indexing the last dimension, the type should be that of the array element.
        let local_ty = if i == 0 {
            elem_ty.clone()
        } else {
            Type::Array(Box::new(elem_ty.clone()), dims[0..i].to_vec())
        };
        arr = Expression::Subscript(
            Codegen,
            Type::Ref(local_ty.clone().into()),
            ty,
            Box::new(arr),
            Box::new(Expression::Variable(
                Loc::Codegen,
                Type::Uint(32),
                indexes[dims.len() - i - 1],
            )),
        );

        // We should only load if the dimension is dynamic.
        if i > 0 && dims[i - 1] == ArrayLength::Dynamic {
            arr = Expression::Load(Loc::Codegen, local_ty.clone(), arr.into());
        }

        ty = local_ty;
    }

    if coerce_pointer_return && !matches!(arr.ty(), Type::Ref(_)) {
        if let Expression::Load(_, _, expr) = arr {
            return *expr;
        } else {
            unreachable!("Expression should be a load");
        }
    }

    arr
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
    let index_temp = vartab.temp_name(format!("for_i_{dimension}").as_str(), &Uint(32));

    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: index_temp,
            expr: Expression::NumberLiteral(Codegen, Uint(32), 0u8.into()),
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
    // Get the array length at dimension 'index'
    let bound = if let ArrayLength::Fixed(dim) = &dims[dimension] {
        Expression::NumberLiteral(Codegen, Uint(32), dim.clone())
    } else {
        let sub_array = index_array(arr.clone(), dims, &indexes[..indexes.len() - 1], false);
        Expression::Builtin(
            Codegen,
            vec![Uint(32)],
            Builtin::ArrayLength,
            vec![sub_array],
        )
    };
    let cond_expr = Expression::UnsignedLess(
        Codegen,
        Expression::Variable(Codegen, Uint(32), index_temp).into(),
        bound.into(),
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
    let index_var = Expression::Variable(Codegen, Uint(32), for_loop.index);
    let one = Expression::NumberLiteral(Codegen, Uint(32), 1u8.into());
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: for_loop.index,
            expr: Expression::Add(Codegen, Uint(32), false, index_var.into(), one.into()),
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
        return Expression::StructMember(Codegen, ty, expr.into(), field);
    }
    let s = Expression::StructMember(Codegen, Type::Ref(ty.clone().into()), expr.into(), field);
    Expression::Load(Codegen, ty, s.into())
}

/// Get the outer array length inside a variable (cannot be used for any dimension).
fn array_outer_length(
    arr: &Expression,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let get_size = Expression::Builtin(
        Codegen,
        vec![Uint(32)],
        Builtin::ArrayLength,
        vec![arr.clone()],
    );
    let array_length = vartab.temp_anonymous(&Uint(32));
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: array_length,
            expr: get_size,
        },
    );
    Expression::Variable(Codegen, Uint(32), array_length)
}

/// Increment an expression by some value.
fn increment_by(expr: Expression, value: Expression) -> Expression {
    Expression::Add(Codegen, Uint(32), false, expr.into(), value.into())
}

/// Increment an expression by four. This is useful because we save array sizes as uint32, so we
/// need to increment the offset by four constantly.
fn increment_four(expr: Expression) -> Expression {
    let four = Expression::NumberLiteral(Codegen, Uint(32), 4.into());
    increment_by(expr, four)
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
fn calculate_array_bytes_size(length_var: usize, elem_ty: &Type, ns: &Namespace) -> Expression {
    let var = Expression::Variable(Codegen, Uint(32), length_var);
    let size = Expression::NumberLiteral(Codegen, Uint(32), elem_ty.memory_size_of(ns));
    Expression::Multiply(Codegen, Uint(32), false, var.into(), size.into())
}

/// Retrieve a dynamic array length from the encoded buffer. It returns the variable number in which
/// the length has been stored
fn retrieve_array_length(
    buffer: &Expression,
    offset: &Expression,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> usize {
    let array_length = vartab.temp_anonymous(&Uint(32));
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: array_length,
            expr: Expression::Builtin(
                Codegen,
                vec![Uint(32)],
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
    let length_var = Expression::Variable(Codegen, Uint(32), length_variable);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: array_var,
            expr: Expression::AllocDynamicBytes(Codegen, ty.clone(), length_var.into(), None),
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
