// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::emit::binary::Binary;
use crate::emit::expression::expression;
use crate::emit::loop_builder::LoopBuilder;
use crate::emit::solana::SolanaTarget;
use crate::emit::{ContractArgs, TargetRuntime, Variable};
use crate::sema::ast;
use inkwell::types::{BasicType, BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use num_traits::ToPrimitive;
use solang_parser::pt::{Loc, StorageType};
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for SolanaTarget {
    /// Solana does not use slot based-storage so override
    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        // binary storage is in 2nd account
        let data = self.contract_storage_data(bin);

        self.storage_free(bin, ty, data, *slot, function, true);
    }

    fn set_storage_extfunc(
        &self,
        _bin: &Binary,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
        _dest_ty: BasicTypeEnum,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        // unused
        unreachable!();
    }

    fn get_storage_string(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        // unused
        unreachable!();
    }

    fn get_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        loc: Loc,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(bin);

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), member, "offset")
            .unwrap()
            .into_int_value();

        let length = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = bin
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "index_in_range")
            .unwrap();

        let get_block = bin.context.append_basic_block(function, "in_range");
        let bang_block = bin.context.append_basic_block(function, "bang_block");

        bin.builder
            .build_conditional_branch(in_range, get_block, bang_block)
            .unwrap();

        bin.builder.position_at_end(bang_block);

        bin.log_runtime_error(
            self,
            "storage array index out of bounds".to_string(),
            Some(loc),
        );
        self.assert_failure(
            bin,
            bin.context.ptr_type(AddressSpace::default()).const_null(),
            bin.context.i32_type().const_zero(),
        );

        bin.builder.position_at_end(get_block);

        let offset = bin.builder.build_int_add(offset, index, "offset").unwrap();

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[offset], "data")
                .unwrap()
        };

        bin.builder
            .build_load(bin.context.i8_type(), member, "val")
            .unwrap()
            .into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        bin: &Binary,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
        loc: Loc,
    ) {
        let data = self.contract_storage_data(bin);

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), member, "offset")
            .unwrap()
            .into_int_value();

        let length = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = bin
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "index_in_range")
            .unwrap();

        let set_block = bin.context.append_basic_block(function, "in_range");
        let bang_block = bin.context.append_basic_block(function, "bang_block");

        bin.builder
            .build_conditional_branch(in_range, set_block, bang_block)
            .unwrap();

        bin.builder.position_at_end(bang_block);
        bin.log_runtime_error(self, "storage index out of bounds".to_string(), Some(loc));
        self.assert_failure(
            bin,
            bin.context.ptr_type(AddressSpace::default()).const_null(),
            bin.context.i32_type().const_zero(),
        );

        bin.builder.position_at_end(set_block);

        let offset = bin.builder.build_int_add(offset, index, "offset").unwrap();

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[offset], "data")
                .unwrap()
        };

        bin.builder.build_store(member, val).unwrap();
    }

    fn storage_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        let account = self.contract_storage_account(bin);

        if let ast::Type::Mapping(ast::Mapping { key, value, .. }) = ty.deref_any() {
            self.sparse_lookup(bin, function, key, value, slot, index)
        } else if ty.is_sparse_solana(bin.ns) {
            // sparse array
            let elem_ty = ty.storage_array_elem().deref_into();

            let key = ast::Type::Uint(256);

            self.sparse_lookup(bin, function, &key, &elem_ty, slot, index)
        } else {
            // 3rd member of account is data pointer
            let data = unsafe {
                bin.builder
                    .build_gep(
                        bin.module.get_struct_type("struct.SolAccountInfo").unwrap(),
                        account,
                        &[
                            bin.context.i32_type().const_zero(),
                            bin.context.i32_type().const_int(3, false),
                        ],
                        "data",
                    )
                    .unwrap()
            };

            let data = bin
                .builder
                .build_load(bin.context.ptr_type(AddressSpace::default()), data, "data")
                .unwrap()
                .into_pointer_value();

            let member = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), data, &[slot], "data")
                    .unwrap()
            };

            let offset = bin
                .builder
                .build_load(bin.context.i32_type(), member, "offset")
                .unwrap()
                .into_int_value();

            let elem_ty = ty.storage_array_elem().deref_into();

            let elem_size = bin
                .context
                .i32_type()
                .const_int(elem_ty.solana_storage_size(bin.ns).to_u64().unwrap(), false);

            bin.builder
                .build_int_add(
                    offset,
                    bin.builder
                        .build_int_mul(index.into_int_value(), elem_size, "")
                        .unwrap(),
                    "",
                )
                .unwrap()
        }
    }

    fn storage_push(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(bin);
        let account = self.contract_storage_account(bin);

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), member, "offset")
            .unwrap()
            .into_int_value();

        let length = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let member_size = bin
            .context
            .i32_type()
            .const_int(ty.storage_slots(bin.ns).to_u64().unwrap(), false);
        let new_length = bin
            .builder
            .build_int_add(length, member_size, "new_length")
            .unwrap();

        let rc = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_realloc").unwrap(),
                &[
                    account.into(),
                    offset.into(),
                    new_length.into(),
                    member.into(),
                ],
                "new_offset",
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

        self.return_code(bin, bin.context.i64_type().const_int(5u64 << 32, false));

        bin.builder.position_at_end(rc_zero);

        let mut new_offset = bin
            .builder
            .build_int_add(
                bin.builder
                    .build_load(bin.context.i32_type(), member, "offset")
                    .unwrap()
                    .into_int_value(),
                length,
                "",
            )
            .unwrap();

        if let Some(val) = val {
            self.storage_store(bin, ty, false, &mut new_offset, val, function, &None);
        }

        if ty.is_reference_type(bin.ns) {
            // Caller expects a reference to storage; note that storage_store() should not modify
            // new_offset even if the argument is mut
            new_offset.into()
        } else {
            val.unwrap()
        }
    }

    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        load: bool,
        loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        let data = self.contract_storage_data(bin);
        let account = self.contract_storage_account(bin);

        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), member, "offset")
            .unwrap()
            .into_int_value();

        let length = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = bin
            .builder
            .build_int_compare(
                IntPredicate::NE,
                bin.context.i32_type().const_zero(),
                length,
                "index_in_range",
            )
            .unwrap();

        let bang_block = bin.context.append_basic_block(function, "bang_block");
        let retrieve_block = bin.context.append_basic_block(function, "in_range");

        bin.builder
            .build_conditional_branch(in_range, retrieve_block, bang_block)
            .unwrap();

        bin.builder.position_at_end(bang_block);
        bin.log_runtime_error(self, "pop from empty storage array".to_string(), Some(loc));
        self.assert_failure(
            bin,
            bin.context.ptr_type(AddressSpace::default()).const_null(),
            bin.context.i32_type().const_zero(),
        );

        bin.builder.position_at_end(retrieve_block);

        let member_size = bin
            .context
            .i32_type()
            .const_int(ty.storage_slots(bin.ns).to_u64().unwrap(), false);

        let new_length = bin
            .builder
            .build_int_sub(length, member_size, "new_length")
            .unwrap();

        let mut old_elem_offset = bin.builder.build_int_add(offset, new_length, "").unwrap();

        let val = if load {
            Some(self.storage_load(bin, ty, &mut old_elem_offset, function, &None))
        } else {
            None
        };

        // delete existing storage -- pointers need to be freed
        self.storage_free(bin, ty, data, old_elem_offset, function, false);

        // we can assume pointer will stay the same after realloc to smaller size
        bin.builder
            .build_call(
                bin.module.get_function("account_data_realloc").unwrap(),
                &[
                    account.into(),
                    offset.into(),
                    new_length.into(),
                    member.into(),
                ],
                "new_offset",
            )
            .unwrap();

        val
    }

    fn storage_array_length(
        &self,
        bin: &Binary<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        elem_ty: &ast::Type,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(bin);

        // the slot is simply the offset after the magic
        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[slot], "data")
                .unwrap()
        };

        let offset = bin
            .builder
            .build_load(bin.context.i32_type(), member, "offset")
            .unwrap()
            .into_int_value();

        let member_size = bin
            .context
            .i32_type()
            .const_int(elem_ty.storage_slots(bin.ns).to_u64().unwrap(), false);

        let length_bytes = bin
            .builder
            .build_call(
                bin.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        bin.builder
            .build_int_unsigned_div(length_bytes, member_size, "")
            .unwrap()
    }

    fn get_storage_int(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ty: IntType<'a>,
    ) -> IntValue<'a> {
        // unused
        unreachable!();
    }

    /// Recursively load a type from binary storage. This overrides the default method
    /// in the trait, which is for chains with 256 bit storage keys.
    fn storage_load(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        _storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(bin);

        // the slot is simply the offset after the magic
        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[*slot], "data")
                .unwrap()
        };

        match ty {
            ast::Type::String | ast::Type::DynamicBytes => {
                let offset = bin
                    .builder
                    .build_load(bin.context.i32_type(), member, "offset")
                    .unwrap()
                    .into_int_value();

                let string_length = bin
                    .builder
                    .build_call(
                        bin.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "free",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let string_data = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), data, &[offset], "string_data")
                        .unwrap()
                };

                bin.builder
                    .build_call(
                        bin.module.get_function("vector_new").unwrap(),
                        &[
                            string_length.into(),
                            bin.context.i32_type().const_int(1, false).into(),
                            string_data.into(),
                        ],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Type::Struct(struct_ty) => {
                let llvm_ty = bin.llvm_type(ty.deref_any());
                // LLVMSizeOf() produces an i64
                let size = bin
                    .builder
                    .build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        bin.context.i32_type(),
                        "size_of",
                    )
                    .unwrap();

                let new = bin
                    .builder
                    .build_call(
                        bin.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                for (i, field) in struct_ty.definition(bin.ns).fields.iter().enumerate() {
                    let field_offset = struct_ty.definition(bin.ns).storage_offsets[i]
                        .to_u64()
                        .unwrap();

                    let mut offset = bin
                        .builder
                        .build_int_add(
                            *slot,
                            bin.context.i32_type().const_int(field_offset, false),
                            "field_offset",
                        )
                        .unwrap();

                    let val = self.storage_load(bin, &field.ty, &mut offset, function, &None);

                    let elem = unsafe {
                        bin.builder
                            .build_gep(
                                llvm_ty,
                                new,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_int(i as u64, false),
                                ],
                                field.name_as_str(),
                            )
                            .unwrap()
                    };

                    let val = if field.ty.is_fixed_reference_type(bin.ns) {
                        let load_ty = bin.llvm_type(&field.ty);
                        bin.builder
                            .build_load(load_ty, val.into_pointer_value(), "elem")
                            .unwrap()
                    } else {
                        val
                    };

                    bin.builder.build_store(elem, val).unwrap();
                }

                new.into()
            }
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = bin.llvm_type(ty.deref_any());

                let dest;
                let length;
                let mut slot = *slot;

                if matches!(dim.last().unwrap(), ast::ArrayLength::Fixed(_)) {
                    // LLVMSizeOf() produces an i64 and malloc takes i32
                    let size = bin
                        .builder
                        .build_int_truncate(
                            llvm_ty.size_of().unwrap(),
                            bin.context.i32_type(),
                            "size_of",
                        )
                        .unwrap();

                    dest = bin
                        .builder
                        .build_call(
                            bin.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    length = bin.context.i32_type().const_int(
                        if let Some(ast::ArrayLength::Fixed(d)) = dim.last() {
                            d.to_u64().unwrap()
                        } else {
                            unreachable!()
                        },
                        false,
                    );
                } else {
                    let llvm_elem_ty = bin.llvm_field_ty(elem_ty);
                    let elem_size = bin
                        .builder
                        .build_int_truncate(
                            llvm_elem_ty.size_of().unwrap(),
                            bin.context.i32_type(),
                            "size_of",
                        )
                        .unwrap();

                    length = self.storage_array_length(bin, function, slot, elem_ty);

                    slot = bin
                        .builder
                        .build_load(bin.context.i32_type(), member, "offset")
                        .unwrap()
                        .into_int_value();

                    dest = bin
                        .vector_new(length, elem_size, None, elem_ty)
                        .into_pointer_value();
                };

                let elem_size = elem_ty.solana_storage_size(bin.ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(bin, function);

                // we need a phi for the offset
                let offset_phi = builder.add_loop_phi(bin, "offset", slot.get_type(), slot.into());

                let index = builder.over(bin, bin.context.i32_type().const_zero(), length);

                let elem = bin.array_subscript(ty.deref_any(), dest, index);

                let elem_ty = ty.array_deref();

                let mut offset_val = offset_phi.into_int_value();

                let val = self.storage_load(
                    bin,
                    elem_ty.deref_memory(),
                    &mut offset_val,
                    function,
                    &None,
                );

                let val = if elem_ty.deref_memory().is_fixed_reference_type(bin.ns) {
                    let load_ty = bin.llvm_type(elem_ty.deref_any());
                    bin.builder
                        .build_load(load_ty, val.into_pointer_value(), "elem")
                        .unwrap()
                } else {
                    val
                };

                bin.builder.build_store(elem, val).unwrap();

                offset_val = bin
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

                dest.into()
            }
            _ => bin
                .builder
                .build_load(bin.llvm_var_ty(ty), member, "")
                .unwrap(),
        }
    }

    fn storage_store(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        offset: &mut IntValue<'a>,
        val: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        _: &Option<StorageType>,
    ) {
        let data = self.contract_storage_data(bin);
        let account = self.contract_storage_account(bin);

        // the slot is simply the offset after the magic
        let member = unsafe {
            bin.builder
                .build_gep(bin.context.i8_type(), data, &[*offset], "data")
                .unwrap()
        };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let new_string_length = bin.vector_len(val);

            let offset = if existing {
                let offset = bin
                    .builder
                    .build_load(bin.context.i32_type(), member, "offset")
                    .unwrap()
                    .into_int_value();

                // get the length of the existing string in storage
                let existing_string_length = bin
                    .builder
                    .build_call(
                        bin.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "length",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // do we need to reallocate?
                let allocation_necessary = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        existing_string_length,
                        new_string_length,
                        "allocation_necessary",
                    )
                    .unwrap();

                let entry = bin.builder.get_insert_block().unwrap();

                let realloc = bin.context.append_basic_block(function, "realloc");
                let memcpy = bin.context.append_basic_block(function, "memcpy");

                bin.builder
                    .build_conditional_branch(allocation_necessary, realloc, memcpy)
                    .unwrap();

                bin.builder.position_at_end(realloc);

                // do not realloc since we're copying everything
                bin.builder
                    .build_call(
                        bin.module.get_function("account_data_free").unwrap(),
                        &[data.into(), offset.into()],
                        "free",
                    )
                    .unwrap();

                // account_data_alloc will return offset = 0 if the string is length 0
                let rc = bin
                    .builder
                    .build_call(
                        bin.module.get_function("account_data_alloc").unwrap(),
                        &[account.into(), new_string_length.into(), member.into()],
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

                bin.builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero)
                    .unwrap();

                bin.builder.position_at_end(rc_not_zero);

                self.return_code(bin, bin.context.i64_type().const_int(5u64 << 32, false));

                bin.builder.position_at_end(rc_zero);

                let new_offset = bin
                    .builder
                    .build_load(bin.context.i32_type(), member, "new_offset")
                    .unwrap();

                bin.builder.build_unconditional_branch(memcpy).unwrap();

                bin.builder.position_at_end(memcpy);

                let offset_phi = bin
                    .builder
                    .build_phi(bin.context.i32_type(), "offset")
                    .unwrap();

                offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

                offset_phi.as_basic_value().into_int_value()
            } else {
                // account_data_alloc will return offset = 0 if the string is length 0
                let rc = bin
                    .builder
                    .build_call(
                        bin.module.get_function("account_data_alloc").unwrap(),
                        &[account.into(), new_string_length.into(), member.into()],
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

                bin.builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero)
                    .unwrap();

                bin.builder.position_at_end(rc_not_zero);

                self.return_code(bin, bin.context.i64_type().const_int(5u64 << 32, false));

                bin.builder.position_at_end(rc_zero);

                bin.builder
                    .build_load(bin.context.i32_type(), member, "new_offset")
                    .unwrap()
                    .into_int_value()
            };

            let dest_string_data = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), data, &[offset], "dest_string_data")
                    .unwrap()
            };

            bin.builder
                .build_call(
                    bin.module.get_function("__memcpy").unwrap(),
                    &[
                        dest_string_data.into(),
                        bin.vector_bytes(val).into(),
                        new_string_length.into(),
                    ],
                    "copied",
                )
                .unwrap();
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // make sure any pointers are freed
            if existing {
                self.storage_free(bin, ty, data, *offset, function, false);
            }

            let length = if let Some(ast::ArrayLength::Fixed(length)) = dim.last() {
                bin.context
                    .i32_type()
                    .const_int(length.to_u64().unwrap(), false)
            } else {
                bin.vector_len(val)
            };

            let mut elem_slot = *offset;

            if Some(&ast::ArrayLength::Dynamic) == dim.last() {
                // reallocate to the right size
                let member_size = bin
                    .context
                    .i32_type()
                    .const_int(elem_ty.solana_storage_size(bin.ns).to_u64().unwrap(), false);
                let new_length = bin
                    .builder
                    .build_int_mul(length, member_size, "new_length")
                    .unwrap();
                let offset = bin
                    .builder
                    .build_load(bin.context.i32_type(), member, "offset")
                    .unwrap()
                    .into_int_value();

                let rc = bin
                    .builder
                    .build_call(
                        bin.module.get_function("account_data_realloc").unwrap(),
                        &[
                            account.into(),
                            offset.into(),
                            new_length.into(),
                            member.into(),
                        ],
                        "new_offset",
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

                self.return_code(bin, bin.context.i64_type().const_int(5u64 << 32, false));

                bin.builder.position_at_end(rc_zero);

                elem_slot = bin
                    .builder
                    .build_load(bin.context.i32_type(), member, "offset")
                    .unwrap()
                    .into_int_value();
            }

            let elem_size = elem_ty.solana_storage_size(bin.ns).to_u64().unwrap();

            // loop over the array
            let mut builder = LoopBuilder::new(bin, function);

            // we need a phi for the offset
            let offset_phi =
                builder.add_loop_phi(bin, "offset", offset.get_type(), elem_slot.into());

            let index = builder.over(bin, bin.context.i32_type().const_zero(), length);

            let elem = bin.array_subscript(ty, val.into_pointer_value(), index);

            let mut offset_val = offset_phi.into_int_value();

            let elem_ty = ty.array_deref();

            self.storage_store(
                bin,
                elem_ty.deref_any(),
                false, // storage already freed with storage_free
                &mut offset_val,
                if elem_ty.deref_memory().is_fixed_reference_type(bin.ns) {
                    elem.into()
                } else {
                    let load_ty = if elem_ty.is_dynamic(bin.ns) {
                        bin.context
                            .ptr_type(AddressSpace::default())
                            .as_basic_type_enum()
                    } else {
                        bin.llvm_type(elem_ty.deref_memory())
                    };
                    bin.builder.build_load(load_ty, elem, "array_elem").unwrap()
                },
                function,
                &None,
            );

            offset_val = bin
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
        } else if let ast::Type::Struct(struct_ty) = ty {
            for (i, field) in struct_ty.definition(bin.ns).fields.iter().enumerate() {
                let field_offset = struct_ty.definition(bin.ns).storage_offsets[i]
                    .to_u64()
                    .unwrap();

                let mut offset = bin
                    .builder
                    .build_int_add(
                        *offset,
                        bin.context.i32_type().const_int(field_offset, false),
                        "field_offset",
                    )
                    .unwrap();

                let val_ty = bin.llvm_type(ty);
                let elem = unsafe {
                    bin.builder
                        .build_gep(
                            val_ty,
                            val.into_pointer_value(),
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                        .unwrap()
                };

                // free any existing dynamic storage
                if existing {
                    self.storage_free(bin, &field.ty, data, offset, function, false);
                }

                self.storage_store(
                    bin,
                    &field.ty,
                    existing,
                    &mut offset,
                    if field.ty.is_fixed_reference_type(bin.ns) {
                        elem.into()
                    } else {
                        let load_ty = if field.ty.is_dynamic(bin.ns) {
                            bin.context
                                .ptr_type(AddressSpace::default())
                                .as_basic_type_enum()
                        } else {
                            bin.llvm_type(&field.ty)
                        };
                        bin.builder
                            .build_load(load_ty, elem, field.name_as_str())
                            .unwrap()
                    },
                    function,
                    &None,
                );
            }
        } else {
            bin.builder.build_store(member, val).unwrap();
        }
    }

    fn keccak256_hash(
        &self,
        _bin: &Binary,
        _src: PointerValue,
        _length: IntValue,
        _dest: PointerValue,
    ) {
        unreachable!();
    }

    fn return_empty_abi(&self, bin: &Binary) {
        // return 0 for success
        bin.builder
            .build_return(Some(&bin.context.i64_type().const_int(0, false)))
            .unwrap();
    }

    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        // the reason code should be null (and already printed)
        bin.builder
            .build_call(
                bin.module.get_function("sol_set_return_data").unwrap(),
                &[
                    data.into(),
                    bin.builder
                        .build_int_z_extend(length, bin.context.i64_type(), "length")
                        .unwrap()
                        .into(),
                ],
                "",
            )
            .unwrap();

        // return 1 for failure
        bin.builder
            .build_return(Some(&bin.context.i64_type().const_int(1u64 << 32, false)))
            .unwrap();
    }

    fn print(&self, bin: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        let string_len64 = bin
            .builder
            .build_int_z_extend(string_len, bin.context.i64_type(), "")
            .unwrap();

        bin.builder
            .build_call(
                bin.module.get_function("sol_log_").unwrap(),
                &[string_ptr.into(), string_len64.into()],
                "",
            )
            .unwrap();
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        address: PointerValue<'b>,
        encoded_args: BasicValueEnum<'b>,
        encoded_args_len: BasicValueEnum<'b>,
        mut contract_args: ContractArgs<'b>,
        _loc: Loc,
    ) {
        contract_args.program_id = Some(address);

        let payload = bin.vector_bytes(encoded_args);
        let payload_len = encoded_args_len.into_int_value();

        assert!(contract_args.accounts.is_some());
        // The AccountMeta array is always present for Solana contracts
        self.build_invoke_signed_c(bin, function, payload, payload_len, contract_args);
    }

    fn builtin_function(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &ast::Function,
        args: &[BasicMetadataValueEnum<'a>],
        first_arg_type: Option<BasicTypeEnum>,
    ) -> Option<BasicValueEnum<'a>> {
        let first_arg_type =
            first_arg_type.expect("solana does not have builtin without any parameter");

        if builtin_func.id.name == "create_program_address" {
            let func = bin
                .module
                .get_function("sol_create_program_address")
                .unwrap();

            let seed_count = bin
                .context
                .i64_type()
                .const_int(first_arg_type.into_array_type().len() as u64, false);

            // address
            let address = bin.build_alloca(function, bin.address_type(), "address");

            bin.builder
                .build_store(address, args[1].into_array_value())
                .unwrap();

            let ret = bin
                .builder
                .build_call(
                    func,
                    &[
                        args[0].into_pointer_value().into(),
                        seed_count.into(),
                        address.into(),
                        args[2], // return value
                    ],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap();
            Some(ret)
        } else if builtin_func.id.name == "try_find_program_address" {
            let func = bin
                .module
                .get_function("sol_try_find_program_address")
                .unwrap();

            let seed_count = bin
                .context
                .i64_type()
                .const_int(first_arg_type.into_array_type().len() as u64, false);

            // address
            let address = bin.build_alloca(function, bin.address_type(), "address");

            bin.builder
                .build_store(address, args[1].into_array_value())
                .unwrap();

            let ret = bin
                .builder
                .build_call(
                    func,
                    &[
                        args[0].into_pointer_value().into(),
                        seed_count.into(),
                        address.into(),
                        args[2], // return address/pubkey
                        args[3], // return seed bump
                    ],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap();
            Some(ret)
        } else {
            unreachable!();
        }
    }

    /// Call external binary
    fn external_call<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        _success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<BasicValueEnum<'b>>,
        mut contract_args: ContractArgs<'b>,
        _ty: ast::CallTy,
        _loc: Loc,
    ) {
        let address = address.unwrap();

        if contract_args.accounts.is_none() {
            contract_args.accounts = Some((
                bin.context.ptr_type(AddressSpace::default()).const_zero(),
                bin.context.i32_type().const_zero(),
            ))
        };

        contract_args.program_id = Some(address.into_pointer_value());
        self.build_invoke_signed_c(bin, function, payload, payload_len, contract_args);
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        let null_u8_ptr = bin.context.ptr_type(AddressSpace::default()).const_zero();

        let length_as_64 = bin
            .builder
            .build_call(
                bin.module.get_function("sol_get_return_data").unwrap(),
                &[
                    null_u8_ptr.into(),
                    bin.context.i64_type().const_zero().into(),
                    null_u8_ptr.into(),
                ],
                "returndatasize",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let length = bin
            .builder
            .build_int_truncate(length_as_64, bin.context.i32_type(), "length")
            .unwrap();

        let malloc_length = bin
            .builder
            .build_int_add(
                length,
                bin.module
                    .get_struct_type("struct.vector")
                    .unwrap()
                    .size_of()
                    .unwrap()
                    .const_cast(bin.context.i32_type(), false),
                "size",
            )
            .unwrap();

        let p = bin
            .builder
            .build_call(
                bin.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let data_len = unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("struct.vector").unwrap(),
                    p,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_zero(),
                    ],
                    "data_len",
                )
                .unwrap()
        };

        bin.builder.build_store(data_len, length).unwrap();

        let data_size = unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("struct.vector").unwrap(),
                    p,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_int(1, false),
                    ],
                    "data_size",
                )
                .unwrap()
        };

        bin.builder.build_store(data_size, length).unwrap();

        let data = unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("struct.vector").unwrap(),
                    p,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_int(2, false),
                    ],
                    "data",
                )
                .unwrap()
        };

        let program_id = bin.build_array_alloca(
            function,
            bin.context.i8_type(),
            bin.context.i32_type().const_int(32, false),
            "program_id",
        );

        bin.builder
            .build_call(
                bin.module.get_function("sol_get_return_data").unwrap(),
                &[data.into(), length_as_64.into(), program_id.into()],
                "",
            )
            .unwrap();

        p
    }

    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>) {
        bin.builder.build_return(Some(&ret)).unwrap();
    }

    /// Value received is not available on solana
    fn value_transferred<'b>(&self, _binary: &Binary<'b>) -> IntValue<'b> {
        unreachable!();
    }

    /// Send value to address
    fn value_transfer<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
        _loc: Loc,
    ) {
        unreachable!();
    }

    /// Terminate execution, destroy binary and send remaining funds to addr
    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: ArrayValue<'b>) {
        unimplemented!();
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        _topics: &[BasicValueEnum<'b>],
    ) {
        let fields = bin.build_array_alloca(
            function,
            bin.module.get_struct_type("SolLogDataField").unwrap(),
            bin.context.i32_type().const_int(1, false),
            "fields",
        );

        let field_data = unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("SolLogDataField").unwrap(),
                    fields,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_zero(),
                    ],
                    "field_data",
                )
                .unwrap()
        };

        let bytes_pointer = bin.vector_bytes(data);
        bin.builder.build_store(field_data, bytes_pointer).unwrap();

        let field_len = unsafe {
            bin.builder
                .build_gep(
                    bin.module.get_struct_type("SolLogDataField").unwrap(),
                    fields,
                    &[
                        bin.context.i32_type().const_zero(),
                        bin.context.i32_type().const_int(1, false),
                    ],
                    "data_len",
                )
                .unwrap()
        };

        bin.builder
            .build_store(
                field_len,
                bin.builder
                    .build_int_z_extend(bin.vector_len(data), bin.context.i64_type(), "data_len64")
                    .unwrap(),
            )
            .unwrap();

        bin.builder
            .build_call(
                bin.module.get_function("sol_log_data").unwrap(),
                &[
                    fields.into(),
                    bin.context.i64_type().const_int(1, false).into(),
                ],
                "",
            )
            .unwrap();
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        bin: &Binary<'b>,
        expr: &codegen::Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        match expr {
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Timestamp,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let parameters = self.sol_parameters(bin);

                let sol_clock = bin.module.get_function("sol_clock").unwrap();

                let clock = bin
                    .builder
                    .build_call(sol_clock, &[parameters.into()], "clock")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // This is struct.clock_layout
                let clock_struct = bin
                    .context
                    .struct_type(&[bin.context.i64_type().as_basic_type_enum(); 5], false);
                let timestamp = bin
                    .builder
                    .build_struct_gep(clock_struct, clock, 4, "unix_timestamp")
                    .unwrap();

                bin.builder
                    .build_load(bin.context.i64_type(), timestamp, "timestamp")
                    .unwrap()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::BlockNumber | codegen::Builtin::Slot,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let parameters = self.sol_parameters(bin);

                let sol_clock = bin.module.get_function("sol_clock").unwrap();

                let clock = bin
                    .builder
                    .build_call(sol_clock, &[parameters.into()], "clock")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // This is struct.clock_layout
                let clock_struct = bin
                    .context
                    .struct_type(&[bin.context.i64_type().as_basic_type_enum(); 5], false);
                let slot = bin
                    .builder
                    .build_struct_gep(clock_struct, clock, 0, "slot")
                    .unwrap();

                bin.builder
                    .build_load(bin.context.i64_type(), slot, "timestamp")
                    .unwrap()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::GetAddress,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let parameters = self.sol_parameters(bin);

                bin.builder
                    .build_load(
                        bin.context.ptr_type(AddressSpace::default()),
                        bin.builder
                            .build_struct_gep(
                                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                                parameters,
                                4,
                                "program_id",
                            )
                            .unwrap(),
                        "program_id",
                    )
                    .unwrap()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Calldata,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let sol_params = self.sol_parameters(bin);

                let input = bin
                    .builder
                    .build_load(
                        bin.context.ptr_type(AddressSpace::default()),
                        bin.builder
                            .build_struct_gep(
                                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                                sol_params,
                                2,
                                "input",
                            )
                            .unwrap(),
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                let input_len = bin
                    .builder
                    .build_load(
                        bin.context.i64_type(),
                        bin.builder
                            .build_struct_gep(
                                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                                sol_params,
                                3,
                                "input_len",
                            )
                            .unwrap(),
                        "data_len",
                    )
                    .unwrap()
                    .into_int_value();

                let input_len = bin
                    .builder
                    .build_int_truncate(input_len, bin.context.i32_type(), "input_len")
                    .unwrap();

                bin.builder
                    .build_call(
                        bin.module.get_function("vector_new").unwrap(),
                        &[
                            input_len.into(),
                            bin.context.i32_type().const_int(1, false).into(),
                            input.into(),
                        ],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Signature,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let sol_params = self.sol_parameters(bin);

                let input = bin
                    .builder
                    .build_load(
                        bin.context.ptr_type(AddressSpace::default()),
                        bin.builder
                            .build_struct_gep(
                                bin.module.get_struct_type("struct.SolParameters").unwrap(),
                                sol_params,
                                2,
                                "input",
                            )
                            .unwrap(),
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                let selector = bin
                    .builder
                    .build_load(bin.context.i64_type(), input, "selector")
                    .unwrap();

                let bswap = bin.llvm_bswap(64);

                bin.builder
                    .build_call(bswap, &[selector.into()], "")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::SignatureVerify,
                args,
                ..
            } => {
                assert_eq!(args.len(), 3);

                let address = bin.build_alloca(function, bin.address_type(), "address");

                bin.builder
                    .build_store(
                        address,
                        expression(self, bin, &args[0], vartab, function).into_array_value(),
                    )
                    .unwrap();

                let message = expression(self, bin, &args[1], vartab, function);
                let signature = expression(self, bin, &args[2], vartab, function);
                let parameters = self.sol_parameters(bin);
                let signature_verify = bin.module.get_function("signature_verify").unwrap();

                let ret = bin
                    .builder
                    .build_call(
                        signature_verify,
                        &[
                            address.into(),
                            message.into(),
                            signature.into(),
                            parameters.into(),
                        ],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                bin.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret,
                        bin.context.i64_type().const_zero(),
                        "success",
                    )
                    .unwrap()
                    .into()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Accounts,
                args,
                ..
            } => {
                assert_eq!(args.len(), 0);

                let parameters = self.sol_parameters(bin);

                unsafe {
                    bin.builder.build_gep(
                        bin.module.get_struct_type("struct.SolParameters").unwrap(),
                        parameters,
                        &[
                            bin.context.i32_type().const_int(0, false),
                            bin.context.i32_type().const_int(0, false),
                            bin.context.i32_type().const_int(0, false),
                        ],
                        "accounts",
                    )
                }
                .unwrap()
                .into()
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::ArrayLength,
                args,
                ..
            } => {
                assert_eq!(args.len(), 1);

                let parameters = self.sol_parameters(bin);

                let ka_num = bin
                    .builder
                    .build_struct_gep(
                        bin.module.get_struct_type("struct.SolParameters").unwrap(),
                        parameters,
                        1,
                        "ka_num",
                    )
                    .unwrap();

                let ka_num = bin
                    .builder
                    .build_load(bin.context.i64_type(), ka_num, "ka_num")
                    .unwrap()
                    .into_int_value();

                bin.builder
                    .build_int_truncate(ka_num, bin.context.i32_type(), "ka_num_32bits")
                    .unwrap()
                    .into()
            }
            codegen::Expression::StructMember { expr, member, .. } => {
                let account_info =
                    expression(self, bin, expr, vartab, function).into_pointer_value();

                self.account_info_member(bin, function, account_info, *member)
            }
            _ => unimplemented!(),
        }
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
    ) -> IntValue<'b> {
        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("sol_keccak256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("sol_sha256", 32),
            _ => unreachable!(),
        };

        let res = bin.build_array_alloca(
            function,
            bin.context.i8_type(),
            bin.context.i32_type().const_int(hashlen, false),
            "res",
        );

        if hash == HashTy::Ripemd160 {
            bin.builder
                .build_call(
                    bin.module.get_function(fname).unwrap(),
                    &[input.into(), input_len.into(), res.into()],
                    "hash",
                )
                .unwrap();
        } else {
            let u64_ty = bin.context.i64_type();

            let sol_keccak256 = bin.module.get_function(fname).unwrap();

            // This is struct.SolBytes
            let sol_bytes = bin.context.struct_type(
                &[
                    bin.context
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum(),
                    bin.context.i64_type().as_basic_type_enum(),
                ],
                false,
            );

            let array = bin.build_alloca(function, sol_bytes, "sol_bytes");

            bin.builder
                .build_store(
                    bin.builder
                        .build_struct_gep(sol_bytes, array, 0, "input")
                        .unwrap(),
                    input,
                )
                .unwrap();

            bin.builder
                .build_store(
                    bin.builder
                        .build_struct_gep(sol_bytes, array, 1, "input_len")
                        .unwrap(),
                    bin.builder
                        .build_int_z_extend(input_len, u64_ty, "input_len")
                        .unwrap(),
                )
                .unwrap();

            bin.builder
                .build_call(
                    sol_keccak256,
                    &[
                        array.into(),
                        bin.context.i32_type().const_int(1, false).into(),
                        res.into(),
                    ],
                    "hash",
                )
                .unwrap();
        }

        // bytes32 needs to reverse bytes
        let temp = bin.build_alloca(
            function,
            bin.llvm_type(&ast::Type::Bytes(hashlen as u8)),
            "hash",
        );

        bin.builder
            .build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[
                    res.into(),
                    temp.into(),
                    bin.context.i32_type().const_int(hashlen, false).into(),
                ],
                "",
            )
            .unwrap();

        bin.builder
            .build_load(
                bin.llvm_type(&ast::Type::Bytes(hashlen as u8)),
                temp,
                "hash",
            )
            .unwrap()
            .into_int_value()
    }

    fn return_abi_data<'b>(
        &self,
        bin: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        bin.builder
            .build_call(
                bin.module.get_function("sol_set_return_data").unwrap(),
                &[data.into(), data_len.into()],
                "",
            )
            .unwrap();

        bin.builder
            .build_return(Some(&bin.return_values[&ReturnCode::Success]))
            .unwrap();
    }
}
