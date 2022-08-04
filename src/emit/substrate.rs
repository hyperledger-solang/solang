// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::codegen::cfg::HashTy;
use crate::sema::ast;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicType, IntType};
use inkwell::values::{
    ArrayValue, BasicValueEnum, CallableValue, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use num_traits::ToPrimitive;
use solang_parser::pt;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::emit::{Binary, TargetRuntime, Variable};

// When using the seal api, we use our own scratch buffer.
const SCRATCH_SIZE: u32 = 32 * 1024;

pub struct SubstrateTarget {
    unique_strings: HashMap<usize, usize>,
}

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Binary<'a> {
        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            std_lib,
            None,
        );

        binary.set_early_value_aborts(contract, ns);

        let scratch_len = binary.module.add_global(
            context.i32_type(),
            Some(AddressSpace::Generic),
            "scratch_len",
        );
        scratch_len.set_linkage(Linkage::Internal);
        scratch_len.set_initializer(&context.i32_type().get_undef());

        binary.scratch_len = Some(scratch_len);

        let scratch = binary.module.add_global(
            context.i8_type().array_type(SCRATCH_SIZE),
            Some(AddressSpace::Generic),
            "scratch",
        );
        scratch.set_linkage(Linkage::Internal);
        scratch.set_initializer(&context.i8_type().array_type(SCRATCH_SIZE).get_undef());
        binary.scratch = Some(scratch);

        let mut b = SubstrateTarget {
            unique_strings: HashMap::new(),
        };

        b.declare_externals(&binary);

        b.emit_functions(&mut binary, contract, ns);

        b.emit_deploy(&mut binary, contract, ns);
        b.emit_call(&binary, contract, ns);

        binary.internalize(&[
            "deploy",
            "call",
            "seal_input",
            "seal_set_storage",
            "seal_get_storage",
            "seal_clear_storage",
            "seal_hash_keccak_256",
            "seal_hash_sha2_256",
            "seal_hash_blake2_128",
            "seal_hash_blake2_256",
            "seal_return",
            "seal_debug_message",
            "seal_instantiate",
            "seal_call",
            "seal_value_transferred",
            "seal_minimum_balance",
            "seal_weight_to_fee",
            "seal_random",
            "seal_address",
            "seal_balance",
            "seal_block_number",
            "seal_now",
            "seal_gas_price",
            "seal_gas_left",
            "seal_caller",
            "seal_tombstone_deposit",
            "seal_terminate",
            "seal_deposit_event",
            "seal_transfer",
        ]);

        binary
    }

    fn public_function_prelude<'a>(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        abort_value_transfers: bool,
        ns: &ast::Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // after copying stratch, first thing to do is abort value transfers if constructors not payable
        if abort_value_transfers {
            self.abort_if_value_transfer(binary, function, ns);
        }

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "");

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        // copy arguments from input buffer
        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        binary.builder.build_call(
            binary.module.get_function("seal_input").unwrap(),
            &[scratch_buf.into(), scratch_len.into()],
            "",
        );

        let args = binary.builder.build_pointer_cast(
            scratch_buf,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );
        let args_length = binary.builder.build_load(scratch_len, "input_len");

        // store the length in case someone wants it via msg.data
        binary.builder.build_store(
            binary.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        (args, args_length.into_int_value())
    }

    fn declare_externals(&self, binary: &Binary) {
        let u8_ptr = binary
            .context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .into();
        let u32_val = binary.context.i32_type().into();
        let u32_ptr = binary
            .context
            .i32_type()
            .ptr_type(AddressSpace::Generic)
            .into();
        let u64_val = binary.context.i64_type().into();

        binary.module.add_function(
            "seal_input",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_hash_keccak_256",
            binary.context.void_type().fn_type(
                &[
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    binary.context.i32_type().into(), // len
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_hash_sha2_256",
            binary.context.void_type().fn_type(
                &[
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    binary.context.i32_type().into(), // len
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_hash_blake2_128",
            binary.context.void_type().fn_type(
                &[
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    binary.context.i32_type().into(), // len
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_hash_blake2_256",
            binary.context.void_type().fn_type(
                &[
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    binary.context.i32_type().into(), // len
                    binary
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_random",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_val, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_set_storage",
            binary.context.void_type().fn_type(
                &[
                    u8_ptr,  // key_ptr
                    u8_ptr,  // value_ptr
                    u32_val, // value_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_debug_message",
            binary.context.i32_type().fn_type(
                &[
                    u8_ptr,  // string_ptr
                    u32_val, // string_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_clear_storage",
            binary.context.void_type().fn_type(
                &[
                    u8_ptr, // key_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_get_storage",
            binary
                .context
                .i32_type()
                .fn_type(&[u8_ptr, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_return",
            binary.context.void_type().fn_type(
                &[
                    u32_val, u8_ptr, u32_val, // flags, data ptr, and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_instantiate",
            binary.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // code hash ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                    u8_ptr, u32_ptr, // address ptr and len
                    u8_ptr, u32_ptr, // output ptr and len
                    u8_ptr, u32_val, // salt ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_call",
            binary.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                    u8_ptr, u32_ptr, // output ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_transfer",
            binary.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                    u8_ptr, u32_val, // value ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_value_transferred",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_address",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_balance",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_minimum_balance",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_block_number",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_now",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_tombstone_deposit",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_weight_to_fee",
            binary
                .context
                .void_type()
                .fn_type(&[u64_val, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_gas_left",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_caller",
            binary
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_terminate",
            binary.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "seal_deposit_event",
            binary.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // topic ptr and len
                    u8_ptr, u32_val, // data ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );
    }

    fn emit_deploy(&mut self, binary: &mut Binary, contract: &ast::Contract, ns: &ast::Namespace) {
        let initializer = self.emit_initializer(binary, contract, ns);

        // create deploy function
        let function = binary.module.add_function(
            "deploy",
            binary.context.void_type().fn_type(&[], false),
            None,
        );

        // deploy always receives an endowment so no value check here
        let (deploy_args, deploy_args_length) =
            self.public_function_prelude(binary, function, false, ns);

        // init our storage vars
        binary.builder.build_call(initializer, &[], "");

        let fallback_block = binary.context.append_basic_block(function, "fallback");

        self.emit_function_dispatch(
            binary,
            contract,
            ns,
            pt::FunctionTy::Constructor,
            deploy_args,
            deploy_args_length,
            function,
            &binary.functions,
            Some(fallback_block),
            |_| false,
        );

        // emit fallback code
        binary.builder.position_at_end(fallback_block);

        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );
    }

    fn emit_call(&mut self, binary: &Binary, contract: &ast::Contract, ns: &ast::Namespace) {
        // create call function
        let function = binary.module.add_function(
            "call",
            binary.context.void_type().fn_type(&[], false),
            None,
        );

        let (call_args, call_args_length) = self.public_function_prelude(
            binary,
            function,
            binary.function_abort_value_transfers,
            ns,
        );

        self.emit_function_dispatch(
            binary,
            contract,
            ns,
            pt::FunctionTy::Function,
            call_args,
            call_args_length,
            function,
            &binary.functions,
            None,
            |func| !binary.function_abort_value_transfers && func.nonpayable,
        );
    }

    /// ABI decode a single primitive
    fn decode_primitive<'b>(
        &self,
        binary: &Binary<'b>,
        ty: &ast::Type,
        src: PointerValue<'b>,
        ns: &ast::Namespace,
    ) -> (BasicValueEnum<'b>, u64) {
        match ty {
            ast::Type::Bool => {
                let val = binary.builder.build_int_compare(
                    IntPredicate::EQ,
                    binary.builder.build_load(src, "abi_bool").into_int_value(),
                    binary.context.i8_type().const_int(1, false),
                    "bool",
                );
                (val.into(), 1)
            }
            ast::Type::Uint(bits) | ast::Type::Int(bits) => {
                let int_type = binary.context.custom_width_int_type(*bits as u32);

                let val = binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        src,
                        int_type.ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "",
                );

                // substrate only supports power-of-two types; step over the
                // the remainer

                // FIXME: we should do some type-checking here and ensure that the
                // encoded value fits into our smaller type
                let len = bits.next_power_of_two() as u64 / 8;

                (val, len)
            }
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                let val = binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        src,
                        binary.address_type(ns).ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "",
                );

                let len = ns.address_length as u64;

                (val, len)
            }
            ast::Type::Bytes(len) => {
                let int_type = binary.context.custom_width_int_type(*len as u32 * 8);

                let buf = binary.builder.build_alloca(int_type, "buf");

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                binary.builder.build_call(
                    binary.module.get_function("__beNtoleN").unwrap(),
                    &[
                        src.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                buf,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .context
                            .i32_type()
                            .const_int(*len as u64, false)
                            .into(),
                    ],
                    "",
                );

                (
                    binary.builder.build_load(buf, &format!("bytes{}", len)),
                    *len as u64,
                )
            }
            _ => unreachable!(),
        }
    }

    /// Check that data has not overrun end, and whether end == data to check we do not have
    /// trailing data
    fn check_overrun(
        &self,
        binary: &Binary,
        function: FunctionValue,
        data: PointerValue,
        end: PointerValue,
        end_is_data: bool,
    ) {
        let in_bounds = binary.builder.build_int_compare(
            if end_is_data {
                IntPredicate::EQ
            } else {
                IntPredicate::ULE
            },
            binary
                .builder
                .build_ptr_to_int(data, binary.context.i32_type(), "args"),
            binary
                .builder
                .build_ptr_to_int(end, binary.context.i32_type(), "end"),
            "is_done",
        );

        let success_block = binary.context.append_basic_block(function, "success");
        let bail_block = binary.context.append_basic_block(function, "bail");
        binary
            .builder
            .build_conditional_branch(in_bounds, success_block, bail_block);

        binary.builder.position_at_end(bail_block);

        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(success_block);
    }

    /// recursively encode a single ty
    fn decode_ty<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue,
        ty: &ast::Type,
        data: &mut PointerValue<'b>,
        end: PointerValue<'b>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        match &ty {
            ast::Type::Bool
            | ast::Type::Address(_)
            | ast::Type::Contract(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_) => {
                let (arg, arglen) = self.decode_primitive(binary, ty, *data, ns);

                *data = unsafe {
                    binary.builder.build_gep(
                        *data,
                        &[binary.context.i32_type().const_int(arglen, false)],
                        "abi_ptr",
                    )
                };

                self.check_overrun(binary, function, *data, end, false);

                arg
            }
            ast::Type::Enum(n) => self.decode_ty(binary, function, &ns.enums[*n].ty, data, end, ns),
            ast::Type::UserType(n) => {
                self.decode_ty(binary, function, &ns.user_types[*n].ty, data, end, ns)
            }
            ast::Type::Struct(str_ty) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                let size = llvm_ty
                    .size_of()
                    .unwrap()
                    .const_cast(binary.context.i32_type(), false);

                let new = binary
                    .builder
                    .build_call(
                        binary.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let dest = binary.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    "dest",
                );

                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            dest,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    let val = self.decode_ty(binary, function, &field.ty, data, end, ns);

                    let val = if field.ty.deref_memory().is_fixed_reference_type() {
                        binary
                            .builder
                            .build_load(val.into_pointer_value(), field.name_as_str())
                    } else {
                        val
                    };

                    binary.builder.build_store(elem, val);
                }

                dest.into()
            }
            ast::Type::Array(_, dim) => {
                if let Some(ast::ArrayLength::Fixed(d)) = dim.last() {
                    let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                    let size = llvm_ty
                        .size_of()
                        .unwrap()
                        .const_cast(binary.context.i32_type(), false);

                    let ty = ty.array_deref();

                    let new = binary
                        .builder
                        .build_call(
                            binary.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let dest = binary.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );

                    binary.emit_static_loop_with_pointer(
                        function,
                        binary.context.i64_type().const_zero(),
                        binary
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        data,
                        |index: IntValue<'b>, data: &mut PointerValue<'b>| {
                            let elem = unsafe {
                                binary.builder.build_gep(
                                    dest,
                                    &[binary.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val = self.decode_ty(binary, function, &ty, data, end, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type() {
                                binary.builder.build_load(val.into_pointer_value(), "elem")
                            } else {
                                val
                            };

                            binary.builder.build_store(elem, val);
                        },
                    );

                    dest.into()
                } else {
                    let len = binary
                        .builder
                        .build_alloca(binary.context.i32_type(), "length");

                    *data = binary
                        .builder
                        .build_call(
                            binary.module.get_function("compact_decode_u32").unwrap(),
                            &[(*data).into(), len.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let len = binary.builder.build_load(len, "array.len").into_int_value();

                    // details about our array elements
                    let elem_ty = binary.llvm_field_ty(&ty.array_elem(), ns);
                    let elem_size = elem_ty
                        .size_of()
                        .unwrap()
                        .const_cast(binary.context.i32_type(), false);

                    let init = binary.builder.build_int_to_ptr(
                        binary.context.i32_type().const_all_ones(),
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    let v = binary
                        .builder
                        .build_call(
                            binary.module.get_function("vector_new").unwrap(),
                            &[len.into(), elem_size.into(), init.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    binary.emit_loop_cond_first_with_pointer(
                        function,
                        binary.context.i32_type().const_zero(),
                        len,
                        data,
                        |elem_no: IntValue<'b>, data: &mut PointerValue<'b>| {
                            let index = binary.builder.build_int_mul(elem_no, elem_size, "");

                            let element_start = unsafe {
                                binary.builder.build_gep(
                                    v,
                                    &[
                                        binary.context.i32_type().const_zero(),
                                        binary.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let elem = binary.builder.build_pointer_cast(
                                element_start,
                                elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            let ty = ty.array_deref();

                            let val = self.decode_ty(binary, function, &ty, data, end, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type() {
                                binary.builder.build_load(val.into_pointer_value(), "elem")
                            } else {
                                val
                            };

                            binary.builder.build_store(elem, val);
                        },
                    );
                    v.into()
                }
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let from = binary.builder.build_alloca(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "from",
                );

                binary.builder.build_store(from, *data);

                let v = binary
                    .builder
                    .build_call(
                        binary.module.get_function("scale_decode_string").unwrap(),
                        &[from.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                *data = binary.builder.build_load(from, "data").into_pointer_value();

                self.check_overrun(binary, function, *data, end, false);

                v
            }
            ast::Type::Ref(ty) => self.decode_ty(binary, function, ty, data, end, ns),
            ast::Type::ExternalFunction { .. } => {
                let address =
                    self.decode_ty(binary, function, &ast::Type::Address(false), data, end, ns);
                let selector =
                    self.decode_ty(binary, function, &ast::Type::Uint(32), data, end, ns);

                let ty = binary.llvm_type(ty, ns);

                let ef = binary
                    .builder
                    .build_call(
                        binary.module.get_function("__malloc").unwrap(),
                        &[ty.into_pointer_type()
                            .get_element_type()
                            .size_of()
                            .unwrap()
                            .const_cast(binary.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ef =
                    binary
                        .builder
                        .build_pointer_cast(ef, ty.into_pointer_type(), "function_type");

                let address_member = unsafe {
                    binary.builder.build_gep(
                        ef,
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(1, false),
                        ],
                        "address",
                    )
                };

                binary.builder.build_store(address_member, address);

                let selector_member = unsafe {
                    binary.builder.build_gep(
                        ef,
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_zero(),
                        ],
                        "selector",
                    )
                };

                binary.builder.build_store(selector_member, selector);

                ef.into()
            }
            _ => unreachable!(),
        }
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        binary: &Binary,
        load: bool,
        ty: &ast::Type,
        dest: PointerValue,
        arg: BasicValueEnum,
        ns: &ast::Namespace,
    ) -> u64 {
        match ty {
            ast::Type::Bool => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                binary.builder.build_store(
                    dest,
                    binary.builder.build_int_z_extend(
                        arg.into_int_value(),
                        binary.context.i8_type(),
                        "bool",
                    ),
                );

                1
            }
            ast::Type::Uint(_) | ast::Type::Int(_) => {
                let len = match ty {
                    ast::Type::Uint(n) | ast::Type::Int(n) => *n as u64 / 8,
                    _ => ns.address_length as u64,
                };

                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // substrate only supports power-of-two types; upcast to correct type
                let power_of_two_len = len.next_power_of_two();

                let arg = if len == power_of_two_len {
                    arg.into_int_value()
                } else if ty.is_signed_int() {
                    binary.builder.build_int_s_extend(
                        arg.into_int_value(),
                        binary
                            .context
                            .custom_width_int_type(power_of_two_len as u32 * 8),
                        "",
                    )
                } else {
                    binary.builder.build_int_z_extend(
                        arg.into_int_value(),
                        binary
                            .context
                            .custom_width_int_type(power_of_two_len as u32 * 8),
                        "",
                    )
                };

                binary.builder.build_store(
                    binary.builder.build_pointer_cast(
                        dest,
                        arg.get_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    arg,
                );

                power_of_two_len
            }
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                binary.builder.build_store(
                    binary.builder.build_pointer_cast(
                        dest,
                        binary.address_type(ns).ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    arg.into_array_value(),
                );

                ns.address_length as u64
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = binary
                        .builder
                        .build_alloca(arg.into_int_value().get_type(), &format!("bytes{}", n));

                    binary.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                val,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        dest.into(),
                        binary.context.i32_type().const_int(*n as u64, false).into(),
                    ],
                    "",
                );

                *n as u64
            }
            _ => unimplemented!(),
        }
    }

    /// recursively encode argument. The encoded data is written to the data pointer,
    /// and the pointer is updated point after the encoded data.
    ///
    /// FIXME: this function takes a "load" arguments, which tells the encoded whether the data should be
    /// dereferenced. However, this is already encoded by the fact it is a Type::Ref(..) type. So, the load
    /// argument should be removed from this function.
    pub fn encode_ty<'x>(
        &self,
        binary: &Binary<'x>,
        ns: &ast::Namespace,
        load: bool,
        packed: bool,
        function: FunctionValue,
        ty: &ast::Type,
        arg: BasicValueEnum<'x>,
        data: &mut PointerValue<'x>,
    ) {
        match &ty {
            ast::Type::Bool
            | ast::Type::Address(_)
            | ast::Type::Contract(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_) => {
                let arglen = self.encode_primitive(binary, load, ty, *data, arg, ns);

                *data = unsafe {
                    binary.builder.build_gep(
                        *data,
                        &[binary.context.i32_type().const_int(arglen, false)],
                        "",
                    )
                };
            }
            ast::Type::UserType(no) => self.encode_ty(
                binary,
                ns,
                load,
                packed,
                function,
                &ns.user_types[*no].ty,
                arg,
                data,
            ),
            ast::Type::Enum(no) => self.encode_ty(
                binary,
                ns,
                load,
                packed,
                function,
                &ns.enums[*no].ty,
                arg,
                data,
            ),
            ast::Type::Array(_, dim) if matches!(dim.last(), Some(ast::ArrayLength::Fixed(_))) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(arg.into_pointer_value(), "")
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_array = binary.context.append_basic_block(function, "null_array");
                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let dim = ty.array_length().unwrap().to_u64().unwrap();

                let elem_ty = ty.array_deref();

                let is_null = binary.builder.build_is_null(arg, "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut normal_data = *data;

                binary.emit_static_loop_with_pointer(
                    function,
                    binary.context.i64_type().const_zero(),
                    binary.context.i64_type().const_int(dim, false),
                    &mut normal_data,
                    |index, elem_data| {
                        let elem = unsafe {
                            binary.builder.build_gep(
                                arg,
                                &[binary.context.i32_type().const_zero(), index],
                                "index_access",
                            )
                        };

                        self.encode_ty(
                            binary,
                            ns,
                            !elem_ty.is_fixed_reference_type(),
                            packed,
                            function,
                            &elem_ty,
                            elem.into(),
                            elem_data,
                        );
                    },
                );

                binary.builder.build_unconditional_branch(done_array);

                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                let mut null_data = *data;

                let elem = binary.default_value(elem_ty.deref_any(), ns);

                binary.emit_static_loop_with_pointer(
                    function,
                    binary.context.i64_type().const_zero(),
                    binary.context.i64_type().const_int(dim, false),
                    &mut null_data,
                    |_, elem_data| {
                        self.encode_ty(
                            binary,
                            ns,
                            false,
                            packed,
                            function,
                            elem_ty.deref_any(),
                            elem,
                            elem_data,
                        );
                    },
                );

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let either_data = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "either_data",
                );

                either_data.add_incoming(&[(&normal_data, normal_array), (&null_data, null_array)]);

                *data = either_data.as_basic_value().into_pointer_value()
            }
            ast::Type::Array(..) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = binary.vector_len(arg);

                if !packed {
                    *data = binary
                        .builder
                        .build_call(
                            binary.module.get_function("compact_encode_u32").unwrap(),
                            &[(*data).into(), len.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                let elem_ty = ty.array_deref();

                binary.emit_loop_cond_first_with_pointer(
                    function,
                    binary.context.i32_type().const_zero(),
                    len,
                    data,
                    |elem_no, data| {
                        let elem =
                            binary.array_subscript(ty, arg.into_pointer_value(), elem_no, ns);

                        self.encode_ty(
                            binary,
                            ns,
                            !elem_ty.deref_any().is_fixed_reference_type(),
                            packed,
                            function,
                            elem_ty.deref_any(),
                            elem.into(),
                            data,
                        );
                    },
                );
            }
            ast::Type::Struct(str_ty) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(
                            arg.into_pointer_value(),
                            &format!("encode_{}", str_ty.definition(ns).name),
                        )
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary.builder.build_is_null(arg, "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_data = *data;
                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    self.encode_ty(
                        binary,
                        ns,
                        !field.ty.is_fixed_reference_type(),
                        packed,
                        function,
                        &field.ty,
                        elem.into(),
                        &mut normal_data,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_data = *data;

                for field in &str_ty.definition(ns).fields {
                    let elem = binary.default_value(&field.ty, ns);

                    self.encode_ty(
                        binary,
                        ns,
                        false,
                        packed,
                        function,
                        &field.ty,
                        elem,
                        &mut null_data,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let either_data = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "either_data",
                );

                either_data
                    .add_incoming(&[(&normal_data, normal_struct), (&null_data, null_struct)]);

                *data = either_data.as_basic_value().into_pointer_value()
            }
            ast::Type::Ref(ty) => {
                self.encode_ty(
                    binary,
                    ns,
                    !ty.is_fixed_reference_type(),
                    packed,
                    function,
                    ty,
                    arg,
                    data,
                );
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let string_len = binary.vector_len(arg);

                let string_data = binary.vector_bytes(arg);

                if !packed {
                    let function = binary.module.get_function("scale_encode_string").unwrap();

                    *data = binary
                        .builder
                        .build_call(
                            function,
                            &[(*data).into(), string_data.into(), string_len.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else {
                    binary.builder.build_call(
                        binary.module.get_function("__memcpy").unwrap(),
                        &[
                            (*data).into(),
                            binary
                                .builder
                                .build_pointer_cast(
                                    string_data,
                                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            string_len.into(),
                        ],
                        "",
                    );

                    *data = unsafe { binary.builder.build_gep(*data, &[string_len], "") };
                }
            }
            ast::Type::ExternalFunction { .. } => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let address_member = unsafe {
                    binary.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(1, false),
                        ],
                        "address",
                    )
                };

                let address = binary.builder.build_load(address_member, "address");

                self.encode_ty(
                    binary,
                    ns,
                    false,
                    false,
                    function,
                    &ast::Type::Address(false),
                    address,
                    data,
                );

                let selector_member = unsafe {
                    binary.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_zero(),
                        ],
                        "selector",
                    )
                };

                let selector = binary.builder.build_load(selector_member, "selector");

                self.encode_ty(
                    binary,
                    ns,
                    false,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    selector,
                    data,
                );
            }
            _ => unreachable!(),
        };
    }

    /// Calculate the maximum space a type will need when encoded. This is used for
    /// allocating enough space to do abi encoding. The length for vectors is always
    /// assumed to be five, even when it can be encoded in less bytes. The overhead
    /// of calculating the exact size is not worth reducing the malloc by a few bytes.
    ///
    /// FIXME: this function takes a "load" arguments, which tells the encoded whether the data should be
    /// dereferenced. However, this is already encoded by the fact it is a Type::Ref(..) type. So, the load
    /// argument should be removed from this function.
    pub fn encoded_length<'x>(
        &self,
        arg: BasicValueEnum<'x>,
        load: bool,
        packed: bool,
        ty: &ast::Type,
        function: FunctionValue,
        binary: &Binary<'x>,
        ns: &ast::Namespace,
    ) -> IntValue<'x> {
        match ty {
            ast::Type::Bool => binary.context.i32_type().const_int(1, false),
            ast::Type::Uint(n) | ast::Type::Int(n) => {
                binary.context.i32_type().const_int(*n as u64 / 8, false)
            }
            ast::Type::Bytes(n) => binary.context.i32_type().const_int(*n as u64, false),
            ast::Type::Address(_) | ast::Type::Contract(_) => binary
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false),
            ast::Type::Enum(n) => {
                self.encoded_length(arg, load, packed, &ns.enums[*n].ty, function, binary, ns)
            }
            ast::Type::Struct(str_ty) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(
                            arg.into_pointer_value(),
                            &format!("encoded_length_struct_{}", str_ty.definition(ns).name),
                        )
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary.builder.build_is_null(arg, "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_sum = binary.context.i32_type().const_zero();

                // avoid generating load instructions for structs with only fixed fields
                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    normal_sum = binary.builder.build_int_add(
                        normal_sum,
                        self.encoded_length(
                            elem.into(),
                            !field.ty.is_fixed_reference_type(),
                            packed,
                            &field.ty,
                            function,
                            binary,
                            ns,
                        ),
                        "",
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_sum = binary.context.i32_type().const_zero();

                for field in &str_ty.definition(ns).fields {
                    null_sum = binary.builder.build_int_add(
                        null_sum,
                        self.encoded_length(
                            binary.default_value(&field.ty, ns),
                            false,
                            packed,
                            &field.ty,
                            function,
                            binary,
                            ns,
                        ),
                        "",
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let sum = binary.builder.build_phi(binary.context.i32_type(), "sum");

                sum.add_incoming(&[(&normal_sum, normal_struct), (&null_sum, null_struct)]);

                sum.as_basic_value().into_int_value()
            }
            ast::Type::Array(_, dims)
                if matches!(dims.last(), Some(ast::ArrayLength::Fixed(_))) =>
            {
                let array_length = binary
                    .context
                    .i32_type()
                    .const_int(ty.array_length().unwrap().to_u64().unwrap(), false);

                let elem_ty = ty.array_deref();

                if elem_ty.is_dynamic(ns) {
                    let arg = if load {
                        binary
                            .builder
                            .build_load(arg.into_pointer_value(), "")
                            .into_pointer_value()
                    } else {
                        arg.into_pointer_value()
                    };

                    let normal_array = binary.context.append_basic_block(function, "normal_array");
                    let null_array = binary.context.append_basic_block(function, "null_array");
                    let done_array = binary.context.append_basic_block(function, "done_array");

                    let is_null = binary.builder.build_is_null(arg, "is_null");

                    binary
                        .builder
                        .build_conditional_branch(is_null, null_array, normal_array);

                    binary.builder.position_at_end(normal_array);

                    let mut normal_length = binary.context.i32_type().const_zero();

                    // if the array contains dynamic elements, we have to iterate over
                    // every one and calculate its length
                    binary.emit_static_loop_with_int(
                        function,
                        binary.context.i32_type().const_zero(),
                        array_length,
                        &mut normal_length,
                        |index, sum| {
                            let elem = unsafe {
                                binary.builder.build_gep(
                                    arg,
                                    &[binary.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            *sum = binary.builder.build_int_add(
                                self.encoded_length(
                                    elem.into(),
                                    !elem_ty.deref_memory().is_fixed_reference_type(),
                                    packed,
                                    &elem_ty,
                                    function,
                                    binary,
                                    ns,
                                ),
                                *sum,
                                "",
                            );
                        },
                    );

                    binary.builder.build_unconditional_branch(done_array);

                    let normal_array = binary.builder.get_insert_block().unwrap();

                    binary.builder.position_at_end(null_array);

                    let elem = binary.default_value(elem_ty.deref_any(), ns);

                    let null_length = binary.builder.build_int_mul(
                        self.encoded_length(
                            elem,
                            false,
                            packed,
                            elem_ty.deref_any(),
                            function,
                            binary,
                            ns,
                        ),
                        array_length,
                        "",
                    );

                    binary.builder.build_unconditional_branch(done_array);

                    let null_array = binary.builder.get_insert_block().unwrap();

                    binary.builder.position_at_end(done_array);

                    let encoded_length = binary
                        .builder
                        .build_phi(binary.context.i32_type(), "encoded_length");

                    encoded_length.add_incoming(&[
                        (&normal_length, normal_array),
                        (&null_length, null_array),
                    ]);

                    encoded_length.as_basic_value().into_int_value()
                } else {
                    // elements have static length
                    let elem = binary.default_value(elem_ty.deref_any(), ns);

                    binary.builder.build_int_mul(
                        self.encoded_length(
                            elem,
                            false,
                            packed,
                            elem_ty.deref_any(),
                            function,
                            binary,
                            ns,
                        ),
                        array_length,
                        "",
                    )
                }
            }
            ast::Type::Array(_, dims) if dims.last() == Some(&ast::ArrayLength::Dynamic) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut encoded_length = binary.context.i32_type().const_int(5, false);

                let array_length = binary.vector_len(arg);

                let elem_ty = ty.array_deref();
                let llvm_elem_ty = binary.llvm_field_ty(&elem_ty, ns);

                if elem_ty.is_dynamic(ns) {
                    // if the array contains elements of dynamic length, we have to iterate over all of them
                    binary.emit_loop_cond_first_with_int(
                        function,
                        binary.context.i32_type().const_zero(),
                        array_length,
                        &mut encoded_length,
                        |index, sum| {
                            let index = binary.builder.build_int_mul(
                                index,
                                llvm_elem_ty
                                    .into_pointer_type()
                                    .get_element_type()
                                    .size_of()
                                    .unwrap()
                                    .const_cast(binary.context.i32_type(), false),
                                "",
                            );

                            let p = unsafe {
                                binary.builder.build_gep(
                                    arg.into_pointer_value(),
                                    &[
                                        binary.context.i32_type().const_zero(),
                                        binary.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "index_access",
                                )
                            };
                            let elem = binary.builder.build_pointer_cast(
                                p,
                                llvm_elem_ty.into_pointer_type(),
                                "elem",
                            );

                            *sum = binary.builder.build_int_add(
                                self.encoded_length(
                                    elem.into(),
                                    !elem_ty.deref_memory().is_fixed_reference_type(),
                                    packed,
                                    &elem_ty,
                                    function,
                                    binary,
                                    ns,
                                ),
                                *sum,
                                "",
                            );
                        },
                    );

                    encoded_length
                } else {
                    // elements have static length
                    let elem = binary.default_value(elem_ty.deref_any(), ns);

                    binary.builder.build_int_add(
                        encoded_length,
                        binary.builder.build_int_mul(
                            self.encoded_length(
                                elem,
                                false,
                                packed,
                                elem_ty.deref_any(),
                                function,
                                binary,
                                ns,
                            ),
                            array_length,
                            "",
                        ),
                        "",
                    )
                }
            }
            ast::Type::Ref(r) => self.encoded_length(arg, load, packed, r, function, binary, ns),
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // A string or bytes type has to be encoded by: one compact integer for
                // the length, followed by the bytes themselves. Here we assume that the
                // length requires 5 bytes.
                let len = binary.vector_len(arg);

                if packed {
                    len
                } else {
                    binary.builder.build_int_add(
                        len,
                        binary.context.i32_type().const_int(5, false),
                        "",
                    )
                }
            }
            ast::Type::ExternalFunction { .. } => {
                // address + 4 bytes selector
                binary
                    .context
                    .i32_type()
                    .const_int(ns.address_length as u64 + 4, false)
            }
            _ => unreachable!(),
        }
    }

    /// Create a unique salt each time this function is called.
    fn contract_unique_salt<'x>(
        &mut self,
        binary: &'x Binary,
        binary_no: usize,
        ns: &ast::Namespace,
    ) -> (PointerValue<'x>, IntValue<'x>) {
        let counter = *self.unique_strings.get(&binary_no).unwrap_or(&0);

        let binary_name = &ns.contracts[binary_no].name;

        let unique = format!("{}-{}", binary_name, counter);

        let salt = binary.emit_global_string(
            &format!("salt_{}_{}", binary_name, counter),
            blake2_rfc::blake2b::blake2b(32, &[], unique.as_bytes()).as_bytes(),
            true,
        );

        self.unique_strings.insert(binary_no, counter + 1);

        (salt, binary.context.i32_type().const_int(32, false))
    }
}

impl<'a> TargetRuntime<'a> for SubstrateTarget {
    fn storage_delete_single_slot(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
    ) {
        binary.builder.build_call(
            binary.module.get_function("seal_clear_storage").unwrap(),
            &[binary
                .builder
                .build_pointer_cast(
                    slot,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "",
        );
    }

    fn set_storage(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        // TODO: check for non-zero
        let dest_ty = dest.get_type().get_element_type();

        let dest_size = if dest_ty.is_array_type() {
            dest_ty
                .into_array_type()
                .size_of()
                .expect("array should be fixed size")
                .const_cast(binary.context.i32_type(), false)
        } else {
            dest_ty
                .into_int_type()
                .size_of()
                .const_cast(binary.context.i32_type(), false)
        };

        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest_size.into(),
            ],
            "",
        );
    }

    fn set_storage_extfunc(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest.get_type()
                    .get_element_type()
                    .size_of()
                    .unwrap()
                    .const_cast(binary.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn get_storage_extfunc(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        let ty = binary.llvm_type(
            &ast::Type::ExternalFunction {
                params: Vec::new(),
                mutability: ast::Mutability::Nonpayable(pt::Loc::Codegen),
                returns: Vec::new(),
            },
            ns,
        );

        let len = ty
            .into_pointer_type()
            .get_element_type()
            .size_of()
            .unwrap()
            .const_cast(binary.context.i32_type(), false);

        let ef = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[len.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();
        binary.builder.build_store(scratch_len, len);

        let _exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    ef.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        // TODO: decide behaviour if not exist

        binary
            .builder
            .build_pointer_cast(ef, ty.into_pointer_type(), "function_type")
    }

    fn set_storage_string(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        let len = binary.vector_len(dest);
        let data = binary.vector_bytes(dest);

        let exists = binary.builder.build_int_compare(
            IntPredicate::NE,
            len,
            binary.context.i32_type().const_zero(),
            "exists",
        );

        let delete_block = binary.context.append_basic_block(function, "delete_block");

        let set_block = binary.context.append_basic_block(function, "set_block");

        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, set_block, delete_block);

        binary.builder.position_at_end(set_block);

        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        data,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                len.into(),
            ],
            "",
        );

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(delete_block);

        binary.builder.build_call(
            binary.module.get_function("seal_clear_storage").unwrap(),
            &[binary
                .builder
                .build_pointer_cast(
                    slot,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "",
        );

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(done_storage);
    }

    /// Read from substrate storage
    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();
        let ty_len = ty.size_of().const_cast(binary.context.i32_type(), false);
        binary.builder.build_store(scratch_len, ty_len);

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let entry = binary.builder.get_insert_block().unwrap();
        let retrieve_block = binary.context.append_basic_block(function, "in_storage");
        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        binary.builder.position_at_end(retrieve_block);

        let dest = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            ty.ptr_type(AddressSpace::Generic),
            "scratch_ty_buf",
        );

        let loaded_int = binary.builder.build_load(dest, "int");

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(done_storage);

        let res = binary.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[(&loaded_int, retrieve_block), (&ty.const_zero(), entry)]);

        res.as_basic_value().into_int_value()
    }

    /// Read string from substrate storage
    fn get_storage_string(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let ty = binary
            .module
            .get_struct_type("struct.vector")
            .unwrap()
            .ptr_type(AddressSpace::Generic);

        let entry = binary.builder.get_insert_block().unwrap();

        let retrieve_block = binary
            .context
            .append_basic_block(function, "retrieve_block");

        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        binary.builder.position_at_end(retrieve_block);

        let length = binary.builder.build_load(scratch_len, "string_len");

        let loaded_string = binary
            .builder
            .build_call(
                binary.module.get_function("vector_new").unwrap(),
                &[
                    length.into(),
                    binary.context.i32_type().const_int(1, false).into(),
                    scratch_buf.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(done_storage);

        let res = binary.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[
            (&loaded_string, retrieve_block),
            (
                &binary
                    .module
                    .get_struct_type("struct.vector")
                    .unwrap()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                entry,
            ),
        ]);

        res.as_basic_value().into_pointer_value()
    }

    /// Read string from substrate storage
    fn get_storage_address(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &ast::Namespace,
    ) -> ArrayValue<'a> {
        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(
                        binary.builder.build_pointer_cast(
                            scratch_buf,
                            binary.address_type(ns).ptr_type(AddressSpace::Generic),
                            "address_ptr",
                        ),
                        "address",
                    )
                    .into_array_value(),
                binary.address_type(ns).const_zero(),
                "retrieved_address",
            )
            .into_array_value()
    }

    /// Read string from substrate storage
    fn get_storage_bytes_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary.builder.build_load(scratch_len, "string_len"),
                binary.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let retrieve_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.scratch.unwrap().as_pointer_value(),
                &[binary.context.i32_type().const_zero(), index],
                "data_offset",
            )
        };

        binary.builder.build_load(offset, "value").into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        binary: &Binary,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
    ) {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary.builder.build_load(scratch_len, "string_len"),
                binary.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let retrieve_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.scratch.unwrap().as_pointer_value(),
                &[binary.context.i32_type().const_zero(), index],
                "data_offset",
            )
        };

        // set the result
        binary.builder.build_store(offset, val);

        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot_ptr,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                length.into(),
            ],
            "",
        );
    }

    /// Push a byte onto a bytes string in storage
    fn storage_push(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
        _ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let val = val.unwrap();

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        // Since we are going to add one byte, we set the buffer length to one less. This will
        // trap for us if it does not fit, so we don't have to code this ourselves
        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64 - 1, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary.builder.build_load(scratch_len, "string_len"),
                binary.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // set the result
        let offset = unsafe {
            binary.builder.build_gep(
                binary.scratch.unwrap().as_pointer_value(),
                &[binary.context.i32_type().const_zero(), length],
                "data_offset",
            )
        };

        binary.builder.build_store(offset, val);

        // Set the new length
        let length = binary.builder.build_int_add(
            length,
            binary.context.i32_type().const_int(1, false),
            "new_length",
        );

        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot_ptr,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                length.into(),
            ],
            "",
        );

        val
    }

    /// Pop a value from a bytes string
    fn storage_pop(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        _ty: &ast::Type,
        slot: IntValue<'a>,
        load: bool,
        _ns: &ast::Namespace,
    ) -> Option<BasicValueEnum<'a>> {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary.builder.build_load(scratch_len, "string_len"),
                binary.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range = binary.builder.build_int_compare(
            IntPredicate::NE,
            binary.context.i32_type().const_zero(),
            length,
            "index_in_range",
        );

        let retrieve_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(retrieve_block);

        // Set the new length
        let new_length = binary.builder.build_int_sub(
            length,
            binary.context.i32_type().const_int(1, false),
            "new_length",
        );

        let val = if load {
            let offset = unsafe {
                binary.builder.build_gep(
                    binary.scratch.unwrap().as_pointer_value(),
                    &[binary.context.i32_type().const_zero(), new_length],
                    "data_offset",
                )
            };

            Some(binary.builder.build_load(offset, "popped_value"))
        } else {
            None
        };

        binary.builder.build_call(
            binary.module.get_function("seal_set_storage").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot_ptr,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                new_length.into(),
            ],
            "",
        );

        val
    }

    /// Calculate length of storage dynamic bytes
    fn storage_array_length(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        _ty: &ast::Type,
        _ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("seal_get_storage").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            binary.context.i32_type().const_zero(),
            "storage_exists",
        );

        binary
            .builder
            .build_select(
                exists,
                binary.builder.build_load(scratch_len, "string_len"),
                binary.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value()
    }

    fn return_empty_abi(&self, binary: &Binary) {
        binary.builder.build_call(
            binary.module.get_function("seal_return").unwrap(),
            &[
                binary.context.i32_type().const_zero().into(),
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_zero()
                    .into(),
                binary.context.i32_type().const_zero().into(),
            ],
            "",
        );

        binary.builder.build_unreachable();
    }

    fn return_code<'b>(&self, binary: &'b Binary, _ret: IntValue<'b>) {
        // we can't return specific errors
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );
    }

    /// Call the  keccak256 host function
    fn keccak256_hash(
        &self,
        binary: &Binary,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
        _ns: &ast::Namespace,
    ) {
        binary.builder.build_call(
            binary.module.get_function("seal_hash_keccak_256").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        src,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "src",
                    )
                    .into(),
                length.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "dest",
                    )
                    .into(),
            ],
            "",
        );
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("seal_return").unwrap(),
            &[
                binary.context.i32_type().const_zero().into(),
                data.into(),
                length.into(),
            ],
            "",
        );

        binary.builder.build_unreachable();
    }

    fn assert_failure<'b>(&self, binary: &'b Binary, _data: PointerValue, _length: IntValue) {
        // insert "unreachable" instruction; not that build_unreachable() tells the compiler
        // that this code path is not reachable and may be discarded.
        let asm_fn = binary.context.void_type().fn_type(&[], false);

        let asm = binary.context.create_inline_asm(
            asm_fn,
            "unreachable".to_string(),
            "".to_string(),
            true,
            false,
            None,
            false,
        );

        let callable = CallableValue::try_from(asm).unwrap();

        binary.builder.build_call(callable, &[], "unreachable");

        binary.builder.build_unreachable();
    }

    fn abi_decode<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        datalength: IntValue<'b>,
        spec: &[ast::Parameter],
        ns: &ast::Namespace,
    ) {
        let mut argsdata = binary.builder.build_pointer_cast(
            data,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        let argsend = unsafe { binary.builder.build_gep(argsdata, &[datalength], "argsend") };

        for param in spec {
            args.push(self.decode_ty(binary, function, &param.ty, &mut argsdata, argsend, ns));
        }

        self.check_overrun(binary, function, argsdata, argsend, true);
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> PointerValue<'b> {
        // first calculate how much memory we need to allocate
        let mut length = binary.context.i32_type().const_zero();

        debug_assert_eq!(packed.len() + args.len(), tys.len());

        let mut tys_iter = tys.iter();

        // note that encoded_length return the exact value for packed encoding
        for arg in packed {
            let ty = tys_iter.next().unwrap();

            length = binary.builder.build_int_add(
                length,
                self.encoded_length(*arg, false, true, ty, function, binary, ns),
                "",
            );
        }

        for arg in args {
            let ty = tys_iter.next().unwrap();

            length = binary.builder.build_int_add(
                length,
                self.encoded_length(*arg, false, false, ty, function, binary, ns),
                "",
            );
        }

        let malloc_length = binary.builder.build_int_add(
            length,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .size_of()
                .unwrap()
                .const_cast(binary.context.i32_type(), false),
            "size",
        );

        let p = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let v = binary.builder.build_pointer_cast(
            p,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "string",
        );

        // if it's packed, we have the correct length already
        if args.is_empty() {
            let data_len = unsafe {
                binary.builder.build_gep(
                    v,
                    &[
                        binary.context.i32_type().const_zero(),
                        binary.context.i32_type().const_zero(),
                    ],
                    "data_len",
                )
            };

            binary.builder.build_store(data_len, length);
        }

        let data_size = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(1, false),
                ],
                "data_size",
            )
        };

        binary.builder.build_store(data_size, length);

        let data = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(2, false),
                ],
                "data",
            )
        };

        // now encode each of the arguments
        let data = binary.builder.build_pointer_cast(
            data,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        let mut argsdata = data;

        let mut tys_iter = tys.iter();

        for arg in packed {
            let ty = tys_iter.next().unwrap();

            self.encode_ty(binary, ns, false, true, function, ty, *arg, &mut argsdata);
        }

        for arg in args {
            let ty = tys_iter.next().unwrap();

            self.encode_ty(binary, ns, false, false, function, ty, *arg, &mut argsdata);
        }

        if !args.is_empty() {
            let length = binary.builder.build_int_sub(
                binary
                    .builder
                    .build_ptr_to_int(argsdata, binary.context.i32_type(), "end"),
                binary
                    .builder
                    .build_ptr_to_int(data, binary.context.i32_type(), "begin"),
                "datalength",
            );

            let data_len = unsafe {
                binary.builder.build_gep(
                    v,
                    &[
                        binary.context.i32_type().const_zero(),
                        binary.context.i32_type().const_zero(),
                    ],
                    "data_len",
                )
            };

            binary.builder.build_store(data_len, length);
        }

        v
    }

    ///  ABI encode the return values for the function
    fn abi_encode<'b>(
        &self,
        binary: &Binary<'b>,
        selector: Option<IntValue<'b>>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        // first calculate how much memory we need to allocate
        let mut length = binary.context.i32_type().const_zero();

        // note that encoded_length overestimates how data we need
        for (i, ty) in tys.iter().enumerate() {
            length = binary.builder.build_int_add(
                length,
                self.encoded_length(args[i], load, false, ty, function, binary, ns),
                "",
            );
        }

        if let Some(selector) = selector {
            length = binary.builder.build_int_add(
                length,
                selector
                    .get_type()
                    .size_of()
                    .const_cast(binary.context.i32_type(), false),
                "",
            );
        }

        let data = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // now encode each of the arguments
        let mut argsdata = data;

        if let Some(selector) = selector {
            binary.builder.build_store(
                binary.builder.build_pointer_cast(
                    data,
                    selector.get_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                selector,
            );

            argsdata = unsafe {
                binary.builder.build_gep(
                    data,
                    &[selector
                        .get_type()
                        .size_of()
                        .const_cast(binary.context.i32_type(), false)],
                    "",
                )
            };
        }

        for (i, ty) in tys.iter().enumerate() {
            self.encode_ty(
                binary,
                ns,
                load,
                false,
                function,
                ty,
                args[i],
                &mut argsdata,
            );
        }

        // we cannot use the length returned by encoded_length; calculate actual length
        let length = binary.builder.build_int_sub(
            binary
                .builder
                .build_ptr_to_int(argsdata, binary.context.i32_type(), "end"),
            binary
                .builder
                .build_ptr_to_int(data, binary.context.i32_type(), "begin"),
            "datalength",
        );

        (data, length)
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("seal_debug_message").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }

    fn create_contract<'b>(
        &mut self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
        _space: Option<IntValue<'b>>,
        ns: &ast::Namespace,
    ) {
        let created_contract = &ns.contracts[contract_no];

        let constructor = match constructor_no {
            Some(function_no) => &ns.functions[function_no],
            None => &created_contract.default_constructor.as_ref().unwrap().0,
        };

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        // salt
        let salt_buf =
            binary.build_alloca(function, binary.context.i8_type().array_type(32), "salt");
        let salt_buf = binary.builder.build_pointer_cast(
            salt_buf,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "salt_buf",
        );
        let salt_len = binary.context.i32_type().const_int(32, false);

        if let Some(salt) = salt {
            let salt_ty = ast::Type::Uint(256);

            binary.builder.build_store(
                binary.builder.build_pointer_cast(
                    salt_buf,
                    binary
                        .llvm_type(&salt_ty, ns)
                        .ptr_type(AddressSpace::Generic),
                    "salt",
                ),
                salt,
            );
        } else {
            let (ptr, len) = self.contract_unique_salt(binary, contract_no, ns);

            binary.builder.build_store(scratch_len, salt_len);

            binary.builder.build_call(
                binary.module.get_function("seal_random").unwrap(),
                &[ptr.into(), len.into(), salt_buf.into(), scratch_len.into()],
                "random",
            );
        }

        let tys: Vec<ast::Type> = constructor.params.iter().map(|p| p.ty.clone()).collect();

        // input
        let (input, input_len) = self.abi_encode(
            binary,
            Some(
                binary
                    .context
                    .i32_type()
                    .const_int(constructor.selector().to_be() as u64, false),
            ),
            false,
            function,
            args,
            &tys,
            ns,
        );

        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");

        // balance is a u128, make sure it's enough to cover existential_deposit
        if let Some(value) = value {
            binary.builder.build_store(value_ptr, value);
        } else {
            let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

            binary.builder.build_store(
                scratch_len,
                binary
                    .context
                    .i32_type()
                    .const_int(ns.value_length as u64, false),
            );

            binary.builder.build_call(
                binary.module.get_function("seal_minimum_balance").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_len.into(),
                ],
                "minimum_balance",
            );
        }

        assert!(!created_contract.code.is_empty());

        // code hash
        let codehash = binary.emit_global_string(
            &format!("binary_{}_codehash", created_contract.name),
            blake2_rfc::blake2b::blake2b(32, &[], &created_contract.code).as_bytes(),
            true,
        );

        let address_len_ptr = binary
            .builder
            .build_alloca(binary.context.i32_type(), "address_len_ptr");

        binary.builder.build_store(
            address_len_ptr,
            binary
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false),
        );

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let ret = binary
            .builder
            .build_call(
                binary.module.get_function("seal_instantiate").unwrap(),
                &[
                    codehash.into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false)
                        .into(),
                    gas.into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.value_length as u64, false)
                        .into(),
                    input.into(),
                    input_len.into(),
                    address.into(),
                    address_len_ptr.into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                    salt_buf.into(),
                    salt_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // return success or not in success variable; do not abort execution
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");

            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            self.assert_failure(
                binary,
                scratch_buf,
                binary
                    .builder
                    .build_load(scratch_len, "string_len")
                    .into_int_value(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    /// Call external binary
    fn external_call<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        _accounts: Option<(PointerValue<'b>, IntValue<'b>)>,
        _ty: ast::CallTy,
        ns: &ast::Namespace,
    ) {
        // balance is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");
        binary.builder.build_store(value_ptr, value);

        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        // do the actual call
        let ret = binary
            .builder
            .build_call(
                binary.module.get_function("seal_call").unwrap(),
                &[
                    address.unwrap().into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false)
                        .into(),
                    gas.into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.value_length as u64, false)
                        .into(),
                    payload.into(),
                    payload_len.into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // do not abort execution; return success or not in success variable
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");

            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            self.assert_failure(
                binary,
                scratch_buf,
                binary
                    .builder
                    .build_load(scratch_len, "string_len")
                    .into_int_value(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    /// Send value to address
    fn value_transfer<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        address: PointerValue<'b>,
        value: IntValue<'b>,
        ns: &ast::Namespace,
    ) {
        // balance is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");
        binary.builder.build_store(value_ptr, value);

        // do the actual call
        let ret = binary
            .builder
            .build_call(
                binary.module.get_function("seal_transfer").unwrap(),
                &[
                    address.into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false)
                        .into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.value_length as u64, false)
                        .into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // do not abort execution; return success or not in success variable
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");

            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            self.assert_failure(
                binary,
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                binary.context.i32_type().const_zero(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    fn return_data<'b>(&self, binary: &Binary<'b>, _function: FunctionValue) -> PointerValue<'b> {
        let scratch_buf = binary.builder.build_pointer_cast(
            binary.scratch.unwrap().as_pointer_value(),
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        let length = binary.builder.build_load(scratch_len, "string_len");

        binary
            .builder
            .build_call(
                binary.module.get_function("vector_new").unwrap(),
                &[
                    length.into(),
                    binary.context.i32_type().const_int(1, false).into(),
                    scratch_buf.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Substrate value is usually 128 bits
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &ast::Namespace) -> IntValue<'b> {
        let value = binary.builder.build_alloca(binary.value_type(ns), "value");

        let value_len = binary
            .builder
            .build_alloca(binary.context.i32_type(), "value_len");

        binary.builder.build_store(
            value_len,
            binary
                .context
                .i32_type()
                .const_int(ns.value_length as u64, false),
        );

        binary.builder.build_call(
            binary
                .module
                .get_function("seal_value_transferred")
                .unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        value,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                value_len.into(),
            ],
            "value_transferred",
        );

        binary
            .builder
            .build_load(value, "value_transferred")
            .into_int_value()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &ast::Namespace) {
        let address = binary
            .builder
            .build_alloca(binary.address_type(ns), "address");

        binary.builder.build_store(address, addr);

        binary.builder.build_call(
            binary.module.get_function("seal_terminate").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .context
                    .i32_type()
                    .const_int(ns.address_length as u64, false)
                    .into(),
            ],
            "terminated",
        );

        binary.builder.build_unreachable();
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        binary: &Binary<'b>,
        _function: FunctionValue<'b>,

        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("seal_hash_keccak_256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("seal_hash_sha2_256", 32),
            HashTy::Blake2_128 => ("seal_hash_blake2_128", 16),
            HashTy::Blake2_256 => ("seal_hash_blake2_256", 32),
        };

        let res = binary.builder.build_array_alloca(
            binary.context.i8_type(),
            binary.context.i32_type().const_int(hashlen, false),
            "res",
        );

        binary.builder.build_call(
            binary.module.get_function(fname).unwrap(),
            &[input.into(), input_len.into(), res.into()],
            "hash",
        );

        // bytes32 needs to reverse bytes
        let temp = binary.builder.build_alloca(
            binary.llvm_type(&ast::Type::Bytes(hashlen as u8), ns),
            "hash",
        );

        binary.builder.build_call(
            binary.module.get_function("__beNtoleN").unwrap(),
            &[
                res.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        temp,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        binary.builder.build_load(temp, "hash").into_int_value()
    }

    /// Substrate events should be prefixed with the index of the event in the metadata
    fn event_id<'b>(
        &self,
        binary: &Binary<'b>,
        contract: &ast::Contract,
        event_no: usize,
    ) -> Option<IntValue<'b>> {
        let event_id = contract
            .sends_events
            .iter()
            .position(|e| *e == event_no)
            .unwrap();

        Some(binary.context.i8_type().const_int(event_id as u64, false))
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        binary: &Binary<'b>,
        contract: &ast::Contract,
        function: FunctionValue<'b>,
        event_no: usize,
        data: &[BasicValueEnum<'b>],
        data_tys: &[ast::Type],
        topics: &[BasicValueEnum<'b>],
        topic_tys: &[ast::Type],
        ns: &ast::Namespace,
    ) {
        let topic_count = topics.len();
        let topic_size = binary.context.i32_type().const_int(
            if topic_count > 0 {
                32 * topic_count as u64 + 1
            } else {
                0
            },
            false,
        );

        let topic_buf = if topic_count > 0 {
            // the topic buffer is a vector of hashes.
            let topic_buf =
                binary
                    .builder
                    .build_array_alloca(binary.context.i8_type(), topic_size, "topic");

            // a vector with scale encoding first has the length. Since we will never have more than
            // 64 topics (we're limited to 4 at the moment), we can assume this is a single byte
            binary.builder.build_store(
                topic_buf,
                binary
                    .context
                    .i8_type()
                    .const_int(topic_count as u64 * 4, false),
            );

            let mut dest = unsafe {
                binary.builder.build_gep(
                    topic_buf,
                    &[binary.context.i32_type().const_int(1, false)],
                    "dest",
                )
            };

            binary.builder.build_call(
                binary.module.get_function("__bzero8").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            dest,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "dest",
                        )
                        .into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(topic_count as u64 * 4, false)
                        .into(),
                ],
                "",
            );

            for (i, topic) in topics.iter().enumerate() {
                let mut data = dest;
                self.encode_ty(
                    binary,
                    ns,
                    false,
                    true,
                    function,
                    &topic_tys[i],
                    *topic,
                    &mut data,
                );

                dest = unsafe {
                    binary.builder.build_gep(
                        dest,
                        &[binary.context.i32_type().const_int(32, false)],
                        "dest",
                    )
                };
            }

            topic_buf
        } else {
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null()
        };

        let (data_ptr, data_len) = self.abi_encode(
            binary,
            self.event_id(binary, contract, event_no),
            false,
            function,
            data,
            data_tys,
            ns,
        );

        binary.builder.build_call(
            binary.module.get_function("seal_deposit_event").unwrap(),
            &[
                topic_buf.into(),
                topic_size.into(),
                data_ptr.into(),
                data_len.into(),
            ],
            "",
        );
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        binary: &Binary<'b>,
        expr: &codegen::Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        macro_rules! get_seal_value {
            ($name:literal, $func:literal, $width:expr) => {{
                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int($width as u64 / 8, false),
                );

                binary.builder.build_call(
                    binary.module.get_function($func).unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    $name,
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary
                            .context
                            .custom_width_int_type($width)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    $name,
                )
            }};
        }

        match expr {
            codegen::Expression::Builtin(_, _, codegen::Builtin::Calldata, _) => {
                // allocate vector for input
                let v = binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            binary
                                .builder
                                .build_load(binary.calldata_len.as_pointer_value(), "calldata_len")
                                .into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            binary
                                .builder
                                .build_int_to_ptr(
                                    binary.context.i32_type().const_all_ones(),
                                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "no_initializer",
                                )
                                .into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let data = unsafe {
                    binary.builder.build_gep(
                        v.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(2, false),
                        ],
                        "",
                    )
                };

                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                // copy arguments from input buffer
                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int(SCRATCH_SIZE as u64, false),
                );

                // retrieve the data
                binary.builder.build_call(
                    binary.module.get_function("seal_input").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                data,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "data",
                            )
                            .into(),
                        scratch_len.into(),
                    ],
                    "",
                );

                v
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::BlockNumber, _) => {
                let block_number =
                    get_seal_value!("block_number", "seal_block_number", 32).into_int_value();

                // Cast to 64 bit
                binary
                    .builder
                    .build_int_z_extend_or_bit_cast(
                        block_number,
                        binary.context.i64_type(),
                        "block_number",
                    )
                    .into()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Timestamp, _) => {
                let milliseconds = get_seal_value!("timestamp", "seal_now", 64).into_int_value();

                // Solidity expects the timestamp in seconds, not milliseconds
                binary
                    .builder
                    .build_int_unsigned_div(
                        milliseconds,
                        binary.context.i64_type().const_int(1000, false),
                        "seconds",
                    )
                    .into()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Gasleft, _) => {
                get_seal_value!("gas_left", "seal_gas_left", 64)
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Gasprice, expr) => {
                // gasprice is available as "tx.gasprice" which will give you the price for one unit
                // of gas, or "tx.gasprice(uint64)" which will give you the price of N gas units
                let gas = if expr.is_empty() {
                    binary.context.i64_type().const_int(1, false)
                } else {
                    self.expression(binary, &expr[0], vartab, function, ns)
                        .into_int_value()
                };

                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.value_length as u64, false),
                );

                binary.builder.build_call(
                    binary.module.get_function("seal_weight_to_fee").unwrap(),
                    &[gas.into(), scratch_buf.into(), scratch_len.into()],
                    "gas_price",
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary
                            .context
                            .custom_width_int_type(ns.value_length as u32 * 8)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "price",
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Sender, _) => {
                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false),
                );

                binary.builder.build_call(
                    binary.module.get_function("seal_caller").unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    "caller",
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary.address_type(ns).ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "caller",
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Value, _) => {
                self.value_transferred(binary, ns).into()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::MinimumBalance, _) => {
                get_seal_value!(
                    "minimum_balance",
                    "seal_minimum_balance",
                    ns.value_length as u32 * 8
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::TombstoneDeposit, _) => {
                get_seal_value!(
                    "tombstone_deposit",
                    "seal_tombstone_deposit",
                    ns.value_length as u32 * 8
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Random, args) => {
                let subject = self
                    .expression(binary, &args[0], vartab, function, ns)
                    .into_pointer_value();

                let subject_data = unsafe {
                    binary.builder.build_gep(
                        subject,
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(2, false),
                        ],
                        "subject_data",
                    )
                };

                let subject_len = unsafe {
                    binary.builder.build_gep(
                        subject,
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_zero(),
                        ],
                        "subject_len",
                    )
                };

                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary
                    .builder
                    .build_store(scratch_len, binary.context.i32_type().const_int(32, false));

                binary.builder.build_call(
                    binary.module.get_function("seal_random").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                subject_data,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "subject_data",
                            )
                            .into(),
                        binary.builder.build_load(subject_len, "subject_len").into(),
                        scratch_buf.into(),
                        scratch_len.into(),
                    ],
                    "random",
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary
                            .context
                            .custom_width_int_type(256)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "hash",
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::GetAddress, _) => {
                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false),
                );

                binary.builder.build_call(
                    binary.module.get_function("seal_address").unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    "address",
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary.address_type(ns).ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "self_address",
                )
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Balance, _) => {
                let scratch_buf = binary.builder.build_pointer_cast(
                    binary.scratch.unwrap().as_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.value_length as u64, false),
                );

                binary.builder.build_call(
                    binary.module.get_function("seal_balance").unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    "balance",
                );

                binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        scratch_buf,
                        binary.value_type(ns).ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "balance",
                )
            }
            _ => unreachable!("{:?}", expr),
        }
    }
}
