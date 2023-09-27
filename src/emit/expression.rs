// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::codegen::revert::PanicCode;
use crate::codegen::{Builtin, Expression};
use crate::emit::binary::Binary;
use crate::emit::math::{build_binary_op_with_overflow_check, multiply, power};
use crate::emit::strings::{format_string, string_location};
use crate::emit::{loop_builder::LoopBuilder, BinaryOp, TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast::{ArrayLength, Namespace, RetrieveType, StructType, Type};
use crate::Target;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, StringRadix};
use inkwell::values::{ArrayValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};
use num_bigint::Sign;
use num_traits::ToPrimitive;
use std::collections::HashMap;

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
    ns: &Namespace,
) -> BasicValueEnum<'a> {
    match e {
        Expression::FunctionArg { arg_no, .. } => function.get_nth_param(*arg_no as u32).unwrap(),
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
            if bs.len() > ns.address_length {
                // remove leading bytes
                for _ in 0..bs.len() - ns.address_length {
                    bs.remove(0);
                }
            } else {
                // insert leading bytes
                let val = if value.sign() == Sign::Minus { 0xff } else { 0 };

                for _ in 0..ns.address_length - bs.len() {
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
            bin.number_literal(ty.bits(ns) as u32, value, ns).into()
        }
        Expression::StructLiteral {
            ty, values: fields, ..
        } => {
            let struct_ty = bin.llvm_type(ty, ns);

            let s = bin
                .builder
                .build_call(
                    bin.module.get_function("__malloc").unwrap(),
                    &[struct_ty
                        .size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false)
                        .into()],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            for (i, expr) in fields.iter().enumerate() {
                let elemptr = unsafe {
                    bin.builder.build_gep(
                        struct_ty,
                        s,
                        &[
                            bin.context.i32_type().const_zero(),
                            bin.context.i32_type().const_int(i as u64, false),
                        ],
                        "struct member",
                    )
                };

                let elem = expression(target, bin, expr, vartab, function, ns);

                let elem = if expr.ty().is_fixed_reference_type(ns) {
                    let load_type = bin.llvm_type(&expr.ty(), ns);
                    bin.builder
                        .build_load(load_type, elem.into_pointer_value(), "elem")
                } else {
                    elem
                };

                bin.builder.build_store(elemptr, elem);
            }

            s.into()
        }
        Expression::BytesLiteral { value: bs, .. } => {
            let ty = bin.context.custom_width_int_type((bs.len() * 8) as u32);

            // hex"11223344" should become i32 0x11223344
            let s = hex::encode(bs);

            ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                .unwrap()
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
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            if !overflowing {
                let signed = ty.is_signed_int(ns);
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Add,
                    signed,
                    ns,
                    *loc,
                )
                .into()
            } else {
                bin.builder.build_int_add(left, right, "").into()
            }
        }
        Expression::Subtract {
            loc,
            ty,
            overflowing,
            left,
            right,
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            if !overflowing {
                let signed = ty.is_signed_int(ns);
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Subtract,
                    signed,
                    ns,
                    *loc,
                )
                .into()
            } else {
                bin.builder.build_int_sub(left, right, "").into()
            }
        }
        Expression::Multiply {
            loc,
            ty: res_ty,
            overflowing,
            left,
            right,
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            multiply(
                target,
                bin,
                function,
                *overflowing,
                left,
                right,
                res_ty.is_signed_int(ns),
                ns,
                *loc,
            )
            .into()
        }
        Expression::UnsignedDivide {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("udivmod{div_bits}");

                let f = bin
                    .module
                    .get_function(&name)
                    .expect("div function missing");

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder.build_store(
                    dividend,
                    if bits < div_bits {
                        bin.builder.build_int_z_extend(left, ty, "")
                    } else {
                        left
                    },
                );

                bin.builder.build_store(
                    divisor,
                    if bits < div_bits {
                        bin.builder.build_int_z_extend(right, ty, "")
                    } else {
                        right
                    },
                );

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret.into_int_value(),
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc), ns);
                let (revert_out, revert_out_len) =
                    bin.panic_data_const(ns, PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(ty, quotient, "quotient")
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(quotient, left.get_type(), "")
                } else {
                    quotient
                }
                .into()
            } else {
                bin.builder.build_int_unsigned_div(left, right, "").into()
            }
        }
        Expression::SignedDivide {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("sdivmod{div_bits}");

                let f = bin
                    .module
                    .get_function(&name)
                    .expect("div function missing");

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder.build_store(
                    dividend,
                    if bits < div_bits {
                        bin.builder.build_int_s_extend(left, ty, "")
                    } else {
                        left
                    },
                );

                bin.builder.build_store(
                    divisor,
                    if bits < div_bits {
                        bin.builder.build_int_s_extend(right, ty, "")
                    } else {
                        right
                    },
                );

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret.into_int_value(),
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc), ns);
                let (revert_out, revert_out_len) =
                    bin.panic_data_const(ns, PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(ty, quotient, "quotient")
                    .into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(quotient, left.get_type(), "")
                } else {
                    quotient
                }
                .into()
            } else if ns.target == Target::Solana {
                // no signed div on BPF; do abs udev and then negate if needed
                let left_negative = bin.builder.build_int_compare(
                    IntPredicate::SLT,
                    left,
                    left.get_type().const_zero(),
                    "left_negative",
                );

                let left = bin
                    .builder
                    .build_select(
                        left_negative,
                        bin.builder.build_int_neg(left, "signed_left"),
                        left,
                        "left_abs",
                    )
                    .into_int_value();

                let right_negative = bin.builder.build_int_compare(
                    IntPredicate::SLT,
                    right,
                    right.get_type().const_zero(),
                    "right_negative",
                );

                let right = bin
                    .builder
                    .build_select(
                        right_negative,
                        bin.builder.build_int_neg(right, "signed_right"),
                        right,
                        "right_abs",
                    )
                    .into_int_value();

                let res = bin.builder.build_int_unsigned_div(left, right, "");

                let negate_result =
                    bin.builder
                        .build_xor(left_negative, right_negative, "negate_result");

                bin.builder.build_select(
                    negate_result,
                    bin.builder.build_int_neg(res, "unsigned_res"),
                    res,
                    "res",
                )
            } else {
                bin.builder.build_int_signed_div(left, right, "").into()
            }
        }
        Expression::UnsignedModulo {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("udivmod{div_bits}");

                let f = bin
                    .module
                    .get_function(&name)
                    .expect("div function missing");

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder.build_store(
                    dividend,
                    if bits < div_bits {
                        bin.builder.build_int_z_extend(left, ty, "")
                    } else {
                        left
                    },
                );

                bin.builder.build_store(
                    divisor,
                    if bits < div_bits {
                        bin.builder.build_int_z_extend(right, ty, "")
                    } else {
                        right
                    },
                );

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "udiv",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret.into_int_value(),
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc), ns);
                let (revert_out, revert_out_len) =
                    bin.panic_data_const(ns, PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let rem = bin.builder.build_load(ty, rem, "urem").into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(rem, bin.context.custom_width_int_type(bits), "")
                } else {
                    rem
                }
                .into()
            } else {
                bin.builder.build_int_unsigned_rem(left, right, "").into()
            }
        }
        Expression::SignedModulo {
            loc, left, right, ..
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            let bits = left.get_type().get_bit_width();

            if bits > 64 {
                let div_bits = if bits <= 128 { 128 } else { 256 };

                let name = format!("sdivmod{div_bits}");

                let f = bin
                    .module
                    .get_function(&name)
                    .expect("div function missing");

                let ty = bin.context.custom_width_int_type(div_bits);

                let dividend = bin.build_alloca(function, ty, "dividend");
                let divisor = bin.build_alloca(function, ty, "divisor");
                let rem = bin.build_alloca(function, ty, "remainder");
                let quotient = bin.build_alloca(function, ty, "quotient");

                bin.builder.build_store(
                    dividend,
                    if bits < div_bits {
                        bin.builder.build_int_s_extend(left, ty, "")
                    } else {
                        left
                    },
                );

                bin.builder.build_store(
                    divisor,
                    if bits < div_bits {
                        bin.builder.build_int_s_extend(right, ty, "")
                    } else {
                        right
                    },
                );

                let ret = bin
                    .builder
                    .build_call(
                        f,
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                        "sdiv",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret.into_int_value(),
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // throw division by zero error should be an assert
                bin.log_runtime_error(target, "division by zero".to_string(), Some(*loc), ns);
                let (revert_out, revert_out_len) =
                    bin.panic_data_const(ns, PanicCode::DivisionByZero);
                target.assert_failure(bin, revert_out, revert_out_len);

                bin.builder.position_at_end(success_block);

                let rem = bin.builder.build_load(ty, rem, "srem").into_int_value();

                if bits < div_bits {
                    bin.builder
                        .build_int_truncate(rem, bin.context.custom_width_int_type(bits), "")
                } else {
                    rem
                }
                .into()
            } else if ns.target == Target::Solana {
                // no signed rem on BPF; do abs udev and then negate if needed
                let left_negative = bin.builder.build_int_compare(
                    IntPredicate::SLT,
                    left,
                    left.get_type().const_zero(),
                    "left_negative",
                );

                let left = bin.builder.build_select(
                    left_negative,
                    bin.builder.build_int_neg(left, "signed_left"),
                    left,
                    "left_abs",
                );

                let right_negative = bin.builder.build_int_compare(
                    IntPredicate::SLT,
                    right,
                    right.get_type().const_zero(),
                    "right_negative",
                );

                let right = bin.builder.build_select(
                    right_negative,
                    bin.builder.build_int_neg(right, "signed_right"),
                    right,
                    "right_abs",
                );

                let res = bin.builder.build_int_unsigned_rem(
                    left.into_int_value(),
                    right.into_int_value(),
                    "",
                );

                bin.builder.build_select(
                    left_negative,
                    bin.builder.build_int_neg(res, "unsigned_res"),
                    res,
                    "res",
                )
            } else {
                bin.builder.build_int_signed_rem(left, right, "").into()
            }
        }
        Expression::Power {
            loc,
            ty: res_ty,
            overflowing,
            base: l,
            exp: r,
        } => {
            let left = expression(target, bin, l, vartab, function, ns);
            let right = expression(target, bin, r, vartab, function, ns);

            let bits = left.into_int_value().get_type().get_bit_width();
            let o = bin.build_alloca(function, left.get_type(), "");
            let f = power(
                target,
                bin,
                *overflowing,
                bits,
                res_ty.is_signed_int(ns),
                o,
                ns,
                *loc,
            );

            // If the function returns zero, then the operation was successful.
            let error_return = bin
                .builder
                .build_call(f, &[left.into(), right.into(), o.into()], "power")
                .try_as_basic_value()
                .left()
                .unwrap();

            // Load the result pointer
            let res = bin.builder.build_load(left.get_type(), o, "");

            // A return other than zero will abort execution. We need to check if power() returned a zero or not.
            let error_block = bin.context.append_basic_block(function, "error");
            let return_block = bin.context.append_basic_block(function, "return_block");

            let error_ret = bin.builder.build_int_compare(
                IntPredicate::NE,
                error_return.into_int_value(),
                error_return.get_type().const_zero().into_int_value(),
                "",
            );

            bin.builder
                .build_conditional_branch(error_ret, error_block, return_block);
            bin.builder.position_at_end(error_block);

            bin.log_runtime_error(target, "math overflow".to_string(), Some(*loc), ns);
            let (revert_out, revert_out_len) = bin.panic_data_const(ns, PanicCode::MathOverflow);
            target.assert_failure(bin, revert_out, revert_out_len);

            bin.builder.position_at_end(return_block);

            res
        }
        Expression::Equal { left, right, .. } => {
            if left.ty().is_address() {
                let mut res = bin.context.bool_type().const_int(1, false);
                let left = expression(target, bin, left, vartab, function, ns).into_array_value();
                let right = expression(target, bin, right, vartab, function, ns).into_array_value();

                // TODO: Address should be passed around as pointer. Once this is done, we can replace
                // this with a call to address_equal()
                for index in 0..ns.address_length {
                    let l = bin
                        .builder
                        .build_extract_value(left, index as u32, "left")
                        .unwrap()
                        .into_int_value();
                    let r = bin
                        .builder
                        .build_extract_value(right, index as u32, "right")
                        .unwrap()
                        .into_int_value();

                    res = bin.builder.build_and(
                        res,
                        bin.builder.build_int_compare(IntPredicate::EQ, l, r, ""),
                        "cmp",
                    );
                }

                res.into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::EQ, left, right, "")
                    .into()
            }
        }
        Expression::NotEqual { left, right, .. } => {
            if left.ty().is_address() {
                let mut res = bin.context.bool_type().const_int(0, false);
                let left = expression(target, bin, left, vartab, function, ns).into_array_value();
                let right = expression(target, bin, right, vartab, function, ns).into_array_value();

                // TODO: Address should be passed around as pointer. Once this is done, we can replace
                // this with a call to address_equal()
                for index in 0..ns.address_length {
                    let l = bin
                        .builder
                        .build_extract_value(left, index as u32, "left")
                        .unwrap()
                        .into_int_value();
                    let r = bin
                        .builder
                        .build_extract_value(right, index as u32, "right")
                        .unwrap()
                        .into_int_value();

                    res = bin.builder.build_or(
                        res,
                        bin.builder.build_int_compare(IntPredicate::NE, l, r, ""),
                        "cmp",
                    );
                }

                res.into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::NE, left, right, "")
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
                    ns,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

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
                    ns,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

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
                    ns,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

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
                    ns,
                )
                .into()
            } else {
                let left = expression(target, bin, left, vartab, function, ns).into_int_value();
                let right = expression(target, bin, right, vartab, function, ns).into_int_value();

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
                    .into()
            }
        }
        Expression::Variable { var_no, .. } => vartab[var_no].value,
        Expression::GetRef { expr, .. } => {
            let address = expression(target, bin, expr, vartab, function, ns).into_array_value();

            let stack = bin.build_alloca(function, address.get_type(), "address");

            bin.builder.build_store(stack, address);

            stack.into()
        }
        Expression::Load { ty, expr, .. } => {
            let ptr = expression(target, bin, expr, vartab, function, ns).into_pointer_value();

            if ty.is_reference_type(ns) && !ty.is_fixed_reference_type(ns) {
                let loaded_type = bin.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                let value = bin.builder.build_load(loaded_type, ptr, "");
                // if the pointer is null, it needs to be allocated
                let allocation_needed = bin
                    .builder
                    .build_is_null(value.into_pointer_value(), "allocation_needed");

                let allocate = bin.context.append_basic_block(function, "allocate");
                let already_allocated = bin
                    .context
                    .append_basic_block(function, "already_allocated");

                bin.builder.build_conditional_branch(
                    allocation_needed,
                    allocate,
                    already_allocated,
                );

                let entry = bin.builder.get_insert_block().unwrap();

                bin.builder.position_at_end(allocate);

                // allocate a new struct
                let ty = expr.ty();

                let llvm_ty = bin.llvm_type(ty.deref_memory(), ns);

                let new_struct = bin
                    .builder
                    .build_call(
                        bin.module.get_function("__malloc").unwrap(),
                        &[llvm_ty
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                bin.builder.build_store(ptr, new_struct);

                bin.builder.build_unconditional_branch(already_allocated);

                bin.builder.position_at_end(already_allocated);

                // insert phi node
                let combined_struct_ptr = bin.builder.build_phi(
                    llvm_ty.ptr_type(AddressSpace::default()),
                    &format!("ptr_{}", ty.to_string(ns)),
                );

                combined_struct_ptr.add_incoming(&[(&value, entry), (&new_struct, allocate)]);

                combined_struct_ptr.as_basic_value()
            } else {
                let loaded_type = bin.llvm_type(ty, ns);
                bin.builder.build_load(loaded_type, ptr, "")
            }
        }

        Expression::ZeroExt { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(ty, ns);

            bin.builder
                .build_int_z_extend(e, ty.into_int_type(), "")
                .into()
        }
        Expression::Negate {
            loc,
            expr,
            overflowing,
            ..
        } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();

            if *overflowing {
                bin.builder.build_int_neg(e, "").into()
            } else {
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    e.get_type().const_zero(),
                    e,
                    BinaryOp::Subtract,
                    true,
                    ns,
                    *loc,
                )
                .into()
            }
        }
        Expression::SignExt { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(ty, ns);

            bin.builder
                .build_int_s_extend(e, ty.into_int_type(), "")
                .into()
        }
        Expression::Trunc { ty, expr, .. } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(ty, ns);

            bin.builder
                .build_int_truncate(e, ty.into_int_type(), "")
                .into()
        }
        Expression::Cast { ty: to, expr, .. } => {
            let from = expr.ty();

            let e = expression(target, bin, expr, vartab, function, ns);

            runtime_cast(bin, function, &from, to, e, ns)
        }
        Expression::BytesCast {
            ty: Type::DynamicBytes,
            from: Type::Bytes(_),
            expr,
            ..
        } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();

            let size = e.get_type().get_bit_width() / 8;
            let size = bin.context.i32_type().const_int(size as u64, false);
            let elem_size = bin.context.i32_type().const_int(1, false);

            // Swap the byte order
            let bytes_ptr = bin.build_alloca(function, e.get_type(), "bytes_ptr");
            bin.builder.build_store(bytes_ptr, e);
            let init = bin.build_alloca(function, e.get_type(), "init");
            bin.builder.build_call(
                bin.module.get_function("__leNtobeN").unwrap(),
                &[bytes_ptr.into(), init.into(), size.into()],
                "",
            );

            bin.builder
                .build_call(
                    bin.module.get_function("vector_new").unwrap(),
                    &[size.into(), elem_size.into(), init.into()],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        }
        Expression::BytesCast {
            loc,
            ty: Type::Bytes(n),
            from: Type::DynamicBytes,
            expr: e,
        } => {
            let array = expression(target, bin, e, vartab, function, ns);

            let len = bin.vector_len(array);

            // Check if equal to n
            let is_equal_to_n = bin.builder.build_int_compare(
                IntPredicate::EQ,
                len,
                bin.context.i32_type().const_int(*n as u64, false),
                "is_equal_to_n",
            );
            let cast = bin.context.append_basic_block(function, "cast");
            let error = bin.context.append_basic_block(function, "error");
            bin.builder
                .build_conditional_branch(is_equal_to_n, cast, error);

            bin.builder.position_at_end(error);
            bin.log_runtime_error(target, "bytes cast error".to_string(), Some(*loc), ns);
            let (revert_out, revert_out_len) = bin.panic_data_const(ns, PanicCode::Generic);
            target.assert_failure(bin, revert_out, revert_out_len);

            bin.builder.position_at_end(cast);
            let bytes_ptr = bin.vector_bytes(array);

            // Switch byte order
            let ty = bin.context.custom_width_int_type(*n as u32 * 8);
            let le_bytes_ptr = bin.build_alloca(function, ty, "le_bytes");

            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[bytes_ptr.into(), le_bytes_ptr.into(), len.into()],
                "",
            );
            bin.builder.build_load(ty, le_bytes_ptr, "bytes")
        }
        Expression::Not { expr, .. } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();

            bin.builder
                .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                .into()
        }
        Expression::BitwiseNot { expr, .. } => {
            let e = expression(target, bin, expr, vartab, function, ns).into_int_value();

            bin.builder.build_not(e, "").into()
        }
        Expression::BitwiseOr { left, right: r, .. } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder.build_or(left, right, "").into()
        }
        Expression::BitwiseAnd { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            bin.builder.build_and(left, right, "").into()
        }
        Expression::BitwiseXor { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            bin.builder.build_xor(left, right, "").into()
        }
        Expression::ShiftLeft { left, right, .. } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            bin.builder.build_left_shift(left, right, "").into()
        }
        Expression::ShiftRight {
            left,
            right,
            signed,
            ..
        } => {
            let left = expression(target, bin, left, vartab, function, ns).into_int_value();
            let right = expression(target, bin, right, vartab, function, ns).into_int_value();

            bin.builder
                .build_right_shift(left, right, *signed, "")
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
                let index = expression(target, bin, index, vartab, function, ns).into_int_value();
                let slot = expression(target, bin, a, vartab, function, ns).into_int_value();
                target
                    .get_storage_bytes_subscript(bin, function, slot, index, *loc, ns)
                    .into()
            } else if ty.is_contract_storage() {
                let array = expression(target, bin, a, vartab, function, ns).into_int_value();
                let index = expression(target, bin, index, vartab, function, ns);

                target
                    .storage_subscript(bin, function, ty, array, index, ns)
                    .into()
            } else if elem_ty.is_builtin_struct() == Some(StructType::AccountInfo) {
                let array = expression(target, bin, a, vartab, function, ns).into_pointer_value();
                let index = expression(target, bin, index, vartab, function, ns).into_int_value();

                let llvm_ty = bin.module.get_struct_type("struct.SolAccountInfo").unwrap();
                unsafe {
                    bin.builder
                        .build_gep(llvm_ty, array, &[index], "account_info")
                        .into()
                }
            } else if ty.is_dynamic_memory() {
                let array = expression(target, bin, a, vartab, function, ns);

                let mut array_index =
                    expression(target, bin, index, vartab, function, ns).into_int_value();

                // bounds checking already done; we can down-cast if necessary
                if array_index.get_type().get_bit_width() > 32 {
                    array_index = bin.builder.build_int_truncate(
                        array_index,
                        bin.context.i32_type(),
                        "index",
                    );
                }

                let index = bin.builder.build_int_mul(
                    array_index,
                    bin.llvm_type(elem_ty.deref_memory(), ns)
                        .size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false),
                    "",
                );

                unsafe {
                    bin.builder.build_gep(
                        bin.context.i8_type(),
                        bin.vector_bytes(array),
                        &[index],
                        "index_access",
                    )
                }
                .into()
            } else {
                let array = expression(target, bin, a, vartab, function, ns).into_pointer_value();
                let index = expression(target, bin, index, vartab, function, ns).into_int_value();

                let llvm_ty = bin.llvm_type(ty.deref_memory(), ns);
                unsafe {
                    bin.builder
                        .build_gep(
                            llvm_ty,
                            array,
                            &[bin.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                        .into()
                }
            }
        }
        Expression::StructMember { expr, .. }
            if expr.ty().is_builtin_struct() == Some(StructType::AccountInfo) =>
        {
            target.builtin(bin, e, vartab, function, ns)
        }
        Expression::StructMember { expr, member, .. } => {
            let struct_ty = bin.llvm_type(expr.ty().deref_memory(), ns);
            let struct_ptr =
                expression(target, bin, expr, vartab, function, ns).into_pointer_value();

            bin.builder
                .build_struct_gep(struct_ty, struct_ptr, *member as u32, "struct member")
                .unwrap()
                .into()
        }
        Expression::ConstArrayLiteral {
            dimensions, values, ..
        } => {
            // For const arrays (declared with "constant" keyword, we should create a global constant
            let mut dims = dimensions.iter();

            let exprs = values
                .iter()
                .map(|e| expression(target, bin, e, vartab, function, ns).into_int_value())
                .collect::<Vec<IntValue>>();
            let ty = exprs[0].get_type();

            let top_size = *dims.next().unwrap();

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
            let ty = bin.llvm_type(ty, ns);

            let p = bin
                .builder
                .build_call(
                    bin.module.get_function("__malloc").unwrap(),
                    &[ty.size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false)
                        .into()],
                    "array_literal",
                )
                .try_as_basic_value()
                .left()
                .unwrap();

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
                };

                let elem = expression(target, bin, expr, vartab, function, ns);

                let elem = if expr.ty().is_fixed_reference_type(ns) {
                    let load_type = bin.llvm_type(&expr.ty(), ns);
                    bin.builder
                        .build_load(load_type, elem.into_pointer_value(), "elem")
                } else {
                    elem
                };

                bin.builder.build_store(elemptr, elem);
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
                let init = initializer.as_ref().unwrap();

                let data = bin.emit_global_string("const_string", init, true);

                bin.llvm_type(ty, ns)
                    .into_struct_type()
                    .const_named_struct(&[
                        data.into(),
                        bin.context
                            .custom_width_int_type(ns.target.ptr_size().into())
                            .const_int(init.len() as u64, false)
                            .into(),
                    ])
                    .into()
            } else {
                let elem = match ty {
                    Type::Slice(_) | Type::String | Type::DynamicBytes => Type::Bytes(1),
                    _ => ty.array_elem(),
                };

                let size = expression(target, bin, size, vartab, function, ns).into_int_value();

                let elem_size = bin
                    .llvm_type(&elem, ns)
                    .size_of()
                    .unwrap()
                    .const_cast(bin.context.i32_type(), false);

                bin.vector_new(size, elem_size, initializer.as_ref()).into()
            }
        }
        Expression::Builtin {
            kind: Builtin::ArrayLength,
            args,
            ..
        } if args[0].ty().array_deref().is_builtin_struct().is_none() => {
            let array = expression(target, bin, &args[0], vartab, function, ns);

            bin.vector_len(array).into()
        }
        Expression::Builtin {
            tys: returns,
            kind: Builtin::ReadFromBuffer,
            args,
            ..
        } => {
            let v = expression(target, bin, &args[0], vartab, function, ns);
            let offset = expression(target, bin, &args[1], vartab, function, ns).into_int_value();

            let data = if args[0].ty().is_dynamic_memory() {
                bin.vector_bytes(v)
            } else {
                v.into_pointer_value()
            };

            let start = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), data, &[offset], "start")
            };

            if matches!(returns[0], Type::Bytes(_) | Type::FunctionSelector) {
                let n = returns[0].bytes(ns);
                let bytes_ty = bin.context.custom_width_int_type(n as u32 * 8);

                let store = bin.build_alloca(function, bytes_ty, "stack");
                bin.builder.build_call(
                    bin.module.get_function("__beNtoleN").unwrap(),
                    &[
                        start.into(),
                        store.into(),
                        bin.context.i32_type().const_int(n as u64, false).into(),
                    ],
                    "",
                );
                bin.builder
                    .build_load(bytes_ty, store, &format!("bytes{n}"))
            } else {
                bin.builder
                    .build_load(bin.llvm_type(&returns[0], ns), start, "value")
            }
        }
        Expression::Keccak256 { exprs, .. } => {
            let mut length = bin.context.i32_type().const_zero();
            let mut values: Vec<(BasicValueEnum, IntValue, Type)> = Vec::new();

            // first we need to calculate the length of the buffer and get the types/lengths
            for e in exprs {
                let v = expression(target, bin, e, vartab, function, ns);

                let len = match e.ty() {
                    Type::DynamicBytes | Type::String => bin.vector_len(v),
                    _ => v
                        .get_type()
                        .size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false),
                };

                length = bin.builder.build_int_add(length, len, "");

                values.push((v, len, e.ty()));
            }

            //  now allocate a buffer
            let src = bin
                .builder
                .build_array_alloca(bin.context.i8_type(), length, "keccak_src");

            // fill in all the fields
            let mut offset = bin.context.i32_type().const_zero();

            for (v, len, ty) in values {
                let elem = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), src, &[offset], "elem")
                };

                offset = bin.builder.build_int_add(offset, len, "");

                match ty {
                    Type::DynamicBytes | Type::String => {
                        let data = bin.vector_bytes(v);

                        bin.builder.build_call(
                            bin.module.get_function("__memcpy").unwrap(),
                            &[elem.into(), data.into(), len.into()],
                            "",
                        );
                    }
                    _ => {
                        bin.builder.build_store(elem, v);
                    }
                }
            }
            let dst_type = bin.context.custom_width_int_type(256);
            let dst = bin.builder.build_alloca(dst_type, "keccak_dst");

            target.keccak256_hash(bin, src, length, dst, ns);

            bin.builder.build_load(dst_type, dst, "keccak256_hash")
        }
        Expression::StringCompare { left, right, .. } => {
            let (left, left_len) = string_location(target, bin, left, vartab, function, ns);
            let (right, right_len) = string_location(target, bin, right, vartab, function, ns);

            bin.builder
                .build_call(
                    bin.module.get_function("__memcmp").unwrap(),
                    &[left.into(), left_len.into(), right.into(), right_len.into()],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        }
        Expression::StringConcat { left, right, .. } => {
            let (left, left_len) = string_location(target, bin, left, vartab, function, ns);
            let (right, right_len) = string_location(target, bin, right, vartab, function, ns);

            bin.builder
                .build_call(
                    bin.module.get_function("concat").unwrap(),
                    &[left.into(), left_len.into(), right.into(), right_len.into()],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        }
        Expression::ReturnData { .. } => target.return_data(bin, function).into(),
        Expression::StorageArrayLength { array, elem_ty, .. } => {
            let slot = expression(target, bin, array, vartab, function, ns).into_int_value();

            target
                .storage_array_length(bin, function, slot, elem_ty, ns)
                .into()
        }
        Expression::Builtin {
            kind: Builtin::Signature,
            ..
        } if ns.target != Target::Solana => {
            // need to byte-reverse selector
            let selector_type = bin.context.i32_type();
            let selector = bin.build_alloca(function, selector_type, "selector");

            // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[
                    bin.selector.as_pointer_value().into(),
                    selector.into(),
                    bin.context.i32_type().const_int(4, false).into(),
                ],
                "",
            );

            bin.builder.build_load(selector_type, selector, "selector")
        }
        Expression::Builtin {
            kind: Builtin::AddMod,
            args,
            ..
        } => {
            let arith_ty = bin.context.custom_width_int_type(512);
            let res_ty = bin.context.custom_width_int_type(256);

            let x = expression(target, bin, &args[0], vartab, function, ns).into_int_value();
            let y = expression(target, bin, &args[1], vartab, function, ns).into_int_value();
            let k = expression(target, bin, &args[2], vartab, function, ns).into_int_value();
            let dividend = bin.builder.build_int_add(
                bin.builder.build_int_z_extend(x, arith_ty, "wide_x"),
                bin.builder.build_int_z_extend(y, arith_ty, "wide_y"),
                "x_plus_y",
            );

            let divisor = bin.builder.build_int_z_extend(k, arith_ty, "wide_k");

            let pdividend = bin.build_alloca(function, arith_ty, "dividend");
            let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
            let rem = bin.build_alloca(function, arith_ty, "remainder");
            let quotient = bin.build_alloca(function, arith_ty, "quotient");

            bin.builder.build_store(pdividend, dividend);
            bin.builder.build_store(pdivisor, divisor);

            let ret = bin
                .builder
                .build_call(
                    bin.module.get_function("udivmod512").unwrap(),
                    &[
                        pdividend.into(),
                        pdivisor.into(),
                        rem.into(),
                        quotient.into(),
                    ],
                    "quotient",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let success = bin.builder.build_int_compare(
                IntPredicate::EQ,
                ret,
                bin.context.i32_type().const_zero(),
                "success",
            );

            let success_block = bin.context.append_basic_block(function, "success");
            let bail_block = bin.context.append_basic_block(function, "bail");
            bin.builder
                .build_conditional_branch(success, success_block, bail_block);

            bin.builder.position_at_end(bail_block);

            // On Solana the return type is 64 bit
            let ret: BasicValueEnum = bin
                .builder
                .build_int_z_extend(
                    ret,
                    bin.return_values[&ReturnCode::Success].get_type(),
                    "ret",
                )
                .into();

            bin.builder.build_return(Some(&ret));
            bin.builder.position_at_end(success_block);

            let remainder = bin
                .builder
                .build_load(arith_ty, rem, "remainder")
                .into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
                .into()
        }
        Expression::Builtin {
            kind: Builtin::MulMod,
            args,
            ..
        } => {
            let arith_ty = bin.context.custom_width_int_type(512);
            let res_ty = bin.context.custom_width_int_type(256);

            let x = expression(target, bin, &args[0], vartab, function, ns).into_int_value();
            let y = expression(target, bin, &args[1], vartab, function, ns).into_int_value();
            let x_m = bin.build_alloca(function, arith_ty, "x_m");
            let y_m = bin.build_alloca(function, arith_ty, "x_y");
            let x_times_y_m = bin.build_alloca(function, arith_ty, "x_times_y_m");

            bin.builder
                .build_store(x_m, bin.builder.build_int_z_extend(x, arith_ty, "wide_x"));
            bin.builder
                .build_store(y_m, bin.builder.build_int_z_extend(y, arith_ty, "wide_y"));

            bin.builder.build_call(
                bin.module.get_function("__mul32").unwrap(),
                &[
                    x_m.into(),
                    y_m.into(),
                    x_times_y_m.into(),
                    bin.context.i32_type().const_int(512 / 32, false).into(),
                ],
                "",
            );
            let k = expression(target, bin, &args[2], vartab, function, ns).into_int_value();
            let dividend = bin.builder.build_load(arith_ty, x_times_y_m, "x_t_y");

            let divisor = bin.builder.build_int_z_extend(k, arith_ty, "wide_k");

            let pdividend = bin.build_alloca(function, arith_ty, "dividend");
            let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
            let rem = bin.build_alloca(function, arith_ty, "remainder");
            let quotient = bin.build_alloca(function, arith_ty, "quotient");

            bin.builder.build_store(pdividend, dividend);
            bin.builder.build_store(pdivisor, divisor);

            let ret = bin
                .builder
                .build_call(
                    bin.module.get_function("udivmod512").unwrap(),
                    &[
                        pdividend.into(),
                        pdivisor.into(),
                        rem.into(),
                        quotient.into(),
                    ],
                    "quotient",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let success = bin.builder.build_int_compare(
                IntPredicate::EQ,
                ret,
                bin.context.i32_type().const_zero(),
                "success",
            );

            let success_block = bin.context.append_basic_block(function, "success");
            let bail_block = bin.context.append_basic_block(function, "bail");
            bin.builder
                .build_conditional_branch(success, success_block, bail_block);

            bin.builder.position_at_end(bail_block);

            // On Solana the return type is 64 bit
            let ret: BasicValueEnum = bin
                .builder
                .build_int_z_extend(
                    ret,
                    bin.return_values[&ReturnCode::Success].get_type(),
                    "ret",
                )
                .into();

            bin.builder.build_return(Some(&ret));

            bin.builder.position_at_end(success_block);

            let remainder = bin
                .builder
                .build_load(arith_ty, rem, "quotient")
                .into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
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
            let v = expression(target, bin, &args[0], vartab, function, ns);

            let hash = match hash {
                Builtin::Ripemd160 => HashTy::Ripemd160,
                Builtin::Sha256 => HashTy::Sha256,
                Builtin::Keccak256 => HashTy::Keccak256,
                Builtin::Blake2_128 => HashTy::Blake2_128,
                Builtin::Blake2_256 => HashTy::Blake2_256,
                _ => unreachable!(),
            };

            target
                .hash(
                    bin,
                    function,
                    hash,
                    bin.vector_bytes(v),
                    bin.vector_len(v),
                    ns,
                )
                .into()
        }
        Expression::Builtin { .. } => target.builtin(bin, e, vartab, function, ns),
        Expression::InternalFunctionCfg { cfg_no, .. } => bin.functions[cfg_no]
            .as_global_value()
            .as_pointer_value()
            .into(),
        Expression::FormatString { args: fields, .. } => {
            format_string(target, bin, fields, vartab, function, ns)
        }

        Expression::AdvancePointer {
            pointer,
            bytes_offset,
        } => {
            let pointer = if pointer.ty().is_dynamic_memory() {
                bin.vector_bytes(expression(target, bin, pointer, vartab, function, ns))
            } else {
                expression(target, bin, pointer, vartab, function, ns).into_pointer_value()
            };
            let offset =
                expression(target, bin, bytes_offset, vartab, function, ns).into_int_value();
            let advanced = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), pointer, &[offset], "adv_pointer")
            };

            advanced.into()
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
    binary: &Binary<'a>,
    left: &Expression,
    right: &Expression,
    op: inkwell::IntPredicate,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
    ns: &Namespace,
) -> IntValue<'a> {
    let l = expression(target, binary, left, vartab, function, ns).into_array_value();
    let r = expression(target, binary, right, vartab, function, ns).into_array_value();

    let left = binary.build_alloca(function, binary.address_type(ns), "left");
    let right = binary.build_alloca(function, binary.address_type(ns), "right");

    binary.builder.build_store(left, l);
    binary.builder.build_store(right, r);

    let res = binary
        .builder
        .build_call(
            binary.module.get_function("__memcmp_ord").unwrap(),
            &[
                left.into(),
                right.into(),
                binary
                    .context
                    .i32_type()
                    .const_int(ns.address_length as u64, false)
                    .into(),
            ],
            "",
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

    binary
        .builder
        .build_int_compare(op, res, binary.context.i32_type().const_zero(), "")
}

fn runtime_cast<'a>(
    bin: &Binary<'a>,
    function: FunctionValue<'a>,
    from: &Type,
    to: &Type,
    val: BasicValueEnum<'a>,
    ns: &Namespace,
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
            assert_eq!(from.bytes(ns), to.bytes(ns),);

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
            let llvm_ty = bin.llvm_type(from, ns);

            let src = bin.build_alloca(function, llvm_ty, "dest");

            bin.builder.build_store(src, val.into_int_value());

            let dest = bin.build_alloca(function, bin.address_type(ns), "address");

            let len = bin
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false);

            bin.builder.build_call(
                bin.module.get_function("__leNtobeN").unwrap(),
                &[src.into(), dest.into(), len.into()],
                "",
            );

            bin.builder.build_load(bin.address_type(ns), dest, "val")
        }
        (Type::Address(_), Type::Bytes(_) | Type::Int(_) | Type::Uint(_) | Type::Value) => {
            let llvm_ty = bin.llvm_type(to, ns);

            let src = bin.build_alloca(function, bin.address_type(ns), "address");

            bin.builder.build_store(src, val.into_array_value());

            let dest = bin.build_alloca(function, llvm_ty, "dest");

            let len = bin
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false);

            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[src.into(), dest.into(), len.into()],
                "",
            );

            bin.builder.build_load(llvm_ty, dest, "val")
        }
        (Type::Bool, Type::Int(_) | Type::Uint(_)) => bin
            .builder
            .build_int_cast(
                val.into_int_value(),
                bin.llvm_type(to, ns).into_int_type(),
                "bool_to_int_cast",
            )
            .into(),
        (_, Type::Uint(_)) if !from.is_contract_storage() && from.is_reference_type(ns) => bin
            .builder
            .build_ptr_to_int(
                val.into_pointer_value(),
                bin.llvm_type(to, ns).into_int_type(),
                "ptr_to_int",
            )
            .into(),
        (Type::Uint(_), _) if to.is_reference_type(ns) => bin
            .builder
            .build_int_to_ptr(
                val.into_int_value(),
                bin.llvm_type(to, ns).ptr_type(AddressSpace::default()),
                "int_to_ptr",
            )
            .into(),
        (Type::DynamicBytes | Type::String, Type::Slice(_)) => {
            let slice_ty = bin.llvm_type(to, ns);
            let slice = bin.build_alloca(function, slice_ty, "slice");

            let data = bin.vector_bytes(val);

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap();

            bin.builder.build_store(data_ptr, data);

            let len =
                bin.builder
                    .build_int_z_extend(bin.vector_len(val), bin.context.i64_type(), "len");

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap();

            bin.builder.build_store(len_ptr, len);

            bin.builder.build_load(slice_ty, slice, "slice")
        }
        (Type::Address(_), Type::Slice(_)) => {
            let slice_ty = bin.llvm_type(to, ns);
            let slice = bin.build_alloca(function, slice_ty, "slice");
            let address = bin.build_alloca(function, bin.llvm_type(from, ns), "address");

            bin.builder.build_store(address, val);

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap();

            bin.builder.build_store(data_ptr, address);

            let len = bin
                .context
                .i64_type()
                .const_int(ns.address_length as u64, false);

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap();

            bin.builder.build_store(len_ptr, len);

            bin.builder.build_load(slice_ty, slice, "slice")
        }
        (Type::Bytes(bytes_length), Type::Slice(_)) => {
            let llvm_ty = bin.llvm_type(from, ns);
            let src = bin.build_alloca(function, llvm_ty, "src");

            bin.builder.build_store(src, val.into_int_value());

            let dest = bin.build_alloca(
                function,
                bin.context.i8_type().array_type((*bytes_length).into()),
                "dest",
            );

            bin.builder.build_call(
                bin.module.get_function("__leNtobeN").unwrap(),
                &[
                    src.into(),
                    dest.into(),
                    bin.context
                        .i32_type()
                        .const_int((*bytes_length).into(), false)
                        .into(),
                ],
                "",
            );

            let slice_ty = bin.llvm_type(to, ns);
            let slice = bin.build_alloca(function, slice_ty, "slice");

            let data_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 0, "data")
                .unwrap();

            bin.builder.build_store(data_ptr, dest);

            let len = bin
                .context
                .i64_type()
                .const_int((*bytes_length).into(), false);

            let len_ptr = bin
                .builder
                .build_struct_gep(slice_ty, slice, 1, "len")
                .unwrap();

            bin.builder.build_store(len_ptr, len);

            bin.builder.build_load(slice_ty, slice, "slice")
        }
        _ => unreachable!(),
    }
}

/// Emit expression into a slice
pub(super) fn expression_to_slice<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    e: &Expression,
    to: &Type,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
    ns: &Namespace,
) -> (PointerValue<'a>, IntValue<'a>) {
    emit_context!(bin);

    let Type::Slice(to_elem_ty) = to else {
        unreachable!()
    };

    let llvm_to = bin.llvm_type(to, ns);

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
                    ns,
                );

                let output_ptr = unsafe {
                    bin.builder.build_gep(
                        llvm_to,
                        output,
                        &[i32_const!(i.into()), i32_zero!()],
                        "output_ptr",
                    )
                };

                bin.builder.build_store(output_ptr, ptr);

                let output_len = unsafe {
                    bin.builder.build_gep(
                        llvm_to,
                        output,
                        &[i32_const!(i.into()), i32_const!(1)],
                        "output_len",
                    )
                };

                bin.builder.build_store(output_len, len);
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

            let val = expression(target, bin, e, vartab, function, ns);

            basic_value_to_slice(bin, val, &from, to, function, ns)
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
    ns: &Namespace,
) -> (PointerValue<'a>, IntValue<'a>) {
    emit_context!(bin);

    match from.deref_memory() {
        Type::Slice(_) | Type::DynamicBytes | Type::String => {
            let data = bin.vector_bytes(val);
            let len = bin.vector_len(val);

            (data, len)
        }
        Type::Address(_) => {
            let address = call!("__malloc", &[i32_const!(ns.address_length as u64).into()])
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            bin.builder.build_store(address, val);

            let len = i64_const!(ns.address_length as u64);

            (address, len)
        }
        Type::Bytes(bytes_length) => {
            let llvm_ty = bin.llvm_type(from, ns);
            let src = bin.build_alloca(function, llvm_ty, "src");

            bin.builder.build_store(src, val.into_int_value());

            let bytes_length: u64 = (*bytes_length).into();

            let dest = call!("__malloc", &[i32_const!(bytes_length).into()])
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            bin.builder.build_call(
                bin.module.get_function("__leNtobeN").unwrap(),
                &[src.into(), dest.into(), i32_const!(bytes_length).into()],
                "",
            );

            let len = i64_const!(bytes_length);

            (dest, len)
        }
        Type::Array(_, dims) => {
            let to_elem = to.array_elem();

            let to = bin.llvm_type(to, ns);

            let (val, from) = if let Type::Ref(ty) = from {
                let val = bin.builder.build_load(
                    bin.llvm_type(from, ns),
                    val.into_pointer_value(),
                    "val",
                );

                (val, ty.as_ref())
            } else {
                (val, from)
            };

            let length = match dims.last().unwrap() {
                ArrayLength::Dynamic => bin.vector_len(val),
                ArrayLength::Fixed(len) => i32_const!(len.to_u64().unwrap()),
                _ => unreachable!(),
            };

            // In Program Runtime v1, we can't do dynamic alloca. Remove the malloc once we move to
            // program runtime v2
            let size = bin.builder.build_int_mul(
                bin.builder.build_int_truncate(
                    bin.llvm_type(&Type::Slice(Type::Bytes(1).into()), ns)
                        .size_of()
                        .unwrap(),
                    bin.context.i32_type(),
                    "slice_size",
                ),
                length,
                "size",
            );

            let output = call!("__malloc", &[size.into()])
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            // loop over seeds
            let mut builder = LoopBuilder::new(bin, function);

            let index = builder.over(bin, i32_zero!(), length);

            // get value from array
            let input_elem = bin.array_subscript(from, val.into_pointer_value(), index, ns);

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
                    .build_load(bin.llvm_field_ty(&from_elem, ns), input_elem, "elem")
            } else {
                input_elem.into()
            };

            let (data, len) =
                basic_value_to_slice(bin, input_elem, &from_elem, &to_elem, function, ns);

            let output_data = unsafe {
                bin.builder
                    .build_gep(to, output, &[index, i32_zero!()], "output_data")
            };

            bin.builder.build_store(output_data, data);

            let output_len = unsafe {
                bin.builder
                    .build_gep(to, output, &[index, i32_const!(1)], "output_len")
            };

            bin.builder.build_store(output_len, len);

            builder.finish(bin);

            let length = bin
                .builder
                .build_int_z_extend(length, bin.context.i64_type(), "length");

            (output, length)
        }
        _ => unreachable!(),
    }
}
