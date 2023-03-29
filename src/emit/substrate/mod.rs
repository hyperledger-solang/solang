// SPDX-License-Identifier: Apache-2.0

use crate::{codegen::Options, sema::ast};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::BasicType;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use num_traits::ToPrimitive;
use solang_parser::pt;

use crate::emit::functions::{abort_if_value_transfer, emit_functions, emit_initializer};
use crate::emit::{Binary, TargetRuntime};

mod dispatch;
mod storage;
pub(super) mod target;

// When using the seal api, we use our own scratch buffer.
const SCRATCH_SIZE: u32 = 32 * 1024;

#[macro_export]
macro_rules! emit_context {
    ($binary:expr) => {
        #[allow(unused_macros)]
        macro_rules! byte_ptr {
            () => {
                $binary.context.i8_type().ptr_type(AddressSpace::default())
            };
        }

        #[allow(unused_macros)]
        macro_rules! i32_const {
            ($val:expr) => {
                $binary.context.i32_type().const_int($val, false)
            };
        }

        #[allow(unused_macros)]
        macro_rules! i32_zero {
            () => {
                $binary.context.i32_type().const_zero()
            };
        }

        #[allow(unused_macros)]
        macro_rules! call {
            ($name:expr, $args:expr) => {
                $binary
                    .builder
                    .build_call($binary.module.get_function($name).unwrap(), $args, "")
            };
            ($name:expr, $args:expr, $call_name:literal) => {
                $binary.builder.build_call(
                    $binary.module.get_function($name).unwrap(),
                    $args,
                    $call_name,
                )
            };
        }

        #[allow(unused_macros)]
        macro_rules! seal_get_storage {
            ($key_ptr:expr, $key_len:expr, $value_ptr:expr, $value_len:expr) => {
                call!(
                    "seal_get_storage",
                    &[$key_ptr, $key_len, $value_ptr, $value_len]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
            };
        }

        #[allow(unused_macros)]
        macro_rules! seal_set_storage {
            ($key_ptr:expr, $key_len:expr, $value_ptr:expr, $value_len:expr) => {
                call!(
                    "seal_set_storage",
                    &[$key_ptr, $key_len, $value_ptr, $value_len]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
            };
        }

        #[allow(unused_macros)]
        macro_rules! scratch_buf {
            () => {
                (
                    $binary.scratch.unwrap().as_pointer_value(),
                    $binary.scratch_len.unwrap().as_pointer_value(),
                )
            };
        }
    };
}

pub struct SubstrateTarget;

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        opt: &'a Options,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename.as_str(),
            opt,
            std_lib,
            None,
        );

        binary.set_early_value_aborts(contract, ns);

        let scratch_len = binary.module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "scratch_len",
        );
        scratch_len.set_linkage(Linkage::Internal);
        scratch_len.set_initializer(&context.i32_type().get_undef());

        binary.scratch_len = Some(scratch_len);

        let scratch = binary.module.add_global(
            context.i8_type().array_type(SCRATCH_SIZE),
            Some(AddressSpace::default()),
            "scratch",
        );
        scratch.set_linkage(Linkage::Internal);
        scratch.set_initializer(&context.i8_type().array_type(SCRATCH_SIZE).get_undef());
        binary.scratch = Some(scratch);

        let mut target = SubstrateTarget;

        target.declare_externals(&binary);

        emit_functions(&mut target, &mut binary, contract, ns);

        target.emit_deploy(&mut binary, contract, ns);
        target.emit_call(&binary, contract, ns);

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
            "instantiation_nonce",
            "seal_address",
            "seal_balance",
            "seal_block_number",
            "seal_now",
            "seal_gas_price",
            "seal_gas_left",
            "seal_caller",
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
        function_name: &str,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // after copying stratch, first thing to do is abort value transfers if constructors not payable
        if abort_value_transfers {
            abort_if_value_transfer(self, binary, function, ns, function_name);
        }

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "");

        let scratch_buf = binary.scratch.unwrap().as_pointer_value();
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

        let args_length =
            binary
                .builder
                .build_load(binary.context.i32_type(), scratch_len, "input_len");

        // store the length in case someone wants it via msg.data
        binary.builder.build_store(
            binary.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        (scratch_buf, args_length.into_int_value())
    }

    fn declare_externals(&self, binary: &Binary) {
        let ctx = binary.context;
        let u8_ptr = ctx.i8_type().ptr_type(AddressSpace::default()).into();
        let u32_val = ctx.i32_type().into();
        let u32_ptr = ctx.i32_type().ptr_type(AddressSpace::default()).into();
        let u64_val = ctx.i64_type().into();

        macro_rules! external {
            ($name:literal, $fn_type:ident, $( $args:expr ),*) => {
                binary.module.add_function(
                    $name,
                    ctx.$fn_type().fn_type(&[$($args),*], false),
                    Some(Linkage::External),
                );
            };
        }

        external!("seal_input", void_type, u8_ptr, u32_ptr);
        external!("seal_hash_keccak_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_sha2_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_blake2_128", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_blake2_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("instantiation_nonce", i64_type,);
        external!(
            "seal_set_storage",
            i32_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_val
        );
        external!("seal_debug_message", i32_type, u8_ptr, u32_val);
        external!("seal_clear_storage", i32_type, u8_ptr, u32_val);
        external!(
            "seal_get_storage",
            i32_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr
        );
        external!("seal_return", void_type, u32_val, u8_ptr, u32_val);
        external!(
            "seal_instantiate",
            i32_type,
            u8_ptr,
            u64_val,
            u8_ptr,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr,
            u8_ptr,
            u32_ptr,
            u8_ptr,
            u32_val
        );
        external!(
            "seal_call",
            i32_type,
            u32_val,
            u8_ptr,
            u64_val,
            u8_ptr,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr
        );
        external!("seal_transfer", i32_type, u8_ptr, u32_val, u8_ptr, u32_val);
        external!("seal_value_transferred", void_type, u8_ptr, u32_ptr);
        external!("seal_address", void_type, u8_ptr, u32_ptr);
        external!("seal_balance", void_type, u8_ptr, u32_ptr);
        external!("seal_minimum_balance", void_type, u8_ptr, u32_ptr);
        external!("seal_block_number", void_type, u8_ptr, u32_ptr);
        external!("seal_now", void_type, u8_ptr, u32_ptr);
        external!("seal_weight_to_fee", void_type, u64_val, u8_ptr, u32_ptr);
        external!("seal_gas_left", void_type, u8_ptr, u32_ptr);
        external!("seal_caller", void_type, u8_ptr, u32_ptr);
        external!("seal_terminate", void_type, u8_ptr);
        external!(
            "seal_deposit_event",
            void_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_val
        );
    }

    fn emit_deploy(&mut self, binary: &mut Binary, contract: &ast::Contract, ns: &ast::Namespace) {
        let initializer = emit_initializer(self, binary, contract, ns);

        // create deploy function
        let function = binary.module.add_function(
            "deploy",
            binary.context.void_type().fn_type(&[], false),
            None,
        );

        // deploy always receives an endowment so no value check here
        let (deploy_args, deploy_args_length) =
            self.public_function_prelude(binary, function, false, ns, "deploy");

        // init our storage vars
        binary.builder.build_call(initializer, &[], "");

        let dispatcher = binary.module.get_function("substrate_dispatch").unwrap();
        let args = vec![
            BasicMetadataValueEnum::PointerValue(deploy_args),
            BasicMetadataValueEnum::IntValue(deploy_args_length),
            BasicMetadataValueEnum::IntValue(self.value_transferred(binary, ns)),
        ];
        binary
            .builder
            .build_call(dispatcher, &args, "substrate_dispatcher");
        binary.builder.build_unreachable();
    }

    fn emit_call(&mut self, binary: &Binary, contract: &ast::Contract, ns: &ast::Namespace) {
        // create call function
        let function = binary.module.add_function(
            "call",
            binary.context.void_type().fn_type(&[], false),
            None,
        );

        let (contract_args, contract_args_length) = self.public_function_prelude(
            binary,
            function,
            binary.function_abort_value_transfers,
            ns,
            "call",
        );

        let dispatcher = binary.module.get_function("substrate_dispatch").unwrap();
        let args = vec![
            BasicMetadataValueEnum::PointerValue(contract_args),
            BasicMetadataValueEnum::IntValue(contract_args_length),
            BasicMetadataValueEnum::IntValue(self.value_transferred(binary, ns)),
        ];
        binary
            .builder
            .build_call(dispatcher, &args, "substrate_dispatcher");
        binary.builder.build_unreachable();
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
                    binary
                        .builder
                        .build_load(binary.context.i8_type(), src, "abi_bool")
                        .into_int_value(),
                    binary.context.i8_type().const_int(1, false),
                    "bool",
                );
                (val.into(), 1)
            }
            ast::Type::Uint(bits) | ast::Type::Int(bits) => {
                let int_type = binary.context.custom_width_int_type(*bits as u32);

                let val = binary.builder.build_load(int_type, src, "");

                // substrate only supports power-of-two types; step over the
                // the remainer

                // FIXME: we should do some type-checking here and ensure that the
                // encoded value fits into our smaller type
                let len = bits.next_power_of_two() as u64 / 8;

                (val, len)
            }
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                let val = binary.builder.build_load(binary.address_type(ns), src, "");

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
                        buf.into(),
                        binary
                            .context
                            .i32_type()
                            .const_int(*len as u64, false)
                            .into(),
                    ],
                    "",
                );

                (
                    binary
                        .builder
                        .build_load(int_type, buf, &format!("bytes{len}")),
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
                .ptr_type(AddressSpace::default())
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
                        binary.context.i8_type(),
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

                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            llvm_ty,
                            new,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    let val = self.decode_ty(binary, function, &field.ty, data, end, ns);

                    let val = if field.ty.deref_memory().is_fixed_reference_type(ns) {
                        let field_ty = binary.llvm_type(&field.ty, ns);
                        binary.builder.build_load(
                            field_ty,
                            val.into_pointer_value(),
                            field.name_as_str(),
                        )
                    } else {
                        val
                    };

                    binary.builder.build_store(elem, val);
                }

                new.into()
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
                                    llvm_ty,
                                    new,
                                    &[binary.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val = self.decode_ty(binary, function, &ty, data, end, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type(ns) {
                                let field_ty = binary.llvm_type(ty.deref_memory(), ns);
                                binary.builder.build_load(
                                    field_ty,
                                    val.into_pointer_value(),
                                    "elem",
                                )
                            } else {
                                val
                            };

                            binary.builder.build_store(elem, val);
                        },
                    );

                    new.into()
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

                    let len = binary
                        .builder
                        .build_load(binary.context.i32_type(), len, "array.len")
                        .into_int_value();

                    // details about our array elements
                    let elem_ty = binary.llvm_field_ty(&ty.array_elem(), ns);
                    let elem_size = elem_ty
                        .size_of()
                        .unwrap()
                        .const_cast(binary.context.i32_type(), false);

                    let init = binary.builder.build_int_to_ptr(
                        binary.context.i32_type().const_all_ones(),
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
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
                                    binary.context.get_struct_type("struct.vector").unwrap(),
                                    v,
                                    &[
                                        binary.context.i32_type().const_zero(),
                                        binary.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let ty = ty.array_deref();

                            let val = self.decode_ty(binary, function, &ty, data, end, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type(ns) {
                                let load_ty = binary.llvm_type(ty.deref_memory(), ns);
                                binary
                                    .builder
                                    .build_load(load_ty, val.into_pointer_value(), "elem")
                            } else {
                                val
                            };

                            binary.builder.build_store(element_start, val);
                        },
                    );
                    v.into()
                }
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let from = binary.builder.build_alloca(
                    binary.context.i8_type().ptr_type(AddressSpace::default()),
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

                *data = binary
                    .builder
                    .build_load(
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        from,
                        "data",
                    )
                    .into_pointer_value();

                self.check_overrun(binary, function, *data, end, false);

                v
            }
            ast::Type::Ref(ty) => self.decode_ty(binary, function, ty, data, end, ns),
            ast::Type::ExternalFunction { .. } => {
                let address =
                    self.decode_ty(binary, function, &ast::Type::Address(false), data, end, ns);
                let selector =
                    self.decode_ty(binary, function, &ast::Type::Bytes(4), data, end, ns);

                let ty = binary.llvm_type(ty, ns);

                let ef = binary
                    .builder
                    .build_call(
                        binary.module.get_function("__malloc").unwrap(),
                        &[ty.size_of()
                            .unwrap()
                            .const_cast(binary.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let address_member = unsafe {
                    binary.builder.build_gep(
                        ty,
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
                        ty,
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
                    let load_ty = binary.llvm_type(ty, ns);
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
                    let load_ty = binary.llvm_type(ty, ns);
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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

                binary.builder.build_store(dest, arg);

                power_of_two_len
            }
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(binary.address_type(ns), arg.into_pointer_value(), "")
                } else {
                    arg
                };

                binary.builder.build_store(dest, arg.into_array_value());

                ns.address_length as u64
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = binary
                        .builder
                        .build_alloca(arg.into_int_value().get_type(), &format!("bytes{n}"));

                    binary.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[
                        val.into(),
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
                        binary.context.i8_type(),
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
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
                                binary.llvm_type(ty, ns),
                                arg,
                                &[binary.context.i32_type().const_zero(), index],
                                "index_access",
                            )
                        };

                        self.encode_ty(
                            binary,
                            ns,
                            !elem_ty.is_fixed_reference_type(ns),
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
                    binary.context.i8_type().ptr_type(AddressSpace::default()),
                    "either_data",
                );

                either_data.add_incoming(&[(&normal_data, normal_array), (&null_data, null_array)]);

                *data = either_data.as_basic_value().into_pointer_value()
            }
            ast::Type::Array(..) => {
                let arg = if load {
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
                            !elem_ty.deref_any().is_fixed_reference_type(ns),
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
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(
                            load_ty,
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
                            binary.llvm_type(ty, ns),
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
                        !field.ty.is_fixed_reference_type(ns),
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
                    binary.context.i8_type().ptr_type(AddressSpace::default()),
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
                    !ty.is_fixed_reference_type(ns),
                    packed,
                    function,
                    ty,
                    arg,
                    data,
                );
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
                        &[(*data).into(), string_data.into(), string_len.into()],
                        "",
                    );

                    *data = unsafe {
                        binary
                            .builder
                            .build_gep(binary.context.i8_type(), *data, &[string_len], "")
                    };
                }
            }
            ast::Type::ExternalFunction { .. } => {
                let arg = if load {
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let address_member = unsafe {
                    binary.builder.build_gep(
                        binary.llvm_type(ty, ns),
                        arg.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(1, false),
                        ],
                        "address",
                    )
                };

                let address =
                    binary
                        .builder
                        .build_load(binary.address_type(ns), address_member, "address");

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
                        binary.llvm_type(ty, ns),
                        arg.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_zero(),
                        ],
                        "selector",
                    )
                };

                let selector = binary.builder.build_load(
                    binary.context.i32_type(),
                    selector_member,
                    "selector",
                );

                self.encode_ty(
                    binary,
                    ns,
                    false,
                    false,
                    function,
                    &ast::Type::Bytes(4),
                    selector,
                    data,
                );
            }
            ast::Type::FunctionSelector => self.encode_ty(
                binary,
                ns,
                load,
                packed,
                function,
                &ast::Type::Bytes(4),
                arg,
                data,
            ),
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
            ast::Type::FunctionSelector => binary
                .context
                .i32_type()
                .const_int(ns.target.selector_length() as u64, false),
            ast::Type::Address(_) | ast::Type::Contract(_) => binary
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false),
            ast::Type::Enum(n) => SubstrateTarget::encoded_length(
                arg,
                load,
                packed,
                &ns.enums[*n].ty,
                function,
                binary,
                ns,
            ),
            ast::Type::Struct(str_ty) => {
                let arg = if load {
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(
                            load_ty,
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
                            binary.llvm_type(ty, ns),
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
                        SubstrateTarget::encoded_length(
                            elem.into(),
                            !field.ty.is_fixed_reference_type(ns),
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
                        SubstrateTarget::encoded_length(
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
                        let load_ty = binary.llvm_var_ty(ty, ns);
                        binary
                            .builder
                            .build_load(load_ty, arg.into_pointer_value(), "")
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
                                    binary.llvm_type(ty, ns),
                                    arg,
                                    &[binary.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            *sum = binary.builder.build_int_add(
                                SubstrateTarget::encoded_length(
                                    elem.into(),
                                    !elem_ty.deref_memory().is_fixed_reference_type(ns),
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
                        SubstrateTarget::encoded_length(
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
                        SubstrateTarget::encoded_length(
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
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
                                    .size_of()
                                    .unwrap()
                                    .const_cast(binary.context.i32_type(), false),
                                "",
                            );

                            let p = unsafe {
                                binary.builder.build_gep(
                                    binary.llvm_type(ty, ns),
                                    arg.into_pointer_value(),
                                    &[
                                        binary.context.i32_type().const_zero(),
                                        binary.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "index_access",
                                )
                            };

                            *sum = binary.builder.build_int_add(
                                SubstrateTarget::encoded_length(
                                    p.into(),
                                    !elem_ty.deref_memory().is_fixed_reference_type(ns),
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
                            SubstrateTarget::encoded_length(
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
            ast::Type::Ref(r) => {
                SubstrateTarget::encoded_length(arg, load, packed, r, function, binary, ns)
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    let load_ty = binary.llvm_type(ty, ns).ptr_type(AddressSpace::default());
                    binary
                        .builder
                        .build_load(load_ty, arg.into_pointer_value(), "")
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
            ast::Type::UserType(user_type) => Self::encoded_length(
                arg,
                load,
                packed,
                &ns.user_types[*user_type].ty,
                function,
                binary,
                ns,
            ),
            _ => unreachable!(),
        }
    }
}

/// Print the return code of API calls to the debug buffer.
fn log_return_code(binary: &Binary, api: &'static str, code: IntValue) {
    if !binary.options.log_api_return_codes {
        return;
    }

    emit_context!(binary);

    let fmt = format!("call: {api}=");
    let msg = fmt.as_bytes();
    let delimiter = b",\n";
    let delimiter_length = delimiter.len();
    let length = i32_const!(msg.len() as u64 + 16 + delimiter_length as u64);
    let out_buf =
        binary
            .builder
            .build_array_alloca(binary.context.i8_type(), length, "seal_ret_code_buf");
    let mut out_buf_offset = out_buf;

    let msg_string = binary.emit_global_string(&fmt, msg, true);
    let msg_len = binary.context.i32_type().const_int(msg.len() as u64, false);
    call!(
        "__memcpy",
        &[out_buf_offset.into(), msg_string.into(), msg_len.into()]
    );
    out_buf_offset = unsafe {
        binary
            .builder
            .build_gep(binary.context.i8_type(), out_buf_offset, &[msg_len], "")
    };

    let code = binary
        .builder
        .build_int_z_extend(code, binary.context.i64_type(), "val_64bits");
    out_buf_offset = call!("uint2dec", &[out_buf_offset.into(), code.into()])
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let delimiter_string = binary.emit_global_string("delimiter", delimiter, true);
    let lim_len = binary
        .context
        .i32_type()
        .const_int(delimiter_length as u64, false);
    call!(
        "__memcpy",
        &[
            out_buf_offset.into(),
            delimiter_string.into(),
            lim_len.into()
        ]
    );
    out_buf_offset = unsafe {
        binary
            .builder
            .build_gep(binary.context.i8_type(), out_buf_offset, &[lim_len], "")
    };

    let msg_len = binary.builder.build_int_sub(
        binary
            .builder
            .build_ptr_to_int(out_buf_offset, binary.context.i32_type(), "out_buf_idx"),
        binary
            .builder
            .build_ptr_to_int(out_buf, binary.context.i32_type(), "out_buf_ptr"),
        "msg_len",
    );
    call!("seal_debug_message", &[out_buf.into(), msg_len.into()]);
}
