// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]

use crate::codegen::cfg::HashTy;
use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::stylus::StylusTarget;
use crate::emit::{ContractArgs, TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast::{self, CallTy};
use crate::sema::ast::{Function, Namespace, Type};
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
    PointerValue,
};
use solang_parser::pt::{Loc, StorageType};
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for StylusTarget {
    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn storage_load(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
        storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        emit_context!(binary);

        let slot_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "slot")
            .unwrap();

        let value_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "value")
            .unwrap();

        binary.builder.build_store(slot_ptr, *slot).unwrap();

        call!("storage_load_bytes32", &[slot_ptr.into(), value_ptr.into()]);

        binary
            .builder
            .build_load(
                binary.context.custom_width_int_type(256),
                value_ptr,
                "value",
            )
            .unwrap()
    }

    /// Recursively store a type to storage
    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        slot: &mut IntValue<'a>,
        value: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
        storage_type: &Option<StorageType>,
    ) {
        emit_context!(binary);

        let slot_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "slot")
            .unwrap();

        let value_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "value")
            .unwrap();

        binary.builder.build_store(slot_ptr, *slot).unwrap();

        binary.builder.build_store(value_ptr, value).unwrap();

        call!(
            "storage_cache_bytes32",
            &[slot_ptr.into(), value_ptr.into()]
        );

        call!("storage_flush_cache", &[i32_const!(1).into()]);
    }

    /// Recursively clear storage. The default implementation is for slot-based storage
    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
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
        unimplemented!()
    }

    fn get_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!()
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
        ns: &Namespace,
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
        ns: &Namespace,
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
        ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
    }

    fn storage_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
        ns: &Namespace,
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
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!()
    }

    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &Namespace,
        loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!()
    }

    fn storage_array_length(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _elem_ty: &Type,
        _ns: &Namespace,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    /// keccak256 hash
    fn keccak256_hash(
        &self,
        bin: &Binary<'a>,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
        ns: &Namespace,
    ) {
        unimplemented!()
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

        self.assert_failure(bin, byte_ptr!().const_zero(), i32_zero!());
    }

    /// Return failure without any result
    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        emit_context!(bin);

        let one: &dyn BasicValue = &i32_const!(1);
        bin.builder.build_return(Some(one)).unwrap();
    }

    fn builtin_function(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &Function,
        args: &[BasicMetadataValueEnum<'a>],
        first_arg_type: Option<BasicTypeEnum>,
        ns: &Namespace,
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
        ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
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
        ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
    }

    /// send value to address
    fn value_transfer<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
        _ns: &Namespace,
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
        ns: &Namespace,
    ) -> BasicValueEnum<'b> {
        unimplemented!()
    }

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        unimplemented!()
    }

    /// Return the value we received
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &Namespace) -> IntValue<'b> {
        unimplemented!()
    }

    /// Terminate execution, destroy bin and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &Namespace) {
        unimplemented!()
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        string: PointerValue<'b>,
        length: IntValue<'b>,
        ns: &Namespace,
    ) -> IntValue<'b> {
        unimplemented!()
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        topics: &[BasicValueEnum<'b>],
    ) {
        unimplemented!()
    }

    /// Return ABI encoded data
    fn return_abi_data<'b>(
        &self,
        binary: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        emit_context!(binary);

        call!("write_result", &[data.into(), data_len.into()]);

        let zero: &dyn BasicValue = &i32_zero!();
        binary.builder.build_return(Some(zero)).unwrap();
    }
}
