// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::HashTy;
use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::soroban::{
    SorobanTarget, CALL, GET_CONTRACT_DATA, LOG_FROM_LINEAR_MEMORY, PUT_CONTRACT_DATA,
    SYMBOL_NEW_FROM_LINEAR_MEMORY, VECTOR_NEW, VECTOR_NEW_FROM_LINEAR_MEMORY,
};
use crate::emit::ContractArgs;
use crate::emit::{TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast;
use crate::sema::ast::CallTy;
use crate::sema::ast::{Function, Namespace, Type};

use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
    PointerValue,
};

use solang_parser::pt::{Loc, StorageType};

use std::collections::HashMap;

// TODO: Implement TargetRuntime for SorobanTarget.
#[allow(unused_variables)]
impl<'a> TargetRuntime<'a> for SorobanTarget {
    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        todo!()
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
        let storage_type = storage_type_to_int(storage_type);
        emit_context!(binary);
        let ret = call!(
            GET_CONTRACT_DATA,
            &[
                slot.as_basic_value_enum()
                    .into_int_value()
                    .const_cast(binary.context.i64_type(), false)
                    .into(),
                binary
                    .context
                    .i64_type()
                    .const_int(storage_type, false)
                    .into(),
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        ret.into()
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
        storage_type: &Option<StorageType>,
    ) {
        emit_context!(binary);

        let storage_type = storage_type_to_int(storage_type);

        let function_value = binary.module.get_function(PUT_CONTRACT_DATA).unwrap();

        let value = binary
            .builder
            .build_call(
                function_value,
                &[
                    slot.as_basic_value_enum()
                        .into_int_value()
                        .const_cast(binary.context.i64_type(), false)
                        .into(),
                    dest.into(),
                    binary
                        .context
                        .i64_type()
                        .const_int(storage_type, false)
                        .into(),
                ],
                PUT_CONTRACT_DATA,
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
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
    /// TODO: Implement this function, with a call to the `log` function in the Soroban runtime.
    fn print(&self, bin: &Binary, string: PointerValue, length: IntValue) {
        if string.is_const() && length.is_const() {
            let msg_pos = bin
                .builder
                .build_ptr_to_int(string, bin.context.i64_type(), "msg_pos")
                .unwrap();
            let msg_pos = msg_pos.const_cast(bin.context.i64_type(), false);

            let length = length.const_cast(bin.context.i64_type(), false);

            let eight = bin.context.i64_type().const_int(8, false);
            let four = bin.context.i64_type().const_int(4, false);
            let zero = bin.context.i64_type().const_int(0, false);
            let thirty_two = bin.context.i64_type().const_int(32, false);

            // XDR encode msg_pos and length
            let msg_pos_encoded = bin
                .builder
                .build_left_shift(msg_pos, thirty_two, "temp")
                .unwrap();
            let msg_pos_encoded = bin
                .builder
                .build_int_add(msg_pos_encoded, four, "msg_pos_encoded")
                .unwrap();

            let length_encoded = bin
                .builder
                .build_left_shift(length, thirty_two, "temp")
                .unwrap();
            let length_encoded = bin
                .builder
                .build_int_add(length_encoded, four, "length_encoded")
                .unwrap();

            let zero_encoded = bin.builder.build_left_shift(zero, eight, "temp").unwrap();

            let eight_encoded = bin.builder.build_left_shift(eight, eight, "temp").unwrap();
            let eight_encoded = bin
                .builder
                .build_int_add(eight_encoded, four, "eight_encoded")
                .unwrap();

            let call_res = bin
                .builder
                .build_call(
                    bin.module.get_function(LOG_FROM_LINEAR_MEMORY).unwrap(),
                    &[
                        msg_pos_encoded.into(),
                        length_encoded.into(),
                        msg_pos_encoded.into(),
                        four.into(),
                    ],
                    "log",
                )
                .unwrap();
        } else {
            todo!("Dynamic String printing is not yet supported")
        }
    }

    /// Return success without any result
    fn return_empty_abi(&self, bin: &Binary) {
        unimplemented!()
    }

    /// Return failure code
    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>) {
        unimplemented!()
    }

    /// Return failure without any result
    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        bin.builder.build_unreachable().unwrap();
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
        println!("PAYLOAD LEN{:?}", payload_len);

        let eight = bin.context.i64_type().const_int(8, false);
        let four = bin.context.i64_type().const_int(4, false);
        let zero = bin.context.i64_type().const_int(0, false);
        let thirty_two = bin.context.i64_type().const_int(32, false);

        //let callee =unsafe{ bin.builder.build_gep(bin.context.i64_type(), payload[0].into_pointer_value(), ordered_indexes, "symbol")};

        let offset = bin.context.i64_type().const_int(0, false);

        let start = unsafe {
            bin.builder
                .build_gep(
                    bin.context.i64_type().array_type(3),
                    payload,
                    &[bin.context.i64_type().const_zero(), offset],
                    "start",
                )
                .unwrap()
        };

        let symbol = bin
            .builder
            .build_load(bin.context.i64_type(), start, "symbol")
            .unwrap()
            .into_int_value();

        /*println!("External call!!!!");
            let callee_name = bin.vector_bytes(payload[0]);

            let callee_pos = bin
                    .builder
                    .build_ptr_to_int(callee_name, bin.context.i64_type(), "msg_pos")
                    .unwrap().const_cast(bin.context.i64_type(), false);

                    let callee_pos_encoded = bin
                    .builder
                    .build_left_shift(callee_pos, thirty_two, "temp")
                    .unwrap();
                let callee_pos_encoded = bin
                    .builder
                    .build_int_add(callee_pos_encoded, four, "callee_pos_encoded")
                    .unwrap();

            println!("Callee name: {:?}", callee_name);

            let callee_len = bin.vector_len(payload[0]).const_cast(bin.context.i64_type(), false);

            let callee_len_encoded = bin
            .builder
            .build_left_shift(callee_len, thirty_two, "temp")
            .unwrap();

            let callee_len_encoded = bin
                    .builder
                    .build_int_add(callee_len_encoded, four, "callee_pos_encoded")
                    .unwrap();

            println!("Callee len: {:?}", callee_len);


            println!("PAYLOAD [1] {:?}", payload[1]);

            let args_ptr = payload[1].into_pointer_value();

            println!("Args ptr: {:?}", args_ptr);

            let args_len = payload_len;
            println!("Args len: {:?}", args_len);

            let args_len = payload_len.const_cast(bin.context.i64_type(), false);

            let args_len_encoded = bin.builder.build_left_shift(args_len, thirty_two, "temp").unwrap();

            let args_len_encoded = bin
                    .builder
                    .build_int_add(args_len_encoded, four, "args_len_encoded")
                    .unwrap();

            let args_ptr_to_int = bin
                    .builder
                    .build_ptr_to_int(args_ptr, bin.context.i64_type(), "args_ptr")
                    .unwrap();

            let args_ptr_encoded = bin. builder.build_left_shift(args_ptr_to_int, thirty_two, "temp").unwrap();
            let args_ptr_encoded = bin
                    .builder
                    .build_int_add(args_ptr_encoded, four, "args_ptr_encoded")
                    .unwrap();


        /*let symbol = bin
            .builder
            .build_call(
                bin.module.get_function(SYMBOL_NEW_FROM_LINEAR_MEMORY).unwrap(),
                &[
                    callee_pos_encoded.into(),
                    callee_len_encoded.into()
                ],
                "symbol",
            )
            .unwrap().try_as_basic_value().left().unwrap().into_int_value();*/



            let vec_object = bin.builder.build_call(
                bin.module.get_function(VECTOR_NEW_FROM_LINEAR_MEMORY).unwrap(),
                &[
                    args_ptr_encoded.into(),
                    args_len_encoded.into()
                ],
                "vec_object",
            ).unwrap().try_as_basic_value().left().unwrap().into_int_value();*/

        let args_len = bin
            .builder
            .build_int_sub(
                payload_len,
                bin.context.i64_type().const_int(1, false),
                "a7a",
            )
            .unwrap();

        let args_len = bin
            .builder
            .build_int_unsigned_div(args_len, eight, "args_len")
            .unwrap();

        let args_len_encoded = bin
            .builder
            .build_left_shift(args_len, thirty_two, "temp")
            .unwrap();

        let args_len_encoded = bin
            .builder
            .build_int_add(args_len_encoded, four, "args_len_encoded")
            .unwrap();

        let offset = bin.context.i64_type().const_int(1, false);
        let args_ptr = unsafe {
            bin.builder
                .build_gep(
                    bin.context.i64_type().array_type(3),
                    payload,
                    &[bin.context.i64_type().const_zero(), offset],
                    "start",
                )
                .unwrap()
        };

        let args_ptr_to_int = bin
            .builder
            .build_ptr_to_int(args_ptr, bin.context.i64_type(), "args_ptr")
            .unwrap();

        let args_ptr_encoded = bin
            .builder
            .build_left_shift(args_ptr_to_int, thirty_two, "temp")
            .unwrap();
        let args_ptr_encoded = bin
            .builder
            .build_int_add(args_ptr_encoded, four, "args_ptr_encoded")
            .unwrap();

        let vec_object = bin
            .builder
            .build_call(
                bin.module
                    .get_function(VECTOR_NEW_FROM_LINEAR_MEMORY)
                    .unwrap(),
                &[args_ptr_encoded.into(), args_len_encoded.into()],
                "vec_object",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let call_res = bin
            .builder
            .build_call(
                bin.module.get_function(CALL).unwrap(),
                &[address.unwrap().into(), symbol.into(), vec_object.into()],
                "call",
            )
            .unwrap();

        //
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
        println!("Return data!!!!");
        bin.context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default())
            .const_null()
        //unimplemented!()
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
        unimplemented!()
    }
}

fn storage_type_to_int(storage_type: &Option<StorageType>) -> u64 {
    if let Some(storage_type) = storage_type {
        match storage_type {
            StorageType::Temporary(_) => 0,
            StorageType::Persistent(_) => 1,
            StorageType::Instance(_) => 2,
        }
    } else {
        1
    }
}
