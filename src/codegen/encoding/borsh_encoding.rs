use crate::ast::{ArrayLength, Namespace, RetrieveType, Type};
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::{
    calculate_size_args, finish_array_loop, increment_four, load_array_item, load_struct_member,
    load_sub_array, set_array_loop, AbiEncoding,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
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
}

impl AbiEncoding for BorshEncoding {
    fn abi_encode(
        &mut self,
        loc: &Loc,
        args: &[Expression],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = calculate_size_args(self, args, ns, vartab, cfg);

        let encoded_bytes = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: encoded_bytes,
                expr: Expression::AllocDynamicArray(*loc, Type::DynamicBytes, Box::new(size), None),
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

        buffer
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

            Type::String | Type::DynamicBytes | Type::Slice(_) => {
                // When encoding a variable length array, the total size is "length (u32)" + elements
                let length = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![expr.clone()],
                );
                increment_four(length)
            }

            _ => unreachable!("Type should have the same size for all encoding schemes"),
        }
    }
}

impl BorshEncoding {
    pub fn new() -> BorshEncoding {
        BorshEncoding {
            storage_cache: HashMap::new(),
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

            Type::String | Type::DynamicBytes | Type::Slice(_) => {
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
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: var.clone(),
                    },
                );

                // ptr + offset + size_of_integer
                let dest_address = Expression::AdvancePointer {
                    loc: Loc::Codegen,
                    ty: Type::BufferPointer,
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(increment_four(offset.clone())),
                };

                cfg.add(
                    vartab,
                    Instr::MemCopy {
                        source: expr.clone(),
                        destination: dest_address,
                        bytes: var.clone(),
                    },
                );

                increment_four(var)
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

            Type::Struct(struct_no) => self.encode_struct(
                expr,
                buffer,
                offset.clone(),
                &expr_ty,
                *struct_no,
                arg_no,
                ns,
                vartab,
                cfg,
            ),

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
                if let Type::Struct(struct_no) = &**r {
                    // Structs references should not be dereferenced
                    return self.encode_struct(
                        expr,
                        buffer,
                        offset.clone(),
                        &expr_ty,
                        *struct_no,
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
        // Check if we can MemCpy elements into the buffer
        let direct_encoding = if array_ty.is_dynamic(ns) {
            // If this is a dynamic array, we can only MemCpy if its elements are of
            // any primitive type and we don't need to index it.
            dims.len() == 1 && elem_ty.is_primitive()
        } else {
            // If the array is not dynamic, we can MemCpy elements if their are primitive.
            elem_ty.is_primitive()
        };

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
        } else if direct_encoding {
            // Calculate number of elements
            let (bytes_size, offset) = if matches!(dims.last(), Some(&ArrayLength::Fixed(_))) {
                let mut elem_no = BigInt::from(1u8);
                for item in dims {
                    assert!(matches!(item, &ArrayLength::Fixed(_)));
                    elem_no.mul_assign(item.array_length().unwrap());
                }

                let bytes = elem_ty.memory_size_of(ns);
                elem_no.mul_assign(&bytes);
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

                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: Expression::Variable(Loc::Codegen, Type::Uint(32), size_temp),
                    },
                );

                let size = Expression::Multiply(
                    Loc::Codegen,
                    Type::Uint(32),
                    false,
                    Box::new(Expression::Variable(
                        Loc::Codegen,
                        Type::Uint(32),
                        size_temp,
                    )),
                    Box::new(Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        elem_ty.memory_size_of(ns),
                    )),
                );

                (size, increment_four(offset.clone()))
            };

            let dest_address = Expression::AdvancePointer {
                loc: Loc::Codegen,
                pointer: Box::new(buffer.clone()),
                ty: Type::BufferPointer,
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
            if dyn_dims > 0 {
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
        if dims[dimension] == ArrayLength::Dynamic {
            // TODO: This is wired up for the support of dynamic multidimensional arrays, like
            // TODO: 'int[3][][4] vec', but it needs testing, as soon as Solang works with them.
            // TODO: A discussion about this is under way here: https://github.com/hyperledger-labs/solang/issues/932
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
        };

        finish_array_loop(&for_loop, vartab, cfg);
    }

    /// Encode a struct
    fn encode_struct(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        mut offset: Expression,
        expr_ty: &Type,
        struct_no: usize,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = if let Some(no_padding_size) = ns.calculate_struct_non_padded_size(struct_no) {
            let padded_size = expr_ty.solana_storage_size(ns);
            // If the size without padding equals the size with padding, we
            // can memcpy this struct directly.
            if padded_size.eq(&no_padding_size) {
                let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), no_padding_size);
                let dest_address = Expression::AdvancePointer {
                    loc: Loc::Codegen,
                    ty: Type::BufferPointer,
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

        let qty = ns.structs[struct_no].fields.len();
        let first_ty = ns.structs[struct_no].fields[0].ty.clone();
        let loaded = load_struct_member(first_ty, expr.clone(), 0);

        let mut advance = self.encode(&loaded, buffer, &offset, arg_no, ns, vartab, cfg);
        let mut runtime_size = advance.clone();
        for i in 1..qty {
            let ith_type = ns.structs[struct_no].fields[i].ty.clone();
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
}
