// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::HashTy;
use crate::emit::binary::Binary;
use crate::emit::expression::expression;
use crate::emit::storage::StorageSlot;
use crate::emit::substrate::{event_id, SubstrateTarget, SCRATCH_SIZE};
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, Type};
use crate::{codegen, emit_context};
use inkwell::types::{BasicType, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, CallableValue, FunctionValue, IntValue,
    PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use solang_parser::pt;
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for SubstrateTarget {
    fn set_storage_extfunc(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        emit_context!(binary);

        seal_set_storage!(
            cast_byte_ptr!(slot).into(),
            i32_const!(32).into(),
            cast_byte_ptr!(dest).into(),
            dest.get_type()
                .get_element_type()
                .size_of()
                .unwrap()
                .const_cast(binary.context.i32_type(), false)
                .into()
        );
    }

    fn get_storage_extfunc(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        emit_context!(binary);

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

        let ef = call!("__malloc", &[len.into()])
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();
        binary.builder.build_store(scratch_len, len);

        let _exists = call!(
            "seal_get_storage",
            &[
                cast_byte_ptr!(slot).into(),
                i32_const!(32).into(),
                ef.into(),
                scratch_len.into()
            ]
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

        seal_set_storage!(
            cast_byte_ptr!(slot).into(),
            i32_const!(32).into(),
            cast_byte_ptr!(data).into(),
            len.into()
        );

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(delete_block);

        call!(
            "seal_clear_storage",
            &[cast_byte_ptr!(slot).into(), i32_const!(32).into()]
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
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();
        let ty_len = ty.size_of().const_cast(binary.context.i32_type(), false);
        binary.builder.build_store(scratch_len, ty_len);

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                    .ptr_type(AddressSpace::Generic)
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
    ) -> IntValue<'a> {
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                binary.builder.build_load(scratch_len, "string_len"),
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
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.scratch.unwrap().as_pointer_value(),
                &[i32_zero!(), index],
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
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                binary.builder.build_load(scratch_len, "string_len"),
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
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            binary.builder.build_gep(
                binary.scratch.unwrap().as_pointer_value(),
                &[i32_zero!(), index],
                "data_offset",
            )
        };

        // set the result
        binary.builder.build_store(offset, val);

        seal_set_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            length.into()
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
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                binary.builder.build_load(scratch_len, "string_len"),
                i32_zero!().into(),
                "string_length",
            )
            .into_int_value();

        // set the result
        let offset = unsafe {
            binary.builder.build_gep(
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

        seal_set_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            length.into()
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
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                binary.builder.build_load(scratch_len, "string_len"),
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
        self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

        binary.builder.position_at_end(retrieve_block);

        // Set the new length
        let new_length = binary
            .builder
            .build_int_sub(length, i32_const!(1), "new_length");

        let val = if load {
            let offset = unsafe {
                binary.builder.build_gep(
                    binary.scratch.unwrap().as_pointer_value(),
                    &[i32_zero!(), new_length],
                    "data_offset",
                )
            };

            Some(binary.builder.build_load(offset, "popped_value"))
        } else {
            None
        };

        seal_set_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            new_length.into()
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
        emit_context!(binary);

        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(SCRATCH_SIZE as u64));

        let exists = seal_get_storage!(
            cast_byte_ptr!(slot_ptr).into(),
            i32_const!(32).into(),
            scratch_buf.into(),
            scratch_len.into()
        );

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
                binary.builder.build_load(scratch_len, "string_len"),
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
            &[
                cast_byte_ptr!(src).into(),
                length.into(),
                cast_byte_ptr!(dest).into()
            ]
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
        emit_context!(binary);

        // first calculate how much memory we need to allocate
        let mut length = i32_zero!();

        debug_assert_eq!(packed.len() + args.len(), tys.len());

        let mut tys_iter = tys.iter();

        // note that encoded_length return the exact value for packed encoding
        for arg in packed {
            let ty = tys_iter.next().unwrap();

            length = binary.builder.build_int_add(
                length,
                SubstrateTarget::encoded_length(*arg, false, true, ty, function, binary, ns),
                "",
            );
        }

        for arg in args {
            let ty = tys_iter.next().unwrap();

            length = binary.builder.build_int_add(
                length,
                SubstrateTarget::encoded_length(*arg, false, false, ty, function, binary, ns),
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

        let p = call!("__malloc", &[malloc_length.into()])
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
                binary
                    .builder
                    .build_gep(v, &[i32_zero!(), i32_zero!()], "data_len")
            };

            binary.builder.build_store(data_len, length);
        }

        let data_size = unsafe {
            binary
                .builder
                .build_gep(v, &[i32_zero!(), i32_const!(1)], "data_size")
        };

        binary.builder.build_store(data_size, length);

        let data = unsafe {
            binary
                .builder
                .build_gep(v, &[i32_zero!(), i32_const!(2)], "data")
        };

        // now encode each of the arguments
        let data = binary.builder.build_pointer_cast(data, byte_ptr!(), "");

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
                binary
                    .builder
                    .build_gep(v, &[i32_zero!(), i32_zero!()], "data_len")
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
        emit_context!(binary);

        // first calculate how much memory we need to allocate
        let mut length = i32_zero!();

        // note that encoded_length overestimates how data we need
        for (i, ty) in tys.iter().enumerate() {
            length = binary.builder.build_int_add(
                length,
                SubstrateTarget::encoded_length(args[i], load, false, ty, function, binary, ns),
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

        let data = call!("__malloc", &[length.into()])
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
        emit_context!(binary);

        call!(
            "seal_debug_message",
            &[string_ptr.into(), string_len.into()]
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
        emit_context!(binary);

        let created_contract = &ns.contracts[contract_no];

        let constructor = match constructor_no {
            Some(function_no) => &ns.functions[function_no],
            None => &created_contract.default_constructor.as_ref().unwrap().0,
        };

        let (scratch_buf, scratch_len) = scratch_buf!();

        // salt
        let salt_buf =
            binary.build_alloca(function, binary.context.i8_type().array_type(36), "salt");
        let salt_buf = binary
            .builder
            .build_pointer_cast(salt_buf, byte_ptr!(), "salt_buf");
        let salt_len = i32_const!(32);

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

            binary.builder.build_store(scratch_len, i32_const!(36));

            call!(
                "seal_random",
                &[ptr.into(), len.into(), salt_buf.into(), scratch_len.into()],
                "random"
            );
        }

        let tys: Vec<ast::Type> = constructor.params.iter().map(|p| p.ty.clone()).collect();

        // input
        let (input, input_len) = self.abi_encode(
            binary,
            Some(i32_const!(
                u32::from_le_bytes(constructor.selector().try_into().unwrap()) as u64
            )),
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

            binary
                .builder
                .build_store(scratch_len, i32_const!(ns.value_length as u64));

            call!(
                "seal_minimum_balance",
                &[cast_byte_ptr!(value_ptr).into(), scratch_len.into()],
                "minimum_balance"
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
                gas.into(),
                cast_byte_ptr!(value_ptr, "value_transfer").into(),
                input.into(),
                input_len.into(),
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
        _seeds: Option<(PointerValue<'b>, IntValue<'b>)>,
        _ty: ast::CallTy,
        ns: &ast::Namespace,
    ) {
        emit_context!(binary);

        // balance is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");
        binary.builder.build_store(value_ptr, value);

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
                gas.into(),
                cast_byte_ptr!(value_ptr, "value_transfer").into(),
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
                cast_byte_ptr!(value_ptr, "value_transfer").into(),
                i32_const!(ns.value_length as u64).into()
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

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

            self.assert_failure(binary, byte_ptr!().const_null(), i32_zero!());

            binary.builder.position_at_end(success_block);
        }
    }

    fn return_data<'b>(&self, binary: &Binary<'b>, _function: FunctionValue) -> PointerValue<'b> {
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();

        let length = binary.builder.build_load(scratch_len, "string_len");

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
            &[cast_byte_ptr!(value).into(), value_len.into()],
            "value_transferred"
        );

        binary
            .builder
            .build_load(value, "value_transferred")
            .into_int_value()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &ast::Namespace) {
        emit_context!(binary);

        let address = binary
            .builder
            .build_alloca(binary.address_type(ns), "address");

        binary.builder.build_store(address, addr);

        call!(
            "seal_terminate",
            &[cast_byte_ptr!(address).into()],
            "terminated"
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
            &[
                res.into(),
                cast_byte_ptr!(temp).into(),
                i32_const!(hashlen).into()
            ]
        );

        binary.builder.build_load(temp, "hash").into_int_value()
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
                binary
                    .builder
                    .build_gep(topic_buf, &[i32_const!(1)], "dest")
            };

            call!(
                "__bzero8",
                &[
                    cast_byte_ptr!(dest, "dest").into(),
                    i32_const!(topic_count as u64 * 4).into()
                ]
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

                dest = unsafe { binary.builder.build_gep(dest, &[i32_const!(32)], "dest") };
            }

            topic_buf
        } else {
            byte_ptr!().const_null()
        };

        let (data_ptr, data_len) = self.abi_encode(
            binary,
            event_id(binary, contract, event_no),
            false,
            function,
            data,
            data_tys,
            ns,
        );

        call!(
            "seal_deposit_event",
            &[
                topic_buf.into(),
                topic_size.into(),
                data_ptr.into(),
                data_len.into(),
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
                let v = call!(
                    "vector_new",
                    &[
                        binary
                            .builder
                            .build_load(binary.calldata_len.as_pointer_value(), "calldata_len")
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
                call!(
                    "seal_input",
                    &[cast_byte_ptr!(data).into(), scratch_len.into()],
                    "data"
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
                    expression(self, binary, &expr[0], vartab, function, ns).into_int_value()
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
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.address_length as u64));

                call!(
                    "seal_caller",
                    &[scratch_buf.into(), scratch_len.into()],
                    "caller"
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
            codegen::Expression::Builtin(_, _, codegen::Builtin::Random, args) => {
                let subject =
                    expression(self, binary, &args[0], vartab, function, ns).into_pointer_value();

                let subject_data = unsafe {
                    binary
                        .builder
                        .build_gep(subject, &[i32_zero!(), i32_const!(2)], "subject_data")
                };

                let subject_len = unsafe {
                    binary
                        .builder
                        .build_gep(subject, &[i32_zero!(), i32_zero!()], "subject_len")
                };

                let (scratch_buf, scratch_len) = scratch_buf!();

                binary.builder.build_store(scratch_len, i32_const!(36));

                call!(
                    "seal_random",
                    &[
                        cast_byte_ptr!(subject_data, "subject_data").into(),
                        binary.builder.build_load(subject_len, "subject_len").into(),
                        scratch_buf.into(),
                        scratch_len.into()
                    ],
                    "random"
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
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.address_length as u64));

                call!(
                    "seal_address",
                    &[scratch_buf.into(), scratch_len.into()],
                    "address"
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
                let (scratch_buf, scratch_len) = scratch_buf!();

                binary
                    .builder
                    .build_store(scratch_len, i32_const!(ns.value_length as u64));

                call!(
                    "seal_balance",
                    &[scratch_buf.into(), scratch_len.into()],
                    "balance"
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
        _binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        _builtin_func: &Function,
        _args: &[BasicMetadataValueEnum<'a>],
        _ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!()
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
}
