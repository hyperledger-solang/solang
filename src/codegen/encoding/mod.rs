// SPDX-License-Identifier: Apache-2.0

mod borsh_encoding;
mod buffer_validator;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::borsh_encoding::BorshEncoding;
use crate::codegen::expression::load_storage;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type};
use crate::Target;
use num_bigint::BigInt;
use num_traits::One;
use solang_parser::pt::Loc;
use std::ops::{AddAssign, MulAssign};

/// This trait should be implemented by all encoding methods (ethabi, Scale and Borsh), so that
/// we have the same interface for creating encode and decode functions.
pub(super) trait AbiEncoding {
    /// Receive the arguments and returns the variable containing a byte array
    fn abi_encode(
        &mut self,
        loc: &Loc,
        args: &[Expression],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression;

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Vec<Expression>;

    /// Cache items loaded from storage to reuse them later, so we avoid the expensive operation
    /// of loading from storage twice. We need the data in two different passes: first to
    /// calculate its size and then to copy it to the buffer.
    ///
    /// This function serves only to cache Expression::Variable, containing items loaded from storage.
    /// Nothing else should be stored here. For more information, check the comment at
    /// 'struct BorshEncoding' on borsh_encoding.rs
    fn cache_storage_loaded(&mut self, arg_no: usize, expr: Expression);

    /// Some types have sizes that are specific to each encoding scheme, so there is no way to generalize.
    fn get_encoding_size(&self, expr: &Expression, ty: &Type, ns: &Namespace) -> Expression;
}

/// This function should return the correct encoder, given the target
pub(super) fn create_encoder(ns: &Namespace) -> impl AbiEncoding {
    match &ns.target {
        Target::Solana => BorshEncoding::new(),
        _ => unreachable!("Other types of encoding have not been implemented yet"),
    }
}

/// Calculate the size of a set of arguments to encoding functions
fn calculate_size_args<T: AbiEncoding>(
    encoder: &mut T,
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
fn get_expr_size<T: AbiEncoding>(
    encoder: &mut T,
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
            encoder.cache_storage_loaded(arg_no, var.clone());
            size
        }

        _ => encoder.get_encoding_size(expr, &ty, ns),
    }
}

/// Calculate the size of an array
fn calculate_array_size<T: AbiEncoding>(
    encoder: &mut T,
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

    if dyn_dims > 0 {
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
fn calculate_complex_array_size<T: AbiEncoding>(
    encoder: &mut T,
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
fn calculate_struct_size<T: AbiEncoding>(
    encoder: &mut T,
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
    if matches!(ty, Type::Struct(_)) {
        // We should not dereference a struct.
        return Expression::StructMember(
            Loc::Codegen,
            Type::Ref(Box::new(ty)),
            Box::new(expr),
            field,
        );
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
            no_padded_size.eq(&elem_ty.solana_storage_size(ns)) && ns.target == Target::Solana
        } else {
            false
        }
    } else if let Type::Bytes(n) = elem_ty {
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
