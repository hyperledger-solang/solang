use crate::ast::{Namespace, RetrieveType, Type};
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::{
    calculate_size_args, finish_array_loop, increment_four, load_array_item, load_struct_member,
    set_array_loop, Encoding,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use std::collections::HashMap;
use std::ops::MulAssign;

pub(super) struct BorshEncoding {
    storage_cache: HashMap<usize, Expression>,
}

impl Encoding for BorshEncoding {
    fn abi_encode(
        &mut self,
        loc: &Loc,
        args: &[Expression],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = calculate_size_args(self, args, ns, vartab, cfg);

        let temp = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: temp,
                expr: Expression::AllocDynamicArray(*loc, Type::DynamicBytes, Box::new(size), None),
            },
        );

        let mut offset = Expression::NumberLiteral(*loc, Type::Uint(32), BigInt::from(0u8));
        let buffer = Expression::Variable(*loc, Type::DynamicBytes, temp);

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

    fn cache_storage_load(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
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

            Type::String | Type::DynamicBytes | Type::Slice => {
                let get_size = Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![expr.clone()],
                );
                let arr_length_tem = vartab.temp_anonymous(&Type::Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: Loc::Codegen,
                        res: arr_length_tem,
                        expr: get_size,
                    },
                );

                let var = Expression::Variable(Loc::Codegen, Type::Uint(32), arr_length_tem);
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

                Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(1u8))
            }

            Type::Struct(struct_no) => self.encode_struct(
                expr, buffer, offset, &expr_ty, *struct_no, arg_no, ns, vartab, cfg,
            ),

            Type::Array(ty, dims) => {
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
                } else if ty.is_primitive() {
                    // Only the last dimension can be dynamic in Solidity
                    // If the array has known size and is of a primitive type, we can simply do a memory copy

                    // Calculate number of elements
                    let bytes_size = if dims.last().unwrap().is_some() {
                        let mut elem_no = BigInt::from(1u8);
                        for item in dims {
                            assert!(item.is_some());
                            elem_no.mul_assign(item.as_ref().unwrap());
                        }

                        let bytes = ty.size_of(ns);
                        elem_no.mul_assign(&bytes);
                        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), elem_no)
                    } else {
                        let arr_size = Expression::Builtin(
                            Loc::Codegen,
                            vec![Type::Uint(32)],
                            Builtin::ArrayLength,
                            vec![expr.clone()],
                        );
                        Expression::Multiply(
                            Loc::Codegen,
                            Type::Uint(32),
                            false,
                            Box::new(arr_size),
                            Box::new(Expression::NumberLiteral(
                                Loc::Codegen,
                                Type::Uint(32),
                                ty.size_of(ns),
                            )),
                        )
                    };

                    let dest_address = Expression::AdvancePointer {
                        loc: Loc::Codegen,
                        pointer: Box::new(buffer.clone()),
                        ty: Type::BufferPointer,
                        bytes_offset: Box::new(offset.clone()),
                    };
                    cfg.add(
                        vartab,
                        Instr::MemCopy {
                            source: expr.clone(),
                            destination: dest_address,
                            bytes: bytes_size.clone(),
                        },
                    );

                    bytes_size
                } else {
                    // In all other cases, we must loop through the array

                    // If the array is dynamic, we must save its length before all elements
                    let offset = if dims.last().unwrap().is_none() {
                        let dim = Expression::Builtin(
                            Loc::Codegen,
                            vec![Type::Uint(32)],
                            Builtin::ArrayLength,
                            vec![expr.clone()],
                        );
                        cfg.add(
                            vartab,
                            Instr::WriteBuffer {
                                buf: buffer.clone(),
                                offset: offset.clone(),
                                value: dim,
                            },
                        );
                        increment_four(offset.clone())
                    } else {
                        offset.clone()
                    };

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
                    self.encode_array(
                        expr,
                        arg_no,
                        dims,
                        buffer,
                        offset_var,
                        0,
                        ns,
                        vartab,
                        cfg,
                        &mut indexes,
                    );

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
                                Box::new(offset),
                            ),
                        },
                    );
                    Expression::Variable(Loc::Codegen, Type::Uint(32), offset_var)
                };

                size
            }

            Type::UserType(_) | Type::Unresolved | Type::Rational | Type::Unreachable => {
                unreachable!("Type should not exist in codegen")
            }

            Type::InternalFunction { .. }
            | Type::ExternalFunction { .. }
            | Type::Void
            | Type::BufferPointer
            | Type::Mapping(..) => unreachable!("This type cannot be encoded"),

            Type::Ref(r) => {
                if let Type::Struct(struct_no) = &**r {
                    return self.encode_struct(
                        expr, buffer, offset, &expr_ty, *struct_no, arg_no, ns, vartab, cfg,
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

    /// Encode an array
    fn encode_array(
        &mut self,
        arr: &Expression,
        arg_no: usize,
        dims: &Vec<Option<BigInt>>,
        buffer: &Expression,
        offset_var: usize,
        dimension: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        indexes: &mut Vec<usize>,
    ) {
        let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
        cfg.set_basic_block(for_loop.body_block);
        if dims.len() - 1 == dimension {
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
            self.encode_array(
                arr,
                arg_no,
                dims,
                buffer,
                offset_var,
                dimension + 1,
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
        offset: &Expression,
        expr_ty: &Type,
        struct_no: usize,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let size = if let Some(no_pad_size) = ns.is_primitive_type_struct(struct_no) {
            let padded_size = expr_ty.size_of(ns);
            if padded_size.eq(&no_pad_size) {
                let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), no_pad_size);
                let dest_address = Expression::AdvancePointer {
                    loc: Loc::Codegen,
                    ty: Type::BufferPointer,
                    pointer: Box::new(buffer.clone()),
                    bytes_offset: Box::new(offset.clone()),
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
                Some(Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    no_pad_size,
                ))
            }
        } else {
            None
        };

        let qty = ns.structs[struct_no].fields.len();
        let first_ty = ns.structs[struct_no].fields[0].ty.clone();
        let loaded = load_struct_member(first_ty, expr.clone(), 0);

        let mut advance = self.encode(&loaded, buffer, offset, arg_no, ns, vartab, cfg);
        let mut runtime_size = advance.clone();
        for i in 1..qty {
            let ith_type = ns.structs[struct_no].fields[i].ty.clone();
            let offset = Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(offset.clone()),
                Box::new(advance),
            );
            let loaded = load_struct_member(ith_type.clone(), expr.clone(), i);
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
