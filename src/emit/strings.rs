// SPDX-License-Identifier: Apache-2.0

use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::expression::expression;
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast::{FormatArg, Namespace, RetrieveType, StringLocation, Type};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use std::collections::HashMap;

/// Implement "...{}...{}".format(a, b)
pub(super) fn format_string<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    args: &[(FormatArg, Expression)],
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
    ns: &Namespace,
) -> BasicValueEnum<'a> {
    // first we need to calculate the space we need
    let mut length = bin.context.i32_type().const_zero();

    let mut evaluated_arg = Vec::new();

    evaluated_arg.resize(args.len(), None);

    for (i, (spec, arg)) in args.iter().enumerate() {
        let len = if *spec == FormatArg::StringLiteral {
            if let Expression::BytesLiteral(_, _, bs) = arg {
                bin.context.i32_type().const_int(bs.len() as u64, false)
            } else {
                unreachable!();
            }
        } else {
            match arg.ty() {
                // bool: "true" or "false"
                Type::Bool => bin.context.i32_type().const_int(5, false),
                // hex encode bytes
                Type::Contract(_) | Type::Address(_) => bin
                    .context
                    .i32_type()
                    .const_int(ns.address_length as u64 * 2, false),
                Type::Bytes(size) => bin.context.i32_type().const_int(size as u64 * 2, false),
                Type::String => {
                    let val = expression(target, bin, arg, vartab, function, ns);

                    evaluated_arg[i] = Some(val);

                    bin.vector_len(val)
                }
                Type::DynamicBytes => {
                    let val = expression(target, bin, arg, vartab, function, ns);

                    evaluated_arg[i] = Some(val);

                    // will be hex encoded, so double
                    let len = bin.vector_len(val);

                    bin.builder.build_int_add(len, len, "hex_len")
                }
                Type::Uint(bits) if *spec == FormatArg::Hex => {
                    bin.context.i32_type().const_int(bits as u64 / 4 + 2, false)
                }
                Type::Int(bits) if *spec == FormatArg::Hex => {
                    bin.context.i32_type().const_int(bits as u64 / 4 + 3, false)
                }
                Type::Uint(bits) if *spec == FormatArg::Binary => {
                    bin.context.i32_type().const_int(bits as u64 + 2, false)
                }
                Type::Int(bits) if *spec == FormatArg::Binary => {
                    bin.context.i32_type().const_int(bits as u64 + 3, false)
                }
                // bits / 2 is a rough over-estimate of how many decimals we need
                Type::Uint(bits) if *spec == FormatArg::Default => {
                    bin.context.i32_type().const_int(bits as u64 / 2, false)
                }
                Type::Int(bits) if *spec == FormatArg::Default => {
                    bin.context.i32_type().const_int(bits as u64 / 2 + 1, false)
                }
                Type::Enum(enum_no) => bin
                    .context
                    .i32_type()
                    .const_int(ns.enums[enum_no].ty.bits(ns) as u64 / 3, false),
                _ => unimplemented!(),
            }
        };

        length = bin.builder.build_int_add(length, len, "");
    }

    // allocate the string and
    let vector = bin.vector_new(length, bin.context.i32_type().const_int(1, false), None);

    let output_start = bin.vector_bytes(vector.into());

    // now encode each of the arguments
    let mut output = output_start;

    // format it
    for (i, (spec, arg)) in args.iter().enumerate() {
        if *spec == FormatArg::StringLiteral {
            if let Expression::BytesLiteral(_, _, bs) = arg {
                let s = bin.emit_global_string("format_arg", bs, true);
                let len = bin.context.i32_type().const_int(bs.len() as u64, false);

                bin.builder.build_call(
                    bin.module.get_function("__memcpy").unwrap(),
                    &[output.into(), s.into(), len.into()],
                    "",
                );

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[len], "")
                };
            }
        } else {
            let val = evaluated_arg[i]
                .unwrap_or_else(|| expression(target, bin, arg, vartab, function, ns));
            let arg_ty = arg.ty();

            match arg_ty {
                Type::Bool => {
                    let len = bin
                        .builder
                        .build_select(
                            val.into_int_value(),
                            bin.context.i32_type().const_int(4, false),
                            bin.context.i32_type().const_int(5, false),
                            "bool_length",
                        )
                        .into_int_value();

                    let s = bin.builder.build_select(
                        val.into_int_value(),
                        bin.emit_global_string("bool_true", b"true", true),
                        bin.emit_global_string("bool_false", b"false", true),
                        "bool_value",
                    );

                    bin.builder.build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    );

                    output = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[len], "")
                    };
                }
                Type::String => {
                    let s = bin.vector_bytes(val);
                    let len = bin.vector_len(val);

                    bin.builder.build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    );

                    output = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[len], "")
                    };
                }
                Type::DynamicBytes => {
                    let s = bin.vector_bytes(val);
                    let len = bin.vector_len(val);

                    bin.builder.build_call(
                        bin.module.get_function("hex_encode").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    );

                    let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                    output = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[hex_len], "")
                    };
                }
                Type::Address(_) | Type::Contract(_) => {
                    // for Solana/Substrate, we should encode in base58
                    let buf = bin.build_alloca(function, bin.address_type(ns), "address");

                    bin.builder.build_store(buf, val.into_array_value());

                    let len = bin
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false);

                    bin.builder.build_call(
                        bin.module.get_function("hex_encode").unwrap(),
                        &[output.into(), buf.into(), len.into()],
                        "",
                    );

                    let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                    output = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[hex_len], "")
                    };
                }
                Type::Bytes(size) => {
                    let buf = bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "bytesN");

                    bin.builder.build_store(buf, val.into_int_value());

                    let len = bin.context.i32_type().const_int(size as u64, false);

                    bin.builder.build_call(
                        bin.module.get_function("hex_encode_rev").unwrap(),
                        &[output.into(), buf.into(), len.into()],
                        "",
                    );

                    let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                    output = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[hex_len], "")
                    };
                }
                Type::Enum(_) => {
                    let val = bin.builder.build_int_z_extend(
                        val.into_int_value(),
                        bin.context.i64_type(),
                        "val_64bits",
                    );

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint2dec").unwrap(),
                            &[output.into(), val.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }
                Type::Uint(bits) => {
                    if *spec == FormatArg::Default && bits <= 64 {
                        let val = if bits == 64 {
                            val.into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val.into_int_value(),
                                bin.context.i64_type(),
                                "val_64bits",
                            )
                        };

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint2dec").unwrap(),
                                &[output.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else if *spec == FormatArg::Default && bits <= 128 {
                        let val = if bits == 128 {
                            val.into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val.into_int_value(),
                                bin.context.custom_width_int_type(128),
                                "val_128bits",
                            )
                        };

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint128dec").unwrap(),
                                &[output.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else if *spec == FormatArg::Default {
                        let val = if bits == 256 {
                            val.into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val.into_int_value(),
                                bin.context.custom_width_int_type(256),
                                "val_256bits",
                            )
                        };

                        let pval = bin.build_alloca(
                            function,
                            bin.context.custom_width_int_type(256),
                            "int",
                        );

                        bin.builder.build_store(pval, val);

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint256dec").unwrap(),
                                &[output.into(), pval.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else {
                        let buf = bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "uint");

                        bin.builder.build_store(buf, val.into_int_value());

                        let len = bin.context.i32_type().const_int(bits as u64 / 8, false);

                        let func_name = if *spec == FormatArg::Hex {
                            "uint2hex"
                        } else {
                            "uint2bin"
                        };

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function(func_name).unwrap(),
                                &[output.into(), buf.into(), len.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                }
                Type::Int(bits) => {
                    let val = val.into_int_value();

                    let is_negative = bin.builder.build_int_compare(
                        IntPredicate::SLT,
                        val,
                        val.get_type().const_zero(),
                        "negative",
                    );

                    let entry = bin.builder.get_insert_block().unwrap();
                    let positive = bin.context.append_basic_block(function, "int_positive");
                    let negative = bin.context.append_basic_block(function, "int_negative");

                    bin.builder
                        .build_conditional_branch(is_negative, negative, positive);

                    bin.builder.position_at_end(negative);

                    // add "-" to output and negate our val
                    bin.builder
                        .build_store(output, bin.context.i8_type().const_int('-' as u64, false));

                    let minus_len = bin.context.i32_type().const_int(1, false);

                    let neg_data = unsafe {
                        bin.builder
                            .build_gep(bin.context.i8_type(), output, &[minus_len], "")
                    };
                    let neg_val = bin.builder.build_int_neg(val, "negative_int");

                    bin.builder.build_unconditional_branch(positive);

                    bin.builder.position_at_end(positive);

                    let data_phi = bin.builder.build_phi(output.get_type(), "data");
                    let val_phi = bin.builder.build_phi(val.get_type(), "val");

                    data_phi.add_incoming(&[(&neg_data, negative), (&output, entry)]);
                    val_phi.add_incoming(&[(&neg_val, negative), (&val, entry)]);

                    if *spec == FormatArg::Default && bits <= 64 {
                        let val = if bits == 64 {
                            val_phi.as_basic_value().into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.i64_type(),
                                "val_64bits",
                            )
                        };

                        let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint2dec").unwrap(),
                                &[output_after_minus.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else if *spec == FormatArg::Default && bits <= 128 {
                        let val = if bits == 128 {
                            val_phi.as_basic_value().into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.custom_width_int_type(128),
                                "val_128bits",
                            )
                        };

                        let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint128dec").unwrap(),
                                &[output_after_minus.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else if *spec == FormatArg::Default {
                        let val = if bits == 256 {
                            val_phi.as_basic_value().into_int_value()
                        } else {
                            bin.builder.build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.custom_width_int_type(256),
                                "val_256bits",
                            )
                        };

                        let pval = bin.build_alloca(
                            function,
                            bin.context.custom_width_int_type(256),
                            "int",
                        );

                        bin.builder.build_store(pval, val);

                        let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint256dec").unwrap(),
                                &[output_after_minus.into(), pval.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    } else {
                        let buf = bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "int");

                        bin.builder
                            .build_store(buf, val_phi.as_basic_value().into_int_value());

                        let len = bin.context.i32_type().const_int(bits as u64 / 8, false);

                        let func_name = if *spec == FormatArg::Hex {
                            "uint2hex"
                        } else {
                            "uint2bin"
                        };

                        let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function(func_name).unwrap(),
                                &[output_after_minus.into(), buf.into(), len.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                }
                _ => unimplemented!(),
            }
        }
    }

    // write the final length into the vector
    let length = bin.builder.build_int_sub(
        bin.builder
            .build_ptr_to_int(output, bin.context.i32_type(), "end"),
        bin.builder
            .build_ptr_to_int(output_start, bin.context.i32_type(), "begin"),
        "datalength",
    );

    let data_len = unsafe {
        bin.builder.build_gep(
            bin.module.get_struct_type("struct.vector").unwrap(),
            vector,
            &[
                bin.context.i32_type().const_zero(),
                bin.context.i32_type().const_zero(),
            ],
            "data_len",
        )
    };

    bin.builder.build_store(data_len, length);

    vector.into()
}

/// Load a string from expression or create global
pub(super) fn string_location<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    location: &StringLocation<Expression>,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
    ns: &Namespace,
) -> (PointerValue<'a>, IntValue<'a>) {
    match location {
        StringLocation::CompileTime(literal) => (
            bin.emit_global_string("const_string", literal, true),
            bin.context
                .i32_type()
                .const_int(literal.len() as u64, false),
        ),
        StringLocation::RunTime(e) => {
            let v = expression(target, bin, e, vartab, function, ns);

            (bin.vector_bytes(v), bin.vector_len(v))
        }
    }
}
