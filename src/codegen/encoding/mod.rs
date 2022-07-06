mod borsh_encoding;

use crate::ast::{Namespace, RetrieveType, Type};
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::borsh_encoding::BorshEncoding;
use crate::codegen::expression::load_storage;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::Target;
use num_bigint::BigInt;
use solang_parser::pt::Loc;

pub(super) trait Encoding {
    /// Receive the arguments and returns the variable containing a byte array
    fn abi_encode(
        &mut self,
        loc: &Loc,
        args: &[Expression],
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression;

    /// Cache items loaded from storage to reuse the later
    fn cache_storage_load(&mut self, arg_no: usize, expr: Expression);
}

pub(super) fn create_encoder(ns: &Namespace) -> impl Encoding {
    match &ns.target {
        Target::Solana => BorshEncoding::new(),
        _ => unreachable!("Other types of encoding have not been implemented yet"),
    }
}

/// Calculate the size of a set of arguments to encoding functions
fn calculate_size_args<T: Encoding>(
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
fn get_expr_size<T: Encoding>(
    encoder: &mut T,
    arg_no: usize,
    expr: &Expression,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let ty = expr.ty().unwrap_user_type(ns);
    match &ty {
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

        Type::Value => {
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(ns.value_length))
        }

        Type::String | Type::DynamicBytes | Type::Slice => {
            // when encoding a variable length array, the total size is "length (u32)" + elements
            let length = Expression::Builtin(
                Loc::Codegen,
                vec![Type::Uint(32)],
                Builtin::ArrayLength,
                vec![expr.clone()],
            );
            increment_four(length)
        }

        Type::Struct(struct_no) => {
            calculate_struct_size(encoder, arg_no, expr, *struct_no, ns, vartab, cfg)
        }

        Type::Array(ty, dims) => {
            let primitive_size = if ty.is_primitive() {
                Some(ty.memory_size_of(ns))
            } else if let Type::Struct(struct_no) = &**ty {
                ns.is_primitive_type_struct(*struct_no)
            } else {
                None
            };

            let size_var = if let Some(compile_type_size) = primitive_size {
                // the array saves primitive-type elements, its size is sizeof(type)*vec.length
                let mut size = get_array_length(expr, dims, 0);

                for i in 1..dims.len() {
                    let local_size = get_array_length(expr, dims, i);
                    size = Expression::Multiply(
                        Loc::Codegen,
                        Type::Uint(32),
                        false,
                        Box::new(size),
                        Box::new(local_size),
                    );
                }

                let type_size =
                    Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), compile_type_size);
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
                        expr: Expression::NumberLiteral(
                            Loc::Codegen,
                            Type::Uint(32),
                            BigInt::from(0u8),
                        ),
                    },
                );
                let mut index_vec: Vec<usize> = Vec::new();
                calculate_array_size(
                    encoder,
                    arg_no,
                    expr,
                    dims,
                    0,
                    size_var,
                    ns,
                    &mut index_vec,
                    vartab,
                    cfg,
                );
                size_var
            };

            if dims.last().unwrap().is_none() {
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
                                BigInt::from(4u8),
                            )),
                        ),
                    },
                );
            }

            Expression::Variable(Loc::Codegen, Type::Uint(32), size_var)
        }

        Type::UserType(_) | Type::Unresolved | Type::Rational => {
            unreachable!("Type should not exist in codegen")
        }

        Type::InternalFunction { .. }
        | Type::ExternalFunction { .. }
        | Type::Void
        | Type::Unreachable
        | Type::BufferPointer
        | Type::Mapping(..) => unreachable!("This type cannot be encoded"),

        Type::Ref(r) => {
            if let Type::Struct(struct_no) = &**r {
                return calculate_struct_size(encoder, arg_no, expr, *struct_no, ns, vartab, cfg);
            }
            let loaded = Expression::Load(Loc::Codegen, *r.clone(), Box::new(expr.clone()));
            get_expr_size(encoder, arg_no, &loaded, ns, vartab, cfg)
        }

        Type::StorageRef(_, r) => {
            let var = load_storage(&Loc::Codegen, r, expr.clone(), cfg, vartab);
            let size = get_expr_size(encoder, arg_no, &var, ns, vartab, cfg);
            encoder.cache_storage_load(arg_no, size.clone());
            size
        }
    }
}

/// Calculate the size of an array
fn calculate_array_size<T: Encoding>(
    encoder: &mut T,
    arg_no: usize,
    arr: &Expression,
    dims: &Vec<Option<BigInt>>,
    dimension: usize,
    size_var_no: usize,
    ns: &Namespace,
    indexes: &mut Vec<usize>,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) {
    let for_loop = set_array_loop(arr, dims, dimension, indexes, vartab, cfg);
    cfg.set_basic_block(for_loop.body_block);
    if dims.len() - 1 == dimension {
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
        calculate_array_size(
            encoder,
            arg_no,
            arr,
            dims,
            dimension + 1,
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
fn get_array_length(arr: &Expression, dims: &[Option<BigInt>], index: usize) -> Expression {
    if let Some(dim) = &dims[index] {
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), dim.clone())
    } else {
        Expression::Builtin(
            Loc::Codegen,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![arr.clone()],
        )
    }
}

/// Retrieves the size of a struct
fn calculate_struct_size<T: Encoding>(
    encoder: &mut T,
    arg_no: usize,
    expr: &Expression,
    struct_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    if let Some(struct_size) = ns.is_primitive_type_struct(struct_no) {
        return Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), struct_size);
    }

    let first_type = ns.structs[struct_no].fields[0].ty.clone();
    let first_field = load_struct_member(first_type, expr.clone(), 0);
    let mut size = get_expr_size(encoder, arg_no, &first_field, ns, vartab, cfg);
    for i in 1..ns.structs[struct_no].fields.len() {
        let ty = ns.structs[struct_no].fields[i].ty.clone();
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
fn load_array_item(arr: &Expression, dims: &[Option<BigInt>], indexes: &[usize]) -> Expression {
    let mut ty = arr.ty();
    let elem_ty = ty.elem_ty();
    let mut deref = arr.clone();
    for i in (1..dims.len()).rev() {
        let local_ty = Type::Array(Box::new(elem_ty.clone()), dims[0..i].to_vec());
        deref = Expression::Subscript(
            Loc::Codegen,
            Type::Ref(Box::new(local_ty.clone())),
            ty,
            Box::new(deref.clone()),
            Box::new(Expression::Variable(
                Loc::Codegen,
                Type::Uint(32),
                indexes[i],
            )),
        );
        ty = local_ty;
    }
    Expression::Subscript(
        Loc::Codegen,
        Type::Ref(Box::new(elem_ty)),
        ty,
        Box::new(deref),
        Box::new(Expression::Variable(
            Loc::Codegen,
            Type::Uint(32),
            indexes[0],
        )),
    )
}

/// This struct manages for-loops created when encoding arrays
struct ForLoop {
    pub cond_block: usize,
    pub next_block: usize,
    pub body_block: usize,
    pub end_block: usize,
    pub index: usize,
}

/// Set up the loop to encode an array
fn set_array_loop(
    arr: &Expression,
    dims: &[Option<BigInt>],
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
    let bound = get_array_length(arr, dims, dimension);
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

/// Closes the for-loop when encoding an array
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
