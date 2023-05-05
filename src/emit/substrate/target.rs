// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{HashTy, ReturnCode};
use crate::codegen::error_msg_with_loc;
use crate::emit::binary::Binary;
use crate::emit::expression::{expression, string_to_basic_value};
use crate::emit::storage::StorageSlot;
use crate::emit::substrate::{log_return_code, SubstrateTarget, SCRATCH_SIZE};
use crate::emit::{ContractArgs, TargetRuntime, Variable};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, Type};
use crate::{codegen, emit_context};
use inkwell::types::{BasicType, BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use solang_parser::pt::Loc;
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for SubstrateTarget {
    fn set_storage_extfunc(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    ) {
        emit_context!(binary);

        let ret = seal_set_storage!(
            slot.into(),
            i32_const!(32).into(),
            dest.into(),
            dest_ty
                .size_of()
                .unwrap()
                .const_cast(binary.context.i32_type(), false)
                .into()
        );

        log_return_code(binary, "seal_set_storage", ret);
    }

    fn get_storage_extfunc(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        emit_context!(binary);

        // This is the size of the external function struct
        let len = ns.address_length + 4;

        let ef = call!(
            "__malloc",
            &[binary
                .context
                .i32_type()
                .const_int(len as u64, false)
                .into()]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();
        binary.builder.build_store(
            scratch_len,
            binary.context.i64_type().const_int(len as u64, false),
        );

        let ret = call!(
            "seal_get_storage",
            &[
                slot.into(),
                i32_const!(32).into(),
                ef.into(),
                scratch_len.into()
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        log_return_code(binary, "seal_get_storage: ", ret);

        // TODO: decide behaviour if not exist

        ef
    }

    fn set_storage_string(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        emit_context!(binary);

        let len = binary.vector_len(dest);
        let data = binary.vector_bytes(dest);

        let exists = binary
            .builder
            .build_int_compare(IntPredicate::NE, len, i32_zero!(), "exists");

        let delete_block = binary.context.append_basic_block(function, "delete_block");

        let set_block = binary.context.append_basic_block(function, "set_block");

        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, set_block, delete_block);

        binary.builder.position_at_end(set_block);

        let ret = seal_set_storage!(slot.into(), i32_const!(32).into(), data.into(), len.into());

        log_return_code(binary, "seal_set_storage", ret);

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(delete_block);

        let ret = call!("seal_clear_storage", &[slot.into(), i32_const!(32).into()])
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        log_return_code(binary, "seal_clear_storage", ret);

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
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();
        let ty_len = ty.size_of().const_cast(binary.context.i32_type(), false);
        binary.builder.build_store(scratch_len, ty_len);

        let exists = seal_get_storage!(
            slot.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage: ", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let entry = binary.builder.get_insert_block().unwrap();
        let retrieve_block = binary.context.append_basic_block(function, "in_storage");
        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        binary.builder.position_at_end(retrieve_block);

        let loaded_int =
            binary
                .builder
                .build_load(ty, binary.scratch.unwrap().as_pointer_value(), "int");

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
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            slot.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage: ", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let ty = binary
            .module
            .get_struct_type("struct.vector")
            .unwrap()
            .ptr_type(AddressSpace::default());

        let entry = binary.builder.get_insert_block().unwrap();

        let retrieve_block = binary
            .context
            .append_basic_block(function, "retrieve_block");

        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        binary.builder.position_at_end(retrieve_block);

        let length =
            binary
                .builder
                .build_load(binary.context.i32_type(), scratch_len, "string_len");

        let loaded_string = call!(
            "vector_new",
            &[length.into(), i32_const!(1).into(), scratch_buf.into()]
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
                    .ptr_type(AddressSpace::default())
                    .const_null(),
                entry,
            ),
        ]);

        res.as_basic_value().into_pointer_value()
    }

    /// Read string from substrate storage
    fn get_storage_bytes_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        loc: Loc,
        ns: &Namespace,
    ) -> IntValue<'a> {
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len"),
                i32_zero!().into(),
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

        self.log_runtime_error(
            binary,
            "storage array index out of bounds".to_string(),
            Some(loc),
            ns,
        );
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.context.i8_type().array_type(SCRATCH_SIZE),
                binary.scratch.unwrap().as_pointer_value(),
                &[i32_zero!(), index],
                "data_offset",
            )
        };

        binary
            .builder
            .build_load(binary.context.i8_type(), offset, "value")
            .into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        binary: &Binary,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
        ns: &Namespace,
        loc: Loc,
    ) {
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len"),
                i32_zero!().into(),
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
        self.log_runtime_error(
            binary,
            "storage index out of bounds".to_string(),
            Some(loc),
            ns,
        );
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.context.i8_type().array_type(SCRATCH_SIZE),
                binary.scratch.unwrap().as_pointer_value(),
                &[i32_zero!(), index],
                "data_offset",
            )
        };

        // set the result
        binary.builder.build_store(offset, val);

        let ret = seal_set_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            length.into()
        );

        log_return_code(binary, "seal_set_storage", ret);
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
        emit_context!(binary);

        let val = val.unwrap();

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        // Since we are going to add one byte, we set the buffer length to one less. This will
        // trap for us if it does not fit, so we don't have to code this ourselves
        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64 - 1));

        let exists = seal_get_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len"),
                i32_zero!().into(),
                "string_length",
            )
            .into_int_value();

        // set the result
        let offset = unsafe {
            binary.builder.build_gep(
                binary.context.i8_type().array_type(SCRATCH_SIZE),
                binary.scratch.unwrap().as_pointer_value(),
                &[i32_zero!(), length],
                "data_offset",
            )
        };

        binary.builder.build_store(offset, val);

        // Set the new length
        let length = binary
            .builder
            .build_int_add(length, i32_const!(1), "new_length");

        let ret = seal_set_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            length.into()
        );

        log_return_code(binary, "seal_set_storage", ret);

        val
    }

    /// Pop a value from a bytes string
    fn storage_pop(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &ast::Namespace,
        loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        let length = binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len"),
                i32_zero!().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range = binary.builder.build_int_compare(
            IntPredicate::NE,
            i32_zero!(),
            length,
            "index_in_range",
        );

        let retrieve_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.log_runtime_error(
            binary,
            "pop from empty storage array".to_string(),
            Some(loc),
            ns,
        );
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        // Set the new length
        let new_length = binary
            .builder
            .build_int_sub(length, i32_const!(1), "new_length");

        let val = if load {
            let offset = unsafe {
                binary.builder.build_gep(
                    binary.context.i8_type().array_type(SCRATCH_SIZE),
                    binary.scratch.unwrap().as_pointer_value(),
                    &[i32_zero!(), new_length],
                    "data_offset",
                )
            };

            Some(
                binary
                    .builder
                    .build_load(binary.llvm_type(ty, ns), offset, "popped_value"),
            )
        } else {
            None
        };

        let ret = seal_set_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            new_length.into()
        );

        log_return_code(binary, "seal_set_storage", ret);

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
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            slot_ptr.into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

        log_return_code(binary, "seal_get_storage", exists);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists,
            i32_zero!(),
            "storage_exists",
        );

        binary
            .builder
            .build_select(
                exists,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len"),
                i32_zero!().into(),
                "string_length",
            )
            .into_int_value()
    }

    fn return_empty_abi(&self, binary: &Binary) {
        emit_context!(binary);

        call!(
            "seal_return",
            &[
                i32_zero!().into(),
                byte_ptr!().const_zero().into(),
                i32_zero!().into()
            ]
        );

        binary.builder.build_unreachable();
    }

    fn return_code<'b>(&self, binary: &'b Binary, _ret: IntValue<'b>) {
        emit_context!(binary);

        // we can't return specific errors
        self.assert_failure(binary, byte_ptr!().const_zero(), i32_zero!());
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
        emit_context!(binary);

        call!(
            "seal_hash_keccak_256",
            &[src.into(), length.into(), dest.into()]
        );
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        emit_context!(binary);

        call!(
            "seal_return",
            &[i32_zero!().into(), data.into(), length.into()]
        );

        binary.builder.build_unreachable();
    }

    fn return_abi_data<'b>(
        &self,
        binary: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        emit_context!(binary);

        call!(
            "seal_return",
            &[i32_zero!().into(), data.into(), data_len.into()]
        );

        binary
            .builder
            .build_return(Some(&binary.return_values[&ReturnCode::Success]));
    }

    fn assert_failure(&self, binary: &Binary, _data: PointerValue, _length: IntValue) {
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

        binary
            .builder
            .build_indirect_call(asm_fn, asm, &[], "unreachable");

        binary.builder.build_unreachable();
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        emit_context!(binary);

        let ret = call!(
            "seal_debug_message",
            &[string_ptr.into(), string_len.into()]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        log_return_code(binary, "seal_debug_message", ret);
    }

    fn create_contract<'b>(
        &mut self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        address: PointerValue<'b>,
        encoded_args: BasicValueEnum<'b>,
        encoded_args_len: BasicValueEnum<'b>,
        contract_args: ContractArgs<'b>,
        ns: &ast::Namespace,
        loc: Loc,
    ) {
        emit_context!(binary);

        let created_contract = &ns.contracts[contract_no];

        let code = created_contract.emit(ns, binary.options);

        let (scratch_buf, scratch_len) = scratch_buf!();

        // salt
        let salt_buf =
            binary.build_alloca(function, binary.context.i8_type().array_type(32), "salt");
        let salt_len = i32_const!(32);

        let salt = contract_args.salt.unwrap_or_else(|| {
            let nonce = call!("instantiation_nonce", &[], "instantiation_nonce_ext")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();
            log_return_code(binary, "instantiation_nonce", nonce);
            let i256_t = binary.context.custom_width_int_type(256);
            binary
                .builder
                .build_int_z_extend_or_bit_cast(nonce, i256_t, "instantiation_nonce")
        });
        binary.builder.build_store(salt_buf, salt);

        let encoded_args = binary.vector_bytes(encoded_args);

        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");

        // balance is a u128, make sure it's enough to cover existential_deposit
        if let Some(value) = contract_args.value {
            binary.builder.build_store(value_ptr, value);
        } else {
            let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

            binary
                .builder
                .build_store(scratch_len, i32_const!(ns.value_length as u64));

            call!(
                "seal_minimum_balance",
                &[value_ptr.into(), scratch_len.into()],
                "minimum_balance"
            );
        }

        // code hash
        let codehash = binary.emit_global_string(
            &format!("binary_{}_codehash", created_contract.name),
            blake2_rfc::blake2b::blake2b(32, &[], &code).as_bytes(),
            true,
        );

        let address_len_ptr = binary
            .builder
            .build_alloca(binary.context.i32_type(), "address_len_ptr");

        binary
            .builder
            .build_store(address_len_ptr, i32_const!(ns.address_length as u64 * 32));

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64 * 32));

        let ret = call!(
            "seal_instantiate",
            &[
                codehash.into(),
                contract_args.gas.unwrap().into(),
                value_ptr.into(),
                encoded_args.into(),
                encoded_args_len.into(),
                address.into(),
                address_len_ptr.into(),
                scratch_buf.into(),
                scratch_len.into(),
                salt_buf.into(),
                salt_len.into(),
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        log_return_code(binary, "seal_instantiate", ret);

        let is_success =
            binary
                .builder
                .build_int_compare(IntPredicate::EQ, ret, i32_zero!(), "success");

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

            self.log_runtime_error(
                binary,
                "contract creation failed".to_string(),
                Some(loc),
                ns,
            );
            self.assert_failure(
                binary,
                scratch_buf,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len")
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
        contract_args: ContractArgs<'b>,
        _ty: ast::CallTy,
        ns: &ast::Namespace,
        loc: Loc,
    ) {
        emit_context!(binary);

        // balance is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");
        binary
            .builder
            .build_store(value_ptr, contract_args.value.unwrap());

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        // do the actual call
        let ret = call!(
            "seal_call",
            &[
                i32_zero!().into(), // TODO implement flags (mostly used for proxy calls)
                address.unwrap().into(),
                contract_args.gas.unwrap().into(),
                value_ptr.into(),
                payload.into(),
                payload_len.into(),
                scratch_buf.into(),
                scratch_len.into(),
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        log_return_code(binary, "seal_call", ret);

        let is_success =
            binary
                .builder
                .build_int_compare(IntPredicate::EQ, ret, i32_zero!(), "success");

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

            self.log_runtime_error(binary, "external call failed".to_string(), Some(loc), ns);
            self.assert_failure(
                binary,
                scratch_buf,
                binary
                    .builder
                    .build_load(binary.context.i32_type(), scratch_len, "string_len")
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
        loc: Loc,
    ) {
        emit_context!(binary);

        // balance is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");
        binary.builder.build_store(value_ptr, value);

        // do the actual call
        let ret = call!(
            "seal_transfer",
            &[
                address.into(),
                i32_const!(ns.address_length as u64).into(),
                value_ptr.into(),
                i32_const!(ns.value_length as u64).into()
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        log_return_code(binary, "seal_transfer", ret);

        let is_success =
            binary
                .builder
                .build_int_compare(IntPredicate::EQ, ret, i32_zero!(), "success");

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

            self.log_runtime_error(binary, "value transfer failure".to_string(), Some(loc), ns);
            self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

            binary.builder.position_at_end(success_block);
        }
    }

    fn return_data<'b>(&self, binary: &Binary<'b>, _function: FunctionValue) -> PointerValue<'b> {
        emit_context!(binary);

        // The `seal_call` syscall leaves the return data in the scratch buffer
        let (scratch_buf, scratch_len) = scratch_buf!();
        let ty = binary.context.i32_type();
        let length = binary.builder.build_load(ty, scratch_len, "scratch_len");
        call!(
            "vector_new",
            &[length.into(), i32_const!(1).into(), scratch_buf.into(),]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
    }

    /// Substrate value is usually 128 bits
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &ast::Namespace) -> IntValue<'b> {
        emit_context!(binary);

        let value = binary.builder.build_alloca(binary.value_type(ns), "value");

        let value_len = binary
            .builder
            .build_alloca(binary.context.i32_type(), "value_len");

        binary
            .builder
            .build_store(value_len, i32_const!(ns.value_length as u64));

        call!(
            "seal_value_transferred",
            &[value.into(), value_len.into()],
            "value_transferred"
        );

        binary
            .builder
            .build_load(binary.value_type(ns), value, "value_transferred")
            .into_int_value()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &ast::Namespace) {
        emit_context!(binary);

        let address = binary
            .builder
            .build_alloca(binary.address_type(ns), "address");

        binary.builder.build_store(address, addr);

        call!("seal_terminate", &[address.into()], "terminated");

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
        emit_context!(binary);

        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("seal_hash_keccak_256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("seal_hash_sha2_256", 32),
            HashTy::Blake2_128 => ("seal_hash_blake2_128", 16),
            HashTy::Blake2_256 => ("seal_hash_blake2_256", 32),
        };

        let res =
            binary
                .builder
                .build_array_alloca(binary.context.i8_type(), i32_const!(hashlen), "res");

        call!(fname, &[input.into(), input_len.into(), res.into()], "hash");

        // bytes32 needs to reverse bytes
        let temp = binary.builder.build_alloca(
            binary.llvm_type(&ast::Type::Bytes(hashlen as u8), ns),
            "hash",
        );

        call!(
            "__beNtoleN",
            &[res.into(), temp.into(), i32_const!(hashlen).into()]
        );

        binary
            .builder
            .build_load(
                binary.llvm_type(&ast::Type::Bytes(hashlen as u8), ns),
                temp,
                "hash",
            )
            .into_int_value()
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        binary: &Binary<'b>,
        _function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        topics: &[BasicValueEnum<'b>],
    ) {
        emit_context!(binary);

        let topic_count = topics.len();
        let topic_size = i32_const!(if topic_count > 0 {
            32 * topic_count as u64 + 1
        } else {
            0
        });

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
                    binary.context.i8_type(),
                    topic_buf,
                    &[i32_const!(1)],
                    "dest",
                )
            };

            call!(
                "__bzero8",
                &[dest.into(), i32_const!(topic_count as u64 * 4).into()]
            );

            for topic in topics.iter() {
                call!(
                    "__memcpy",
                    &[
                        dest.into(),
                        binary.vector_bytes(*topic).into(),
                        binary.vector_len(*topic).into(),
                    ]
                );

                dest = unsafe {
                    binary.builder.build_gep(
                        binary.context.i8_type(),
                        dest,
                        &[i32_const!(32)],
                        "dest",
                    )
                };
            }

            topic_buf
        } else {
            byte_ptr!().const_null()
        };

        call!(
            "seal_deposit_event",
            &[
                topic_buf.into(),
                topic_size.into(),
                binary.vector_bytes(data).into(),
                binary.vector_len(data).into(),
            ]
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
        emit_context!(binary);

        macro_rules! get_seal_value {
            ($name:literal, $func:literal, $width:expr) => {{
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary.builder.build_store(
                    scratch_len,
                    binary
                        .context
                        .i32_type()
                        .const_int($width as u64 / 8, false),
                );

                call!($func, &[scratch_buf.into(), scratch_len.into()], $name);

                binary.builder.build_load(
                    binary.context.custom_width_int_type($width),
                    scratch_buf,
                    $name,
                )
            }};
        }

        match expr {
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Calldata,
                ..
            } => {
                // allocate vector for input
                let v = call!(
                    "vector_new",
                    &[
                        binary
                            .builder
                            .build_load(
                                binary.context.i32_type(),
                                binary.calldata_len.as_pointer_value(),
                                "calldata_len"
                            )
                            .into(),
                        i32_const!(1).into(),
                        binary
                            .builder
                            .build_int_to_ptr(
                                binary.context.i32_type().const_all_ones(),
                                byte_ptr!(),
                                "no_initializer",
                            )
                            .into(),
                    ]
                )
                .try_as_basic_value()
                .left()
                .unwrap();

                let data = unsafe {
                    binary.builder.build_gep(
                        binary.context.get_struct_type("struct.vector").unwrap(),
                        v.into_pointer_value(),
                        &[i32_zero!(), i32_const!(2)],
                        "",
                    )
                };

                let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

                // copy arguments from input buffer
                binary
                    .builder
                    .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

                // retrieve the data
                call!("seal_input", &[data.into(), scratch_len.into()], "data");

                v
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::BlockNumber,
                ..
            } => {
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
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Timestamp,
                ..
            } => {
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
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Gasleft,
                ..
            } => {
                get_seal_value!("gas_left", "seal_gas_left", 64)
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Gasprice,
                args,
                ..
            } => {
                // gasprice is available as "tx.gasprice" which will give you the price for one unit
                // of gas, or "tx.gasprice(uint64)" which will give you the price of N gas units
                let gas = if args.is_empty() {
                    binary.context.i64_type().const_int(1, false)
                } else {
                    expression(self, binary, &args[0], vartab, function, ns).into_int_value()
                };

                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.value_length as u64));

                call!(
                    "seal_weight_to_fee",
                    &[gas.into(), scratch_buf.into(), scratch_len.into()],
                    "gas_price"
                );

                binary.builder.build_load(
                    binary
                        .context
                        .custom_width_int_type(ns.value_length as u32 * 8),
                    scratch_buf,
                    "price",
                )
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Sender,
                ..
            } => {
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.address_length as u64));

                call!(
                    "seal_caller",
                    &[scratch_buf.into(), scratch_len.into()],
                    "caller"
                );

                binary
                    .builder
                    .build_load(binary.address_type(ns), scratch_buf, "caller")
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Value,
                ..
            } => self.value_transferred(binary, ns).into(),
            codegen::Expression::Builtin {
                kind: codegen::Builtin::MinimumBalance,
                ..
            } => {
                get_seal_value!(
                    "minimum_balance",
                    "seal_minimum_balance",
                    ns.value_length as u32 * 8
                )
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::GetAddress,
                ..
            } => {
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.address_length as u64));

                call!(
                    "seal_address",
                    &[scratch_buf.into(), scratch_len.into()],
                    "address"
                );

                binary
                    .builder
                    .build_load(binary.address_type(ns), scratch_buf, "self_address")
            }
            codegen::Expression::Builtin {
                kind: codegen::Builtin::Balance,
                ..
            } => {
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.value_length as u64));

                call!(
                    "seal_balance",
                    &[scratch_buf.into(), scratch_len.into()],
                    "balance"
                );

                binary
                    .builder
                    .build_load(binary.value_type(ns), scratch_buf, "balance")
            }
            _ => unreachable!("{:?}", expr),
        }
    }

    fn storage_load(
        &self,
        binary: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        // The storage slot is an i256 accessed through a pointer, so we need
        // to store it
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");

        self.storage_load_slot(binary, ty, slot, slot_ptr, function, ns)
    }

    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &Type,
        _existing: bool,
        slot: &mut IntValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");

        self.storage_store_slot(binary, ty, slot, slot_ptr, dest, function, ns);
    }

    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot");

        self.storage_delete_slot(bin, ty, slot, slot_ptr, function, ns);
    }

    fn builtin_function(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        builtin_func: &Function,
        args: &[BasicMetadataValueEnum<'a>],
        _first_arg_type: BasicTypeEnum,
        _ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        emit_context!(binary);

        assert_eq!(builtin_func.name, "chain_extension", "unimplemented");

        let input_ptr = binary.vector_bytes(args[1].into_pointer_value().into());
        let input_len = binary.vector_len(args[1].into_pointer_value().into());
        let (output_ptr, output_len_ptr) = scratch_buf!();

        let ret_code = call!(
            "call_chain_extension",
            &[
                args[0].into_int_value().into(),
                input_ptr.into(),
                input_len.into(),
                output_ptr.into(),
                output_len_ptr.into()
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap();
        log_return_code(binary, "call_chain_extension", ret_code.into_int_value());

        let buf_len = binary
            .builder
            .build_load(binary.context.i32_type(), output_len_ptr, "buf_len")
            .into_int_value();
        let buf = call!(
            "vector_new",
            &[buf_len.into(), i32_const!(1).into(), output_ptr.into(),]
        )
        .try_as_basic_value()
        .left()
        .unwrap();

        binary
            .builder
            .build_store(args[2].into_pointer_value(), buf.into_pointer_value());

        ret_code
    }

    fn storage_subscript(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &Type,
        _slot: IntValue<'a>,
        _index: BasicValueEnum<'a>,
        _ns: &Namespace,
    ) -> IntValue<'a> {
        // not needed for slot-based storage chains
        unimplemented!()
    }

    fn log_runtime_error(
        &self,
        bin: &Binary,
        reason_string: String,
        reason_loc: Option<Loc>,
        ns: &Namespace,
    ) {
        if !bin.options.log_runtime_errors {
            return;
        }
        emit_context!(bin);
        let error_with_loc = error_msg_with_loc(ns, &reason_string, reason_loc);
        let custom_error = string_to_basic_value(bin, ns, error_with_loc + ",\n");
        call!(
            "seal_debug_message",
            &[
                bin.vector_bytes(custom_error).into(),
                bin.vector_len(custom_error).into()
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();
    }
}
