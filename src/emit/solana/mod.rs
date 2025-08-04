// SPDX-License-Identifier: Apache-2.0

pub(super) mod target;

use crate::sema::ast;
use std::cmp::Ordering;

use crate::codegen::{cfg::ReturnCode, Options};
use crate::sema::ast::Type;
use inkwell::module::{Linkage, Module};
use inkwell::types::BasicType;
use inkwell::values::{
    BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress,
};
use inkwell::{context::Context, types::BasicTypeEnum};
use inkwell::{AddressSpace, IntPredicate};
use num_traits::ToPrimitive;

use crate::emit::functions::emit_functions;
use crate::emit::loop_builder::LoopBuilder;
use crate::emit::{Binary, ContractArgs, TargetRuntime};

pub struct SolanaTarget();

// Implement the Solana target which uses BPF
impl SolanaTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        opt: &'a Options,
    ) -> Binary<'a> {
        let mut target = SolanaTarget();
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut bin = Binary::new(
            context,
            ns,
            &contract.id.name,
            filename.as_str(),
            opt,
            std_lib,
            None,
        );

        bin.return_values
            .insert(ReturnCode::Success, context.i64_type().const_zero());
        bin.return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        bin.return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        bin.return_values.insert(
            ReturnCode::InvalidProgramId,
            context.i64_type().const_int(7u64 << 32, false),
        );
        bin.return_values.insert(
            ReturnCode::InvalidDataError,
            context.i64_type().const_int(2, false),
        );
        bin.return_values.insert(
            ReturnCode::AccountDataTooSmall,
            context.i64_type().const_int(5u64 << 32, false),
        );
        // externals
        target.declare_externals(&mut bin);

        emit_functions(&mut target, &mut bin, contract);

        bin.internalize(&[
            "entrypoint",
            "sol_log_",
            "sol_log_pubkey",
            "sol_invoke_signed_c",
            "sol_panic_",
            "sol_get_return_data",
            "sol_set_return_data",
            "sol_create_program_address",
            "sol_try_find_program_address",
            "sol_sha256",
            "sol_keccak256",
            "sol_log_data",
        ]);

        bin
    }

    fn declare_externals(&self, bin: &mut Binary) {
        let void_ty = bin.context.void_type();
        let u8_ptr = bin.context.ptr_type(AddressSpace::default());
        let u64_ty = bin.context.i64_type();
        let u32_ty = bin.context.i32_type();
        let address = bin.context.ptr_type(AddressSpace::default());
        let seeds = bin.llvm_type(&Type::Ref(Box::new(Type::Slice(Box::new(Type::Bytes(1))))));

        let sol_bytes = bin.context.ptr_type(AddressSpace::default());

        let function = bin.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_log_64_",
            void_ty.fn_type(
                &[
                    u64_ty.into(),
                    u64_ty.into(),
                    u64_ty.into(),
                    u64_ty.into(),
                    u64_ty.into(),
                ],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_sha256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_keccak256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_set_return_data",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_get_return_data",
            u64_ty.fn_type(&[u8_ptr.into(), u64_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let fields = bin.context.opaque_struct_type("SolLogDataField");

        fields.set_body(&[u8_ptr.into(), u64_ty.into()], false);

        let function = bin.module.add_function(
            "sol_log_data",
            void_ty.fn_type(
                &[
                    bin.context.ptr_type(AddressSpace::default()).into(),
                    u64_ty.into(),
                ],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_create_program_address",
            u64_ty.fn_type(
                &[seeds.into(), u64_ty.into(), address.into(), address.into()],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_try_find_program_address",
            u64_ty.fn_type(
                &[
                    seeds.into(),
                    u64_ty.into(),
                    address.into(),
                    address.into(),
                    u8_ptr.into(),
                ],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = bin.module.add_function(
            "sol_invoke_signed_c",
            u64_ty.fn_type(
                &[
                    u8_ptr.into(),
                    bin.context.ptr_type(AddressSpace::default()).into(),
                    bin.context.i32_type().into(),
                    bin.context.ptr_type(AddressSpace::default()).into(),
                    bin.context.i32_type().into(),
                ],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);
    }

    /// Returns the SolAccountInfo of the executing binary
    fn contract_storage_account<'b>(&self, bin: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(bin);

        unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("struct.SolParameters").unwrap(),
                    parameters,
                    &[
                        bin.context.i32_type().const_int(0, false),
                        bin.context.i32_type().const_int(0, false),
                        bin.context.i32_type().const_int(0, false),
                    ],
                    "account",
                )
                .unwrap()
        }
    }

    /// Get the pointer to SolParameters
    fn sol_parameters<'b>(&self, bin: &Binary<'b>) -> PointerValue<'b> {
        bin.builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value()
    }

    /// Returns the account data of the executing binary
    fn contract_storage_data<'b>(&self, bin: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(bin);

        bin.builder
            .build_load(
                bin.context.ptr_type(AddressSpace::default()),
                unsafe {
                    bin.builder
                        .build_gep(
                            bin.module.get_struct_type("struct.SolParameters").unwrap(),
                            parameters,
                            &[
                                bin.context.i32_type().const_int(0, false),
                                bin.context.i32_type().const_int(0, false),
                                bin.context.i32_type().const_int(0, false),
                                bin.context.i32_type().const_int(3, false),
                            ],
                            "data",
                        )
                        .unwrap()
                },
                "data",
            )
            .unwrap()
            .into_pointer_value()
    }

    /// Free binary storage and zero out
    fn storage_free<'b>(
        &self,
        bin: &Binary<'b>,
        ty: &ast::Type,
        data: PointerValue<'b>,
        slot: IntValue<'b>,
        function: FunctionValue<'b>,
        zero: bool,
    ) {
        if !zero && !ty.is_dynamic(bin.ns) {
            // nothing to do
            return;
        }

        // the slot is simply the offset after the magic
        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset = bin
                .builder
                .build_load(bin.context.i32_type(), member, "offset")
                .unwrap()
                .into_int_value();

            bin.builder
                .build_call(
                    bin.module.get_function("account_data_free").unwrap(),
                    &[data.into(), offset.into()],
                    "",
                )
                .unwrap();

            // account_data_alloc will return 0 if the string is length 0
            let new_offset = bin.context.i32_type().const_zero();

            bin.builder.build_store(member, new_offset).unwrap();
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // delete the existing storage
            let mut elem_slot = slot;
            let mut free_array = None;

            if elem_ty.is_dynamic(bin.ns) || zero {
                let length = if let Some(ast::ArrayLength::Fixed(length)) = dim.last() {
                    bin.context
                        .i32_type()
                        .const_int(length.to_u64().unwrap(), false)
                } else {
                    elem_slot = bin
                        .builder
                        .build_load(bin.context.i32_type(), member, "offset")
                        .unwrap()
                        .into_int_value();

                    free_array = Some(elem_slot);

                    self.storage_array_length(bin, function, slot, elem_ty)
                };

                let elem_size = elem_ty.solana_storage_size(bin.ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(bin, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(bin, "offset", slot.get_type(), elem_slot.into());

                let _ = builder.over(bin, bin.context.i32_type().const_zero(), length);

                let offset_val = offset_phi.into_int_value();

                let elem_ty = ty.array_deref();

                self.storage_free(bin, elem_ty.deref_any(), data, offset_val, function, zero);

                let offset_val = bin
                    .builder
                    .build_int_add(
                        offset_val,
                        bin.context.i32_type().const_int(elem_size, false),
                        "new_offset",
                    )
                    .unwrap();

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(bin, "offset", offset_val.into());

                // done
                builder.finish(bin);
            }

            // if the array was dynamic, free the array itself
            if let Some(offset) = free_array {
                bin.builder
                    .build_call(
                        bin.module.get_function("account_data_free").unwrap(),
                        &[data.into(), offset.into()],
                        "",
                    )
                    .unwrap();

                if zero {
                    // account_data_alloc will return 0 if the string is length 0
                    let new_offset = bin.context.i32_type().const_zero();

                    bin.builder.build_store(member, new_offset).unwrap();
                }
            }
        } else if let ast::Type::Struct(struct_ty) = ty {
            for (i, field) in struct_ty.definition(bin.ns).fields.iter().enumerate() {
                let field_offset = struct_ty.definition(bin.ns).storage_offsets[i]
                    .to_u64()
                    .unwrap();

                let offset = bin
                    .builder
                    .build_int_add(
                        slot,
                        bin.context.i32_type().const_int(field_offset, false),
                        "field_offset",
                    )
                    .unwrap();

                self.storage_free(bin, &field.ty, data, offset, function, zero);
            }
        } else if matches!(ty, Type::Address(_) | Type::Contract(_)) {
            let ty = bin.llvm_type(ty);

            bin.builder
                .build_store(member, ty.into_array_type().const_zero())
                .unwrap();
        } else {
            let ty = bin.llvm_type(ty);

            bin.builder
                .build_store(member, ty.into_int_type().const_zero())
                .unwrap();
        }
    }

    /// An entry in a sparse array or mapping
    fn sparse_entry<'b>(
        &self,
        bin: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
    ) -> BasicTypeEnum<'b> {
        let key = if matches!(
            key_ty,
            ast::Type::String | ast::Type::DynamicBytes | ast::Type::Mapping(..)
        ) {
            bin.context.i32_type().into()
        } else {
            bin.llvm_type(key_ty)
        };

        bin.context
            .struct_type(
                &[
                    key,                           // key
                    bin.context.i32_type().into(), // next field
                    if value_ty.is_mapping() {
                        bin.context.i32_type().into()
                    } else {
                        bin.llvm_type(value_ty) // value
                    },
                ],
                false,
            )
            .into()
    }

    /// Generate sparse lookup
    fn sparse_lookup_function<'b>(
        &self,
        bin: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
    ) -> FunctionValue<'b> {
        let function_name = format!(
            "sparse_lookup_{}_{}",
            key_ty.to_llvm_string(bin.ns),
            value_ty.to_llvm_string(bin.ns)
        );

        if let Some(function) = bin.module.get_function(&function_name) {
            return function;
        }

        // The function takes an offset (of the mapping or sparse array), the key which
        // is the index, and it should return an offset.
        let function_ty = bin.function_type(
            &[ast::Type::Uint(32), key_ty.clone()],
            &[ast::Type::Uint(32)],
        );

        let function =
            bin.module
                .add_function(&function_name, function_ty, Some(Linkage::Internal));

        let entry = bin.context.append_basic_block(function, "entry");

        bin.builder.position_at_end(entry);

        let offset = function.get_nth_param(0).unwrap().into_int_value();
        let key = function.get_nth_param(1).unwrap();

        let entry_ty = self.sparse_entry(bin, key_ty, value_ty);
        let value_offset = unsafe {
            bin.context
                .ptr_type(AddressSpace::default())
                .const_null()
                .const_gep(
                    entry_ty.as_basic_type_enum(),
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_int(2, false),
                    ],
                )
                .const_to_int(bin.context.i32_type())
        };

        let data = self.contract_storage_data(bin);

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[offset], "data")
        }
        .unwrap();

        let address = bin.build_alloca(function, bin.address_type(), "address");

        // calculate the correct bucket. We have an prime number of
        let bucket = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            bin.builder
                .build_call(
                    bin.module.get_function("vector_hash").unwrap(),
                    &[key.into()],
                    "hash",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if matches!(key_ty, ast::Type::Contract(_) | ast::Type::Address(_)) {
            bin.builder.build_store(address, key).unwrap();

            bin.builder
                .build_call(
                    bin.module.get_function("address_hash").unwrap(),
                    &[address.into()],
                    "hash",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if key_ty.bits(bin.ns) > 64 {
            bin.builder
                .build_int_truncate(key.into_int_value(), bin.context.i64_type(), "")
                .unwrap()
        } else {
            key.into_int_value()
        };

        let bucket = bin
            .builder
            .build_int_unsigned_rem(
                bucket,
                bucket
                    .get_type()
                    .const_int(crate::sema::SOLANA_BUCKET_SIZE, false),
                "",
            )
            .unwrap();

        let first_offset_ptr = unsafe {
            bin.builder
                .build_gep(bin.context.i32_type(), member, &[bucket], "bucket_list")
        }
        .unwrap();

        // we should now loop until offset is zero or we found it
        let loop_entry = bin.context.append_basic_block(function, "loop_entry");
        let end_of_bucket = bin.context.append_basic_block(function, "end_of_bucket");
        let examine_bucket = bin.context.append_basic_block(function, "examine_bucket");
        let found_entry = bin.context.append_basic_block(function, "found_entry");
        let next_entry = bin.context.append_basic_block(function, "next_entry");

        // let's enter the loop
        bin.builder.build_unconditional_branch(loop_entry).unwrap();

        bin.builder.position_at_end(loop_entry);

        // we are walking the bucket list via the offset ptr
        let offset_ptr_phi = bin
            .builder
            .build_phi(bin.context.ptr_type(AddressSpace::default()), "offset_ptr")
            .unwrap();

        offset_ptr_phi.add_incoming(&[(&first_offset_ptr, entry)]);

        // load the offset and check for zero (end of bucket list)
        let offset = bin
            .builder
            .build_load(
                bin.context.i32_type(),
                offset_ptr_phi.as_basic_value().into_pointer_value(),
                "offset",
            )
            .unwrap()
            .into_int_value();

        let is_offset_zero = bin
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                offset,
                offset.get_type().const_zero(),
                "offset_is_zero",
            )
            .unwrap();

        bin.builder
            .build_conditional_branch(is_offset_zero, end_of_bucket, examine_bucket)
            .unwrap();

        bin.builder.position_at_end(examine_bucket);

        // let's compare the key in this entry to the key we are looking for
        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[offset], "data")
        }
        .unwrap();

        let ptr = unsafe {
            bin.builder
                .build_gep(
                    entry_ty,
                    member,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_zero(),
                    ],
                    "key_ptr",
                )
                .unwrap()
        };

        let matches = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let entry_key = bin
                .builder
                .build_load(bin.context.i32_type(), ptr, "key")
                .unwrap();

            // entry_key is an offset
            let entry_data = unsafe {
                bin.builder
                    .build_gep(
                        bin.context.i8_type(),
                        data,
                        &[entry_key.into_int_value()],
                        "data",
                    )
                    .unwrap()
            };
            let entry_length = bin
                .builder
                .build_call(
                    bin.module.get_function("account_data_len").unwrap(),
                    &[data.into(), entry_key.into()],
                    "length",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            bin.builder
                .build_call(
                    bin.module.get_function("__memcmp").unwrap(),
                    &[
                        entry_data.into(),
                        entry_length.into(),
                        bin.vector_bytes(key).into(),
                        bin.vector_len(key).into(),
                    ],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if matches!(key_ty, ast::Type::Address(_) | ast::Type::Contract(_)) {
            bin.builder
                .build_call(
                    bin.module.get_function("address_equal").unwrap(),
                    &[address.into(), ptr.into()],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            let entry_key = bin
                .builder
                .build_load(bin.llvm_type(key_ty), ptr, "key")
                .unwrap();

            bin.builder
                .build_int_compare(
                    IntPredicate::EQ,
                    key.into_int_value(),
                    entry_key.into_int_value(),
                    "matches",
                )
                .unwrap()
        };

        bin.builder
            .build_conditional_branch(matches, found_entry, next_entry)
            .unwrap();

        bin.builder.position_at_end(found_entry);

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        bin.builder
            .build_store(
                ret_offset,
                bin.builder
                    .build_int_add(offset, value_offset, "value_offset")
                    .unwrap(),
            )
            .unwrap();

        bin.builder
            .build_return(Some(&bin.context.i64_type().const_zero()))
            .unwrap();

        bin.builder.position_at_end(next_entry);

        let offset_ptr = bin
            .builder
            .build_struct_gep(entry_ty, member, 1, "offset_ptr")
            .unwrap();

        offset_ptr_phi.add_incoming(&[(&offset_ptr, next_entry)]);

        bin.builder.build_unconditional_branch(loop_entry).unwrap();

        let offset_ptr = offset_ptr_phi.as_basic_value().into_pointer_value();

        bin.builder.position_at_end(end_of_bucket);

        let entry_length = entry_ty
            .size_of()
            .unwrap()
            .const_cast(bin.context.i32_type(), false);

        let account = self.contract_storage_account(bin);

        // account_data_alloc will return offset = 0 if the string is length 0
        let rc = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_alloc").unwrap(),
                &[account.into(), entry_length.into(), offset_ptr.into()],
                "rc",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = bin
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rc,
                bin.context.i64_type().const_zero(),
                "is_rc_zero",
            )
            .unwrap();

        let rc_not_zero = bin.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = bin.context.append_basic_block(function, "rc_zero");

        bin.builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero)
            .unwrap();

        bin.builder.position_at_end(rc_not_zero);

        self.return_code(bin, rc);

        bin.builder.position_at_end(rc_zero);

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), offset_ptr, "new_offset")
            .unwrap()
            .into_int_value();

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[offset], "data")
                .unwrap()
        };

        // Clear memory. The length argument to __bzero8 is in lengths of 8 bytes. We round up to the nearest
        // 8 byte, since account_data_alloc also rounds up to the nearest 8 byte when allocating.
        let length = bin
            .builder
            .build_int_unsigned_div(
                bin.builder
                    .build_int_add(entry_length, bin.context.i32_type().const_int(7, false), "")
                    .unwrap(),
                bin.context.i32_type().const_int(8, false),
                "length_div_8",
            )
            .unwrap();

        bin.builder
            .build_call(
                bin.module.get_function("__bzero8").unwrap(),
                &[member.into(), length.into()],
                "zeroed",
            )
            .unwrap();

        // set key
        if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let new_string_length = bin.vector_len(key);
            let offset_ptr = bin
                .builder
                .build_struct_gep(entry_ty, member, 0, "key_ptr")
                .unwrap();

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = bin
                .builder
                .build_call(
                    bin.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = bin
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rc,
                    bin.context.i64_type().const_zero(),
                    "is_rc_zero",
                )
                .unwrap();

            let rc_not_zero = bin.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = bin.context.append_basic_block(function, "rc_zero");
            let memcpy = bin.context.append_basic_block(function, "memcpy");

            bin.builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero)
                .unwrap();

            bin.builder.position_at_end(rc_not_zero);

            self.return_code(bin, bin.context.i64_type().const_int(5u64 << 32, false));

            bin.builder.position_at_end(rc_zero);

            let new_offset = bin
                .builder
                .build_load(bin.context.i32_type(), offset_ptr, "new_offset")
                .unwrap();

            bin.builder.build_unconditional_branch(memcpy).unwrap();

            bin.builder.position_at_end(memcpy);

            let offset_phi = bin
                .builder
                .build_phi(bin.context.i32_type(), "offset")
                .unwrap();

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                bin.builder
                    .build_gep(
                        bin.context.i8_type(),
                        data,
                        &[offset_phi.as_basic_value().into_int_value()],
                        "dest_string_data",
                    )
                    .unwrap()
            };

            bin.builder
                .build_call(
                    bin.module.get_function("__memcpy").unwrap(),
                    &[
                        dest_string_data.into(),
                        bin.vector_bytes(key).into(),
                        new_string_length.into(),
                    ],
                    "copied",
                )
                .unwrap();
        } else {
            let key_ptr = bin
                .builder
                .build_struct_gep(entry_ty, member, 0, "key_ptr")
                .unwrap();

            bin.builder.build_store(key_ptr, key).unwrap();
        };

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        bin.builder
            .build_store(
                ret_offset,
                bin.builder
                    .build_int_add(offset, value_offset, "value_offset")
                    .unwrap(),
            )
            .unwrap();

        bin.builder
            .build_return(Some(&bin.context.i64_type().const_zero()))
            .unwrap();

        function
    }

    /// Do a lookup/subscript in a sparse array or mapping; this will call a function
    fn sparse_lookup<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        slot: IntValue<'b>,
        index: BasicValueEnum<'b>,
    ) -> IntValue<'b> {
        let offset = bin.build_alloca(function, bin.context.i32_type(), "offset");

        let current_block = bin.builder.get_insert_block().unwrap();

        let lookup = self.sparse_lookup_function(bin, key_ty, value_ty);

        bin.builder.position_at_end(current_block);

        let parameters = self.sol_parameters(bin);

        let rc = bin
            .builder
            .build_call(
                lookup,
                &[slot.into(), index.into(), offset.into(), parameters.into()],
                "mapping_lookup_res",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // either load the result from offset or return failure
        let is_rc_zero = bin
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rc,
                rc.get_type().const_zero(),
                "is_rc_zero",
            )
            .unwrap();

        let rc_not_zero = bin.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = bin.context.append_basic_block(function, "rc_zero");

        bin.builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero)
            .unwrap();

        bin.builder.position_at_end(rc_not_zero);

        self.return_code(bin, rc);

        bin.builder.position_at_end(rc_zero);

        bin.builder
            .build_load(bin.context.i32_type(), offset, "offset")
            .unwrap()
            .into_int_value()
    }

    /// AccountInfo struct member
    fn account_info_member<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        account_info: PointerValue<'b>,
        member: usize,
    ) -> BasicValueEnum<'b> {
        let account_info_ty = bin.module.get_struct_type("struct.SolAccountInfo").unwrap();

        let gep_no = match member.cmp(&2) {
            Ordering::Less => member as u32,
            Ordering::Greater => (member + 1) as u32,
            Ordering::Equal => {
                // The data field is transformed into a slice, so we do not return it directly.
                let data_len = bin
                    .builder
                    .build_load(
                        bin.context.i64_type(),
                        bin.builder
                            .build_struct_gep(account_info_ty, account_info, 2, "data_len")
                            .unwrap(),
                        "data_len",
                    )
                    .unwrap()
                    .into_int_value();

                let data = bin
                    .builder
                    .build_load(
                        bin.context.ptr_type(AddressSpace::default()),
                        bin.builder
                            .build_struct_gep(account_info_ty, account_info, 3, "data")
                            .unwrap(),
                        "data",
                    )
                    .unwrap();

                let slice_alloca = bin.build_alloca(
                    function,
                    bin.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1)))),
                    "slice_alloca",
                );
                let data_elem = bin
                    .builder
                    .build_struct_gep(
                        bin.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1)))),
                        slice_alloca,
                        0,
                        "data",
                    )
                    .unwrap();
                bin.builder.build_store(data_elem, data).unwrap();
                let data_len_elem = bin
                    .builder
                    .build_struct_gep(
                        bin.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1)))),
                        slice_alloca,
                        1,
                        "data_len",
                    )
                    .unwrap();
                bin.builder.build_store(data_len_elem, data_len).unwrap();

                return slice_alloca.as_basic_value_enum();
            }
        };

        bin.builder
            .build_struct_gep(
                account_info_ty,
                account_info,
                gep_no,
                format!("AccountInfo_member_{member}").as_str(),
            )
            .unwrap()
            .as_basic_value_enum()
    }

    /// Construct the LLVM-IR to call 'sol_invoke_signed_c'.
    fn build_invoke_signed_c<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        contract_args: ContractArgs<'b>,
    ) {
        let instruction_ty: BasicTypeEnum = bin
            .context
            .struct_type(
                &[
                    bin.context
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum(),
                    bin.context
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum(),
                    bin.context.i64_type().as_basic_type_enum(),
                    bin.context
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum(),
                    bin.context.i64_type().as_basic_type_enum(),
                ],
                false,
            )
            .as_basic_type_enum();

        let instruction = bin.build_alloca(function, instruction_ty, "instruction");

        bin.builder
            .build_store(
                bin.builder
                    .build_struct_gep(instruction_ty, instruction, 0, "program_id")
                    .unwrap(),
                contract_args.program_id.unwrap(),
            )
            .unwrap();

        bin.builder
            .build_store(
                bin.builder
                    .build_struct_gep(instruction_ty, instruction, 1, "accounts")
                    .unwrap(),
                contract_args.accounts.unwrap().0,
            )
            .unwrap();

        bin.builder
            .build_store(
                bin.builder
                    .build_struct_gep(instruction_ty, instruction, 2, "accounts_len")
                    .unwrap(),
                bin.builder
                    .build_int_z_extend(
                        contract_args.accounts.unwrap().1,
                        bin.context.i64_type(),
                        "accounts_len",
                    )
                    .unwrap(),
            )
            .unwrap();

        bin.builder
            .build_store(
                bin.builder
                    .build_struct_gep(instruction_ty, instruction, 3, "data")
                    .unwrap(),
                payload,
            )
            .unwrap();

        bin.builder
            .build_store(
                bin.builder
                    .build_struct_gep(instruction_ty, instruction, 4, "data_len")
                    .unwrap(),
                bin.builder
                    .build_int_z_extend(payload_len, bin.context.i64_type(), "payload_len")
                    .unwrap(),
            )
            .unwrap();

        let parameters = self.sol_parameters(bin);

        let account_infos = bin
            .builder
            .build_struct_gep(
                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                parameters,
                0,
                "ka",
            )
            .unwrap();

        let account_infos_len = bin
            .builder
            .build_int_truncate(
                bin.builder
                    .build_load(
                        bin.context.i64_type(),
                        bin.builder
                            .build_struct_gep(
                                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                                parameters,
                                1,
                                "ka_num",
                            )
                            .unwrap(),
                        "ka_num",
                    )
                    .unwrap()
                    .into_int_value(),
                bin.context.i32_type(),
                "ka_num",
            )
            .unwrap();

        let external_call = bin.module.get_function("sol_invoke_signed_c").unwrap();

        let (signer_seeds, signer_seeds_len) = if let Some((seeds, len)) = contract_args.seeds {
            (
                seeds,
                bin.builder
                    .build_int_cast(
                        len,
                        external_call.get_type().get_param_types()[4].into_int_type(),
                        "len",
                    )
                    .unwrap(),
            )
        } else {
            (
                external_call.get_type().get_param_types()[3]
                    .into_pointer_type()
                    .const_zero(),
                external_call.get_type().get_param_types()[4]
                    .into_int_type()
                    .const_zero(),
            )
        };

        bin.builder
            .build_call(
                external_call,
                &[
                    instruction.into(),
                    account_infos.into(),
                    account_infos_len.into(),
                    signer_seeds.into(),
                    signer_seeds_len.into(),
                ],
                "",
            )
            .unwrap();
    }
}
