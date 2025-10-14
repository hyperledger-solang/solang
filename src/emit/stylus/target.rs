// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]
#![warn(clippy::renamed_function_params)]

use crate::codegen::cfg::HashTy;
use crate::codegen::{Builtin, Expression};
use crate::emit::binary::Binary;
use crate::emit::storage::StorageSlot;
use crate::emit::stylus::StylusTarget;
use crate::emit::{expression::expression, ContractArgs, TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast::{self, CallTy};
use crate::sema::ast::{Function, Type};
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
    PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use solang_parser::pt::{Loc, StorageType};
use std::collections::HashMap;

impl<'a> StylusTarget {
    pub(crate) fn get_storage_type_int(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
        storage_type: &Option<StorageType>,
    ) -> IntValue<'a> {
        emit_context!(bin);

        let value_ptr = bin.builder.build_alloca(ty, "value").unwrap();

        match storage_type {
            Some(StorageType::Temporary(_)) => {
                call!("transient_load_bytes32", &[slot.into(), value_ptr.into()]);
            }
            _ => {
                call!("storage_load_bytes32", &[slot.into(), value_ptr.into()]);
            }
        }

        bin.builder
            .build_load(ty, value_ptr, "value")
            .unwrap()
            .into_int_value()
    }
}

impl<'a> TargetRuntime<'a> for StylusTarget {
    fn get_storage_int(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        // Use get_storage_type_int instead
        unreachable!();
        /*
        emit_context!(bin);

        let value_ptr = bin.builder.build_alloca(ty, "value").unwrap();

        call!("storage_load_bytes32", &[slot.into(), value_ptr.into()]);

        bin.builder
            .build_load(ty, value_ptr, "value")
            .unwrap()
            .into_int_value()
        */
    }

    fn storage_load(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        // The storage slot is an i256 accessed through a pointer, so we need
        // to store it
        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot").unwrap();

        let value = self.storage_load_slot(bin, ty, slot, slot_ptr, function, storage_type);

        value
    }

    /// Recursively store a type to storage
    fn storage_store(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        slot: &mut IntValue<'a>,
        value: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        storage_type: &Option<StorageType>,
    ) {
        emit_context!(bin);

        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot").unwrap();

        self.storage_store_slot(bin, ty, slot, slot_ptr, value, function, storage_type);

        match storage_type {
            Some(StorageType::Persistent(_)) | None => {
                call!("storage_flush_cache", &[i32_const!(1).into()]);
            }
            _ => {}
        }
    }

    /// Recursively clear storage. The default implementation is for slot-based storage
    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        unimplemented!()
    }

    // Bytes and string have special storage layout
    fn set_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        emit_context!(bin);

        let len = bin.vector_len(dest);
        let data = bin.vector_bytes(dest);

        let len_ptr = bin
            .builder
            .build_alloca(bin.context.i32_type(), "len_ptr")
            .unwrap();
        bin.builder.build_store(len_ptr, len).unwrap();
        call!("storage_cache_bytes32", &[slot.into(), len_ptr.into()]);

        let n_chunks = bin
            .builder
            .build_int_unsigned_div(
                bin.builder
                    .build_int_add(len, i32_const!(31), "len_plus_31")
                    .unwrap(),
                i32_const!(32),
                "n_chunks",
            )
            .unwrap();

        let mut slot = next_slot(bin, slot, 32);

        let slot_ptr = bin.builder.build_alloca(bin.value_type(), "slot").unwrap();

        bin.emit_loop_cond_first_with_int(
            function,
            i32_zero!(),
            n_chunks,
            &mut slot,
            |i_chunk: IntValue<'a>, slot: &mut IntValue<'a>| {
                let i_chunk_as_u256 = bin
                    .builder
                    .build_int_z_extend(i_chunk, bin.value_type(), "i_chunk_as_u256")
                    .unwrap();
                let slot_plus_i_chunk = bin
                    .builder
                    .build_int_add(*slot, i_chunk_as_u256, "slot_plus_i_chunk")
                    .unwrap();
                bin.builder
                    .build_store(slot_ptr, slot_plus_i_chunk)
                    .unwrap();

                let offset = bin
                    .builder
                    .build_int_mul(i_chunk, i32_const!(32), "ptr_plus_offset")
                    .unwrap();
                let chunk = bin
                    .builder
                    .build_load(
                        bin.value_type(),
                        ptr_plus_offset(bin, data, offset),
                        "chunk",
                    )
                    .unwrap();

                let chunk_ptr = bin
                    .builder
                    .build_alloca(bin.value_type(), "chunk_ptr")
                    .unwrap();
                bin.builder.build_store(chunk_ptr, chunk).unwrap();
                call!(
                    "storage_cache_bytes32",
                    &[slot_ptr.into(), chunk_ptr.into()]
                );
            },
        );

        call!("storage_flush_cache", &[i32_const!(1).into()]);
    }

    fn get_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        emit_context!(bin);

        let len_ptr = bin
            .builder
            .build_alloca(bin.context.i32_type(), "len_ptr")
            .unwrap();
        call!("storage_load_bytes32", &[slot.into(), len_ptr.into()]);
        let len = bin
            .builder
            .build_load(bin.context.i32_type(), len_ptr, "len")
            .unwrap()
            .into_int_value();

        let n_chunks = bin
            .builder
            .build_int_unsigned_div(
                bin.builder
                    .build_int_add(len, i32_const!(31), "len_plus_31")
                    .unwrap(),
                i32_const!(32),
                "n_chunks",
            )
            .unwrap();

        let buffer_size = bin
            .builder
            .build_int_mul(n_chunks, i32_const!(32), "buffer_size")
            .unwrap();
        let buffer = call!("__malloc", &[buffer_size.into()])
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let mut slot = next_slot(bin, slot, 32);

        let slot_ptr = bin.builder.build_alloca(bin.value_type(), "slot").unwrap();

        bin.emit_loop_cond_first_with_int(
            function,
            i32_zero!(),
            n_chunks,
            &mut slot,
            |i_chunk: IntValue<'a>, slot: &mut IntValue<'a>| {
                let i_chunk_as_u256 = bin
                    .builder
                    .build_int_z_extend(i_chunk, bin.value_type(), "i_chunk_as_u256")
                    .unwrap();
                let slot_plus_i_chunk = bin
                    .builder
                    .build_int_add(*slot, i_chunk_as_u256, "slot_plus_i_chunk")
                    .unwrap();
                bin.builder
                    .build_store(slot_ptr, slot_plus_i_chunk)
                    .unwrap();

                let chunk_ptr = bin
                    .builder
                    .build_alloca(bin.value_type(), "chunk_ptr")
                    .unwrap();
                call!("storage_load_bytes32", &[slot_ptr.into(), chunk_ptr.into()]);
                let chunk = bin
                    .builder
                    .build_load(bin.value_type(), chunk_ptr, "chunk")
                    .unwrap();

                let offset = bin
                    .builder
                    .build_int_mul(i_chunk, i32_const!(32), "ptr_plus_offset")
                    .unwrap();
                bin.builder
                    .build_store(ptr_plus_offset(bin, buffer, offset), chunk)
                    .unwrap();
            },
        );

        call!(
            "vector_new",
            &[len.into(), i32_const!(1).into(), buffer.into()]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
    }

    fn set_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    ) {
        unimplemented!()
    }

    fn get_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!()
    }

    fn get_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        loc: Loc,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn set_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
        loc: Loc,
    ) {
        emit_context!(bin);

        let len_slot_ptr = bin
            .builder
            .build_alloca(slot.get_type(), "len_slot_ptr")
            .unwrap();
        bin.builder.build_store(len_slot_ptr, slot).unwrap();

        // smoelius: Read length.
        let len_ptr = bin
            .builder
            .build_alloca(bin.context.i32_type(), "len_ptr")
            .unwrap();
        call!(
            "storage_load_bytes32",
            &[len_slot_ptr.into(), len_ptr.into()]
        );
        let len = bin
            .builder
            .build_load(bin.context.i32_type(), len_ptr, "len")
            .unwrap()
            .into_int_value();

        let chunk_slot = next_slot(bin, len_slot_ptr, 32);

        // smoelius: Read chunk.
        let chunk_slot_ptr = bin
            .builder
            .build_alloca(slot.get_type(), "chunk_slot")
            .unwrap();
        let i_chunk = bin
            .builder
            .build_int_unsigned_div(index, i32_const!(32), "i_chunk")
            .unwrap();
        let i_chunk_as_u256 = bin
            .builder
            .build_int_z_extend(i_chunk, bin.value_type(), "i_chunk_as_u256")
            .unwrap();
        let slot_plus_i_chunk = bin
            .builder
            .build_int_add(chunk_slot, i_chunk_as_u256, "slot_plus_i_chunk")
            .unwrap();
        bin.builder
            .build_store(chunk_slot_ptr, slot_plus_i_chunk)
            .unwrap();
        let chunk_ptr = bin
            .builder
            .build_alloca(bin.value_type(), "chunk_ptr")
            .unwrap();
        call!(
            "storage_load_bytes32",
            &[chunk_slot_ptr.into(), chunk_ptr.into()]
        );

        // smoelius: Calculate offset into chunk.
        let offset = bin
            .builder
            .build_int_unsigned_rem(index, i32_const!(32), "offset")
            .unwrap();

        // smoelius: Write byte into chunk.
        let chunk_ptr_as_byte_ptr = bin
            .builder
            .build_pointer_cast(
                chunk_ptr,
                bin.context.ptr_type(AddressSpace::default()),
                "chunk_ptr_as_byte_ptr",
            )
            .unwrap();
        let byte_ptr = ptr_plus_offset(bin, chunk_ptr_as_byte_ptr, offset);
        bin.builder.build_store(byte_ptr, value).unwrap();

        // smoelius: Write updated chunk to storage.
        call!(
            "storage_cache_bytes32",
            &[chunk_slot_ptr.into(), chunk_ptr.into()]
        );

        // smoelius: Update length.
        let index_less_than_len = bin
            .builder
            .build_int_compare(IntPredicate::ULT, index, len, "index_less_than_len")
            .unwrap();
        let len = bin
            .builder
            .build_select(
                index_less_than_len,
                len,
                bin.builder
                    .build_int_add(index, i32_const!(1), "index_plus_1")
                    .unwrap(),
                "updated_length",
            )
            .unwrap();

        // smoelius: Write updated length to storage.
        bin.builder.build_store(len_ptr, len).unwrap();
        call!(
            "storage_cache_bytes32",
            &[len_slot_ptr.into(), len_ptr.into()]
        );

        call!("storage_flush_cache", &[i32_const!(1).into()]);
    }

    fn storage_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn storage_push(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
    ) -> BasicValueEnum<'a> {
        emit_context!(bin);

        let val = val.unwrap();

        // smoelius: Read length.
        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot").unwrap();
        bin.builder.build_store(slot_ptr, slot).unwrap();
        let len_ptr = bin
            .builder
            .build_alloca(bin.context.i32_type(), "len_ptr")
            .unwrap();
        call!("storage_load_bytes32", &[slot_ptr.into(), len_ptr.into()]);
        let len = bin
            .builder
            .build_load(bin.context.i32_type(), len_ptr, "len")
            .unwrap()
            .into_int_value();

        // smoelius: Calculate last chunk index.
        let i_chunk = bin
            .builder
            .build_int_unsigned_div(len, i32_const!(32), "i_chunk")
            .unwrap();

        // smoelius: Calculate last chunk slot.
        let chunk_slot_base = next_slot(bin, slot_ptr, 32);
        let chunk_slot = bin
            .builder
            .build_int_add(
                chunk_slot_base,
                bin.builder
                    .build_int_z_extend(i_chunk, slot.get_type(), "i_chunk_as_slot_type")
                    .unwrap(),
                "chunk_slot",
            )
            .unwrap();

        // smoelius: Read last chunk.
        let chunk_slot_ptr = bin
            .builder
            .build_alloca(slot.get_type(), "chunk_slot")
            .unwrap();
        bin.builder.build_store(chunk_slot_ptr, chunk_slot).unwrap();
        let chunk_ptr = bin
            .builder
            .build_alloca(bin.value_type(), "chunk_ptr")
            .unwrap();
        call!(
            "storage_load_bytes32",
            &[chunk_slot_ptr.into(), chunk_ptr.into()]
        );

        // smoelius: Calculate offset into chunk.
        let offset = bin
            .builder
            .build_int_unsigned_rem(len, i32_const!(32), "offset")
            .unwrap();

        // smoelius: Write byte into chunk.
        let chunk_ptr_as_byte_ptr = bin
            .builder
            .build_pointer_cast(
                chunk_ptr,
                bin.context.ptr_type(AddressSpace::default()),
                "chunk_ptr_as_byte_ptr",
            )
            .unwrap();
        let byte_ptr = ptr_plus_offset(bin, chunk_ptr_as_byte_ptr, offset);
        bin.builder.build_store(byte_ptr, val).unwrap();

        // smoelius: Write updated chunk to storage.
        call!(
            "storage_cache_bytes32",
            &[chunk_slot_ptr.into(), chunk_ptr.into()]
        );

        // smoelius: Update length.
        let len = bin
            .builder
            .build_int_add(len, i32_const!(1), "updated_len")
            .unwrap();

        // smoelius: Write updated length to storage.
        bin.builder.build_store(len_ptr, len).unwrap();
        call!("storage_cache_bytes32", &[slot_ptr.into(), len_ptr.into()]);

        call!("storage_flush_cache", &[i32_const!(1).into()]);

        val
    }

    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        load: bool,
        loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!()
    }

    fn storage_array_length(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        elem_ty: &Type,
    ) -> IntValue<'a> {
        emit_context!(bin);

        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot").unwrap();
        bin.builder.build_store(slot_ptr, slot).unwrap();

        let len_ptr = bin
            .builder
            .build_alloca(bin.context.i32_type(), "len_ptr")
            .unwrap();
        call!("storage_load_bytes32", &[slot_ptr.into(), len_ptr.into()]);
        let len = bin
            .builder
            .build_load(bin.context.i32_type(), len_ptr, "len")
            .unwrap()
            .into_int_value();

        len
    }

    /// keccak256 hash
    fn keccak256_hash(
        &self,
        bin: &Binary<'a>,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        emit_context!(bin);

        call!(
            "native_keccak256",
            &[src.into(), length.into(), dest.into()]
        );
    }

    /// Prints a string
    fn print(&self, bin: &Binary, string: PointerValue, length: IntValue) {
        emit_context!(bin);

        call!("log_txt", &[string.into(), length.into()]);
    }

    /// Return success without any result
    fn return_empty_abi(&self, bin: &Binary) {
        unimplemented!()
    }

    /// Return failure code
    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>) {
        emit_context!(bin);

        self.assert_failure(bin, ptr!().const_zero(), i32_zero!());
    }

    /// Return failure without any result
    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        emit_context!(bin);

        bin.builder
            .build_store(bin.return_code.unwrap().as_pointer_value(), i32_const!(1))
            .unwrap();

        // smoelius: We must return something here, or else the wasm won't parse. But I'm not sure
        // that returning 0 or 1 makes a difference.
        let one: &dyn BasicValue = &i32_const!(1);
        bin.builder.build_return(Some(one)).unwrap();
    }

    fn builtin_function(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &Function,
        args: &[BasicMetadataValueEnum<'a>],
        first_arg_type: Option<BasicTypeEnum>,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!()
    }

    /// Calls constructor
    fn create_contract<'b>(
        &mut self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        address: PointerValue<'b>,
        encoded_args: BasicValueEnum<'b>,
        encoded_args_len: BasicValueEnum<'b>,
        contract_args: ContractArgs<'b>,
        loc: Loc,
    ) {
        emit_context!(bin);

        let revert_data_len = bin
            .builder
            .build_alloca(bin.llvm_type(&ast::Type::Uint(32)), "revert_data_len")
            .unwrap();

        let created_contract = &bin.ns.contracts[contract_no];

        let code = created_contract.emit(bin.ns, bin.options, contract_no);

        let code_ptr =
            bin.emit_global_string(&format!("binary_{}_code", created_contract.id), &code, true);

        let code_len = bin.context.i32_type().const_int(code.len() as u64, false);

        let value = bin.builder.build_alloca(bin.value_type(), "value").unwrap();
        bin.builder
            .build_store(
                value,
                contract_args.value.unwrap_or(bin.value_type().const_zero()),
            )
            .unwrap();

        if let Some(salt) = contract_args.salt {
            // create2
            let salt_buf = bin.builder.build_alloca(bin.value_type(), "salt").unwrap();
            bin.builder.build_store(salt_buf, salt).unwrap();

            call!(
                "create2",
                &[
                    code_ptr.into(),
                    code_len.into(),
                    value.into(),
                    salt_buf.into(),
                    address.into(),
                    revert_data_len.into()
                ],
                "create2"
            );
        } else {
            // create1
            call!(
                "create1",
                &[
                    code_ptr.into(),
                    code_len.into(),
                    value.into(),
                    address.into(),
                    revert_data_len.into()
                ],
                "create1"
            );
        }

        // Set success based on whether the address is zero (contract creation failed)
        if let Some(success) = success {
            let zero_address = bin.address_type().const_zero();

            // Use __memcmp to compare the address with zero
            let zero_ptr = bin.build_alloca(function, bin.address_type(), "zero_ptr");

            bin.builder.build_store(zero_ptr, zero_address).unwrap();
            let cmp_result = bin
                .builder
                .build_call(
                    bin.module.get_function("__memcmp").unwrap(),
                    &[
                        address.into(),
                        bin.context
                            .i32_type()
                            .const_int(bin.ns.address_length as u64, false)
                            .into(),
                        zero_ptr.into(),
                        bin.context
                            .i32_type()
                            .const_int(bin.ns.address_length as u64, false)
                            .into(),
                    ],
                    "address_zero_cmp",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            // __memcmp returns true (1) if memory regions are equal, false (0) if not equal
            // We want success to be 0 if address equals zero (creation failed), 1 if address is non-zero (creation succeeded)
            // So we need to invert the result: if __memcmp returns 1 (equal), we want success to be 0 (failure)
            let success_value = bin
                .builder
                .build_select(
                    bin.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_result,
                            bin.context.bool_type().const_int(1, false),
                            "address_is_zero",
                        )
                        .unwrap(),
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_int(1, false),
                    "success_value",
                )
                .unwrap();
            *success = success_value.into();
        }
    }

    /// call external function
    fn external_call<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<BasicValueEnum<'b>>,
        contract_args: ContractArgs<'b>,
        ty: CallTy,
        loc: Loc,
    ) {
        emit_context!(bin);

        let return_data_len = bin
            .builder
            .build_alloca(bin.llvm_type(&ast::Type::Uint(32)), "return_data_len")
            .unwrap();

        let name = match ty {
            CallTy::Regular => "call_contract",
            CallTy::Delegate => "delegate_call_contract",
            CallTy::Static => "static_call_contract",
        };

        let mut args: Vec<BasicMetadataValueEnum> =
            vec![address.unwrap().into(), payload.into(), payload_len.into()];

        if matches!(ty, CallTy::Regular) {
            let value = bin.builder.build_alloca(bin.value_type(), "value").unwrap();
            bin.builder
                .build_store(value, contract_args.value.unwrap())
                .unwrap();
            // smoelius: Value is little-endian and must be byte-swapped.
            let temp = bin.builder.build_alloca(bin.value_type(), "value").unwrap();
            call!(
                "__leNtobeN",
                &[
                    value.into(),
                    temp.into(),
                    i32_const!(bin.ns.value_length as u64).into()
                ]
            );
            args.push(temp.into())
        }

        let gas = gas_calculation(bin, contract_args.gas.unwrap());

        args.extend_from_slice(&[gas.into(), return_data_len.into()]);

        // smoelius: From: https://github.com/OffchainLabs/stylus-sdk-rs/blob/a9d54f5fac69c5dda3ee2fae0562aaefee5c2aad/src/hostio.rs#L77-L78
        // > The return status indicates whether the call succeeded, and is nonzero on failure.
        let status = call!(name, &args, "external call");

        let temp = bin
            .builder
            .build_load(bin.context.i32_type(), return_data_len, "return_data_len")
            .unwrap();
        bin.builder
            .build_store(bin.return_data_len.unwrap().as_pointer_value(), temp)
            .unwrap();

        if let Some(success) = success {
            // smoelius: `status` is a `u8`, but we need an `i32`. Also, as per the comment above, we
            // need to map 0 to 1, and non-zero to 0.
            let status_inverted = status_inverted(
                bin,
                status.try_as_basic_value().left().unwrap().into_int_value(),
            );

            *success = status_inverted.into();
        }
    }

    /// send value to address
    fn value_transfer<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
        loc: Loc,
    ) {
        unimplemented!()
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        bin: &Binary<'b>,
        expr: &Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        emit_context!(bin);

        match expr {
            Expression::Builtin {
                kind: Builtin::Balance,
                args,
                ..
            } => {
                let address = expression(self, bin, &args[0], vartab, function).into_array_value();

                let address_ptr = bin
                    .builder
                    .build_alloca(bin.address_type(), "address")
                    .unwrap();

                bin.builder.build_store(address_ptr, address).unwrap();

                let balance = bin
                    .builder
                    .build_alloca(bin.value_type(), "balance")
                    .unwrap();

                call!(
                    "account_balance",
                    &[address_ptr.into(), balance.into()],
                    "account_balance"
                );

                // smoelius: Balance is big-endian and must be byte-swapped.
                let temp = bin.builder.build_alloca(bin.value_type(), "hash").unwrap();

                call!(
                    "__beNtoleN",
                    &[
                        balance.into(),
                        temp.into(),
                        i32_const!(bin.ns.value_length as u64).into()
                    ]
                );

                bin.builder
                    .build_load(bin.value_type(), temp, "balance")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::BaseFee,
                ..
            } => {
                let basefee = bin
                    .builder
                    .build_array_alloca(bin.context.i8_type(), i32_const!(256), "basefee")
                    .unwrap();

                call!("block_basefee", &[basefee.into()], "block_basefee");

                bin.builder
                    .build_load(bin.value_type(), basefee, "basefee")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::BlockCoinbase,
                ..
            } => {
                let coinbase = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(bin.ns.address_length as u64),
                        "coinbase",
                    )
                    .unwrap();

                call!("block_coinbase", &[coinbase.into()], "block_coinbase");

                bin.builder
                    .build_load(bin.address_type(), coinbase, "coinbase")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::BlockNumber,
                ..
            } => {
                let number = call!("block_number", &[], "block_number")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                number.into()
            }
            Expression::Builtin {
                kind: Builtin::ChainId,
                ..
            } => {
                let chainid = call!("chainid", &[], "chainid")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                chainid.into()
            }
            Expression::Builtin {
                kind: Builtin::ContractCode,
                args,
                ..
            } => {
                let address = expression(self, bin, &args[0], vartab, function).into_array_value();

                let address_ptr = bin
                    .builder
                    .build_alloca(bin.address_type(), "address")
                    .unwrap();

                bin.builder.build_store(address_ptr, address).unwrap();

                let size = call!(
                    "account_code_size",
                    &[address_ptr.into()],
                    "account_code_size"
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

                let account_code = bin
                    .builder
                    .build_array_alloca(bin.context.i8_type(), size, "account_code")
                    .unwrap();

                call!(
                    "account_code",
                    &[
                        address_ptr.into(),
                        i32_zero!().into(),
                        size.into(),
                        account_code.into(),
                    ],
                    "account_code"
                );

                call!(
                    "vector_new",
                    &[size.into(), i32_const!(1).into(), account_code.into()]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::ContractCodehash,
                args,
                ..
            } => {
                let address = expression(self, bin, &args[0], vartab, function).into_array_value();

                let address_ptr = bin
                    .builder
                    .build_alloca(bin.address_type(), "address")
                    .unwrap();

                bin.builder.build_store(address_ptr, address).unwrap();

                let ty = bin.context.custom_width_int_type(256);

                let digest_ptr = bin.builder.build_alloca(ty, "digest").unwrap();

                call!(
                    "account_codehash",
                    &[address_ptr.into(), digest_ptr.into()],
                    "account_codehash"
                );

                call!(
                    "vector_new",
                    &[
                        i32_const!(32).into(),
                        i32_const!(1).into(),
                        digest_ptr.into()
                    ]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::Gasleft,
                ..
            } => {
                let gasleft = call!("evm_gas_left", &[], "evm_gas_left")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                gasleft.into()
            }
            Expression::Builtin {
                kind: Builtin::GasLimit,
                ..
            } => {
                let gaslimit = call!("block_gas_limit", &[], "block_gas_limit")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                gaslimit.into()
            }
            Expression::Builtin {
                kind: Builtin::GetAddress,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(bin.ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("contract_address", &[address.into()], "contract_address");

                address.into()
            }
            Expression::Builtin {
                kind: Builtin::Calldata,
                ..
            } => {
                let args = bin
                    .builder
                    .build_load(
                        bin.context.ptr_type(AddressSpace::default()),
                        bin.args.unwrap().as_pointer_value(),
                        "args",
                    )
                    .unwrap();
                let args_len = bin
                    .builder
                    .build_load(
                        bin.context.i32_type(),
                        bin.args_len.unwrap().as_pointer_value(),
                        "args_len",
                    )
                    .unwrap();
                let v = bin.vector_new(
                    args_len.into_int_value(),
                    bin.context.i32_type().const_int(1, false),
                    None,
                    &Type::Uint(8),
                );
                let dest = bin.vector_bytes(v);
                call!("__memcpy", &[dest.into(), args.into(), args_len.into()]);
                v
            }
            Expression::Builtin {
                kind: Builtin::Gasprice,
                ..
            } => {
                let gasprice = bin
                    .builder
                    .build_alloca(bin.value_type(), "gasprice")
                    .unwrap();

                call!("tx_gas_price", &[gasprice.into()], "tx_gas_price");

                // smoelius: `gasprice` is big-endian and must be byte-swapped.
                let temp = bin.builder.build_alloca(bin.value_type(), "hash").unwrap();

                call!(
                    "__beNtoleN",
                    &[
                        gasprice.into(),
                        temp.into(),
                        i32_const!(bin.ns.value_length as u64).into()
                    ]
                );

                bin.builder
                    .build_load(bin.value_type(), temp, "balance")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::Origin,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(bin.ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("tx_origin", &[address.into()], "tx_origin");

                bin.builder
                    .build_load(bin.address_type(), address, "tx_origin")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::Sender,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(bin.ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("msg_sender", &[address.into()], "msg_sender");

                bin.builder
                    .build_load(bin.address_type(), address, "caller")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::Timestamp,
                ..
            } => {
                let timestamp = call!("block_timestamp", &[], "block_timestamp")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                timestamp.into()
            }
            Expression::Builtin {
                kind: Builtin::Value,
                ..
            } => self.value_transferred(bin).into(),
            _ => unimplemented!("{expr:?}"),
        }
    }

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        emit_context!(bin);

        // smoelius: To test `return_data_size`, change `any()` to `all()`.
        let size = if cfg!(any()) {
            call!("return_data_size", &[], "return_data_size")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            bin.builder
                .build_load(
                    bin.context.i32_type(),
                    bin.return_data_len.unwrap().as_pointer_value(),
                    "return_data_len",
                )
                .unwrap()
                .into_int_value()
        };

        let return_data = bin
            .builder
            .build_array_alloca(bin.context.i8_type(), size, "return_data")
            .unwrap();

        call!(
            "read_return_data",
            &[return_data.into(), i32_zero!().into(), size.into()],
            "read_return_data"
        );

        call!(
            "vector_new",
            &[size.into(), i32_const!(1).into(), return_data.into()]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
    }

    /// Return the value we received
    fn value_transferred<'b>(&self, bin: &Binary<'b>) -> IntValue<'b> {
        emit_context!(bin);

        let value = bin.builder.build_alloca(bin.value_type(), "value").unwrap();

        call!("msg_value", &[value.into()], "value_transferred");

        // smoelius: `value` is big-endian and must be byte-swapped.
        let temp = bin.builder.build_alloca(bin.value_type(), "value").unwrap();

        call!(
            "__beNtoleN",
            &[
                value.into(),
                temp.into(),
                i32_const!(bin.ns.value_length as u64).into()
            ]
        );

        bin.builder
            .build_load(bin.value_type(), temp, "value")
            .unwrap()
            .into_int_value()
    }

    /// Terminate execution, destroy bin and send remaining funds to addr
    fn selfdestruct<'b>(&self, bin: &Binary<'b>, addr: ArrayValue<'b>) {
        unimplemented!()
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
        emit_context!(bin);

        const FNAME: &str = "native_keccak256";
        const HASHLEN: u64 = 32;

        if hash != HashTy::Keccak256 {
            unimplemented!("{hash:?}");
        }

        let res = bin
            .builder
            .build_array_alloca(bin.context.i8_type(), i32_const!(HASHLEN), "res")
            .unwrap();

        call!(FNAME, &[input.into(), input_len.into(), res.into()], "hash");

        // bytes32 needs to reverse bytes
        let temp = bin
            .builder
            .build_alloca(bin.llvm_type(&ast::Type::Bytes(HASHLEN as u8)), "hash")
            .unwrap();

        call!(
            "__beNtoleN",
            &[res.into(), temp.into(), i32_const!(HASHLEN).into()]
        );

        bin.builder
            .build_load(
                bin.llvm_type(&ast::Type::Bytes(HASHLEN as u8)),
                temp,
                "hash",
            )
            .unwrap()
            .into_int_value()
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        topics: &[BasicValueEnum<'b>],
    ) {
        emit_context!(bin);

        let len = bin.vector_len(data);
        let data = bin.vector_bytes(data);

        let topic_count = topics.len();
        let topic_size = bin
            .builder
            .build_int_add(i32_const!(32 * topic_count as u64), len, "topic_size")
            .unwrap();

        let topic_buf = if topic_count > 0 {
            // the topic buffer is a vector of hashes.
            // smoelius: Followed by `data`.
            let topic_buf = bin
                .builder
                .build_array_alloca(bin.context.i8_type(), topic_size, "topic")
                .unwrap();

            let mut dest = unsafe {
                bin.builder
                    .build_gep(bin.context.i8_type(), topic_buf, &[i32_const!(0)], "dest")
                    .unwrap()
            };

            for topic in topics.iter() {
                call!(
                    "__memcpy",
                    &[
                        dest.into(),
                        bin.vector_bytes(*topic).into(),
                        bin.vector_len(*topic).into(),
                    ]
                );

                dest = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), dest, &[i32_const!(32)], "dest")
                        .unwrap()
                };
            }

            call!("__memcpy", &[dest.into(), data.into(), len.into()]);

            topic_buf
        } else {
            ptr!().const_null()
        };

        call!(
            "emit_log",
            &[
                topic_buf.into(),
                topic_size.into(),
                bin.context
                    .i32_type()
                    .const_int(topics.len() as u64, false)
                    .into()
            ]
        );
    }

    /// Return ABI encoded data
    fn return_abi_data<'b>(
        &self,
        bin: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        emit_context!(bin);

        call!("write_result", &[data.into(), data_len.into()]);

        let zero: &dyn BasicValue = &i32_zero!();
        bin.builder.build_return(Some(zero)).unwrap();
    }
}

use local::{gas_calculation, next_slot, ptr_plus_offset, status_inverted};

mod local {
    #![warn(unused_variables)]

    use super::*;
    use inkwell::IntPredicate;

    pub fn gas_calculation<'a>(bin: &Binary<'a>, gas_value: IntValue<'a>) -> IntValue<'a> {
        if_zero(
            bin,
            bin.context.i64_type(),
            gas_value,
            bin.context.i64_type().const_all_ones(),
            gas_value,
        )
    }

    pub fn status_inverted<'a>(bin: &Binary<'a>, status: IntValue<'a>) -> IntValue<'a> {
        if_zero(
            bin,
            bin.context.i8_type(),
            status,
            bin.context.i32_type().const_int(1, false),
            bin.context.i32_type().const_zero(),
        )
    }

    fn if_zero<'a>(
        bin: &Binary<'a>,
        input_ty: IntType<'a>,
        input: IntValue<'a>,
        zero_output: IntValue<'a>,
        non_zero_output: IntValue<'a>,
    ) -> IntValue<'a> {
        let is_zero = bin
            .builder
            .build_int_compare(IntPredicate::EQ, input, input_ty.const_zero(), "is_zero")
            .unwrap();

        bin.builder
            .build_select(is_zero, zero_output, non_zero_output, "selection")
            .unwrap()
            .into_int_value()
    }

    pub fn next_slot<'a>(
        bin: &Binary<'a>,
        value_ptr: PointerValue<'a>,
        length: u32,
    ) -> IntValue<'a> {
        emit_context!(bin);

        let ty = bin.value_type();

        let digest_ptr = bin.builder.build_alloca(ty, "digest").unwrap();

        call!(
            "native_keccak256",
            &[
                value_ptr.into(),
                i32_const!(length as u64).into(),
                digest_ptr.into()
            ]
        );

        bin.builder
            .build_load(ty, digest_ptr, "digest")
            .unwrap()
            .into_int_value()
    }

    pub fn ptr_plus_offset<'a>(
        bin: &Binary<'a>,
        ptr: PointerValue<'a>,
        offset: IntValue<'a>,
    ) -> PointerValue<'a> {
        let ptr_as_int = bin
            .builder
            .build_ptr_to_int(ptr, offset.get_type(), "ptr_as_int")
            .unwrap();
        let ptr_as_int_plus_offset = bin
            .builder
            .build_int_add(ptr_as_int, offset, "ptr_as_int_plus_offset")
            .unwrap();
        bin.builder
            .build_int_to_ptr(ptr_as_int_plus_offset, ptr.get_type(), "ptr_plus_offset")
            .unwrap()
    }
}
