// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::codegen::{Builtin, Expression};
use crate::emit::binary::Binary;
use crate::emit::math::{build_binary_op_with_overflow_check, multiply, power};
use crate::emit::strings::{format_string, string_location};
use crate::emit::{BinaryOp, TargetRuntime, Variable};
use crate::sema::ast::{Namespace, RetrieveType, StructType, Type};
use crate::Target;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, StringRadix};
use inkwell::values::{ArrayValue, BasicValueEnum, FunctionValue, IntValue};
use inkwell::{AddressSpace, IntPredicate};
use num_bigint::Sign;
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
        Expression::FunctionArg(_, _, pos) => function.get_nth_param(*pos as u32).unwrap(),
        Expression::BoolLiteral(_, val) => {
            bin.context.bool_type().const_int(*val as u64, false).into()
        }
        Expression::NumberLiteral(_, Type::Address(_), val) => {
            // address can be negative; "address(-1)" is 0xffff...
            let mut bs = val.to_signed_bytes_be();

            // make sure it's no more than 32
            if bs.len() > ns.address_length {
                // remove leading bytes
                for _ in 0..bs.len() - ns.address_length {
                    bs.remove(0);
                }
            } else {
                // insert leading bytes
                let val = if val.sign() == Sign::Minus { 0xff } else { 0 };

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
        Expression::NumberLiteral(_, ty, n) => bin.number_literal(ty.bits(ns) as u32, n, ns).into(),
        Expression::StructLiteral(_, ty, exprs) => {
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

            let s = bin.builder.build_pointer_cast(
                s,
                struct_ty.ptr_type(AddressSpace::default()),
                "struct_literal",
            );

            for (i, expr) in exprs.iter().enumerate() {
                let elemptr = unsafe {
                    bin.builder.build_gep(
                        s,
                        &[
                            bin.context.i32_type().const_zero(),
                            bin.context.i32_type().const_int(i as u64, false),
                        ],
                        "struct member",
                    )
                };

                let elem = expression(target, bin, expr, vartab, function, ns);

                let elem = if expr.ty().is_fixed_reference_type() {
                    bin.builder.build_load(elem.into_pointer_value(), "elem")
                } else {
                    elem
                };

                bin.builder.build_store(elemptr, elem);
            }

            s.into()
        }
        Expression::BytesLiteral(_, _, bs) => {
            let ty = bin.context.custom_width_int_type((bs.len() * 8) as u32);

            // hex"11223344" should become i32 0x11223344
            let s = hex::encode(bs);

            ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                .unwrap()
                .into()
        }
        Expression::Add(_, _, unchecked, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            if bin.options.math_overflow_check && !*unchecked {
                let signed = l.ty().is_signed_int();
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Add,
                    signed,
                )
                .into()
            } else {
                bin.builder.build_int_add(left, right, "").into()
            }
        }
        Expression::Subtract(_, _, unchecked, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            if bin.options.math_overflow_check && !*unchecked {
                let signed = l.ty().is_signed_int();
                build_binary_op_with_overflow_check(
                    target,
                    bin,
                    function,
                    left,
                    right,
                    BinaryOp::Subtract,
                    signed,
                )
                .into()
            } else {
                bin.builder.build_int_sub(left, right, "").into()
            }
        }
        Expression::Multiply(_, res_ty, unchecked, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            multiply(
                target,
                bin,
                function,
                *unchecked,
                left,
                right,
                res_ty.is_signed_int(),
            )
            .into()
        }
        Expression::UnsignedDivide(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

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
                target.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(quotient, "quotient")
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
        Expression::SignedDivide(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

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
                target.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(quotient, "quotient")
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
        Expression::UnsignedModulo(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

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
                target.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(success_block);

                let rem = bin.builder.build_load(rem, "urem").into_int_value();

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
        Expression::SignedModulo(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

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
                target.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(success_block);

                let rem = bin.builder.build_load(rem, "srem").into_int_value();

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
        Expression::Power(_, res_ty, unchecked, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns);
            let right = expression(target, bin, r, vartab, function, ns);

            let bits = left.into_int_value().get_type().get_bit_width();
            let o = bin.build_alloca(function, left.get_type(), "");
            let f = power(target, bin, *unchecked, bits, res_ty.is_signed_int(), o);

            // If the function returns zero, then the operation was successful.
            let error_return = bin
                .builder
                .build_call(f, &[left.into(), right.into(), o.into()], "power")
                .try_as_basic_value()
                .left()
                .unwrap();

            // Load the result pointer
            let res = bin.builder.build_load(o, "");

            if !bin.options.math_overflow_check || *unchecked || ns.target != Target::Solana {
                // In Substrate, overflow case will hit an unreachable expression, so no additional checks are needed.
                res
            } else {
                // In Solana, a return other than zero will abort execution. We need to check if power() returned a zero or not.
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

                target.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(return_block);

                res
            }
        }
        Expression::Equal(_, l, r) => {
            if l.ty().is_address() {
                let mut res = bin.context.bool_type().const_int(1, false);
                let left = expression(target, bin, l, vartab, function, ns).into_array_value();
                let right = expression(target, bin, r, vartab, function, ns).into_array_value();

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
                let left = expression(target, bin, l, vartab, function, ns).into_int_value();
                let right = expression(target, bin, r, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::EQ, left, right, "")
                    .into()
            }
        }
        Expression::NotEqual(_, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder
                .build_int_compare(IntPredicate::NE, left, right, "")
                .into()
        }
        Expression::SignedMore(_, l, r) | Expression::UnsignedMore(_, l, r) => {
            if l.ty().is_address() {
                compare_address(target, bin, l, r, IntPredicate::SGT, vartab, function, ns).into()
            } else {
                let left = expression(target, bin, l, vartab, function, ns).into_int_value();
                let right = expression(target, bin, r, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(
                        if matches!(e, Expression::SignedMore(..)) {
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
        Expression::MoreEqual(_, l, r) => {
            if l.ty().is_address() {
                compare_address(target, bin, l, r, IntPredicate::SGE, vartab, function, ns).into()
            } else {
                let left = expression(target, bin, l, vartab, function, ns).into_int_value();
                let right = expression(target, bin, r, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(
                        if l.ty().is_signed_int() {
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
        Expression::SignedLess(_, l, r) | Expression::UnsignedLess(_, l, r) => {
            if l.ty().is_address() {
                compare_address(target, bin, l, r, IntPredicate::SLT, vartab, function, ns).into()
            } else {
                let left = expression(target, bin, l, vartab, function, ns).into_int_value();
                let right = expression(target, bin, r, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(
                        if matches!(e, Expression::SignedLess(..)) {
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
        Expression::LessEqual(_, l, r) => {
            if l.ty().is_address() {
                compare_address(target, bin, l, r, IntPredicate::SLE, vartab, function, ns).into()
            } else {
                let left = expression(target, bin, l, vartab, function, ns).into_int_value();
                let right = expression(target, bin, r, vartab, function, ns).into_int_value();

                bin.builder
                    .build_int_compare(
                        if l.ty().is_signed_int() {
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
        Expression::Variable(_, _, s) => vartab[s].value,
        Expression::GetRef(_, _, expr) => {
            let address = expression(target, bin, expr, vartab, function, ns).into_array_value();

            let stack = bin.build_alloca(function, address.get_type(), "address");

            bin.builder.build_store(stack, address);

            stack.into()
        }
        Expression::Load(_, ty, e) => {
            let ptr = expression(target, bin, e, vartab, function, ns).into_pointer_value();

            let value = bin.builder.build_load(ptr, "");

            if ty.is_reference_type(ns) && !ty.is_fixed_reference_type() {
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
                let ty = e.ty();

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

                let new_struct = bin.builder.build_pointer_cast(
                    new_struct,
                    llvm_ty.ptr_type(AddressSpace::default()),
                    &format!("new_{}", ty.to_string(ns)),
                );

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
                value
            }
        }

        Expression::ZeroExt(_, t, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(t, ns);

            bin.builder
                .build_int_z_extend(e, ty.into_int_type(), "")
                .into()
        }
        Expression::UnaryMinus(_, _, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();

            bin.builder.build_int_neg(e, "").into()
        }
        Expression::SignExt(_, t, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(t, ns);

            bin.builder
                .build_int_s_extend(e, ty.into_int_type(), "")
                .into()
        }
        Expression::Trunc(_, t, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();
            let ty = bin.llvm_type(t, ns);

            bin.builder
                .build_int_truncate(e, ty.into_int_type(), "")
                .into()
        }
        Expression::Cast(_, to, e) => {
            let from = e.ty();

            let e = expression(target, bin, e, vartab, function, ns);

            runtime_cast(bin, function, &from, to, e, ns)
        }
        Expression::BytesCast(_, Type::DynamicBytes, Type::Bytes(_), e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();

            let size = e.get_type().get_bit_width() / 8;
            let size = bin.context.i32_type().const_int(size as u64, false);
            let elem_size = bin.context.i32_type().const_int(1, false);

            // Swap the byte order
            let bytes_ptr = bin.build_alloca(function, e.get_type(), "bytes_ptr");
            bin.builder.build_store(bytes_ptr, e);
            let bytes_ptr = bin.builder.build_pointer_cast(
                bytes_ptr,
                bin.context.i8_type().ptr_type(AddressSpace::default()),
                "bytes_ptr",
            );
            let init = bin.builder.build_pointer_cast(
                bin.build_alloca(function, e.get_type(), "init"),
                bin.context.i8_type().ptr_type(AddressSpace::default()),
                "init",
            );
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
        Expression::BytesCast(_, Type::Bytes(n), Type::DynamicBytes, e) => {
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
            target.assert_failure(
                bin,
                bin.context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .const_null(),
                bin.context.i32_type().const_zero(),
            );

            bin.builder.position_at_end(cast);
            let bytes_ptr = bin.vector_bytes(array);

            // Switch byte order
            let ty = bin.context.custom_width_int_type(*n as u32 * 8);
            let le_bytes_ptr = bin.build_alloca(function, ty, "le_bytes");

            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[
                    bytes_ptr.into(),
                    bin.builder
                        .build_pointer_cast(
                            le_bytes_ptr,
                            bin.context.i8_type().ptr_type(AddressSpace::default()),
                            "le_bytes_ptr",
                        )
                        .into(),
                    len.into(),
                ],
                "",
            );
            bin.builder.build_load(le_bytes_ptr, "bytes")
        }
        Expression::Not(_, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();

            bin.builder
                .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                .into()
        }
        Expression::Complement(_, _, e) => {
            let e = expression(target, bin, e, vartab, function, ns).into_int_value();

            bin.builder.build_not(e, "").into()
        }
        Expression::BitwiseOr(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder.build_or(left, right, "").into()
        }
        Expression::BitwiseAnd(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder.build_and(left, right, "").into()
        }
        Expression::BitwiseXor(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder.build_xor(left, right, "").into()
        }
        Expression::ShiftLeft(_, _, l, r) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder.build_left_shift(left, right, "").into()
        }
        Expression::ShiftRight(_, _, l, r, signed) => {
            let left = expression(target, bin, l, vartab, function, ns).into_int_value();
            let right = expression(target, bin, r, vartab, function, ns).into_int_value();

            bin.builder
                .build_right_shift(left, right, *signed, "")
                .into()
        }
        Expression::Subscript(_, elem_ty, ty, a, i) => {
            if ty.is_storage_bytes() {
                let index = expression(target, bin, i, vartab, function, ns).into_int_value();
                let slot = expression(target, bin, a, vartab, function, ns).into_int_value();
                target
                    .get_storage_bytes_subscript(bin, function, slot, index)
                    .into()
            } else if ty.is_contract_storage() {
                let array = expression(target, bin, a, vartab, function, ns).into_int_value();
                let index = expression(target, bin, i, vartab, function, ns);

                target
                    .storage_subscript(bin, function, ty, array, index, ns)
                    .into()
            } else if elem_ty.is_builtin_struct() == Some(StructType::AccountInfo) {
                let array = expression(target, bin, a, vartab, function, ns).into_pointer_value();
                let index = expression(target, bin, i, vartab, function, ns).into_int_value();

                unsafe {
                    bin.builder
                        .build_gep(array, &[index], "account_info")
                        .into()
                }
            } else if ty.is_dynamic_memory() {
                let array = expression(target, bin, a, vartab, function, ns);

                let ty = bin.llvm_field_ty(elem_ty, ns);

                let mut array_index =
                    expression(target, bin, i, vartab, function, ns).into_int_value();

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
                    ty.into_pointer_type()
                        .get_element_type()
                        .size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false),
                    "",
                );

                let elem = unsafe {
                    bin.builder
                        .build_gep(bin.vector_bytes(array), &[index], "index_access")
                };

                bin.builder
                    .build_pointer_cast(elem, ty.into_pointer_type(), "elem")
                    .into()
            } else {
                let array = expression(target, bin, a, vartab, function, ns).into_pointer_value();
                let index = expression(target, bin, i, vartab, function, ns).into_int_value();

                unsafe {
                    bin.builder
                        .build_gep(
                            array,
                            &[bin.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                        .into()
                }
            }
        }
        Expression::StructMember(_, _, a, _)
            if a.ty().is_builtin_struct() == Some(StructType::AccountInfo) =>
        {
            target.builtin(bin, e, vartab, function, ns)
        }
        Expression::StructMember(_, _, a, i) => {
            let struct_ptr = expression(target, bin, a, vartab, function, ns).into_pointer_value();

            bin.builder
                .build_struct_gep(struct_ptr, *i as u32, "struct member")
                .unwrap()
                .into()
        }
        Expression::ConstArrayLiteral(_, _, dims, exprs) => {
            // For const arrays (declared with "constant" keyword, we should create a global constant
            let mut dims = dims.iter();

            let exprs = exprs
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
        Expression::ArrayLiteral(_, ty, dims, exprs) => {
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

            let array = bin.builder.build_pointer_cast(
                p.into_pointer_value(),
                ty.ptr_type(AddressSpace::default()),
                "array_literal",
            );

            for (i, expr) in exprs.iter().enumerate() {
                let mut ind = vec![bin.context.i32_type().const_zero()];

                let mut e = i as u32;

                for d in dims {
                    ind.insert(1, bin.context.i32_type().const_int((e % *d).into(), false));

                    e /= *d;
                }

                let elemptr = unsafe { bin.builder.build_gep(array, &ind, &format!("elemptr{i}")) };

                let elem = expression(target, bin, expr, vartab, function, ns);

                let elem = if expr.ty().is_fixed_reference_type() {
                    bin.builder.build_load(elem.into_pointer_value(), "elem")
                } else {
                    elem
                };

                bin.builder.build_store(elemptr, elem);
            }

            array.into()
        }
        Expression::AllocDynamicBytes(_, ty, size, init) => {
            if matches!(ty, Type::Slice(_)) {
                let init = init.as_ref().unwrap();

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

                bin.vector_new(size, elem_size, init.as_ref()).into()
            }
        }
        Expression::Builtin(_, _, Builtin::ArrayLength, args)
            if args[0].ty().array_deref().is_builtin_struct().is_none() =>
        {
            let array = expression(target, bin, &args[0], vartab, function, ns);

            bin.vector_len(array).into()
        }
        Expression::Builtin(_, returns, Builtin::ReadFromBuffer, args) => {
            let v = expression(target, bin, &args[0], vartab, function, ns);
            let offset = expression(target, bin, &args[1], vartab, function, ns).into_int_value();

            let data = if args[0].ty().is_dynamic_memory() {
                bin.vector_bytes(v)
            } else {
                v.into_pointer_value()
            };

            let start = unsafe { bin.builder.build_gep(data, &[offset], "start") };

            if matches!(returns[0], Type::Bytes(_) | Type::FunctionSelector) {
                let n = returns[0].bytes(ns);

                let store = bin.build_alloca(
                    function,
                    bin.context.custom_width_int_type(n as u32 * 8),
                    "stack",
                );
                bin.builder.build_call(
                    bin.module.get_function("__beNtoleN").unwrap(),
                    &[
                        bin.builder
                            .build_pointer_cast(
                                start,
                                bin.context.i8_type().ptr_type(AddressSpace::default()),
                                "",
                            )
                            .into(),
                        bin.builder
                            .build_pointer_cast(
                                store,
                                bin.context.i8_type().ptr_type(AddressSpace::default()),
                                "",
                            )
                            .into(),
                        bin.context.i32_type().const_int(n as u64, false).into(),
                    ],
                    "",
                );
                bin.builder.build_load(store, &format!("bytes{n}"))
            } else {
                let start = bin.builder.build_pointer_cast(
                    start,
                    bin.llvm_type(&returns[0], ns)
                        .ptr_type(AddressSpace::default()),
                    "start",
                );

                bin.builder.build_load(start, "value")
            }
        }
        Expression::Keccak256(_, _, exprs) => {
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
                let elem = unsafe { bin.builder.build_gep(src, &[offset], "elem") };

                offset = bin.builder.build_int_add(offset, len, "");

                match ty {
                    Type::DynamicBytes | Type::String => {
                        let data = bin.vector_bytes(v);

                        bin.builder.build_call(
                            bin.module.get_function("__memcpy").unwrap(),
                            &[
                                elem.into(),
                                bin.builder
                                    .build_pointer_cast(
                                        data,
                                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                                        "data",
                                    )
                                    .into(),
                                len.into(),
                            ],
                            "",
                        );
                    }
                    _ => {
                        let elem = bin.builder.build_pointer_cast(
                            elem,
                            v.get_type().ptr_type(AddressSpace::default()),
                            "",
                        );

                        bin.builder.build_store(elem, v);
                    }
                }
            }
            let dst = bin
                .builder
                .build_alloca(bin.context.custom_width_int_type(256), "keccak_dst");

            target.keccak256_hash(bin, src, length, dst, ns);

            bin.builder.build_load(dst, "keccak256_hash")
        }
        Expression::StringCompare(_, l, r) => {
            let (left, left_len) = string_location(target, bin, l, vartab, function, ns);
            let (right, right_len) = string_location(target, bin, r, vartab, function, ns);

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
        Expression::StringConcat(_, _, l, r) => {
            let (left, left_len) = string_location(target, bin, l, vartab, function, ns);
            let (right, right_len) = string_location(target, bin, r, vartab, function, ns);

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
        Expression::ReturnData(_) => target.return_data(bin, function).into(),
        Expression::StorageArrayLength { array, elem_ty, .. } => {
            let slot = expression(target, bin, array, vartab, function, ns).into_int_value();

            target
                .storage_array_length(bin, function, slot, elem_ty, ns)
                .into()
        }
        Expression::AbiEncode {
            tys, packed, args, ..
        } => target
            .abi_encode_to_vector(
                bin,
                function,
                &packed
                    .iter()
                    .map(|a| expression(target, bin, a, vartab, function, ns))
                    .collect::<Vec<BasicValueEnum>>(),
                &args
                    .iter()
                    .map(|a| expression(target, bin, a, vartab, function, ns))
                    .collect::<Vec<BasicValueEnum>>(),
                tys,
                ns,
            )
            .into(),
        Expression::Builtin(_, _, Builtin::Signature, _) if ns.target != Target::Solana => {
            // need to byte-reverse selector
            let selector = bin.build_alloca(function, bin.context.i32_type(), "selector");

            // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[
                    bin.builder
                        .build_pointer_cast(
                            bin.selector.as_pointer_value(),
                            bin.context.i8_type().ptr_type(AddressSpace::default()),
                            "",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            selector,
                            bin.context.i8_type().ptr_type(AddressSpace::default()),
                            "",
                        )
                        .into(),
                    bin.context.i32_type().const_int(4, false).into(),
                ],
                "",
            );

            bin.builder.build_load(selector, "selector")
        }
        Expression::Builtin(_, _, Builtin::AddMod, args) => {
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

            let remainder = bin.builder.build_load(rem, "remainder").into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
                .into()
        }
        Expression::Builtin(_, _, Builtin::MulMod, args) => {
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
                    bin.builder
                        .build_pointer_cast(
                            x_m,
                            bin.context.i32_type().ptr_type(AddressSpace::default()),
                            "left",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            y_m,
                            bin.context.i32_type().ptr_type(AddressSpace::default()),
                            "right",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            x_times_y_m,
                            bin.context.i32_type().ptr_type(AddressSpace::default()),
                            "output",
                        )
                        .into(),
                    bin.context.i32_type().const_int(512 / 32, false).into(),
                ],
                "",
            );
            let k = expression(target, bin, &args[2], vartab, function, ns).into_int_value();
            let dividend = bin.builder.build_load(x_times_y_m, "x_t_y");

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

            let remainder = bin.builder.build_load(rem, "quotient").into_int_value();

            bin.builder
                .build_int_truncate(remainder, res_ty, "quotient")
                .into()
        }
        Expression::Builtin(_, _, hash @ Builtin::Ripemd160, args)
        | Expression::Builtin(_, _, hash @ Builtin::Keccak256, args)
        | Expression::Builtin(_, _, hash @ Builtin::Blake2_128, args)
        | Expression::Builtin(_, _, hash @ Builtin::Blake2_256, args)
        | Expression::Builtin(_, _, hash @ Builtin::Sha256, args) => {
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
        Expression::Builtin(..) => target.builtin(bin, e, vartab, function, ns),
        Expression::InternalFunctionCfg(cfg_no) => bin.functions[cfg_no]
            .as_global_value()
            .as_pointer_value()
            .into(),
        Expression::FormatString(_, args) => format_string(target, bin, args, vartab, function, ns),

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
            let advanced = unsafe { bin.builder.build_gep(pointer, &[offset], "adv_pointer") };

            advanced.into()
        }

        Expression::RationalNumberLiteral(..)
        | Expression::List(..)
        | Expression::Undefined(..)
        | Expression::Poison
        | Expression::BytesCast(..) => {
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
                binary
                    .builder
                    .build_pointer_cast(
                        left,
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        "left",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        right,
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        "right",
                    )
                    .into(),
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
    if matches!(from, Type::Address(_) | Type::Contract(_))
        && matches!(to, Type::Address(_) | Type::Contract(_))
    {
        // no conversion needed
        val
    } else if let Type::Address(_) = to {
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
            &[
                bin.builder
                    .build_pointer_cast(
                        src,
                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                        "address_ptr",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        dest,
                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                        "dest_ptr",
                    )
                    .into(),
                len.into(),
            ],
            "",
        );

        bin.builder.build_load(dest, "val")
    } else if let Type::Address(_) = from {
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
            &[
                bin.builder
                    .build_pointer_cast(
                        src,
                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                        "address_ptr",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        dest,
                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                        "dest_ptr",
                    )
                    .into(),
                len.into(),
            ],
            "",
        );

        bin.builder.build_load(dest, "val")
    } else if matches!(from, Type::Bool) && matches!(to, Type::Int(_) | Type::Uint(_)) {
        bin.builder
            .build_int_cast(
                val.into_int_value(),
                bin.llvm_type(to, ns).into_int_type(),
                "bool_to_int_cast",
            )
            .into()
    } else if from.is_reference_type(ns) && matches!(to, Type::Uint(_)) {
        bin.builder
            .build_ptr_to_int(
                val.into_pointer_value(),
                bin.llvm_type(to, ns).into_int_type(),
                "ptr_to_int",
            )
            .into()
    } else if to.is_reference_type(ns) && matches!(from, Type::Uint(_)) {
        bin.builder
            .build_int_to_ptr(
                val.into_int_value(),
                bin.llvm_type(to, ns).ptr_type(AddressSpace::Generic),
                "int_to_ptr",
            )
            .into()
    } else if matches!((from, to), (Type::DynamicBytes, Type::Slice(_))) {
        let slice = bin.build_alloca(function, bin.llvm_type(to, ns), "slice");

        let data = bin.vector_bytes(val);

        let data_ptr = bin.builder.build_struct_gep(slice, 0, "data").unwrap();

        bin.builder.build_store(data_ptr, data);

        let len =
            bin.builder
                .build_int_z_extend(bin.vector_len(val), bin.context.i64_type(), "len");

        let len_ptr = bin.builder.build_struct_gep(slice, 1, "len").unwrap();

        bin.builder.build_store(len_ptr, len);

        bin.builder.build_load(slice, "slice")
    } else {
        val
    }
}
