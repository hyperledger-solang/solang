// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::HashTy;
use crate::codegen::Builtin;
use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::soroban::{HostFunctions, SorobanTarget};
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

use num_traits::ToPrimitive;

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
            HostFunctions::GetContractData.name(),
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

        let function_value = binary
            .module
            .get_function(HostFunctions::PutContractData.name())
            .unwrap();

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
                HostFunctions::PutContractData.name(),
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
            let length = length.const_cast(bin.context.i64_type(), false);

            let msg_pos_encoded = encode_value(msg_pos, 32, 4, bin);
            let length_encoded = encode_value(length, 32, 4, bin);

            bin.builder
                .build_call(
                    bin.module
                        .get_function(HostFunctions::LogFromLinearMemory.name())
                        .unwrap(),
                    &[
                        msg_pos_encoded.into(),
                        length_encoded.into(),
                        msg_pos_encoded.into(),
                        encode_value(bin.context.i64_type().const_zero(), 32, 4, bin).into(),
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
        let offset = bin.context.i64_type().const_int(0, false);

        let start = unsafe {
            bin.builder
                .build_gep(
                    bin.context.i64_type().array_type(1),
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

        let args_len = bin
            .builder
            .build_int_unsigned_div(
                payload_len,
                bin.context.i64_type().const_int(8, false),
                "args_len",
            )
            .unwrap();

        let args_len = bin
            .builder
            .build_int_sub(
                args_len,
                bin.context.i64_type().const_int(1, false),
                "args_len",
            )
            .unwrap();

        let args_len_encoded = encode_value(args_len, 32, 4, bin);

        let offset = bin.context.i64_type().const_int(1, false);
        let args_ptr = unsafe {
            bin.builder
                .build_gep(
                    bin.context.i64_type().array_type(1),
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

        let args_ptr_encoded = encode_value(args_ptr_to_int, 32, 4, bin);

        let vec_object = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VectorNewFromLinearMemory.name())
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
                bin.module.get_function(HostFunctions::Call.name()).unwrap(),
                &[address.unwrap().into(), symbol.into(), vec_object.into()],
                "call",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let allocate_i64 = bin
            .builder
            .build_alloca(bin.context.i64_type(), "allocate_i64")
            .unwrap();

        bin.builder.build_store(allocate_i64, call_res).unwrap();

        *bin.return_data.borrow_mut() = Some(allocate_i64);
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
        emit_context!(bin);

        match expr {
            Expression::Builtin {
                kind: Builtin::ExtendTtl,
                args,
                ..
            } => {
                // Get arguments
                // (func $extend_contract_data_ttl (param $k_val i64) (param $t_storage_type i64) (param $threshold_u32_val i64) (param $extend_to_u32_val i64) (result i64))
                assert_eq!(args.len(), 4, "extendTtl expects 4 arguments");
                // SAFETY: We already checked that the length of args is 4 so it is safe to unwrap here
                let slot_no = match args.first().unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                        "Expected slot_no to be of type Expression::NumberLiteral. Actual: {:?}",
                        args.get(1).unwrap()
                    ),
                }
                .to_u64()
                .unwrap();
                let threshold = match args.get(1).unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                        "Expected threshold to be of type Expression::NumberLiteral. Actual: {:?}",
                        args.get(1).unwrap()
                    ),
                }
                .to_u64()
                .unwrap();
                let extend_to = match args.get(2).unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                        "Expected extend_to to be of type Expression::NumberLiteral. Actual: {:?}",
                        args.get(2).unwrap()
                    ),
                }
                .to_u64()
                .unwrap();
                let storage_type = match args.get(3).unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                    "Expected storage_type to be of type Expression::NumberLiteral. Actual: {:?}",
                    args.get(3).unwrap()
                ),
                }
                .to_u64()
                .unwrap();

                // Encode the values (threshold and extend_to)
                // See: https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-01.md#tag-values
                let threshold_u32_val = (threshold << 32) + 4;
                let extend_to_u32_val = (extend_to << 32) + 4;

                // Call the function
                let function_name = HostFunctions::ExtendContractDataTtl.name();
                let function_value = bin.module.get_function(function_name).unwrap();

                let value = bin
                    .builder
                    .build_call(
                        function_value,
                        &[
                            bin.context.i64_type().const_int(slot_no, false).into(),
                            bin.context.i64_type().const_int(storage_type, false).into(),
                            bin.context
                                .i64_type()
                                .const_int(threshold_u32_val, false)
                                .into(),
                            bin.context
                                .i64_type()
                                .const_int(extend_to_u32_val, false)
                                .into(),
                        ],
                        function_name,
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                value.into()
            }
            Expression::Builtin {
                kind: Builtin::ExtendInstanceTtl,
                args,
                ..
            } => {
                // Get arguments
                // (func $extend_contract_data_ttl (param $k_val i64) (param $t_storage_type i64) (param $threshold_u32_val i64) (param $extend_to_u32_val i64) (result i64))
                assert_eq!(args.len(), 2, "extendTtl expects 2 arguments");
                // SAFETY: We already checked that the length of args is 2 so it is safe to unwrap here
                let threshold = match args.first().unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                        "Expected threshold to be of type Expression::NumberLiteral. Actual: {:?}",
                        args.get(1).unwrap()
                    ),
                }
                .to_u64()
                .unwrap();
                let extend_to = match args.get(1).unwrap() {
                    Expression::NumberLiteral { value, .. } => value,
                    _ => panic!(
                        "Expected extend_to to be of type Expression::NumberLiteral. Actual: {:?}",
                        args.get(2).unwrap()
                    ),
                }
                .to_u64()
                .unwrap();

                // Encode the values (threshold and extend_to)
                // See: https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-01.md#tag-values
                let threshold_u32_val = (threshold << 32) + 4;
                let extend_to_u32_val = (extend_to << 32) + 4;

                // Call the function
                let function_name = HostFunctions::ExtendCurrentContractInstanceAndCodeTtl.name();
                let function_value = bin.module.get_function(function_name).unwrap();

                let value = bin
                    .builder
                    .build_call(
                        function_value,
                        &[
                            bin.context
                                .i64_type()
                                .const_int(threshold_u32_val, false)
                                .into(),
                            bin.context
                                .i64_type()
                                .const_int(extend_to_u32_val, false)
                                .into(),
                        ],
                        function_name,
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                value.into()
            }
            _ => unimplemented!("unsupported builtin"),
        }
    }

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        bin.return_data.borrow().unwrap()
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

fn encode_value<'a>(value: IntValue<'a>, shift: u64, add: u64, bin: &'a Binary) -> IntValue<'a> {
    let shifted = bin
        .builder
        .build_left_shift(
            value,
            bin.context.i64_type().const_int(shift, false),
            "temp",
        )
        .unwrap();
    bin.builder
        .build_int_add(
            shifted,
            bin.context.i64_type().const_int(add, false),
            "encoded",
        )
        .unwrap()
}
