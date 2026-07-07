// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::codegen::error::CodegenError;
use crate::codegen::revert::PanicCode;
use crate::codegen::{Builtin, Expression};
use crate::emit::binary::Binary;
use crate::emit::math::{build_binary_op_with_overflow_check, multiply, power};
use crate::emit::strings::{format_string, string_location};
use crate::emit::{loop_builder::LoopBuilder, BinaryOp, TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast::{ArrayLength, RetrieveType, StructType, Type};
use crate::Target;
use inkwell::builder::BuilderError;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, StringRadix};
use inkwell::values::{
    ArrayValue, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use num_bigint::Sign;
use num_traits::ToPrimitive;
use std::collections::HashMap;

fn emit_or_panic<T>(result: Result<T, BuilderError>, operation: impl Into<String>) -> T {
    result.unwrap_or_else(|err| panic!("{}", CodegenError::llvm_builder(operation, err)))
}

fn runtime_helper<'a>(
    bin: &Binary<'a>,
    name: &str,
    operation: impl Into<String>,
) -> FunctionValue<'a> {
    bin.module.get_function(name).unwrap_or_else(|| {
        panic!(
            "{}",
            CodegenError::missing_runtime_helper(name, operation, bin.ns.target)
        );
    })
}

fn expect_llvm_entity<T>(
    value: Option<T>,
    operation: impl Into<String>,
    entity: impl Into<String>,
) -> T {
    value.unwrap_or_else(|| {
        panic!("{}", CodegenError::missing_llvm_entity(operation, entity));
    })
}

fn expect_return_value<T>(value: Option<T>, operation: impl Into<String>) -> T {
    expect_llvm_entity(value, operation, "expected return value")
}

fn expect_numeric_conversion<T>(
    value: Option<T>,
    operation: impl Into<String>,
    raw_value: impl Into<String>,
    target_type: impl Into<String>,
) -> T {
    value.unwrap_or_else(|| {
        panic!(
            "{}",
            CodegenError::numeric_conversion(operation, raw_value, target_type)
        );
    })
}

fn invalid_cfg<T>(operation: impl Into<String>, reason: impl Into<String>) -> T {
    panic!("{}", CodegenError::invalid_cfg_invariant(operation, reason))
}

/// The expression function recursively emits code for expressions. The BasicEnumValue it
/// returns depends on the context; if it is simple integer, bool or bytes32 expression, the value
/// is an Intvalue. For references to arrays, it is a PointerValue to the array. For references
/// to storage, it is the storage slot. The references types are dereferenced by the Expression::Load()
/// and Expression::StorageLoad() expression types.
pub(super) fn expression<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    e: &Expression,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
) -> BasicValueEnum<'a> {
    emit_context!(bin);
    match e {
        Expression::FunctionArg { arg_no, .. } => expect_llvm_entity(
            function.get_nth_param(*arg_no as u32),
            "emitting function argument expression",
            format!("function parameter {arg_no}"),
        ),
        Expression::BoolLiteral { value, .. } => bin
            .context
            .bool_type()
            .const_int(*value as u64, false)
            .into(),
        Expression::NumberLiteral {
            ty: Type::Address(_),
            value,
            ..
        } => {
            // address can be negative; "address(-1)" is 0xffff...
            let mut bs = value.to_signed_bytes_be();

            // make sure it's no more than 32
            if bs.len() > bin.ns.address_length {
                // remove leading bytes
                for _ in 0..bs.len() - bin.ns.address_length {
                    bs.remove(0);
                }
            } else {
                // insert leading bytes
                let val = if value.sign() == Sign::Minus { 0xff } else { 0 };

                for _ in 0..bin.ns.address_length - bs.len() {
                    bs.insert(0, val);
                }
            }

            let address = bs
                .iter()
                .map(|b| bin.context.i8_type().const_int(*b as u64, false))
                .collect::<Vec<IntValue>>();

            bin.context.i8_type().const_array(&address).into()
        }
        Expression::NumberLiteral { ty, value, .. } => {
            bin.number_literal(ty.bits(bin.ns) as u32, value).into()
        }
        Expression::StructLiteral {
            ty, values: fields, ..
        } => {
            let struct_ty = bin.llvm_type(ty);
            let s = bin
                .builder
                .build_call(
                    runtime_helper(bin, bin.alloc(), "allocating struct literal"),
                    &[struct_ty
                        .size_of()
                        .unwrap_or_else(|| {
                            panic!(
                                "{}",
                                CodegenError::missing_llvm_entity(
                                    "emitting expression",
                                    "type size"
                                )
                            )
                        })
                        .const_cast(bin.context.i32_type(), false)
                        .into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_pointer_value();

            for (i, expr) in fields.iter().enumerate() {
                let elemptr = unsafe {
                    bin.builder
                        .build_gep(
                            struct_ty,
                            s,
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            "struct member",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                };

                let elem = expression(target, bin, expr, vartab, function);

                let elem = if expr.ty().is_fixed_reference_type(bin.ns) {
                    let load_type = bin.llvm_type(&expr.ty());
                    bin.builder
                        .build_load(load_type, elem.into_pointer_value(), "elem")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    elem
                };

                bin.builder
                    .build_store(elemptr, elem)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });
            }

            s.into()
        }
        Expression::BytesLiteral { value: bs, ty, .. } => {
            // If the type of a BytesLiteral is a String, embedd the bytes in the binary.
            if ty == &Type::String || ty == &Type::Address(true) {
                let data = bin.emit_global_string("const_string", bs, true);

                // A constant string, or array, is represented by a struct with two fields: a pointer to the data, and its length.
                let ty = bin.context.struct_type(
                    &[
                        bin.context.ptr_type(AddressSpace::default()).into(),
                        bin.context.i64_type().into(),
                    ],
                    false,
                );

                return ty
                    .const_named_struct(&[
                        data.into(),
                        bin.context
                            .i64_type()
                            .const_int(bs.len() as u64, false)
                            .into(),
                    ])
                    .into();
            }

            let ty = bin.context.custom_width_int_type((bs.len() * 8) as u32);

            // hex"11223344" should become i32 0x11223344
            let s = hex::encode(bs);

            ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                .unwrap_or_else(|| {
                    panic!(
                        "{}",
                        CodegenError::missing_llvm_entity("emitting expression", "integer literal")
                    )
                })
                .into()
        }
        Expression::Add {
            loc,
            ty,
            overflowing,
            left,
            right,
            ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            if !overflowing {
                let signed = ty.is_signed_int(bin.ns);
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Add,
                    signed,
                    *loc,
                )
                .into()
            } else {
                bin.builder
                    .build_int_add(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::Subtract {
            loc,
            ty,
            overflowing,
            left,
            right,
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            if !overflowing {
                let signed = ty.is_signed_int(bin.ns);
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Subtract,
                    signed,
                    *loc,
                )
                .into()
            } else {
                bin.builder
                    .build_int_sub(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::Multiply {
            loc,
            ty: res_ty,
            overflowing,
            left,
            right,
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            multiply(
                target,
                bin,
                function,
                *overflowing,
                left,
                right,
                res_ty.is_signed_int(bin.ns),
                *loc,
            )
            .into()
        }
        Expression::UnsignedDivide {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("udivmod{div_bits}");

                let f = runtime_helper(bin, &name, format!("{bits}-bit unsigned division"));

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder
                    .build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder
                                .build_int_z_extend(left, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            left
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder
                                .build_int_z_extend(right, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            right
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

                let success = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                emit_or_panic(
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block),
                    format!("emitting division-by-zero guard for {bits}-bit unsigned division"),
                );

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc));
                let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(ty, quotient, "quotient")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(quotient, left.get_type(), "")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    quotient
                }
                .into()
            } else {
                bin.builder
                    .build_int_unsigned_div(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::SignedDivide {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("sdivmod{div_bits}");

                let f = runtime_helper(bin, &name, format!("{bits}-bit signed division"));

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder
                    .build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder
                                .build_int_s_extend(left, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            left
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder
                                .build_int_s_extend(right, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            right
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

                let success = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                emit_or_panic(
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block),
                    format!("emitting division-by-zero guard for {bits}-bit signed division"),
                );

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc));
                let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(ty, quotient, "quotient")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(quotient, left.get_type(), "")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    quotient
                }
                .into()
            } else if bin.ns.target == Target::Solana {
                // no signed div on BPF; do abs udev and then negate if needed
                let left_negative = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let left = bin
                    .builder
                    .build_select(
                        left_negative,
                        bin.builder
                            .build_int_neg(left, "signed_left")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        left,
                        "left_abs",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                let right_negative = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let right = bin
                    .builder
                    .build_select(
                        right_negative,
                        bin.builder
                            .build_int_neg(right, "signed_right")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        right,
                        "right_abs",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                let res = bin
                    .builder
                    .build_int_unsigned_div(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let negate_result = bin
                    .builder
                    .build_xor(left_negative, right_negative, "negate_result")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_select(
                        negate_result,
                        bin.builder
                            .build_int_neg(res, "unsigned_res")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        res,
                        "res",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            } else {
                bin.builder
                    .build_int_signed_div(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::UnsignedModulo {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("udivmod{div_bits}");

                let f = runtime_helper(bin, &name, format!("{bits}-bit unsigned modulo"));

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder
                    .build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder
                                .build_int_z_extend(left, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            left
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder
                                .build_int_z_extend(right, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            right
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

                let success = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                emit_or_panic(
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block),
                    format!("emitting division-by-zero guard for {bits}-bit unsigned modulo"),
                );

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc));
                let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let rem = bin
                    .builder
                    .build_load(ty, rem, "urem")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(rem, bin.context.custom_width_int_type(bits), "")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    rem
                }
                .into()
            } else {
                bin.builder
                    .build_int_unsigned_rem(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::SignedModulo {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("sdivmod{div_bits}");

                let f = runtime_helper(bin, &name, format!("{bits}-bit signed modulo"));

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder
                    .build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder
                                .build_int_s_extend(left, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            left
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder
                                .build_int_s_extend(right, ty, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                })
                        } else {
                            right
                        },
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "sdiv",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

                let success = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                emit_or_panic(
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block),
                    format!("emitting division-by-zero guard for {bits}-bit signed modulo"),
                );

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc));
                let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let rem = bin
                    .builder
                    .build_load(ty, rem, "srem")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(rem, bin.context.custom_width_int_type(bits), "")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    rem
                }
                .into()
            } else if bin.ns.target == Target::Solana {
                // no signed rem on BPF; do abs udev and then negate if needed
                let left_negative = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let left = bin
                    .builder
                    .build_select(
                        left_negative,
                        bin.builder
                            .build_int_neg(left, "signed_left")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        left,
                        "left_abs",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let right_negative = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let right = bin
                    .builder
                    .build_select(
                        right_negative,
                        bin.builder
                            .build_int_neg(right, "signed_right")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        right,
                        "right_abs",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let res = bin
                    .builder
                    .build_int_unsigned_rem(left.into_int_value(), right.into_int_value(), "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_select(
                        left_negative,
                        bin.builder
                            .build_int_neg(res, "unsigned_res")
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            }),
                        res,
                        "res",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            } else {
                bin.builder
                    .build_int_signed_rem(left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::Power {
            loc,
            ty: res_ty,
            overflowing,
            base: l,
            exp: r,
        } => {
            let left = expression(target, bin, l, vartab, function);
            let right = expression(target, bin, r, vartab, function);

            let bits = left.into_int_value().get_type().get_bit_width();
            let o = bin.build_alloca(function, left.get_type(), "");
            let f = power(
                target,
                bin,
                *overflowing,
                bits,
                res_ty.is_signed_int(bin.ns),
                o,
                *loc,
            );

            // If the function returns zero, then the operation was successful.
            let error_return = bin
                .builder
                .build_call(f, &[left.into(), right.into(), o.into()], "power")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

            // Load the result pointer
            let res = bin
                .builder
                .build_load(left.get_type(), o, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            // A return other than zero will abort execution. We need to check if power() returned a zero or not.
            let error_block = bin.context.append_basic_block(function, "error");
            let return_block = bin.context.append_basic_block(function, "return_block");

            let error_ret = bin
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    error_return.into_int_value(),
                    error_return.get_type().const_zero().into_int_value(),
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            emit_or_panic(
                bin.builder
                    .build_conditional_branch(error_ret, error_block, return_block),
                "emitting math-overflow guard for power expression",
            );
            bin.builder.position_at_end(error_block);

            bin.log_runtime_error(target, "math overflow".to_string(), Some(*loc));
            let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::MathOverflow);
            target.assert_failure(bin, revert_out, revert_out_len);

            bin.builder.position_at_end(return_block);

            res
        }
        Expression::Equal { left, right, .. } => {
            if left.ty().is_address() {
                let mut res = bin.context.bool_type().const_int(1, false);
                let left = expression(target, bin, left, vartab, function).into_array_value();
                let right = expression(target, bin, right, vartab, function).into_array_value();

                // TODO: Address should be passed around as pointer. Once this is done, we can replace
                // this with a call to address_equal()
                for index in 0..bin.ns.address_length {
                    let l = bin
                        .builder
                        .build_extract_value(left, index as u32, "left")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into_int_value();
                    let r = bin
                        .builder
                        .build_extract_value(right, index as u32, "right")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into_int_value();

                    res = bin
                        .builder
                        .build_and(
                            res,
                            bin.builder
                                .build_int_compare(IntPredicate::EQ, l, r, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                }),
                            "cmp",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        });
                }

                res.into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::EQ, left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::NotEqual { left, right, .. } => {
            if left.ty().is_address() {
                let mut res = bin.context.bool_type().const_int(0, false);
                let left = expression(target, bin, left, vartab, function).into_array_value();
                let right = expression(target, bin, right, vartab, function).into_array_value();

                // TODO: Address should be passed around as pointer. Once this is done, we can replace
                // this with a call to address_equal()
                for index in 0..bin.ns.address_length {
                    let l = bin
                        .builder
                        .build_extract_value(left, index as u32, "left")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into_int_value();
                    let r = bin
                        .builder
                        .build_extract_value(right, index as u32, "right")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into_int_value();

                    res = bin
                        .builder
                        .build_or(
                            res,
                            bin.builder
                                .build_int_compare(IntPredicate::NE, l, r, "")
                                .unwrap_or_else(|err| {
                                    panic!(
                                        "{}",
                                        CodegenError::llvm_builder("emitting expression", err)
                                    )
                                }),
                            "cmp",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        });
                }

                res.into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::NE, left, right, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::More {
            signed,
            left,
            right,
            ..
        } => {
            if left.ty().is_address() {
                compare_address(
                    target,
                    bin,
                    left,
                    right,
                    IntPredicate::SGT,
                    vartab,
                    function,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(
                        if *signed {
                            IntPredicate::SGT
                        } else {
                            IntPredicate::UGT
                        },
                        left,
                        right,
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::MoreEqual {
            signed,
            left,
            right,
            ..
        } => {
            if left.ty().is_address() {
                compare_address(
                    target,
                    bin,
                    left,
                    right,
                    IntPredicate::SGE,
                    vartab,
                    function,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(
                        if *signed {
                            IntPredicate::SGE
                        } else {
                            IntPredicate::UGE
                        },
                        left,
                        right,
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::Less {
            signed,
            left,
            right,
            ..
        } => {
            if left.ty().is_address() {
                compare_address(
                    target,
                    bin,
                    left,
                    right,
                    IntPredicate::SLT,
                    vartab,
                    function,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(
                        if *signed {
                            IntPredicate::SLT
                        } else {
                            IntPredicate::ULT
                        },
                        left,
                        right,
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::LessEqual {
            signed,
            left,
            right,
            ..
        } => {
            if left.ty().is_address() {
                compare_address(
                    target,
                    bin,
                    left,
                    right,
                    IntPredicate::SLE,
                    vartab,
                    function,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function).into_int_value();
                let right = expression(target, bin, right, vartab, function).into_int_value();

                bin.builder
                    .build_int_compare(
                        if *signed {
                            IntPredicate::SLE
                        } else {
                            IntPredicate::ULE
                        },
                        left,
                        right,
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            }
        }
        Expression::Variable { var_no, .. } => vartab[var_no].value,
        Expression::GetRef { expr, .. } => {
            let address = expression(target, bin, expr, vartab, function).into_array_value();

            let stack = bin.build_alloca(function, address.get_type(), "address");

            bin.builder
                .build_store(stack, address)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            stack.into()
        }
        Expression::Load { ty, expr, .. } => {
            let ptr = expression(target, bin, expr, vartab, function).into_pointer_value();

            if ty.is_reference_type(bin.ns) && !ty.is_fixed_reference_type(bin.ns) {
                let loaded_type = bin.context.ptr_type(AddressSpace::default());
                let value = bin
                    .builder
                    .build_load(loaded_type, ptr, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });
                // if the pointer is null, it needs to be allocated
                let allocation_needed = bin
                    .builder
                    .build_is_null(value.into_pointer_value(), "allocation_needed")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                let allocate = bin.context.append_basic_block(function, "allocate");
                let already_allocated = bin
                    .context
                    .append_basic_block(function, "already_allocated");

                emit_or_panic(
                    bin.builder.build_conditional_branch(
                        allocation_needed,
                        allocate,
                        already_allocated,
                    ),
                    "emitting lazy allocation branch for reference load",
                );

                let entry = expect_llvm_entity(
                    bin.builder.get_insert_block(),
                    "emitting lazy allocation for reference load",
                    "current insertion block",
                );

                bin.builder.position_at_end(allocate);

                // allocate a new struct
                let ty = expr.ty();

                let llvm_ty = bin.llvm_type(ty.deref_memory());

                let new_struct = bin
                    .builder
                    .build_call(
                        runtime_helper(bin, bin.alloc(), "allocating lazy reference load"),
                        &[llvm_ty
                            .size_of()
                            .unwrap_or_else(|| {
                                panic!(
                                    "{}",
                                    CodegenError::missing_llvm_entity(
                                        "emitting expression",
                                        "type size"
                                    )
                                )
                            })
                            .const_cast(bin.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                    .into_pointer_value();

                bin.builder
                    .build_store(ptr, new_struct)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder
                    .build_unconditional_branch(already_allocated)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                bin.builder.position_at_end(already_allocated);

                // insert phi node
                let combined_struct_ptr = bin
                    .builder
                    .build_phi(
                        bin.context.ptr_type(AddressSpace::default()),
                        &format!("ptr_{}", ty.to_string(bin.ns)),
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                combined_struct_ptr.add_incoming(&[(&value, entry), (&new_struct, allocate)]);

                combined_struct_ptr.as_basic_value()
            } else {
                let loaded_type = bin.llvm_type(ty);
                bin.builder
                    .build_load(loaded_type, ptr, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            }
        }

        Expression::ZeroExt { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();
            let ty = bin.llvm_type(ty);

            bin.builder
                .build_int_z_extend(e, ty.into_int_type(), "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Negate {
            loc,
            expr,
            overflowing,
            ..
        } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();

            if *overflowing {
                bin.builder
                    .build_int_neg(e, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .into()
            } else {
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    e.get_type().const_zero(),
                    e,
                    BinaryOp::Subtract,
                    true,
                    *loc,
                )
                .into()
            }
        }
        Expression::SignExt { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();
            let ty = bin.llvm_type(ty);

            bin.builder
                .build_int_s_extend(e, ty.into_int_type(), "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Trunc { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();
            let ty = bin.llvm_type(ty);

            bin.builder
                .build_int_truncate(e, ty.into_int_type(), "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Cast { ty: to, expr, .. } => {
            let from = expr.ty();

            let e = expression(target, bin, expr, vartab, function);

            runtime_cast(bin, function, &from, to, e)
        }
        Expression::BytesCast {
            ty: Type::DynamicBytes,
            from: Type::Bytes(_),
            expr,
            ..
        } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();

            let size = e.get_type().get_bit_width() / 8;
            let size = bin.context.i32_type().const_int(size as u64, false);
            let elem_size = bin.context.i32_type().const_int(1, false);

            // Swap the byte order
            let bytes_ptr = bin.build_alloca(function, e.get_type(), "bytes_ptr");
            bin.builder.build_store(bytes_ptr, e).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });
            let init = bin.build_alloca(function, e.get_type(), "init");
            bin.builder
                .build_call(
                    runtime_helper(bin, "__leNtobeN", "converting bytes to big endian"),
                    &[bytes_ptr.into(), init.into(), size.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let allocator = if bin.ns.target == Target::Soroban {
                bin.builder
                    .build_call(
                        bin.module.get_function("soroban_alloc_init").unwrap(),
                        &[size.into(), init.into()],
                        "soroban_alloc",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            } else {
                bin.builder
                    .build_call(
                        runtime_helper(bin, "vector_new", "creating bytes cast error payload"),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            };
            allocator
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
        }
        Expression::BytesCast {
            loc,
            ty: Type::Bytes(n),
            from: Type::DynamicBytes,
            expr: e,
        } => {
            let array = expression(target, bin, e, vartab, function);

            let len = bin.vector_len(array);

            // Check if equal to n
            let is_equal_to_n = bin
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    len,
                    bin.context.i32_type().const_int(*n as u64, false),
                    "is_equal_to_n",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            let cast = bin.context.append_basic_block(function, "cast");
            let error = bin.context.append_basic_block(function, "error");
            emit_or_panic(
                bin.builder
                    .build_conditional_branch(is_equal_to_n, cast, error),
                format!("emitting dynamic bytes to bytes{n} cast length check"),
            );

            bin.builder.position_at_end(error);
            bin.log_runtime_error(target, "bytes cast error".to_string(), Some(*loc));
            let (revert_out, revert_out_len) = bin.panic_data_const(PanicCode::Generic);
            target.assert_failure(bin, revert_out, revert_out_len);

            bin.builder.position_at_end(cast);
            let bytes_ptr = bin.vector_bytes(array);

            // Switch byte order
            let ty = bin.context.custom_width_int_type(*n as u32 * 8);
            let le_bytes_ptr = bin.build_alloca(function, ty, "le_bytes");

            bin.builder
                .build_call(
                    runtime_helper(bin, "__beNtoleN", "converting bytes to little endian"),
                    &[bytes_ptr.into(), le_bytes_ptr.into(), len.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            bin.builder
                .build_load(ty, le_bytes_ptr, "bytes")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        Expression::Not { expr, .. } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();

            bin.builder
                .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::BitwiseNot { expr, .. } => {
            let e = expression(target, bin, expr, vartab, function).into_int_value();

            bin.builder
                .build_not(e, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::BitwiseOr { left, right: r, .. } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, r, vartab, function).into_int_value();

            bin.builder
                .build_or(left, right, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::BitwiseAnd { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            bin.builder
                .build_and(left, right, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::BitwiseXor { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            bin.builder
                .build_xor(left, right, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::ShiftLeft { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            bin.builder
                .build_left_shift(left, right, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::ShiftRight {
            left,
            right,
            signed,
            ..
        } => {
            let left = expression(target, bin, left, vartab, function).into_int_value();
            let right = expression(target, bin, right, vartab, function).into_int_value();

            bin.builder
                .build_right_shift(left, right, *signed, "")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Subscript {
            loc,
            ty: elem_ty,
            array_ty: ty,
            expr: a,
            index,
        } => {
            if ty.is_storage_bytes() {
                let index = expression(target, bin, index, vartab, function).into_int_value();
                let slot = expression(target, bin, a, vartab, function).into_int_value();
                target
                    .get_storage_bytes_subscript(bin, function, slot, index, *loc)
                    .into()
            } else if ty.is_contract_storage() {
                let array = expression(target, bin, a, vartab, function).into_int_value();
                let index = expression(target, bin, index, vartab, function);

                target
                    .storage_subscript(bin, function, ty, array, index)
                    .as_basic_value_enum()
            } else if elem_ty.is_builtin_struct() == Some(StructType::AccountInfo) {
                let array = expression(target, bin, a, vartab, function).into_pointer_value();
                let index = expression(target, bin, index, vartab, function).into_int_value();

                let llvm_ty = expect_llvm_entity(
                    bin.module.get_struct_type("struct.SolAccountInfo"),
                    "emitting account info member access",
                    "struct.SolAccountInfo",
                );
                unsafe {
                    bin.builder
                        .build_gep(llvm_ty, array, &[index], "account_info")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into()
                }
            } else if ty.is_dynamic_memory() {
                let array = expression(target, bin, a, vartab, function);

                let mut array_index =
                    expression(target, bin, index, vartab, function).into_int_value();

                // bounds checking already done; we can down-cast if necessary
                if array_index.get_type().get_bit_width() > 32 {
                    array_index = bin
                        .builder
                        .build_int_truncate(array_index, bin.context.i32_type(), "index")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        });
                }

                let index = bin
                    .builder
                    .build_int_mul(
                        array_index,
                        bin.llvm_type(elem_ty.deref_memory())
                            .size_of()
                            .unwrap_or_else(|| {
                                panic!(
                                    "{}",
                                    CodegenError::missing_llvm_entity(
                                        "emitting expression",
                                        "type size"
                                    )
                                )
                            })
                            .const_cast(bin.context.i32_type(), false),
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                unsafe {
                    bin.builder
                        .build_gep(
                            bin.context.i8_type(),
                            bin.vector_bytes(array),
                            &[index],
                            "index_access",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                }
                .into()
            } else {
                let array = expression(target, bin, a, vartab, function).into_pointer_value();
                let index = expression(target, bin, index, vartab, function).into_int_value();

                let llvm_ty = bin.llvm_type(ty.deref_memory());
                unsafe {
                    bin.builder
                        .build_gep(
                            llvm_ty,
                            array,
                            &[bin.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                        .into()
                }
            }
        }
        Expression::StructMember { expr, .. }
            if expr.ty().is_builtin_struct() == Some(StructType::AccountInfo) =>
        {
            target.builtin(bin, e, vartab, function)
        }
        Expression::StructMember { expr, member, .. } => {
            let struct_ty = bin.llvm_type(expr.ty().deref_memory());
            let struct_ptr = expression(target, bin, expr, vartab, function).into_pointer_value();

            bin.builder
                .build_struct_gep(struct_ty, struct_ptr, *member as u32, "struct member")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::ConstArrayLiteral {
            dimensions, values, ..
        } => {
            // For const arrays (declared with "constant" keyword, we should create a global constant
            let mut dims = dimensions.iter();

            let exprs = values
                .iter()
                .map(|e| expression(target, bin, e, vartab, function).into_int_value())
                .collect::<Vec<IntValue>>();
            let ty = exprs[0].get_type();

            let top_size = *dims.next().unwrap_or_else(|| {
                invalid_cfg("emitting array literal", "missing array dimension")
            });

            // Create a vector of ArrayValues
            let mut arrays = exprs
                .chunks(top_size as usize)
                .map(|a| ty.const_array(a))
                .collect::<Vec<ArrayValue>>();

            let mut ty = ty.array_type(top_size);

            // for each dimension, split the array into futher arrays
            for d in dims {
                ty = ty.array_type(*d);

                arrays = arrays
                    .chunks(*d as usize)
                    .map(|a| ty.const_array(a))
                    .collect::<Vec<ArrayValue>>();
            }

            // We actually end up with an array with a single entry

            // now we've created the type, and the const array. Put it into a global
            let gv =
                bin.module
                    .add_global(ty, Some(AddressSpace::default()), "const_array_literal");

            gv.set_linkage(Linkage::Internal);

            gv.set_initializer(&arrays[0]);
            gv.set_constant(true);

            gv.as_pointer_value().into()
        }
        Expression::ArrayLiteral {
            ty,
            dimensions,
            values,
            ..
        } => {
            // non-const array literals should alloca'ed and each element assigned
            let ty = bin.llvm_type(ty);

            let p = bin
                .builder
                .build_call(
                    runtime_helper(bin, bin.alloc(), "allocating array literal"),
                    &[ty.size_of()
                        .unwrap_or_else(|| {
                            panic!(
                                "{}",
                                CodegenError::missing_llvm_entity(
                                    "emitting expression",
                                    "type size"
                                )
                            )
                        })
                        .const_cast(bin.context.i32_type(), false)
                        .into()],
                    "array_literal",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"));

            for (i, expr) in values.iter().enumerate() {
                let mut ind = vec![bin.context.i32_type().const_zero()];

                let mut e = i as u32;

                for d in dimensions {
                    ind.insert(1, bin.context.i32_type().const_int((e % *d).into(), false));

                    e /= *d;
                }

                let elemptr = unsafe {
                    bin.builder
                        .build_gep(ty, p.into_pointer_value(), &ind, &format!("elemptr{i}"))
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                };

                let elem = expression(target, bin, expr, vartab, function);

                let elem = if expr.ty().is_fixed_reference_type(bin.ns) {
                    let load_type = bin.llvm_type(&expr.ty());
                    bin.builder
                        .build_load(load_type, elem.into_pointer_value(), "elem")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                } else {
                    elem
                };

                bin.builder
                    .build_store(elemptr, elem)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });
            }

            p
        }
        Expression::AllocDynamicBytes {
            ty,
            size,
            initializer,
            ..
        } => {
            if matches!(ty, Type::Slice(_)) {
                let init = initializer.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{}",
                        CodegenError::invalid_cfg_invariant(
                            "emitting expression",
                            "missing initializer"
                        )
                    )
                });

                let data = bin.emit_global_string("const_string", init, true);

                bin.llvm_type(ty)
                    .into_struct_type()
                    .const_named_struct(&[
                        data.into(),
                        bin.context
                            .custom_width_int_type(bin.ns.target.ptr_size().into())
                            .const_int(init.len() as u64, false)
                            .into(),
                    ])
                    .into()
            } else {
                let elem = match ty {
                    Type::Slice(_) | Type::String | Type::DynamicBytes | Type::Bytes(_) => {
                        Type::Bytes(1)
                    }
                    _ => ty.array_elem(),
                };

                let size = expression(target, bin, size, vartab, function).into_int_value();

                let elem_size = bin
                    .llvm_type(&elem)
                    .size_of()
                    .unwrap_or_else(|| {
                        panic!(
                            "{}",
                            CodegenError::missing_llvm_entity("emitting expression", "type size")
                        )
                    })
                    .const_cast(bin.context.i32_type(), false);

                bin.vector_new(size, elem_size, initializer.as_ref())
            }
        }
        Expression::Builtin {
            kind: Builtin::ArrayLength,
            args,
            ..
        } if args[0].ty().array_deref().is_builtin_struct().is_none() => {
            let array = expression(target, bin, &args[0], vartab, function);

            bin.vector_len(array).into()
        }
        Expression::Builtin {
            tys: returns,
            kind: Builtin::ReadFromBuffer,
            args,
            ..
        } => {
            let v = expression(target, bin, &args[0], vartab, function);
            let offset = expression(target, bin, &args[1], vartab, function).into_int_value();

            let data = if args[0].ty().is_dynamic_memory() {
                bin.vector_bytes(v)
            } else {
                v.into_pointer_value()
            };

            let start = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), data, &[offset], "start")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            };

            if matches!(returns[0], Type::Bytes(_) | Type::FunctionSelector) {
                let n = returns[0].bytes(bin.ns);
                let bytes_ty = bin.context.custom_width_int_type(n as u32 * 8);

                let store = bin.build_alloca(function, bytes_ty, "stack");
                bin.builder
                    .build_call(
                        runtime_helper(bin, "__beNtoleN", "converting array literal bytes"),
                        &[
                            start.into(),
                            store.into(),
                            bin.context.i32_type().const_int(n as u64, false).into(),
                        ],
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });
                bin.builder
                    .build_load(bytes_ty, store, &format!("bytes{n}"))
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            } else {
                bin.builder
                    .build_load(bin.llvm_type(&returns[0]), start, "value")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            }
        }
        Expression::Keccak256 { exprs, .. } => {
            let mut length = bin.context.i32_type().const_zero();
            let mut values: Vec<(BasicValueEnum, IntValue, Type)> = Vec::new();

            // first we need to calculate the length of the buffer and get the types/lengths
            for e in exprs {
                let v = expression(target, bin, e, vartab, function);

                let len = match e.ty() {
                    Type::DynamicBytes | Type::String => bin.vector_len(v),
                    _ => v
                        .get_type()
                        .size_of()
                        .unwrap_or_else(|| {
                            panic!(
                                "{}",
                                CodegenError::missing_llvm_entity(
                                    "emitting expression",
                                    "type size"
                                )
                            )
                        })
                        .const_cast(bin.context.i32_type(), false),
                };

                length = bin
                    .builder
                    .build_int_add(length, len, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                values.push((v, len, e.ty()));
            }

            //  now allocate a buffer
            let src = bin
                .builder
                .build_array_alloca(bin.context.i8_type(), length, "keccak_src")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            // fill in all the fields
            let mut offset = bin.context.i32_type().const_zero();

            for (v, len, ty) in values {
                let elem = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), src, &[offset], "elem")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                };

                offset = bin
                    .builder
                    .build_int_add(offset, len, "")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                match ty {
                    Type::DynamicBytes | Type::String => {
                        let data = bin.vector_bytes(v);

                        bin.builder
                            .build_call(
                                runtime_helper(bin, "__memcpy", "copying string literal data"),
                                &[elem.into(), data.into(), len.into()],
                                "",
                            )
                            .unwrap_or_else(|err| {
                                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                            });
                    }
                    _ => {
                        bin.builder.build_store(elem, v).unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        });
                    }
                }
            }
            let dst_type = bin.context.custom_width_int_type(256);
            let dst = bin
                .builder
                .build_alloca(dst_type, "keccak_dst")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            target.keccak256_hash(bin, src, length, dst);

            bin.builder
                .build_load(dst_type, dst, "keccak256_hash")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        Expression::StringCompare { left, right, .. } => {
            let (left, left_len) = string_location(target, bin, left, vartab, function);
            let (right, right_len) = string_location(target, bin, right, vartab, function);

            bin.builder
                .build_call(
                    runtime_helper(bin, "__memcmp", "comparing string equality"),
                    &[left.into(), left_len.into(), right.into(), right_len.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
        }
        Expression::ReturnData { .. } => target.return_data(bin, function).into(),
        Expression::StorageArrayLength { array, elem_ty, .. } => {
            let slot = expression(target, bin, array, vartab, function).into_int_value();
            target
                .storage_array_length(bin, function, slot, elem_ty)
                .into()
        }
        Expression::Builtin {
            kind: Builtin::Signature,
            ..
        } if bin.ns.target != Target::Solana => {
            // need to byte-reverse selector
            let selector_type = bin.context.i32_type();
            let selector = bin.build_alloca(function, selector_type, "selector");

            // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
            bin.builder
                .build_call(
                    runtime_helper(bin, "__beNtoleN", "loading selector bytes"),
                    &[
                        bin.selector.as_pointer_value().into(),
                        selector.into(),
                        bin.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_load(selector_type, selector, "selector")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        Expression::Builtin {
            kind: Builtin::AddMod,
            args,
            ..
        } => {
            let arith_ty = bin.context.custom_width_int_type(512);
            let res_ty = bin.context.custom_width_int_type(256);

            let x = expression(target, bin, &args[0], vartab, function).into_int_value();
            let y = expression(target, bin, &args[1], vartab, function).into_int_value();
            let k = expression(target, bin, &args[2], vartab, function).into_int_value();
            let dividend = bin
                .builder
                .build_int_add(
                    bin.builder
                        .build_int_z_extend(x, arith_ty, "wide_x")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        }),
                    bin.builder
                        .build_int_z_extend(y, arith_ty, "wide_y")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        }),
                    "x_plus_y",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let divisor = bin
                .builder
                .build_int_z_extend(k, arith_ty, "wide_k")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let pdividend = bin.build_alloca(function, arith_ty, "dividend");
            let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
            let rem = bin.build_alloca(function, arith_ty, "remainder");
            let quotient = bin.build_alloca(function, arith_ty, "quotient");

            bin.builder
                .build_store(pdividend, dividend)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            bin.builder
                .build_store(pdivisor, divisor)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let ret = bin
                .builder
                .build_call(
                    runtime_helper(bin, "udivmod512", "addmod builtin lowering"),
                    &[
                        pdividend.into(),
                        pdivisor.into(),
                        rem.into(),
                        quotient.into(),
                    ],
                    "quotient",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_int_value();

            let success = bin
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    bin.context.i32_type().const_zero(),
                    "success",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let success_block = bin.context.append_basic_block(function, "success");
            let bail_block = bin.context.append_basic_block(function, "bail");
            emit_or_panic(
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block),
                "emitting zero-modulus guard for addmod builtin",
            );

            bin.builder.position_at_end(bail_block);

            // On Solana the return type is 64 bit
            let ret: BasicValueEnum = bin
                .builder
                .build_int_z_extend(
                    ret,
                    bin.return_values[&ReturnCode::Success].get_type(),
                    "ret",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into();

            bin.builder.build_return(Some(&ret)).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });
            bin.builder.position_at_end(success_block);

            let remainder = bin
                .builder
                .build_load(arith_ty, rem, "remainder")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Builtin {
            kind: Builtin::MulMod,
            args,
            ..
        } => {
            let arith_ty = bin.context.custom_width_int_type(512);
            let res_ty = bin.context.custom_width_int_type(256);

            let x = expression(target, bin, &args[0], vartab, function).into_int_value();
            let y = expression(target, bin, &args[1], vartab, function).into_int_value();
            let x_m = bin.build_alloca(function, arith_ty, "x_m");
            let y_m = bin.build_alloca(function, arith_ty, "x_y");
            let x_times_y_m = bin.build_alloca(function, arith_ty, "x_times_y_m");

            bin.builder
                .build_store(
                    x_m,
                    bin.builder
                        .build_int_z_extend(x, arith_ty, "wide_x")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        }),
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            bin.builder
                .build_store(
                    y_m,
                    bin.builder
                        .build_int_z_extend(y, arith_ty, "wide_y")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        }),
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_call(
                    runtime_helper(bin, "__mul32", "mulmod builtin multiplication lowering"),
                    &[
                        x_m.into(),
                        y_m.into(),
                        x_times_y_m.into(),
                        bin.context.i32_type().const_int(512 / 32, false).into(),
                    ],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            let k = expression(target, bin, &args[2], vartab, function).into_int_value();
            let dividend = bin
                .builder
                .build_load(arith_ty, x_times_y_m, "x_t_y")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let divisor = bin
                .builder
                .build_int_z_extend(k, arith_ty, "wide_k")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let pdividend = bin.build_alloca(function, arith_ty, "dividend");
            let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
            let rem = bin.build_alloca(function, arith_ty, "remainder");
            let quotient = bin.build_alloca(function, arith_ty, "quotient");

            bin.builder
                .build_store(pdividend, dividend)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            bin.builder
                .build_store(pdivisor, divisor)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let ret = bin
                .builder
                .build_call(
                    runtime_helper(bin, "udivmod512", "mulmod builtin division lowering"),
                    &[
                        pdividend.into(),
                        pdivisor.into(),
                        rem.into(),
                        quotient.into(),
                    ],
                    "quotient",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_int_value();

            let success = bin
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    bin.context.i32_type().const_zero(),
                    "success",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let success_block = bin.context.append_basic_block(function, "success");
            let bail_block = bin.context.append_basic_block(function, "bail");
            emit_or_panic(
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block),
                "emitting zero-modulus guard for mulmod builtin",
            );

            bin.builder.position_at_end(bail_block);

            // On Solana the return type is 64 bit
            let ret: BasicValueEnum = bin
                .builder
                .build_int_z_extend(
                    ret,
                    bin.return_values[&ReturnCode::Success].get_type(),
                    "ret",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into();

            bin.builder.build_return(Some(&ret)).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            bin.builder.position_at_end(success_block);

            let remainder = bin
                .builder
                .build_load(arith_ty, rem, "quotient")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .into()
        }
        Expression::Builtin {
            kind: hash @ Builtin::Ripemd160,
            args,
            ..
        }
        | Expression::Builtin {
            kind: hash @ Builtin::Keccak256,
            args,
            ..
        }
        | Expression::Builtin {
            kind: hash @ Builtin::Blake2_128,
            args,
            ..
        }
        | Expression::Builtin {
            kind: hash @ Builtin::Blake2_256,
            args,
            ..
        }
        | Expression::Builtin {
            kind: hash @ Builtin::Sha256,
            args,
            ..
        } => {
            let v = expression(target, bin, &args[0], vartab, function);

            let hash = match hash {
                Builtin::Ripemd160 => HashTy::Ripemd160,
                Builtin::Sha256 => HashTy::Sha256,
                Builtin::Keccak256 => HashTy::Keccak256,
                Builtin::Blake2_128 => HashTy::Blake2_128,
                Builtin::Blake2_256 => HashTy::Blake2_256,
                _ => invalid_cfg("emitting hash builtin", "expression is not a hash builtin"),
            };

            target
                .hash(bin, function, hash, bin.vector_bytes(v), bin.vector_len(v))
                .into()
        }
        Expression::Builtin {
            kind: Builtin::Concat,
            args,
            ..
        } => {
            let vector_ty = expect_llvm_entity(
                bin.module.get_struct_type("struct.vector"),
                "emitting concat builtin",
                "struct.vector",
            );

            let mut length = i32_zero!();

            let args: Vec<_> = args
                .iter()
                .map(|arg| {
                    let v = expression(target, bin, arg, vartab, function);

                    length = bin
                        .builder
                        .build_int_add(length, bin.vector_len(v), "length")
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        });

                    v
                })
                .collect();

            let size = bin
                .builder
                .build_int_add(
                    length,
                    vector_ty
                        .size_of()
                        .unwrap_or_else(|| {
                            panic!(
                                "{}",
                                CodegenError::missing_llvm_entity(
                                    "emitting expression",
                                    "type size"
                                )
                            )
                        })
                        .const_cast(bin.context.i32_type(), false),
                    "size",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let v = bin
                .builder
                .build_call(
                    runtime_helper(bin, bin.alloc(), "allocating concat result"),
                    &[size.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_pointer_value();

            let mut dest = bin.vector_bytes(v.into());

            for arg in args {
                let from = bin.vector_bytes(arg);
                let len = bin.vector_len(arg);

                dest = bin
                    .builder
                    .build_call(
                        runtime_helper(bin, "__memcpy", "copying concat argument"),
                        &[dest.into(), from.into(), len.into()],
                        "",
                    )
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                    .into_pointer_value();
            }

            // Update the len and size field of the vector struct
            let len_ptr = bin
                .builder
                .build_struct_gep(vector_ty, v, 0, "len")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });
            bin.builder
                .build_store(len_ptr, length)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let size_ptr = bin
                .builder
                .build_struct_gep(vector_ty, v, 1, "size")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_store(size_ptr, length)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            v.into()
        }
        Expression::Builtin { .. } => target.builtin(bin, e, vartab, function),
        Expression::InternalFunctionCfg { cfg_no, .. } => bin.functions[cfg_no]
            .as_global_value()
            .as_pointer_value()
            .into(),
        Expression::FormatString { args: fields, .. } => {
            format_string(target, bin, fields, vartab, function)
        }

        Expression::AdvancePointer {
            pointer,
            bytes_offset,
        } => {
            let pointer = if pointer.ty().is_dynamic_memory() {
                bin.vector_bytes(expression(target, bin, pointer, vartab, function))
            } else {
                expression(target, bin, pointer, vartab, function).into_pointer_value()
            };
            let offset = expression(target, bin, bytes_offset, vartab, function).into_int_value();
            let advanced = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), pointer, &[offset], "adv_pointer")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            };

            advanced.into()
        }

        Expression::VectorData { pointer } => {
            let ptr = expression(target, bin, pointer, vartab, function);
            let data = bin.vector_bytes(ptr);
            let res = bin
                .builder
                .build_ptr_to_int(data, bin.context.i32_type(), "ptr_as_int32");

            res.unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            })
            .into()
        }

        Expression::RationalNumberLiteral { .. }
        | Expression::Undefined { .. }
        | Expression::Poison
        | Expression::BytesCast { .. } => {
            unreachable!("should not exist in cfg")
        }
    }
}

pub(super) fn compare_address<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    left: &Expression,
    right: &Expression,
    op: inkwell::IntPredicate,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
) -> IntValue<'a> {
    let l = expression(target, bin, left, vartab, function).into_array_value();
    let r = expression(target, bin, right, vartab, function).into_array_value();

    let left = bin.build_alloca(function, bin.address_type(), "left");
    let right = bin.build_alloca(function, bin.address_type(), "right");

    bin.builder
        .build_store(left, l)
        .unwrap_or_else(|err| panic!("{}", CodegenError::llvm_builder("emitting expression", err)));
    bin.builder
        .build_store(right, r)
        .unwrap_or_else(|err| panic!("{}", CodegenError::llvm_builder("emitting expression", err)));

    let res = bin
        .builder
        .build_call(
            runtime_helper(bin, "__memcmp_ord", "comparing byte slices"),
            &[
                left.into(),
                right.into(),
                bin.context
                    .i32_type()
                    .const_int(bin.ns.address_length as u64, false)
                    .into(),
            ],
            "",
        )
        .unwrap_or_else(|err| panic!("{}", CodegenError::llvm_builder("emitting expression", err)))
        .try_as_basic_value()
        .left()
        .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
        .into_int_value();

    bin.builder
        .build_int_compare(op, res, bin.context.i32_type().const_zero(), "")
        .unwrap_or_else(|err| panic!("{}", CodegenError::llvm_builder("emitting expression", err)))
}

fn runtime_cast<'a>(
    bin: &Binary<'a>,
    function: FunctionValue<'a>,
    from: &Type,
    to: &Type,
    val: BasicValueEnum<'a>,
) -> BasicValueEnum<'a> {
    match (from, to) {
        // no conversion needed
        (from, to) if from == to => val,

        (Type::Address(_) | Type::Contract(_), Type::Address(_) | Type::Contract(_)) => val,
        (
            Type::ExternalFunction { .. } | Type::Struct(StructType::ExternalFunction),
            Type::ExternalFunction { .. } | Type::Struct(StructType::ExternalFunction),
        ) => val,
        (
            Type::Uint(_)
            | Type::Int(_)
            | Type::Value
            | Type::Bytes(_)
            | Type::UserType(_)
            | Type::Enum(_)
            | Type::FunctionSelector,
            Type::Uint(_)
            | Type::Int(_)
            | Type::Value
            | Type::Bytes(_)
            | Type::Enum(_)
            | Type::UserType(_)
            | Type::FunctionSelector,
        ) => {
            assert_eq!(from.bytes(bin.ns), to.bytes(bin.ns),);

            val
        }
        (Type::String | Type::DynamicBytes, Type::String | Type::DynamicBytes) => val,
        (
            Type::InternalFunction {
                params: from_params,
                returns: from_returns,
                ..
            },
            Type::InternalFunction {
                params: to_params,
                returns: to_returns,
                ..
            },
        ) if from_params == to_params && from_returns == to_returns => val,

        (Type::Bytes(_) | Type::Int(_) | Type::Uint(_) | Type::Value, Type::Address(_)) => {
            let llvm_ty = bin.llvm_type(from);

            let src = bin.build_alloca(function, llvm_ty, "dest");

            bin.builder
                .build_store(src, val.into_int_value())
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let dest = bin.build_alloca(function, bin.address_type(), "address");

            let len = bin
                .context
                .i32_type()
                .const_int(bin.ns.address_length as u64, false);

            bin.builder
                .build_call(
                    runtime_helper(bin, "__leNtobeN", "converting fixed bytes to big endian"),
                    &[src.into(), dest.into(), len.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_load(bin.address_type(), dest, "val")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        (Type::Address(_), Type::Bytes(_) | Type::Int(_) | Type::Uint(_) | Type::Value) => {
            let llvm_ty = bin.llvm_type(to);

            let src = bin.build_alloca(function, bin.address_type(), "address");

            bin.builder
                .build_store(src, val.into_array_value())
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let dest = bin.build_alloca(function, llvm_ty, "dest");

            let len = bin
                .context
                .i32_type()
                .const_int(bin.ns.address_length as u64, false);

            bin.builder
                .build_call(
                    runtime_helper(bin, "__beNtoleN", "converting fixed bytes to little endian"),
                    &[src.into(), dest.into(), len.into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_load(llvm_ty, dest, "val")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        (Type::Bool, Type::Int(_) | Type::Uint(_)) => bin
            .builder
            .build_int_z_extend(
                val.into_int_value(),
                bin.llvm_type(to).into_int_type(),
                "bool_to_int_cast",
            )
            .unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            })
            .into(),
        (_, Type::Uint(_)) if !from.is_contract_storage() && from.is_reference_type(bin.ns) => bin
            .builder
            .build_ptr_to_int(
                val.into_pointer_value(),
                bin.llvm_type(to).into_int_type(),
                "ptr_to_int",
            )
            .unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            })
            .into(),
        (Type::Uint(_), _) if to.is_reference_type(bin.ns) => bin
            .builder
            .build_int_to_ptr(
                val.into_int_value(),
                bin.context.ptr_type(AddressSpace::default()),
                "int_to_ptr",
            )
            .unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            })
            .into(),
        (Type::DynamicBytes | Type::String, Type::Slice(_)) => {
            let slice_ty = bin.llvm_type(to);
            let slice = bin.build_alloca(function, slice_ty, "slice");

            let data = bin.vector_bytes(val);

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_store(data_ptr, data)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let len = bin
                .builder
                .build_int_z_extend(bin.vector_len(val), bin.context.i64_type(), "len")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder.build_store(len_ptr, len).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            bin.builder
                .build_load(slice_ty, slice, "slice")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        (Type::Address(_), Type::Slice(_)) => {
            let slice_ty = bin.llvm_type(to);
            let slice = bin.build_alloca(function, slice_ty, "slice");
            let address = bin.build_alloca(function, bin.llvm_type(from), "address");

            bin.builder.build_store(address, val).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_store(data_ptr, address)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let len = bin
                .context
                .i64_type()
                .const_int(bin.ns.address_length as u64, false);

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder.build_store(len_ptr, len).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            bin.builder
                .build_load(slice_ty, slice, "slice")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        (Type::Bytes(bytes_length), Type::Slice(_)) => {
            let llvm_ty = bin.llvm_type(from);
            let src = bin.build_alloca(function, llvm_ty, "src");

            bin.builder
                .build_store(src, val.into_int_value())
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let dest = bin.build_alloca(
                function,
                bin.context.i8_type().array_type((*bytes_length).into()),
                "dest",
            );

            bin.builder
                .build_call(
                    runtime_helper(bin, "__leNtobeN", "converting address to bytes"),
                    &[
                        src.into(),
                        dest.into(),
                        bin.context
                            .i32_type()
                            .const_int((*bytes_length).into(), false)
                            .into(),
                    ],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let slice_ty = bin.llvm_type(to);
            let slice = bin.build_alloca(function, slice_ty, "slice");

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder
                .build_store(data_ptr, dest)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let len = bin
                .context
                .i64_type()
                .const_int((*bytes_length).into(), false);

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            bin.builder.build_store(len_ptr, len).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            bin.builder
                .build_load(slice_ty, slice, "slice")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                })
        }
        _ => invalid_cfg(
            "casting basic value to slice",
            "unsupported source expression",
        ),
    }
}

/// Emit a codegen expression as a slice; the result is a pointer to the data and a length. This is
/// needed for Solana syscalls that take slices, and will be useful for when we start supporting
/// slices in Solidity (e.g. foo[2:3])
pub(super) fn expression_to_slice<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    e: &Expression,
    to: &Type,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
) -> (PointerValue<'a>, IntValue<'a>) {
    emit_context!(bin);

    let Type::Slice(to_elem_ty) = to else {
        invalid_cfg(
            "emitting expression as slice",
            "destination type is not a slice",
        )
    };

    let llvm_to = bin.llvm_type(to);

    match e {
        Expression::ArrayLiteral {
            dimensions, values, ..
        } => {
            let length = dimensions[0];

            let llvm_length = i32_const!(length.into());

            let output = bin.build_array_alloca(function, llvm_to, llvm_length, "seeds");

            for i in 0..length {
                let (ptr, len) = expression_to_slice(
                    target,
                    bin,
                    &values[i as usize],
                    to_elem_ty,
                    vartab,
                    function,
                );

                // SAFETY: llvm_to is an array of slices, so i is slice no and 0 is the data ptr
                // of the slice struct. Since indexes are correct for type it is safe.
                let output_ptr = unsafe {
                    bin.builder
                        .build_gep(
                            llvm_to,
                            output,
                            &[i32_const!(i.into()), i32_zero!()],
                            "output_ptr",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                };

                bin.builder
                    .build_store(output_ptr, ptr)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });

                // SAFETY: llvm_to is an array of slices, so i is slice no and 1 is the len ptr
                // of the slice struct. Since indexes are correct for type it is safe.
                let output_len = unsafe {
                    bin.builder
                        .build_gep(
                            llvm_to,
                            output,
                            &[i32_const!(i.into()), i32_const!(1)],
                            "output_len",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        })
                };

                bin.builder
                    .build_store(output_len, len)
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    });
            }

            (output, llvm_length)
        }
        Expression::AllocDynamicBytes {
            initializer: Some(initializer),
            ..
        } => {
            let ptr = bin.emit_global_string("slice_constant", initializer, true);
            let len = i64_const!(initializer.len() as u64);

            (ptr, len)
        }
        _ => {
            let from = e.ty();

            let val = expression(target, bin, e, vartab, function);

            basic_value_to_slice(bin, val, &from, to, function)
        }
    }
}

/// Convert basic enum value to a slice. This function calls itself recursively
/// for arrays (become slices of slices).
fn basic_value_to_slice<'a>(
    bin: &Binary<'a>,
    val: BasicValueEnum<'a>,
    from: &Type,
    to: &Type,
    function: FunctionValue<'a>,
) -> (PointerValue<'a>, IntValue<'a>) {
    emit_context!(bin);

    match from {
        Type::Slice(_) | Type::DynamicBytes | Type::String => {
            let data = bin.vector_bytes(val);
            let len = bin.vector_len(val);
            let len = bin
                .builder
                .build_int_z_extend(len, bin.context.i64_type(), "ext")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            (data, len)
        }
        Type::Address(_) => {
            let address = call!(
                bin.alloc(),
                &[i32_const!(bin.ns.address_length as u64).into()]
            )
            .try_as_basic_value()
            .left()
            .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
            .into_pointer_value();

            bin.builder.build_store(address, val).unwrap_or_else(|err| {
                panic!("{}", CodegenError::llvm_builder("emitting expression", err))
            });

            let len = i64_const!(bin.ns.address_length as u64);

            (address, len)
        }
        Type::Bytes(bytes_length) => {
            let llvm_ty = bin.llvm_type(from);
            let src = bin.build_alloca(function, llvm_ty, "src");

            bin.builder
                .build_store(src, val.into_int_value())
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let bytes_length: u64 = (*bytes_length).into();

            let dest = call!("__malloc", &[i32_const!(bytes_length).into()])
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_pointer_value();

            bin.builder
                .build_call(
                    runtime_helper(bin, "__leNtobeN", "converting array bytes"),
                    &[src.into(), dest.into(), i32_const!(bytes_length).into()],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let len = i64_const!(bytes_length);

            (dest, len)
        }
        Type::Array(_, dims) => {
            let to_elem = to.array_elem();

            let to = bin.llvm_type(to);

            let length = match dims
                .last()
                .unwrap_or_else(|| invalid_cfg("casting array to bytes", "missing array dimension"))
            {
                ArrayLength::Dynamic => bin.vector_len(val),
                ArrayLength::Fixed(len) => i32_const!(expect_numeric_conversion(
                    len.to_u64(),
                    "casting fixed array to bytes",
                    len.to_string(),
                    "u64",
                )),
                _ => invalid_cfg("casting array to bytes", "invalid array length kind"),
            };

            // FIXME: In Program Runtime v1, we can't do dynamic alloca. Remove the malloc once we move to
            // program runtime v2
            let size = bin
                .builder
                .build_int_mul(
                    bin.builder
                        .build_int_truncate(
                            bin.llvm_type(&Type::Slice(Type::Bytes(1).into()))
                                .size_of()
                                .unwrap_or_else(|| {
                                    panic!(
                                        "{}",
                                        CodegenError::missing_llvm_entity(
                                            "emitting expression",
                                            "type size"
                                        )
                                    )
                                }),
                            bin.context.i32_type(),
                            "slice_size",
                        )
                        .unwrap_or_else(|err| {
                            panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                        }),
                    length,
                    "size",
                )
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            let output = call!(bin.alloc(), &[size.into()])
                .try_as_basic_value()
                .left()
                .unwrap_or_else(|| expect_return_value(None, "reading LLVM call return value"))
                .into_pointer_value();

            // loop over seeds
            let mut builder = LoopBuilder::new(bin, function);

            let index = builder.over(bin, i32_zero!(), length);

            // get value from array
            let input_elem = bin.array_subscript(from, val.into_pointer_value(), index);

            let from_elem = from.array_elem();

            // If the element is a fixed-length array, do not load it as it's stored in place and not
            // as a pointer.
            let load = if let Type::Array(_, dims) = &from_elem {
                matches!(dims.last(), Some(ArrayLength::Dynamic))
            } else {
                true
            };

            let input_elem = if load {
                bin.builder
                    .build_load(bin.llvm_field_ty(&from_elem), input_elem, "elem")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            } else {
                input_elem.into()
            };

            let (data, len) = basic_value_to_slice(bin, input_elem, &from_elem, &to_elem, function);

            // SAFETY: to is an array of slices, so index is slice no and 0 is the data ptr
            // of the slice struct. Since indexes are correct from type it is safe.
            let output_data = unsafe {
                bin.builder
                    .build_gep(to, output, &[index, i32_zero!()], "output_data")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            };

            bin.builder
                .build_store(output_data, data)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            // SAFETY: to is an array of slices, so index is slice no and 1 is the len ptr
            // of the slice struct. Since indexes are correct from type it is safe.
            let output_len = unsafe {
                bin.builder
                    .build_gep(to, output, &[index, i32_const!(1)], "output_len")
                    .unwrap_or_else(|err| {
                        panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                    })
            };

            bin.builder
                .build_store(output_len, len)
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            builder.finish(bin);

            let length = bin
                .builder
                .build_int_z_extend(length, bin.context.i64_type(), "length")
                .unwrap_or_else(|err| {
                    panic!("{}", CodegenError::llvm_builder("emitting expression", err))
                });

            (output, length)
        }
        _ => invalid_cfg(
            "emitting expression as slice",
            "unsupported source expression",
        ),
    }
}
