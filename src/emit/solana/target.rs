// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::emit::binary::Binary;
use crate::emit::expression::expression;
use crate::emit::loop_builder::LoopBuilder;
use crate::emit::solana::SolanaTarget;
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast;
use inkwell::types::{BasicType, BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use num_traits::ToPrimitive;
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for SolanaTarget {
    /// Solana does not use slot based-storage so override
    fn storage_delete(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) {
        // binary storage is in 2nd account
        let data = self.contract_storage_data(binary);

        self.storage_free(binary, ty, data, *slot, function, true, ns);
    }

    fn set_storage_extfunc(
        &self,
        _binary: &Binary,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        // unused
        unreachable!();
    }

    fn get_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        // unused
        unreachable!();
    }

    fn get_storage_bytes_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::default()),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let get_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, get_block, bang_block);

        binary.builder.position_at_end(bang_block);

        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(get_block);

        let offset = binary.builder.build_int_add(offset, index, "offset");

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };

        binary.builder.build_load(member, "val").into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        binary: &Binary,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
    ) {
        let data = self.contract_storage_data(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::default()),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let set_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, set_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(set_block);

        let offset = binary.builder.build_int_add(offset, index, "offset");

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };

        binary.builder.build_store(member, val);
    }

    fn storage_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
        ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let account = self.contract_storage_account(binary);

        if let ast::Type::Mapping(ast::Mapping { key, value, .. }) = ty.deref_any() {
            self.sparse_lookup(binary, function, key, value, slot, index, ns)
        } else if ty.is_sparse_solana(ns) {
            // sparse array
            let elem_ty = ty.storage_array_elem().deref_into();

            let key = ast::Type::Uint(256);

            self.sparse_lookup(binary, function, &key, &elem_ty, slot, index, ns)
        } else {
            // 3rd member of account is data pointer
            let data = unsafe {
                binary.builder.build_gep(
                    account,
                    &[
                        binary.context.i32_type().const_zero(),
                        binary.context.i32_type().const_int(3, false),
                    ],
                    "data",
                )
            };

            let data = binary.builder.build_load(data, "data").into_pointer_value();

            let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::default()),
                "offset_ptr",
            );

            let offset = binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let elem_ty = ty.storage_array_elem().deref_into();

            let elem_size = binary
                .context
                .i32_type()
                .const_int(elem_ty.solana_storage_size(ns).to_u64().unwrap(), false);

            binary.builder.build_int_add(
                offset,
                binary
                    .builder
                    .build_int_mul(index.into_int_value(), elem_size, ""),
                "",
            )
        }
    }

    fn storage_push(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::default()),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let member_size = binary
            .context
            .i32_type()
            .const_int(ty.storage_slots(ns).to_u64().unwrap(), false);
        let new_length = binary
            .builder
            .build_int_add(length, member_size, "new_length");

        let rc = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_realloc").unwrap(),
                &[
                    account.into(),
                    offset.into(),
                    new_length.into(),
                    offset_ptr.into(),
                ],
                "new_offset",
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

        self.return_code(
            binary,
            binary.context.i64_type().const_int(5u64 << 32, false),
        );

        binary.builder.position_at_end(rc_zero);

        let mut new_offset = binary.builder.build_int_add(
            binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value(),
            length,
            "",
        );

        if let Some(val) = val {
            self.storage_store(binary, ty, false, &mut new_offset, val, function, ns);
        }

        if ty.is_reference_type(ns) {
            // Caller expects a reference to storage; note that storage_store() should not modify
            // new_offset even if the argument is mut
            new_offset.into()
        } else {
            val.unwrap()
        }
    }

    fn storage_pop(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &ast::Namespace,
    ) -> Option<BasicValueEnum<'a>> {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::default()),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = binary.builder.build_int_compare(
            IntPredicate::NE,
            binary.context.i32_type().const_zero(),
            length,
            "index_in_range",
        );

        let bang_block = binary.context.append_basic_block(function, "bang_block");
        let retrieve_block = binary.context.append_basic_block(function, "in_range");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(retrieve_block);

        let member_size = binary
            .context
            .i32_type()
            .const_int(ty.storage_slots(ns).to_u64().unwrap(), false);

        let new_length = binary
            .builder
            .build_int_sub(length, member_size, "new_length");

        let mut old_elem_offset = binary.builder.build_int_add(offset, new_length, "");

        let val = if load {
            Some(self.storage_load(binary, ty, &mut old_elem_offset, function, ns))
        } else {
            None
        };

        // delete existing storage -- pointers need to be freed
        self.storage_free(binary, ty, data, old_elem_offset, function, false, ns);

        // we can assume pointer will stay the same after realloc to smaller size
        binary.builder.build_call(
            binary.module.get_function("account_data_realloc").unwrap(),
            &[
                account.into(),
                offset.into(),
                new_length.into(),
                offset_ptr.into(),
            ],
            "new_offset",
        );

        val
    }

    fn storage_array_length(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        elem_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };

        let offset = binary
            .builder
            .build_load(
                binary.builder.build_pointer_cast(
                    member,
                    binary.context.i32_type().ptr_type(AddressSpace::default()),
                    "",
                ),
                "offset",
            )
            .into_int_value();

        let member_size = binary
            .context
            .i32_type()
            .const_int(elem_ty.storage_slots(ns).to_u64().unwrap(), false);

        let length_bytes = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        binary
            .builder
            .build_int_unsigned_div(length_bytes, member_size, "")
    }

    fn get_storage_int(
        &self,
        _binary: &Binary<'a>,
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
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[*slot], "data") };

        match ty {
            ast::Type::String | ast::Type::DynamicBytes => {
                let offset = binary
                    .builder
                    .build_load(
                        binary.builder.build_pointer_cast(
                            member,
                            binary.context.i32_type().ptr_type(AddressSpace::default()),
                            "",
                        ),
                        "offset",
                    )
                    .into_int_value();

                let string_length = binary
                    .builder
                    .build_call(
                        binary.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "free",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let string_data =
                    unsafe { binary.builder.build_gep(data, &[offset], "string_data") };

                binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            string_length.into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            string_data.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Type::Struct(struct_ty) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);
                // LLVMSizeOf() produces an i64
                let size = binary.builder.build_int_truncate(
                    llvm_ty.size_of().unwrap(),
                    binary.context.i32_type(),
                    "size_of",
                );

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
                    llvm_ty.ptr_type(AddressSpace::default()),
                    "dest",
                );

                for (i, field) in struct_ty.definition(ns).fields.iter().enumerate() {
                    let field_offset = struct_ty.definition(ns).storage_offsets[i]
                        .to_u64()
                        .unwrap();

                    let mut offset = binary.builder.build_int_add(
                        *slot,
                        binary.context.i32_type().const_int(field_offset, false),
                        "field_offset",
                    );

                    let val = self.storage_load(binary, &field.ty, &mut offset, function, ns);

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

                    let val = if field.ty.is_fixed_reference_type() {
                        binary.builder.build_load(val.into_pointer_value(), "elem")
                    } else {
                        val
                    };

                    binary.builder.build_store(elem, val);
                }

                dest.into()
            }
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                let dest;
                let length;
                let mut slot = *slot;

                if matches!(dim.last().unwrap(), ast::ArrayLength::Fixed(_)) {
                    // LLVMSizeOf() produces an i64 and malloc takes i32
                    let size = binary.builder.build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        binary.context.i32_type(),
                        "size_of",
                    );

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

                    dest = binary.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::default()),
                        "dest",
                    );
                    length = binary.context.i32_type().const_int(
                        if let Some(ast::ArrayLength::Fixed(d)) = dim.last() {
                            d.to_u64().unwrap()
                        } else {
                            unreachable!()
                        },
                        false,
                    );
                } else {
                    let llvm_elem_ty = binary.llvm_field_ty(elem_ty, ns);
                    let elem_size = binary.builder.build_int_truncate(
                        llvm_elem_ty.size_of().unwrap(),
                        binary.context.i32_type(),
                        "size_of",
                    );

                    length = self.storage_array_length(binary, function, slot, elem_ty, ns);

                    slot = binary
                        .builder
                        .build_load(
                            binary.builder.build_pointer_cast(
                                member,
                                binary.context.i32_type().ptr_type(AddressSpace::default()),
                                "",
                            ),
                            "offset",
                        )
                        .into_int_value();

                    dest = binary.vector_new(length, elem_size, None);
                };

                let elem_size = elem_ty.solana_storage_size(ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(binary, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(binary, "offset", slot.get_type(), slot.into());

                let index = builder.over(binary, binary.context.i32_type().const_zero(), length);

                let elem = binary.array_subscript(ty.deref_any(), dest, index, ns);

                let elem_ty = ty.array_deref();

                let mut offset_val = offset_phi.into_int_value();

                let val = self.storage_load(
                    binary,
                    elem_ty.deref_memory(),
                    &mut offset_val,
                    function,
                    ns,
                );

                let val = if elem_ty.deref_memory().is_fixed_reference_type() {
                    binary.builder.build_load(val.into_pointer_value(), "elem")
                } else {
                    val
                };

                binary.builder.build_store(elem, val);

                offset_val = binary.builder.build_int_add(
                    offset_val,
                    binary.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(binary, "offset", offset_val.into());

                // done
                builder.finish(binary);

                dest.into()
            }
            _ => binary.builder.build_load(
                binary.builder.build_pointer_cast(
                    member,
                    binary.llvm_type(ty, ns).ptr_type(AddressSpace::default()),
                    "",
                ),
                "",
            ),
        }
    }

    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        offset: &mut IntValue<'a>,
        val: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[*offset], "data") };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::default()),
                "offset_ptr",
            );

            let new_string_length = binary.vector_len(val);

            let offset = if existing {
                let offset = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                // get the length of the existing string in storage
                let existing_string_length = binary
                    .builder
                    .build_call(
                        binary.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "length",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // do we need to reallocate?
                let allocation_necessary = binary.builder.build_int_compare(
                    IntPredicate::NE,
                    existing_string_length,
                    new_string_length,
                    "allocation_necessary",
                );

                let entry = binary.builder.get_insert_block().unwrap();

                let realloc = binary.context.append_basic_block(function, "realloc");
                let memcpy = binary.context.append_basic_block(function, "memcpy");

                binary
                    .builder
                    .build_conditional_branch(allocation_necessary, realloc, memcpy);

                binary.builder.position_at_end(realloc);

                // do not realloc since we're copying everything
                binary.builder.build_call(
                    binary.module.get_function("account_data_free").unwrap(),
                    &[data.into(), offset.into()],
                    "free",
                );

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

                binary
                    .builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

                binary.builder.position_at_end(rc_not_zero);

                self.return_code(
                    binary,
                    binary.context.i64_type().const_int(5u64 << 32, false),
                );

                binary.builder.position_at_end(rc_zero);

                let new_offset = binary.builder.build_load(offset_ptr, "new_offset");

                binary.builder.build_unconditional_branch(memcpy);

                binary.builder.position_at_end(memcpy);

                let offset_phi = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "offset");

                offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

                offset_phi.as_basic_value().into_int_value()
            } else {
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

                binary
                    .builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

                binary.builder.position_at_end(rc_not_zero);

                self.return_code(
                    binary,
                    binary.context.i64_type().const_int(5u64 << 32, false),
                );

                binary.builder.position_at_end(rc_zero);

                binary
                    .builder
                    .build_load(offset_ptr, "new_offset")
                    .into_int_value()
            };

            let dest_string_data = unsafe {
                binary
                    .builder
                    .build_gep(data, &[offset], "dest_string_data")
            };

            binary.builder.build_call(
                binary.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    binary.vector_bytes(val).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // make sure any pointers are freed
            if existing {
                self.storage_free(binary, ty, data, *offset, function, false, ns);
            }

            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::default()),
                "offset_ptr",
            );

            let length = if let Some(ast::ArrayLength::Fixed(length)) = dim.last() {
                binary
                    .context
                    .i32_type()
                    .const_int(length.to_u64().unwrap(), false)
            } else {
                binary.vector_len(val)
            };

            let mut elem_slot = *offset;

            if Some(&ast::ArrayLength::Dynamic) == dim.last() {
                // reallocate to the right size
                let member_size = binary
                    .context
                    .i32_type()
                    .const_int(elem_ty.solana_storage_size(ns).to_u64().unwrap(), false);
                let new_length = binary
                    .builder
                    .build_int_mul(length, member_size, "new_length");
                let offset = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                let rc = binary
                    .builder
                    .build_call(
                        binary.module.get_function("account_data_realloc").unwrap(),
                        &[
                            account.into(),
                            offset.into(),
                            new_length.into(),
                            offset_ptr.into(),
                        ],
                        "new_offset",
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

                self.return_code(
                    binary,
                    binary.context.i64_type().const_int(5u64 << 32, false),
                );

                binary.builder.position_at_end(rc_zero);

                elem_slot = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();
            }

            let elem_size = elem_ty.solana_storage_size(ns).to_u64().unwrap();

            // loop over the array
            let mut builder = LoopBuilder::new(binary, function);

            // we need a phi for the offset
            let offset_phi =
                builder.add_loop_phi(binary, "offset", offset.get_type(), elem_slot.into());

            let index = builder.over(binary, binary.context.i32_type().const_zero(), length);

            let elem = binary.array_subscript(ty, val.into_pointer_value(), index, ns);

            let mut offset_val = offset_phi.into_int_value();

            let elem_ty = ty.array_deref();

            self.storage_store(
                binary,
                elem_ty.deref_any(),
                false, // storage already freed with storage_free
                &mut offset_val,
                if elem_ty.deref_memory().is_fixed_reference_type() {
                    elem.into()
                } else {
                    binary.builder.build_load(elem, "array_elem")
                },
                function,
                ns,
            );

            offset_val = binary.builder.build_int_add(
                offset_val,
                binary.context.i32_type().const_int(elem_size, false),
                "new_offset",
            );

            // set the offset for the next iteration of the loop
            builder.set_loop_phi_value(binary, "offset", offset_val.into());

            // done
            builder.finish(binary);
        } else if let ast::Type::Struct(struct_ty) = ty {
            for (i, field) in struct_ty.definition(ns).fields.iter().enumerate() {
                let field_offset = struct_ty.definition(ns).storage_offsets[i]
                    .to_u64()
                    .unwrap();

                let mut offset = binary.builder.build_int_add(
                    *offset,
                    binary.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                let elem = unsafe {
                    binary.builder.build_gep(
                        val.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(i as u64, false),
                        ],
                        field.name_as_str(),
                    )
                };

                // free any existing dynamic storage
                if existing {
                    self.storage_free(binary, &field.ty, data, offset, function, false, ns);
                }

                self.storage_store(
                    binary,
                    &field.ty,
                    existing,
                    &mut offset,
                    if field.ty.is_fixed_reference_type() {
                        elem.into()
                    } else {
                        binary.builder.build_load(elem, field.name_as_str())
                    },
                    function,
                    ns,
                );
            }
        } else {
            binary.builder.build_store(
                binary.builder.build_pointer_cast(
                    member,
                    val.get_type().ptr_type(AddressSpace::default()),
                    "",
                ),
                val,
            );
        }
    }

    fn keccak256_hash(
        &self,
        _binary: &Binary,
        _src: PointerValue,
        _length: IntValue,
        _dest: PointerValue,
        _ns: &ast::Namespace,
    ) {
        unreachable!();
    }

    fn return_empty_abi(&self, binary: &Binary) {
        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        // set return data
        binary.builder.build_call(
            binary.module.get_function("sol_set_return_data").unwrap(),
            &[
                data.into(),
                binary
                    .builder
                    .build_int_z_extend(length, binary.context.i64_type(), "length")
                    .into(),
            ],
            "",
        );

        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_int(0, false)));
    }

    fn assert_failure(&self, binary: &Binary, data: PointerValue, length: IntValue) {
        // the reason code should be null (and already printed)
        binary.builder.build_call(
            binary.module.get_function("sol_set_return_data").unwrap(),
            &[
                data.into(),
                binary
                    .builder
                    .build_int_z_extend(length, binary.context.i64_type(), "length")
                    .into(),
            ],
            "",
        );

        // return 1 for failure
        binary.builder.build_return(Some(
            &binary.context.i64_type().const_int(1u64 << 32, false),
        ));
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        _binary: &Binary<'b>,
        _function: FunctionValue<'b>,
        _packed: &[BasicValueEnum<'b>],
        _args: &[BasicValueEnum<'b>],
        _tys: &[ast::Type],
        _ns: &ast::Namespace,
    ) -> PointerValue<'b> {
        unreachable!("ABI encoding is implemented in code generation for Solana")
    }

    fn abi_encode(
        &self,
        _binary: &Binary<'a>,
        _selector: Option<IntValue<'a>>,
        _load: bool,
        _function: FunctionValue<'a>,
        _args: &[BasicValueEnum<'a>],
        _tys: &[ast::Type],
        _ns: &ast::Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        unreachable!("ABI encoding is implemented in code generation for Solana")
    }

    fn abi_decode<'b>(
        &self,
        _binary: &Binary<'b>,
        _function: FunctionValue<'b>,
        _args: &mut Vec<BasicValueEnum<'b>>,
        _data: PointerValue<'b>,
        _length: IntValue<'b>,
        _spec: &[ast::Parameter],
        _ns: &ast::Namespace,
    ) {
        unreachable!("ABI encoding is implemented in code generation for Solana.")
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        let string_len64 =
            binary
                .builder
                .build_int_z_extend(string_len, binary.context.i64_type(), "");

        binary.builder.build_call(
            binary.module.get_function("sol_log_").unwrap(),
            &[string_ptr.into(), string_len64.into()],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        address: PointerValue<'b>,
        encoded_args: BasicValueEnum<'b>,
        encoded_args_len: BasicValueEnum<'b>,
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
        seeds: Option<(PointerValue<'b>, IntValue<'b>)>,
        ns: &ast::Namespace,
    ) {
        let const_program_id = binary.builder.build_pointer_cast(
            binary.emit_global_string(
                "const_program_id",
                ns.contracts[contract_no].program_id.as_ref().unwrap(),
                true,
            ),
            binary
                .module
                .get_struct_type("struct.SolPubkey")
                .unwrap()
                .ptr_type(AddressSpace::default()),
            "const_program_id",
        );

        let sol_params = function.get_last_param().unwrap().into_pointer_value();

        let create_contract = binary.module.get_function("create_contract").unwrap();

        let address = binary.builder.build_pointer_cast(
            address,
            binary
                .module
                .get_struct_type("struct.SolPubkey")
                .unwrap()
                .ptr_type(AddressSpace::default()),
            "address",
        );

        let (signer_seeds, signer_seeds_len) = if let Some((seeds, len)) = seeds {
            (
                binary.builder.build_pointer_cast(
                    seeds,
                    create_contract.get_type().get_param_types()[4].into_pointer_type(),
                    "seeds",
                ),
                binary.builder.build_int_cast(
                    len,
                    create_contract.get_type().get_param_types()[5].into_int_type(),
                    "len",
                ),
            )
        } else {
            (
                create_contract.get_type().get_param_types()[4]
                    .const_zero()
                    .into_pointer_value(),
                create_contract.get_type().get_param_types()[5]
                    .const_zero()
                    .into_int_value(),
            )
        };

        let ret = binary
            .builder
            .build_call(
                create_contract,
                &[
                    binary.vector_bytes(encoded_args).into(),
                    encoded_args_len.into(),
                    address.into(),
                    const_program_id.into(),
                    signer_seeds.into(),
                    signer_seeds_len.into(),
                    sol_params.into(),
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
            binary.context.i64_type().const_zero(),
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

            binary.builder.build_return(Some(&ret));

            binary.builder.position_at_end(success_block);
        }
    }

    fn builtin_function(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &ast::Function,
        args: &[BasicMetadataValueEnum<'a>],
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        if builtin_func.name == "create_program_address" {
            let func = binary
                .module
                .get_function("sol_create_program_address")
                .unwrap();

            // first argument are the seeds
            let seeds = binary.builder.build_pointer_cast(
                args[0].into_pointer_value(),
                func.get_first_param()
                    .unwrap()
                    .get_type()
                    .into_pointer_type(),
                "seeds",
            );

            let seed_count = binary.context.i64_type().const_int(
                args[0]
                    .into_pointer_value()
                    .get_type()
                    .get_element_type()
                    .into_array_type()
                    .len() as u64,
                false,
            );

            // address
            let address = binary.build_alloca(function, binary.address_type(ns), "address");

            binary
                .builder
                .build_store(address, args[1].into_array_value());

            binary
                .builder
                .build_call(
                    func,
                    &[
                        seeds.into(),
                        seed_count.into(),
                        address.into(),
                        args[2], // return value
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        } else if builtin_func.name == "try_find_program_address" {
            let func = binary
                .module
                .get_function("sol_try_find_program_address")
                .unwrap();

            // first argument are the seeds
            let seeds = binary.builder.build_pointer_cast(
                args[0].into_pointer_value(),
                func.get_first_param()
                    .unwrap()
                    .get_type()
                    .into_pointer_type(),
                "seeds",
            );

            let seed_count = binary.context.i64_type().const_int(
                args[0]
                    .into_pointer_value()
                    .get_type()
                    .get_element_type()
                    .into_array_type()
                    .len() as u64,
                false,
            );

            // address
            let address = binary.build_alloca(function, binary.address_type(ns), "address");

            binary
                .builder
                .build_store(address, args[1].into_array_value());

            binary
                .builder
                .build_call(
                    func,
                    &[
                        seeds.into(),
                        seed_count.into(),
                        address.into(),
                        args[2], // return address/pubkey
                        args[3], // return seed bump
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        } else {
            unreachable!();
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
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        accounts: Option<(PointerValue<'b>, IntValue<'b>)>,
        seeds: Option<(PointerValue<'b>, IntValue<'b>)>,
        _ty: ast::CallTy,
        _ns: &ast::Namespace,
    ) {
        let address = address.unwrap();

        let ret = if let Some((accounts, accounts_len)) = accounts {
            // build instruction
            let instruction_ty: BasicTypeEnum = binary
                .module
                .get_struct_type("struct.SolInstruction")
                .unwrap()
                .into();

            let instruction = binary.build_alloca(function, instruction_ty, "instruction");

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(instruction, 0, "program_id")
                    .unwrap(),
                binary.builder.build_pointer_cast(
                    address,
                    binary
                        .module
                        .get_struct_type("struct.SolPubkey")
                        .unwrap()
                        .ptr_type(AddressSpace::default()),
                    "SolPubkey",
                ),
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(instruction, 1, "accounts")
                    .unwrap(),
                binary.builder.build_pointer_cast(
                    accounts,
                    binary
                        .module
                        .get_struct_type("struct.SolAccountMeta")
                        .unwrap()
                        .ptr_type(AddressSpace::default()),
                    "SolAccountMeta",
                ),
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(instruction, 2, "accounts_len")
                    .unwrap(),
                binary.builder.build_int_z_extend(
                    accounts_len,
                    binary.context.i64_type(),
                    "accounts_len",
                ),
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(instruction, 3, "data")
                    .unwrap(),
                binary.builder.build_pointer_cast(
                    payload,
                    binary.context.i8_type().ptr_type(AddressSpace::default()),
                    "data",
                ),
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(instruction, 4, "data_len")
                    .unwrap(),
                binary.builder.build_int_z_extend(
                    payload_len,
                    binary.context.i64_type(),
                    "payload_len",
                ),
            );

            let parameters = self.sol_parameters(binary);

            let account_infos = binary.builder.build_pointer_cast(
                binary
                    .builder
                    .build_struct_gep(parameters, 0, "ka")
                    .unwrap(),
                binary
                    .module
                    .get_struct_type("struct.SolAccountInfo")
                    .unwrap()
                    .ptr_type(AddressSpace::default()),
                "SolAccountInfo",
            );

            let account_infos_len = binary.builder.build_int_truncate(
                binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(parameters, 1, "ka_num")
                            .unwrap(),
                        "ka_num",
                    )
                    .into_int_value(),
                binary.context.i32_type(),
                "ka_num",
            );

            let external_call = binary.module.get_function("sol_invoke_signed_c").unwrap();

            let (signer_seeds, signer_seeds_len) = if let Some((seeds, len)) = seeds {
                (
                    binary.builder.build_pointer_cast(
                        seeds,
                        external_call.get_type().get_param_types()[3].into_pointer_type(),
                        "seeds",
                    ),
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

            binary
                .builder
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
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            let parameters = self.sol_parameters(binary);

            let external_call = binary.module.get_function("external_call").unwrap();

            // cast [u8; 32]* to SolPubkey*
            let address = binary.builder.build_pointer_cast(
                address,
                binary
                    .module
                    .get_struct_type("struct.SolPubkey")
                    .unwrap()
                    .ptr_type(AddressSpace::default()),
                "address",
            );

            binary
                .builder
                .build_call(
                    external_call,
                    &[
                        address.into(),
                        payload.into(),
                        payload_len.into(),
                        parameters.into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        };

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i64_type().const_zero(),
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

            // should we log "call failed?"
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
    }

    /// Get return buffer for external call
    fn return_data<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
    ) -> PointerValue<'b> {
        let null_u8_ptr = binary
            .context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .const_zero();

        let length_as_64 = binary
            .builder
            .build_call(
                binary.module.get_function("sol_get_return_data").unwrap(),
                &[
                    null_u8_ptr.into(),
                    binary.context.i64_type().const_zero().into(),
                    null_u8_ptr.into(),
                ],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let length =
            binary
                .builder
                .build_int_truncate(length_as_64, binary.context.i32_type(), "length");

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
                .ptr_type(AddressSpace::default()),
            "string",
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

        let program_id = binary.build_array_alloca(
            function,
            binary.context.i8_type(),
            binary.context.i32_type().const_int(32, false),
            "program_id",
        );

        binary.builder.build_call(
            binary.module.get_function("sol_get_return_data").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        data,
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        "",
                    )
                    .into(),
                length_as_64.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        program_id,
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        v
    }

    fn return_code<'b>(&self, binary: &'b Binary, ret: IntValue<'b>) {
        binary.builder.build_return(Some(&ret));
    }

    /// Value received is not available on solana
    fn value_transferred<'b>(&self, _binary: &Binary<'b>, _ns: &ast::Namespace) -> IntValue<'b> {
        unreachable!();
    }

    /// Send value to address
    fn value_transfer<'b>(
        &self,
        binary: &Binary<'b>,
        _function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        address: PointerValue<'b>,
        value: IntValue<'b>,
        _ns: &ast::Namespace,
    ) {
        let parameters = self.sol_parameters(binary);

        if let Some(success) = success {
            *success = binary
                .builder
                .build_call(
                    binary.module.get_function("sol_try_transfer").unwrap(),
                    &[address.into(), value.into(), parameters.into()],
                    "success",
                )
                .try_as_basic_value()
                .left()
                .unwrap();
        } else {
            binary.builder.build_call(
                binary.module.get_function("sol_transfer").unwrap(),
                &[address.into(), value.into(), parameters.into()],
                "",
            );
        }
    }

    /// Terminate execution, destroy binary and send remaining funds to addr
    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: ArrayValue<'b>, _ns: &ast::Namespace) {
        unimplemented!();
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        _topics: &[BasicValueEnum<'b>],
    ) {
        let fields = binary.build_array_alloca(
            function,
            binary.module.get_struct_type("SolLogDataField").unwrap(),
            binary.context.i32_type().const_int(1, false),
            "fields",
        );

        let field_data = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_zero(),
                ],
                "field_data",
            )
        };

        let bytes_pointer = binary.vector_bytes(data);
        binary.builder.build_store(field_data, bytes_pointer);

        let field_len = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(1, false),
                ],
                "data_len",
            )
        };

        binary.builder.build_store(
            field_len,
            binary.builder.build_int_z_extend(
                binary.vector_len(data),
                binary.context.i64_type(),
                "data_len64",
            ),
        );

        binary.builder.build_call(
            binary.module.get_function("sol_log_data").unwrap(),
            &[
                fields.into(),
                binary.context.i64_type().const_int(1, false).into(),
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
        match expr {
            codegen::Expression::Builtin(_, _, codegen::Builtin::Timestamp, _) => {
                let parameters = self.sol_parameters(binary);

                let sol_clock = binary.module.get_function("sol_clock").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    parameters,
                    sol_clock.get_type().get_param_types()[0].into_pointer_type(),
                    "",
                );

                let clock = binary
                    .builder
                    .build_call(sol_clock, &[arg1.into()], "clock")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let timestamp = binary
                    .builder
                    .build_struct_gep(clock, 4, "unix_timestamp")
                    .unwrap();

                binary.builder.build_load(timestamp, "timestamp")
            }
            codegen::Expression::Builtin(
                _,
                _,
                codegen::Builtin::BlockNumber | codegen::Builtin::Slot,
                _,
            ) => {
                let parameters = self.sol_parameters(binary);

                let sol_clock = binary.module.get_function("sol_clock").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    parameters,
                    sol_clock.get_type().get_param_types()[0].into_pointer_type(),
                    "",
                );

                let clock = binary
                    .builder
                    .build_call(sol_clock, &[arg1.into()], "clock")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let slot = binary.builder.build_struct_gep(clock, 0, "slot").unwrap();

                binary.builder.build_load(slot, "timestamp")
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::GetAddress, _) => {
                let parameters = self.sol_parameters(binary);

                let key = unsafe {
                    binary.builder.build_gep(
                        parameters,
                        &[
                            binary.context.i32_type().const_int(0, false), // first SolParameters
                            binary.context.i32_type().const_int(0, false), // first field of SolParameters
                            binary.context.i32_type().const_int(0, false), // first element of ka[]
                            binary.context.i32_type().const_int(0, false), // first field of SolAccountInfo (key)
                        ],
                        "key",
                    )
                };

                // SolPubkey** => [u8; 32]**
                let value = binary.builder.build_pointer_cast(
                    key,
                    binary
                        .address_type(ns)
                        .ptr_type(AddressSpace::default())
                        .ptr_type(AddressSpace::default()),
                    "",
                );

                let key_pointer = binary.builder.build_load(value, "key_pointer");

                binary
                    .builder
                    .build_load(key_pointer.into_pointer_value(), "key")
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::ProgramId, _) => {
                let parameters = self.sol_parameters(binary);

                let account_id = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(parameters, 4, "program_id")
                            .unwrap(),
                        "program_id",
                    )
                    .into_pointer_value();

                let value = binary.builder.build_pointer_cast(
                    account_id,
                    binary.address_type(ns).ptr_type(AddressSpace::default()),
                    "",
                );

                binary.builder.build_load(value, "program_id")
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Calldata, _) => {
                let sol_params = self.sol_parameters(binary);

                let input = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(sol_params, 2, "input")
                            .unwrap(),
                        "data",
                    )
                    .into_pointer_value();

                let input_len = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(sol_params, 3, "input_len")
                            .unwrap(),
                        "data_len",
                    )
                    .into_int_value();

                let input_len = binary.builder.build_int_truncate(
                    input_len,
                    binary.context.i32_type(),
                    "input_len",
                );

                binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            input_len.into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            input.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Signature, _) => {
                let sol_params = self.sol_parameters(binary);

                let input = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(sol_params, 2, "input")
                            .unwrap(),
                        "data",
                    )
                    .into_pointer_value();

                let selector = binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        input,
                        binary.context.i64_type().ptr_type(AddressSpace::default()),
                        "selector",
                    ),
                    "selector",
                );

                let bswap = binary.llvm_bswap(64);

                binary
                    .builder
                    .build_call(bswap, &[selector.into()], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::SignatureVerify, args) => {
                assert_eq!(args.len(), 3);

                let address = binary.build_alloca(function, binary.address_type(ns), "address");

                binary.builder.build_store(
                    address,
                    expression(self, binary, &args[0], vartab, function, ns).into_array_value(),
                );

                let message = expression(self, binary, &args[1], vartab, function, ns);
                let signature = expression(self, binary, &args[2], vartab, function, ns);
                let parameters = self.sol_parameters(binary);
                let signature_verify = binary.module.get_function("signature_verify").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    message.into_pointer_value(),
                    signature_verify.get_type().get_param_types()[1].into_pointer_type(),
                    "",
                );

                let arg2 = binary.builder.build_pointer_cast(
                    signature.into_pointer_value(),
                    signature_verify.get_type().get_param_types()[2].into_pointer_type(),
                    "",
                );

                let ret = binary
                    .builder
                    .build_call(
                        signature_verify,
                        &[
                            binary
                                .builder
                                .build_pointer_cast(
                                    address,
                                    binary.context.i8_type().ptr_type(AddressSpace::default()),
                                    "",
                                )
                                .into(),
                            arg1.into(),
                            arg2.into(),
                            parameters.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                binary
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret,
                        binary.context.i64_type().const_zero(),
                        "success",
                    )
                    .into()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Balance, args) => {
                assert_eq!(args.len(), 1);

                let address = binary.build_alloca(function, binary.address_type(ns), "address");

                binary.builder.build_store(
                    address,
                    expression(self, binary, &args[0], vartab, function, ns).into_array_value(),
                );

                let account_lamport = binary.module.get_function("sol_account_lamport").unwrap();

                let parameters = self.sol_parameters(binary);

                let params = account_lamport.get_type().get_param_types();

                let lamport = binary
                    .builder
                    .build_call(
                        account_lamport,
                        &[
                            binary
                                .builder
                                .build_pointer_cast(address, params[0].into_pointer_type(), "")
                                .into(),
                            binary
                                .builder
                                .build_pointer_cast(parameters, params[1].into_pointer_type(), "")
                                .into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                binary.builder.build_load(lamport, "lamport")
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::Accounts, _) => {
                let parameters = self.sol_parameters(binary);

                unsafe {
                    binary.builder.build_gep(
                        parameters,
                        &[
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                        ],
                        "accounts",
                    )
                }
                .into()
            }
            codegen::Expression::Builtin(_, _, codegen::Builtin::ArrayLength, _) => {
                let parameters = self.sol_parameters(binary);

                let ka_num = binary
                    .builder
                    .build_struct_gep(parameters, 1, "ka_num")
                    .unwrap();

                let ka_num = binary.builder.build_load(ka_num, "ka_num").into_int_value();

                binary
                    .builder
                    .build_int_truncate(ka_num, binary.context.i32_type(), "ka_num_32bits")
                    .into()
            }
            codegen::Expression::StructMember(_, _, a, member) => {
                let account_info =
                    expression(self, binary, a, vartab, function, ns).into_pointer_value();

                self.account_info_member(binary, function, account_info, *member, ns)
            }
            _ => unimplemented!(),
        }
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("sol_keccak256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("sol_sha256", 32),
            _ => unreachable!(),
        };

        let res = binary.build_array_alloca(
            function,
            binary.context.i8_type(),
            binary.context.i32_type().const_int(hashlen, false),
            "res",
        );

        if hash == HashTy::Ripemd160 {
            binary.builder.build_call(
                binary.module.get_function(fname).unwrap(),
                &[input.into(), input_len.into(), res.into()],
                "hash",
            );
        } else {
            let u64_ty = binary.context.i64_type();

            let sol_keccak256 = binary.module.get_function(fname).unwrap();

            // The first argument is a SolBytes *, get the struct
            let sol_bytes = sol_keccak256.get_type().get_param_types()[0]
                .into_pointer_type()
                .get_element_type()
                .into_struct_type();

            let array = binary.build_alloca(function, sol_bytes, "sol_bytes");

            binary.builder.build_store(
                binary.builder.build_struct_gep(array, 0, "input").unwrap(),
                input,
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(array, 1, "input_len")
                    .unwrap(),
                binary
                    .builder
                    .build_int_z_extend(input_len, u64_ty, "input_len"),
            );

            binary.builder.build_call(
                sol_keccak256,
                &[
                    array.into(),
                    binary.context.i32_type().const_int(1, false).into(),
                    res.into(),
                ],
                "hash",
            );
        }

        // bytes32 needs to reverse bytes
        let temp = binary.build_alloca(
            function,
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
                        binary.context.i8_type().ptr_type(AddressSpace::default()),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        binary.builder.build_load(temp, "hash").into_int_value()
    }

    fn return_abi_data<'b>(
        &self,
        binary: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        binary.builder.build_call(
            binary.module.get_function("sol_set_return_data").unwrap(),
            &[data.into(), data_len.into()],
            "",
        );

        binary
            .builder
            .build_return(Some(&binary.return_values[&ReturnCode::Success]));
    }
}
