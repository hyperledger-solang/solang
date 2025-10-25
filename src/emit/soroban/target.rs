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
use crate::sema::ast::{Function, Type};

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
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        todo!()
    }

    fn storage_load(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        let storage_type = storage_type_to_int(storage_type);
        emit_context!(bin);

        let slot = if slot.is_const() {
            slot.as_basic_value_enum()
                .into_int_value()
                .const_cast(bin.context.i64_type(), false)
        } else {
            *slot
        };

        // If we are loading a struct, we need to load each field separately and put it in a buffer of this format: [ field1, field2, ... ] where each field is a Soroban tagged value of type i64
        // We loop over each field, call GetContractData for each field and put it in the buffer
        if let Type::Struct(ast::StructType::UserDefined(n)) = ty {
            let field_count = &bin.ns.structs[*n].fields.len();

            // call soroban_get_fields to get a buffer with all fields
            let struct_buffer =
                soroban_get_fields_to_val_buffer(bin, function, slot, *field_count, storage_type);

            return struct_buffer.as_basic_value_enum();
        }

        // === Call HasContractData ===
        let has_data_val = call!(
            HostFunctions::HasContractData.name(),
            &[
                slot.into(),
                bin.context.i64_type().const_int(storage_type, false).into(),
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

        // === Use helper to check if it's true ===
        let condition = is_val_true(bin, has_data_val);

        // === Prepare blocks ===
        let parent = function;
        let then_bb = bin.context.append_basic_block(parent, "has_data");
        let else_bb = bin.context.append_basic_block(parent, "no_data");
        let merge_bb = bin.context.append_basic_block(parent, "merge");

        bin.builder
            .build_conditional_branch(condition, then_bb, else_bb)
            .unwrap();

        // === THEN block: call GetContractData ===
        bin.builder.position_at_end(then_bb);
        let value_from_contract = call!(
            HostFunctions::GetContractData.name(),
            &[
                slot.into(),
                bin.context.i64_type().const_int(storage_type, false).into(),
            ]
        )
        .try_as_basic_value()
        .left()
        .unwrap();
        bin.builder.build_unconditional_branch(merge_bb).unwrap();
        let then_value = value_from_contract;

        // === ELSE block: return default ===
        bin.builder.position_at_end(else_bb);
        let default_value = type_to_tagged_zero_val(bin, ty);

        bin.builder.build_unconditional_branch(merge_bb).unwrap();

        // === MERGE block with phi node ===
        bin.builder.position_at_end(merge_bb);
        let phi = bin
            .builder
            .build_phi(bin.context.i64_type(), "storage_result")
            .unwrap();
        phi.add_incoming(&[(&then_value, then_bb), (&default_value, else_bb)]);

        phi.as_basic_value()
    }

    /// Recursively store a type to storage
    fn storage_store(
        &self,
        bin: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        slot: &mut IntValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        storage_type: &Option<StorageType>,
    ) {
        emit_context!(bin);

        let storage_type = storage_type_to_int(storage_type);

        let function_value = bin
            .module
            .get_function(HostFunctions::PutContractData.name())
            .unwrap();
        let slot = if slot.is_const() {
            slot.as_basic_value_enum()
                .into_int_value()
                .const_cast(bin.context.i64_type(), false)
        } else {
            *slot
        };

        // In case of struct, we receive a buffer in that format: [ field1, field2, ... ] where each field is a Soroban tagged value of type i64
        // therefore, for each field, we need to extract it from the buffer and call PutContractData for each field separately
        if let Type::Struct(ast::StructType::UserDefined(n)) = ty {
            let field_count = &bin.ns.structs[*n].fields.len();

            let data_ptr = bin.vector_bytes(dest);

            // call soroban_put_fields for each field
            soroban_put_fields_from_val_buffer(
                bin,
                function,
                slot,
                data_ptr,
                *field_count,
                storage_type,
            );
            return;
        }

        let value = bin
            .builder
            .build_call(
                function_value,
                &[
                    slot.into(),
                    dest.into(),
                    bin.context.i64_type().const_int(storage_type, false).into(),
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
        unimplemented!()
    }

    fn storage_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        let vec_new = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VectorNew.name())
                    .unwrap(),
                &[],
                "vec_new",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // push the slot to the vector as U32Val
        let slot_encoded = encode_value(
            if slot.get_type().get_bit_width() == 64 {
                slot
            } else {
                bin.builder
                    .build_int_z_extend(slot, bin.context.i64_type(), "slot64")
                    .unwrap()
            },
            32,
            4,
            bin,
        );
        let res = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VecPushBack.name())
                    .unwrap(),
                &[vec_new.as_basic_value_enum().into(), slot_encoded.into()],
                "push",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // push the index to the vector
        let res = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VecPushBack.name())
                    .unwrap(),
                &[res.as_basic_value_enum().into(), index.into()],
                "push",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        res
    }

    fn storage_push(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
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
    ) {
        unimplemented!()
    }

    /// Prints a string
    /// TODO: Implement this function, with a call to the `log` function in the Soroban runtime.
    fn print<'b>(&self, bin: &Binary<'b>, string: PointerValue<'b>, length: IntValue<'b>) {
        let msg_pos = bin
            .builder
            .build_ptr_to_int(string, bin.context.i64_type(), "msg_pos")
            .unwrap();

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
                payload_len.get_type().const_int(8, false),
                "args_len",
            )
            .unwrap();

        let args_len = bin
            .builder
            .build_int_sub(
                args_len,
                args_len.get_type().const_int(1, false),
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
    fn value_transferred<'b>(&self, bin: &Binary<'b>) -> IntValue<'b> {
        unimplemented!()
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
        string: PointerValue<'b>,
        length: IntValue<'b>,
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
        bin: &Binary<'b>,
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

fn encode_value<'a>(
    mut value: IntValue<'a>,
    shift: u64,
    add: u64,
<<<<<<< HEAD
    bin: &Binary<'a>,
=======
    bin: &'a Binary,
>>>>>>> 858952288164aeca78aef0c7b59ca971b76c3e63
) -> IntValue<'a> {
    match value.get_type().get_bit_width() {
        32 =>
        // extend to 64 bits
        {
            value = bin
                .builder
                .build_int_z_extend(value, bin.context.i64_type(), "temp")
                .unwrap();
        }
        64 => (),
        _ => unreachable!(),
    }

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

fn is_val_true<'ctx>(bin: &Binary<'ctx>, val: IntValue<'ctx>) -> IntValue<'ctx> {
    let tag_mask = bin.context.i64_type().const_int(0xff, false);
    let tag_true = bin.context.i64_type().const_int(1, false);

    let tag = bin
        .builder
        .build_and(val, tag_mask, "val_tag")
        .expect("build_and failed");

    bin.builder
        .build_int_compare(inkwell::IntPredicate::EQ, tag, tag_true, "is_val_true")
        .expect("build_int_compare failed")
}

/// Returns a Val representing a default zero value with the correct Soroban Tag.
pub fn type_to_tagged_zero_val<'ctx>(bin: &Binary<'ctx>, ty: &Type) -> IntValue<'ctx> {
    let context = &bin.context;
    let i64_type = context.i64_type();

    // Tag definitions from CAP-0046
    let tag = match ty {
        Type::Bool => 0,        // Tag::False
        Type::Uint(32) => 4,    // Tag::U32Val
        Type::Int(32) => 5,     // Tag::I32Val
        Type::Uint(64) => 6,    // Tag::U64Small
        Type::Int(64) => 7,     // Tag::I64Small
        Type::Uint(128) => 10,  // Tag::U128Small
        Type::Int(128) => 11,   // Tag::I128Small
        Type::Uint(256) => 12,  // Tag::U256Small
        Type::Int(256) => 13,   // Tag::I256Small
        Type::String => 73,     // Tag::StringObject
        Type::Address(_) => 77, // Tag::AddressObject
        Type::Void => 2,        // Tag::Void
        _ => {
            // Fallback to Void for unsupported types
            2 // Tag::Void
        }
    };

    // All zero body + tag in lower 8 bits
    let tag_val: u64 = tag;
    i64_type.const_int(tag_val, false)
}

/// Given a linear-memory buffer of consecutive Soroban Val-encoded i64 fields
/// [field0, field1, ...], push a field index onto `base_key_vec` and call
/// PutContractData for each field separately.
///
/// - `base_key_vec`: i64 Val for a Soroban Vector key (e.g., [slot, mapping_index])
/// - `buffer_ptr`: pointer to the first byte of the buffer (i8*)
/// - `field_count`: number of 64-bit Val entries in the buffer
/// - `storage_type`: Soroban storage type tag (Temporary/Persistent/Instance)
pub fn soroban_put_fields_from_val_buffer<'a>(
    bin: &Binary<'a>,
    _function: FunctionValue<'a>,
    base_key_vec: IntValue<'a>,
    buffer_ptr: PointerValue<'a>,
    field_count: usize,
    storage_type: u64,
) {
    emit_context!(bin);

    let i64_t = bin.context.i64_type();

    for i in 0..field_count {
        // Compute pointer to field i: buffer_ptr + i * 8
        let byte_offset = i64_t.const_int(i as u64, false);
        let field_byte_ptr = unsafe {
            bin.builder
                .build_gep(i64_t, buffer_ptr, &[byte_offset], "field_byte_ptr")
                .unwrap()
        };

        // Cast to i64* and load the Val-encoded i64
        let field_val_i64 = bin
            .builder
            .build_load(i64_t, field_byte_ptr, "field_val_i64")
            .unwrap()
            .into_int_value();

        // Extend key with field index: push U32Val(i)
        let idx_u32 = bin.context.i32_type().const_int(i as u64, false);
        let idx_val = encode_value(idx_u32, 32, 4, bin);
        let field_key_vec = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VecPushBack.name())
                    .unwrap(),
                &[base_key_vec.into(), idx_val.into()],
                "key_push_field",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // Store this field value under the extended key
        // storage_type is a plain u64 here
        let storage_ty_val = bin.context.i64_type().const_int(storage_type, false);
        let _ = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::PutContractData.name())
                    .unwrap(),
                &[
                    field_key_vec.into(),
                    field_val_i64.into(),
                    storage_ty_val.into(),
                ],
                "put_field_from_buffer",
            )
            .unwrap();
    }
}

/// Fetch each field value from storage using `base_key_vec` extended with the field index,
/// and write them into a freshly allocated linear buffer as consecutive i64 Soroban Vals.
/// Returns a pointer to `struct.vector` whose payload size is `field_count * 8` bytes.
pub fn soroban_get_fields_to_val_buffer<'a>(
    bin: &Binary<'a>,
    function: FunctionValue<'a>,
    base_key_vec: IntValue<'a>,
    field_count: usize,
    storage_type: u64,
) -> PointerValue<'a> {
    emit_context!(bin);

    // Allocate zero-initialized buffer
    let size_bytes = bin
        .context
        .i32_type()
        .const_int((field_count as u64) * 8, false);

    let vec_ptr = bin
        .builder
        .build_call(
            bin.module.get_function("soroban_malloc").unwrap(),
            &[size_bytes.into()],
            "soroban_malloc",
        )
        .unwrap()
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let storage_ty_i64 = bin.context.i64_type().const_int(storage_type, false);

    for i in 0..field_count {
        // key = base_key_vec ++ U32Val(i)
        let idx_u32 = bin.context.i32_type().const_int(i as u64, false);
        let idx_val = encode_value(idx_u32, 32, 4, bin);
        let field_key_vec = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::VecPushBack.name())
                    .unwrap(),
                &[base_key_vec.into(), idx_val.into()],
                "key_push_field",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // has = HasContractData(key, storage_type)
        let has_val = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::HasContractData.name())
                    .unwrap(),
                &[field_key_vec.into(), storage_ty_i64.into()],
                "has_field",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        let cond = is_val_true(bin, has_val);

        // Blocks
        let then_bb = bin.context.append_basic_block(function, "load_field");
        let else_bb = bin.context.append_basic_block(function, "skip_field");
        let cont_bb = bin.context.append_basic_block(function, "cont_field");

        bin.builder
            .build_conditional_branch(cond, then_bb, else_bb)
            .unwrap();

        // THEN: fetch and store val into buffer[i]
        bin.builder.position_at_end(then_bb);
        let val_i64 = bin
            .builder
            .build_call(
                bin.module
                    .get_function(HostFunctions::GetContractData.name())
                    .unwrap(),
                &[field_key_vec.into(), storage_ty_i64.into()],
                "get_field",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        let idx64 = bin.context.i64_type().const_int((i) as u64, false);
        let elem_ptr = unsafe {
            bin.builder
                .build_gep(bin.context.i64_type(), vec_ptr, &[idx64], "elem_ptr")
                .unwrap()
        };
        bin.builder.build_store(elem_ptr, val_i64).unwrap();
        bin.builder.build_unconditional_branch(cont_bb).unwrap();

        // ELSE: leave zero (already zero-initialized)
        bin.builder.position_at_end(else_bb);
        bin.builder.build_unconditional_branch(cont_bb).unwrap();

        // CONT
        bin.builder.position_at_end(cont_bb);
    }

    //bin.vector_bytes(  vec_ptr.as_basic_value_enum())
    vec_ptr
}
