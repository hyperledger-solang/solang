use crate::codegen::cfg::HashTy;
use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::soroban::SorobanTarget;
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast;
use crate::sema::ast::{Contract, Function, Namespace, Type};
use inkwell::types::IntType;
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use std::collections::HashMap;

#[allow(unused_variables)] // TODO: Remove when implementing TargetRuntime.
impl<'a> TargetRuntime<'a> for SorobanTarget {
    fn abi_decode<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
        ns: &Namespace,
    ) {
        unimplemented!();
    }

    /// Abi encode with optional four bytes selector. The load parameter should be set if the args are
    /// pointers to data, not the actual data  itself.
    fn abi_encode(
        &self,
        bin: &Binary<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue<'a>,
        args: &[BasicValueEnum<'a>],
        tys: &[Type],
        ns: &Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        unimplemented!();
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[Type],
        ns: &Namespace,
    ) -> PointerValue<'b> {
        unimplemented!();
    }

    fn get_storage_int(
        &self,
        bin: &Binary<'a>,
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
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }

    /// Recursively store a type to storage
    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        slot: &mut IntValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) {
        unimplemented!();
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
        unimplemented!();
    }

    // Bytes and string have special storage layout
    fn set_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        unimplemented!();
    }

    fn get_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        unimplemented!();
    }

    fn get_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &Namespace,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn get_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    fn set_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
    ) {
        unimplemented!();
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
        unimplemented!();
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
        unimplemented!();
    }

    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &Namespace,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!();
    }

    fn storage_array_length(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _elem_ty: &Type,
        _ns: &Namespace,
    ) -> IntValue<'a> {
        unimplemented!();
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
        unimplemented!();
    }

    /// Prints a string
    fn print(&self, bin: &Binary, string: PointerValue, length: IntValue) {
        unimplemented!();
    }

    /// Return success without any result
    fn return_empty_abi(&self, bin: &Binary) {
        unimplemented!();
    }

    /// Return failure code
    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>) {
        unimplemented!();
    }

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, bin: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        unimplemented!();
    }

    /// Return failure without any result
    fn assert_failure<'b>(&self, bin: &'b Binary, data: PointerValue, length: IntValue) {
        unimplemented!();
    }

    fn builtin_function(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &Function,
        args: &[BasicMetadataValueEnum<'a>],
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
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
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
        seeds: Option<(PointerValue<'b>, IntValue<'b>)>,
        ns: &Namespace,
    ) {
        unimplemented!();
    }

    /// call external function
    fn external_call<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        accounts: Option<(PointerValue<'b>, IntValue<'b>)>,
        seeds: Option<(PointerValue<'b>, IntValue<'b>)>,
        ty: ast::CallTy,
        ns: &Namespace,
    ) {
        unimplemented!();
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
    ) {
        unimplemented!();
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
        unimplemented!();
    }

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        unimplemented!();
    }

    /// Return the value we received
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &Namespace) -> IntValue<'b> {
        unimplemented!();
    }

    /// Terminate execution, destroy bin and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &Namespace) {
        unimplemented!();
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
        unimplemented!();
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        bin: &Binary<'b>,
        contract: &Contract,
        function: FunctionValue<'b>,
        event_no: usize,
        data: &[BasicValueEnum<'b>],
        data_tys: &[Type],
        topics: &[BasicValueEnum<'b>],
        topic_tys: &[Type],
        ns: &Namespace,
    ) {
        unimplemented!();
    }

    /// Return ABI encoded data
    fn return_abi_data<'b>(
        &self,
        binary: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        unimplemented!();
    }
}
