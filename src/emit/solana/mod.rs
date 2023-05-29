// SPDX-License-Identifier: Apache-2.0

pub(super) mod target;

use crate::sema::ast;
use crate::Target;
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
        let mut binary = Binary::new(
            context,
            Target::Solana,
            &contract.name,
            filename.as_str(),
            opt,
            std_lib,
            None,
        );

        binary
            .return_values
            .insert(ReturnCode::Success, context.i64_type().const_zero());
        binary.return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        binary.return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        binary.return_values.insert(
            ReturnCode::InvalidProgramId,
            context.i64_type().const_int(7u64 << 32, false),
        );
        binary.return_values.insert(
            ReturnCode::InvalidDataError,
            context.i64_type().const_int(2, false),
        );
        binary.return_values.insert(
            ReturnCode::AccountDataTooSmall,
            context.i64_type().const_int(5u64 << 32, false),
        );
        // externals
        target.declare_externals(&mut binary, ns);

        emit_functions(&mut target, &mut binary, contract, ns);

        binary.internalize(&[
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

        binary
    }

    fn declare_externals(&self, binary: &mut Binary, ns: &ast::Namespace) {
        let void_ty = binary.context.void_type();
        let u8_ptr = binary.context.i8_type().ptr_type(AddressSpace::default());
        let u64_ty = binary.context.i64_type();
        let u32_ty = binary.context.i32_type();
        let address = binary.address_type(ns).ptr_type(AddressSpace::default());
        let seeds = binary.llvm_type(
            &Type::Ref(Box::new(Type::Slice(Box::new(Type::Bytes(1))))),
            ns,
        );

        let sol_bytes = binary
            .context
            .struct_type(&[u8_ptr.into(), u64_ty.into()], false)
            .ptr_type(AddressSpace::default());

        let function = binary.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
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

        let function = binary.module.add_function(
            "sol_sha256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_keccak256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_set_return_data",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_get_return_data",
            u64_ty.fn_type(&[u8_ptr.into(), u64_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let fields = binary.context.opaque_struct_type("SolLogDataField");

        fields.set_body(&[u8_ptr.into(), u64_ty.into()], false);

        let function = binary.module.add_function(
            "sol_log_data",
            void_ty.fn_type(
                &[
                    fields.ptr_type(AddressSpace::default()).into(),
                    u64_ty.into(),
                ],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
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

        let function = binary.module.add_function(
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
    }

    /// Returns the SolAccountInfo of the executing binary
    fn contract_storage_account<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(binary);

        unsafe {
            binary.builder.build_gep(
                binary
                    .module
                    .get_struct_type("struct.SolParameters")
                    .unwrap(),
                parameters,
                &[
                    binary.context.i32_type().const_int(0, false),
                    binary.context.i32_type().const_int(0, false),
                    binary.context.i32_type().const_int(0, false),
                ],
                "account",
            )
        }
    }

    /// Get the pointer to SolParameters
    fn sol_parameters<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        binary
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value()
    }

    /// Returns the account data of the executing binary
    fn contract_storage_data<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(binary);

        binary
            .builder
            .build_load(
                binary.context.i8_type().ptr_type(AddressSpace::default()),
                unsafe {
                    binary.builder.build_gep(
                        binary
                            .module
                            .get_struct_type("struct.SolParameters")
                            .unwrap(),
                        parameters,
                        &[
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(3, false),
                        ],
                        "data",
                    )
                },
                "data",
            )
            .into_pointer_value()
    }

    /// Free binary storage and zero out
    fn storage_free<'b>(
        &self,
        binary: &Binary<'b>,
        ty: &ast::Type,
        data: PointerValue<'b>,
        slot: IntValue<'b>,
        function: FunctionValue<'b>,
        zero: bool,
        ns: &ast::Namespace,
    ) {
        if !zero && !ty.is_dynamic(ns) {
            // nothing to do
            return;
        }

        // the slot is simply the offset after the magic
        let member = unsafe {
            binary
                .builder
                .build_gep(binary.context.i8_type(), data, &[slot], "data")
        };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset = binary
                .builder
                .build_load(binary.context.i32_type(), member, "offset")
                .into_int_value();

            binary.builder.build_call(
                binary.module.get_function("account_data_free").unwrap(),
                &[data.into(), offset.into()],
                "",
            );

            // account_data_alloc will return 0 if the string is length 0
            let new_offset = binary.context.i32_type().const_zero();

            binary.builder.build_store(member, new_offset);
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // delete the existing storage
            let mut elem_slot = slot;
            let mut free_array = None;

            if elem_ty.is_dynamic(ns) || zero {
                let length = if let Some(ast::ArrayLength::Fixed(length)) = dim.last() {
                    binary
                        .context
                        .i32_type()
                        .const_int(length.to_u64().unwrap(), false)
                } else {
                    elem_slot = binary
                        .builder
                        .build_load(binary.context.i32_type(), member, "offset")
                        .into_int_value();

                    free_array = Some(elem_slot);

                    self.storage_array_length(binary, function, slot, elem_ty, ns)
                };

                let elem_size = elem_ty.solana_storage_size(ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(binary, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(binary, "offset", slot.get_type(), elem_slot.into());

                let _ = builder.over(binary, binary.context.i32_type().const_zero(), length);

                let offset_val = offset_phi.into_int_value();

                let elem_ty = ty.array_deref();

                self.storage_free(
                    binary,
                    elem_ty.deref_any(),
                    data,
                    offset_val,
                    function,
                    zero,
                    ns,
                );

                let offset_val = binary.builder.build_int_add(
                    offset_val,
                    binary.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(binary, "offset", offset_val.into());

                // done
                builder.finish(binary);
            }

            // if the array was dynamic, free the array itself
            if let Some(offset) = free_array {
                binary.builder.build_call(
                    binary.module.get_function("account_data_free").unwrap(),
                    &[data.into(), offset.into()],
                    "",
                );

                if zero {
                    // account_data_alloc will return 0 if the string is length 0
                    let new_offset = binary.context.i32_type().const_zero();

                    binary.builder.build_store(member, new_offset);
                }
            }
        } else if let ast::Type::Struct(struct_ty) = ty {
            for (i, field) in struct_ty.definition(ns).fields.iter().enumerate() {
                let field_offset = struct_ty.definition(ns).storage_offsets[i]
                    .to_u64()
                    .unwrap();

                let offset = binary.builder.build_int_add(
                    slot,
                    binary.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                self.storage_free(binary, &field.ty, data, offset, function, zero, ns);
            }
        } else if matches!(ty, Type::Address(_) | Type::Contract(_)) {
            let ty = binary.llvm_type(ty, ns);

            binary
                .builder
                .build_store(member, ty.into_array_type().const_zero());
        } else {
            let ty = binary.llvm_type(ty, ns);

            binary
                .builder
                .build_store(member, ty.into_int_type().const_zero());
        }
    }

    /// An entry in a sparse array or mapping
    fn sparse_entry<'b>(
        &self,
        binary: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> BasicTypeEnum<'b> {
        let key = if matches!(
            key_ty,
            ast::Type::String | ast::Type::DynamicBytes | ast::Type::Mapping(..)
        ) {
            binary.context.i32_type().into()
        } else {
            binary.llvm_type(key_ty, ns)
        };

        binary
            .context
            .struct_type(
                &[
                    key,                              // key
                    binary.context.i32_type().into(), // next field
                    if value_ty.is_mapping() {
                        binary.context.i32_type().into()
                    } else {
                        binary.llvm_type(value_ty, ns) // value
                    },
                ],
                false,
            )
            .into()
    }

    /// Generate sparse lookup
    fn sparse_lookup_function<'b>(
        &self,
        binary: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> FunctionValue<'b> {
        let function_name = format!(
            "sparse_lookup_{}_{}",
            key_ty.to_llvm_string(ns),
            value_ty.to_llvm_string(ns)
        );

        if let Some(function) = binary.module.get_function(&function_name) {
            return function;
        }

        // The function takes an offset (of the mapping or sparse array), the key which
        // is the index, and it should return an offset.
        let function_ty = binary.function_type(
            &[ast::Type::Uint(32), key_ty.clone()],
            &[ast::Type::Uint(32)],
            ns,
        );

        let function =
            binary
                .module
                .add_function(&function_name, function_ty, Some(Linkage::Internal));

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        let offset = function.get_nth_param(0).unwrap().into_int_value();
        let key = function.get_nth_param(1).unwrap();

        let entry_ty = self.sparse_entry(binary, key_ty, value_ty, ns);
        let value_offset = unsafe {
            entry_ty
                .ptr_type(AddressSpace::default())
                .const_null()
                .const_gep(
                    entry_ty.as_basic_type_enum(),
                    &[
                        binary.context.i32_type().const_zero(),
                        binary.context.i32_type().const_int(2, false),
                    ],
                )
                .const_to_int(binary.context.i32_type())
        };

        let data = self.contract_storage_data(binary);

        let member = unsafe {
            binary
                .builder
                .build_gep(binary.context.i8_type(), data, &[offset], "data")
        };

        let address = binary.build_alloca(function, binary.address_type(ns), "address");

        // calculate the correct bucket. We have an prime number of
        let bucket = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            binary
                .builder
                .build_call(
                    binary.module.get_function("vector_hash").unwrap(),
                    &[key.into()],
                    "hash",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if matches!(key_ty, ast::Type::Contract(_) | ast::Type::Address(_)) {
            binary.builder.build_store(address, key);

            binary
                .builder
                .build_call(
                    binary.module.get_function("address_hash").unwrap(),
                    &[address.into()],
                    "hash",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if key_ty.bits(ns) > 64 {
            binary
                .builder
                .build_int_truncate(key.into_int_value(), binary.context.i64_type(), "")
        } else {
            key.into_int_value()
        };

        let bucket = binary.builder.build_int_unsigned_rem(
            bucket,
            bucket
                .get_type()
                .const_int(crate::sema::SOLANA_BUCKET_SIZE, false),
            "",
        );

        let first_offset_ptr = unsafe {
            binary
                .builder
                .build_gep(binary.context.i32_type(), member, &[bucket], "bucket_list")
        };

        // we should now loop until offset is zero or we found it
        let loop_entry = binary.context.append_basic_block(function, "loop_entry");
        let end_of_bucket = binary.context.append_basic_block(function, "end_of_bucket");
        let examine_bucket = binary
            .context
            .append_basic_block(function, "examine_bucket");
        let found_entry = binary.context.append_basic_block(function, "found_entry");
        let next_entry = binary.context.append_basic_block(function, "next_entry");

        // let's enter the loop
        binary.builder.build_unconditional_branch(loop_entry);

        binary.builder.position_at_end(loop_entry);

        // we are walking the bucket list via the offset ptr
        let offset_ptr_phi = binary.builder.build_phi(
            binary.context.i32_type().ptr_type(AddressSpace::default()),
            "offset_ptr",
        );

        offset_ptr_phi.add_incoming(&[(&first_offset_ptr, entry)]);

        // load the offset and check for zero (end of bucket list)
        let offset = binary
            .builder
            .build_load(
                binary.context.i32_type(),
                offset_ptr_phi.as_basic_value().into_pointer_value(),
                "offset",
            )
            .into_int_value();

        let is_offset_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            offset,
            offset.get_type().const_zero(),
            "offset_is_zero",
        );

        binary
            .builder
            .build_conditional_branch(is_offset_zero, end_of_bucket, examine_bucket);

        binary.builder.position_at_end(examine_bucket);

        // let's compare the key in this entry to the key we are looking for
        let member = unsafe {
            binary
                .builder
                .build_gep(binary.context.i8_type(), data, &[offset], "data")
        };

        let ptr = unsafe {
            binary.builder.build_gep(
                entry_ty,
                member,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_zero(),
                ],
                "key_ptr",
            )
        };

        let matches = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let entry_key = binary
                .builder
                .build_load(binary.context.i32_type(), ptr, "key");

            // entry_key is an offset
            let entry_data = unsafe {
                binary.builder.build_gep(
                    binary.context.i8_type(),
                    data,
                    &[entry_key.into_int_value()],
                    "data",
                )
            };
            let entry_length = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_len").unwrap(),
                    &[data.into(), entry_key.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            binary
                .builder
                .build_call(
                    binary.module.get_function("__memcmp").unwrap(),
                    &[
                        entry_data.into(),
                        entry_length.into(),
                        binary.vector_bytes(key).into(),
                        binary.vector_len(key).into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if matches!(key_ty, ast::Type::Address(_) | ast::Type::Contract(_)) {
            binary
                .builder
                .build_call(
                    binary.module.get_function("address_equal").unwrap(),
                    &[address.into(), ptr.into()],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            let entry_key = binary
                .builder
                .build_load(binary.llvm_type(key_ty, ns), ptr, "key");

            binary.builder.build_int_compare(
                IntPredicate::EQ,
                key.into_int_value(),
                entry_key.into_int_value(),
                "matches",
            )
        };

        binary
            .builder
            .build_conditional_branch(matches, found_entry, next_entry);

        binary.builder.position_at_end(found_entry);

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        binary.builder.build_store(
            ret_offset,
            binary
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_zero()));

        binary.builder.position_at_end(next_entry);

        let offset_ptr = binary
            .builder
            .build_struct_gep(entry_ty, member, 1, "offset_ptr")
            .unwrap();

        offset_ptr_phi.add_incoming(&[(&offset_ptr, next_entry)]);

        binary.builder.build_unconditional_branch(loop_entry);

        let offset_ptr = offset_ptr_phi.as_basic_value().into_pointer_value();

        binary.builder.position_at_end(end_of_bucket);

        let entry_length = entry_ty
            .size_of()
            .unwrap()
            .const_cast(binary.context.i32_type(), false);

        let account = self.contract_storage_account(binary);

        // account_data_alloc will return offset = 0 if the string is length 0
        let rc = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_alloc").unwrap(),
                &[account.into(), entry_length.into(), offset_ptr.into()],
                "rc",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            binary.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = binary.context.append_basic_block(function, "rc_zero");

        binary
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        binary.builder.position_at_end(rc_not_zero);

        self.return_code(binary, rc);

        binary.builder.position_at_end(rc_zero);

        let offset = binary
            .builder
            .build_load(binary.context.i32_type(), offset_ptr, "new_offset")
            .into_int_value();

        let member = unsafe {
            binary
                .builder
                .build_gep(binary.context.i8_type(), data, &[offset], "data")
        };

        // Clear memory. The length argument to __bzero8 is in lengths of 8 bytes. We round up to the nearest
        // 8 byte, since account_data_alloc also rounds up to the nearest 8 byte when allocating.
        let length = binary.builder.build_int_unsigned_div(
            binary.builder.build_int_add(
                entry_length,
                binary.context.i32_type().const_int(7, false),
                "",
            ),
            binary.context.i32_type().const_int(8, false),
            "length_div_8",
        );

        binary.builder.build_call(
            binary.module.get_function("__bzero8").unwrap(),
            &[member.into(), length.into()],
            "zeroed",
        );

        // set key
        if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let new_string_length = binary.vector_len(key);
            let offset_ptr = binary
                .builder
                .build_struct_gep(entry_ty, member, 0, "key_ptr")
                .unwrap();

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = binary.builder.build_int_compare(
                IntPredicate::EQ,
                rc,
                binary.context.i64_type().const_zero(),
                "is_rc_zero",
            );

            let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = binary.context.append_basic_block(function, "rc_zero");
            let memcpy = binary.context.append_basic_block(function, "memcpy");

            binary
                .builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

            binary.builder.position_at_end(rc_not_zero);

            self.return_code(
                binary,
                binary.context.i64_type().const_int(5u64 << 32, false),
            );

            binary.builder.position_at_end(rc_zero);

            let new_offset =
                binary
                    .builder
                    .build_load(binary.context.i32_type(), offset_ptr, "new_offset");

            binary.builder.build_unconditional_branch(memcpy);

            binary.builder.position_at_end(memcpy);

            let offset_phi = binary
                .builder
                .build_phi(binary.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                binary.builder.build_gep(
                    binary.context.i8_type(),
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            binary.builder.build_call(
                binary.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    binary.vector_bytes(key).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else {
            let key_ptr = binary
                .builder
                .build_struct_gep(entry_ty, member, 0, "key_ptr")
                .unwrap();

            binary.builder.build_store(key_ptr, key);
        };

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        binary.builder.build_store(
            ret_offset,
            binary
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_zero()));

        function
    }

    /// Do a lookup/subscript in a sparse array or mapping; this will call a function
    fn sparse_lookup<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        slot: IntValue<'b>,
        index: BasicValueEnum<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let offset = binary.build_alloca(function, binary.context.i32_type(), "offset");

        let current_block = binary.builder.get_insert_block().unwrap();

        let lookup = self.sparse_lookup_function(binary, key_ty, value_ty, ns);

        binary.builder.position_at_end(current_block);

        let parameters = self.sol_parameters(binary);

        let rc = binary
            .builder
            .build_call(
                lookup,
                &[slot.into(), index.into(), offset.into(), parameters.into()],
                "mapping_lookup_res",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // either load the result from offset or return failure
        let is_rc_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            rc.get_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = binary.context.append_basic_block(function, "rc_zero");

        binary
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        binary.builder.position_at_end(rc_not_zero);

        self.return_code(binary, rc);

        binary.builder.position_at_end(rc_zero);

        binary
            .builder
            .build_load(binary.context.i32_type(), offset, "offset")
            .into_int_value()
    }

    /// AccountInfo struct member
    fn account_info_member<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        account_info: PointerValue<'b>,
        member: usize,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        let account_info_ty = binary
            .module
            .get_struct_type("struct.SolAccountInfo")
            .unwrap();

        let gep_no = match member.cmp(&2) {
            Ordering::Less => member as u32,
            Ordering::Greater => (member + 1) as u32,
            Ordering::Equal => {
                // The data field is transformed into a slice, so we do not return it directly.
                let data_len = binary
                    .builder
                    .build_load(
                        binary.context.i64_type(),
                        binary
                            .builder
                            .build_struct_gep(account_info_ty, account_info, 2, "data_len")
                            .unwrap(),
                        "data_len",
                    )
                    .into_int_value();

                let data = binary.builder.build_load(
                    binary.context.i8_type().ptr_type(AddressSpace::default()),
                    binary
                        .builder
                        .build_struct_gep(account_info_ty, account_info, 3, "data")
                        .unwrap(),
                    "data",
                );

                let slice_alloca = binary.build_alloca(
                    function,
                    binary.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1))), ns),
                    "slice_alloca",
                );
                let data_elem = binary
                    .builder
                    .build_struct_gep(
                        binary.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1))), ns),
                        slice_alloca,
                        0,
                        "data",
                    )
                    .unwrap();
                binary.builder.build_store(data_elem, data);
                let data_len_elem = binary
                    .builder
                    .build_struct_gep(
                        binary.llvm_type(&ast::Type::Slice(Box::new(Type::Bytes(1))), ns),
                        slice_alloca,
                        1,
                        "data_len",
                    )
                    .unwrap();
                binary.builder.build_store(data_len_elem, data_len);

                return slice_alloca.as_basic_value_enum();
            }
        };

        binary
            .builder
            .build_struct_gep(
                account_info_ty,
                account_info,
                gep_no,
                format!("AccountInfo_member_{}", member).as_str(),
            )
            .unwrap()
            .as_basic_value_enum()
    }

    /// Construct the LLVM-IR to call 'external_call' from solana.c
    fn build_external_call<'b>(
        &self,
        binary: &Binary,
        address: PointerValue<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        contract_args: ContractArgs<'b>,
        ns: &ast::Namespace,
    ) {
        let parameters = self.sol_parameters(binary);
        let external_call = binary.module.get_function("external_call").unwrap();

        let program_id = contract_args.program_id.unwrap_or_else(|| {
            binary
                .llvm_type(&Type::Address(false), ns)
                .ptr_type(AddressSpace::default())
                .const_null()
        });

        let (seeds, seeds_len) = contract_args
            .seeds
            .map(|(seeds, len)| {
                (
                    seeds,
                    binary.builder.build_int_cast(
                        len,
                        external_call.get_type().get_param_types()[5].into_int_type(),
                        "len",
                    ),
                )
            })
            .unwrap_or((
                external_call.get_type().get_param_types()[4]
                    .ptr_type(AddressSpace::default())
                    .const_null(),
                external_call.get_type().get_param_types()[5]
                    .into_int_type()
                    .const_zero(),
            ));

        binary.builder.build_call(
            external_call,
            &[
                payload.into(),
                payload_len.into(),
                address.into(),
                program_id.into(),
                seeds.into(),
                seeds_len.into(),
                parameters.into(),
            ],
            "",
        );
    }

    /// Construct the LLVM-IR to call 'sol_invoke_signed_c'.
    fn build_invoke_signed_c<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        contract_args: ContractArgs<'b>,
    ) {
        let instruction_ty: BasicTypeEnum = binary
            .module
            .get_struct_type("struct.SolInstruction")
            .unwrap()
            .into();

        let instruction = binary.build_alloca(function, instruction_ty, "instruction");

        binary.builder.build_store(
            binary
                .builder
                .build_struct_gep(instruction_ty, instruction, 0, "program_id")
                .unwrap(),
            contract_args.program_id.unwrap(),
        );

        binary.builder.build_store(
            binary
                .builder
                .build_struct_gep(instruction_ty, instruction, 1, "accounts")
                .unwrap(),
            contract_args.accounts.unwrap().0,
        );

        binary.builder.build_store(
            binary
                .builder
                .build_struct_gep(instruction_ty, instruction, 2, "accounts_len")
                .unwrap(),
            binary.builder.build_int_z_extend(
                contract_args.accounts.unwrap().1,
                binary.context.i64_type(),
                "accounts_len",
            ),
        );

        binary.builder.build_store(
            binary
                .builder
                .build_struct_gep(instruction_ty, instruction, 3, "data")
                .unwrap(),
            payload,
        );

        binary.builder.build_store(
            binary
                .builder
                .build_struct_gep(instruction_ty, instruction, 4, "data_len")
                .unwrap(),
            binary.builder.build_int_z_extend(
                payload_len,
                binary.context.i64_type(),
                "payload_len",
            ),
        );

        let parameters = self.sol_parameters(binary);

        let account_infos = binary
            .builder
            .build_struct_gep(
                binary
                    .module
                    .get_struct_type("struct.SolParameters")
                    .unwrap(),
                parameters,
                0,
                "ka",
            )
            .unwrap();

        let account_infos_len = binary.builder.build_int_truncate(
            binary
                .builder
                .build_load(
                    binary.context.i64_type(),
                    binary
                        .builder
                        .build_struct_gep(
                            binary
                                .module
                                .get_struct_type("struct.SolParameters")
                                .unwrap(),
                            parameters,
                            1,
                            "ka_num",
                        )
                        .unwrap(),
                    "ka_num",
                )
                .into_int_value(),
            binary.context.i32_type(),
            "ka_num",
        );

        let external_call = binary.module.get_function("sol_invoke_signed_c").unwrap();

        let (signer_seeds, signer_seeds_len) = if let Some((seeds, len)) = contract_args.seeds {
            (
                seeds,
                binary.builder.build_int_cast(
                    len,
                    external_call.get_type().get_param_types()[4].into_int_type(),
                    "len",
                ),
            )
        } else {
            (
                external_call.get_type().get_param_types()[3]
                    .const_zero()
                    .into_pointer_value(),
                external_call.get_type().get_param_types()[4]
                    .const_zero()
                    .into_int_value(),
            )
        };

        binary.builder.build_call(
            external_call,
            &[
                instruction.into(),
                account_infos.into(),
                account_infos_len.into(),
                signer_seeds.into(),
                signer_seeds_len.into(),
            ],
            "",
        );
    }
}
