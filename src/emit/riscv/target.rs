// SPDX-License-Identifier: Apache-2.0

use crate::emit::binary::Binary;
use crate::emit::{ContractArgs, HashTy, TargetRuntime, Variable};
use crate::sema::ast::{ArrayLength, CallTy, Type};
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum, IntType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};
use num_traits::ToPrimitive;
use solang_parser::pt::Loc;
use solang_parser::pt::StorageType;
use std::collections::HashMap;

pub(crate) struct RiscvTargetRuntime;

/// r55 storage maps 256-bit slots to 256-bit words. Every value type takes
/// one slot; structs and fixed-size arrays take consecutive slots, one per
/// value inside them. Values are stored as solc stores an unpacked value:
///
/// | Solidity type                   | LLVM value  | word in the slot  |
/// |---------------------------------|-------------|-------------------|
/// | `uintN`, `intN`, enum, `bytesN` | `iN`        | zero extended     |
/// | `bool`                          | `i1`        | 0 or 1            |
/// | `address`, contract             | `[20 x i8]` | uint160           |
impl RiscvTargetRuntime {
    fn sload<'a>(bin: &Binary<'a>, slot: IntValue<'a>) -> IntValue<'a> {
        let i256_ty = bin.context.custom_width_int_type(256);
        let key = Self::split_256(slot, bin);

        // The four result limbs, least significant first, are an i256 in
        // little-endian memory.
        let out = bin.builder.build_alloca(i256_ty, "sload_out").unwrap();
        let args: Vec<BasicMetadataValueEnum> = vec![
            key[0].into(),
            key[1].into(),
            key[2].into(),
            key[3].into(),
            out.into(),
        ];
        bin.builder
            .build_call(bin.module.get_function("__sys_sload").unwrap(), &args, "")
            .unwrap();

        bin.builder
            .build_load(i256_ty, out, "sload")
            .unwrap()
            .into_int_value()
    }

    fn sstore<'a>(bin: &Binary<'a>, slot: IntValue<'a>, value: IntValue<'a>) {
        let args: Vec<BasicMetadataValueEnum> = Self::split_256(slot, bin)
            .into_iter()
            .chain(Self::split_256(value, bin))
            .map(Into::into)
            .collect();
        bin.builder
            .build_call(bin.module.get_function("__sys_sstore").unwrap(), &args, "")
            .unwrap();
    }

    /// Split an i256 into four i64 limbs, least significant first, the order
    /// r55 reassembles them in (`U256::from_limbs`).
    fn split_256<'a>(value: IntValue<'a>, bin: &Binary<'a>) -> Vec<IntValue<'a>> {
        let i64_ty = bin.context.i64_type();
        let value_ty = value.get_type();
        (0..4u64)
            .map(|i| {
                let shifted = bin
                    .builder
                    .build_right_shift(value, value_ty.const_int(64 * i, false), false, "limb")
                    .unwrap();
                bin.builder
                    .build_int_truncate(shifted, i64_ty, "limb")
                    .unwrap()
            })
            .collect()
    }

    fn to_slot_word<'a>(bin: &Binary<'a>, ty: &Type, value: BasicValueEnum<'a>) -> IntValue<'a> {
        let i256_ty = bin.context.custom_width_int_type(256);

        let value = if value.is_pointer_value() {
            bin.builder
                .build_load(bin.llvm_type(ty), value.into_pointer_value(), "value")
                .unwrap()
        } else {
            value
        };

        let int = match ty {
            // The address bytes are big-endian.
            Type::Address(_) | Type::Contract(_) => {
                let i160_ty = bin.context.custom_width_int_type(160);
                let tmp = bin
                    .builder
                    .build_alloca(bin.address_type(), "address")
                    .unwrap();
                bin.builder
                    .build_store(tmp, value.into_array_value())
                    .unwrap();
                let swapped = bin
                    .builder
                    .build_load(i160_ty, tmp, "address_bytes")
                    .unwrap()
                    .into_int_value();
                bin.builder
                    .build_call(bin.llvm_bswap(160), &[swapped.into()], "address")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value()
            }
            _ => value.into_int_value(),
        };

        if int.get_type() == i256_ty {
            int
        } else {
            bin.builder
                .build_int_z_extend(int, i256_ty, "slot_word")
                .unwrap()
        }
    }

    fn from_slot_word<'a>(bin: &Binary<'a>, ty: &Type, word: IntValue<'a>) -> BasicValueEnum<'a> {
        match ty {
            Type::Bool => bin
                .builder
                .build_int_compare(IntPredicate::NE, word, word.get_type().const_zero(), "bool")
                .unwrap()
                .into(),
            Type::Address(_) | Type::Contract(_) => {
                let i160_ty = bin.context.custom_width_int_type(160);
                let int = bin
                    .builder
                    .build_int_truncate(word, i160_ty, "address")
                    .unwrap();
                let swapped = bin
                    .builder
                    .build_call(bin.llvm_bswap(160), &[int.into()], "address_bytes")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let tmp = bin.builder.build_alloca(i160_ty, "address").unwrap();
                bin.builder.build_store(tmp, swapped).unwrap();
                bin.builder
                    .build_load(bin.address_type(), tmp, "address")
                    .unwrap()
            }
            _ => bin
                .builder
                .build_int_truncate_or_bit_cast(word, bin.llvm_type(ty).into_int_type(), "value")
                .unwrap()
                .into(),
        }
    }

    fn storage_type(bin: &Binary, ty: &Type) -> Type {
        ty.deref_any().clone().unwrap_user_type(bin.ns)
    }

    fn check_single_slot(bin: &Binary, ty: &Type) {
        if !matches!(
            ty,
            Type::Bool
                | Type::Int(_)
                | Type::Uint(_)
                | Type::Value
                | Type::Enum(_)
                | Type::Bytes(_)
                | Type::Address(_)
                | Type::Contract(_)
        ) {
            unimplemented!(
                "the riscv target cannot keep `{}` in storage yet",
                ty.to_string(bin.ns)
            );
        }
    }

    fn next_slot<'a>(bin: &Binary<'a>, slot: &mut IntValue<'a>) {
        *slot = bin
            .builder
            .build_int_add(*slot, slot.get_type().const_int(1, false), "next_slot")
            .unwrap();
    }

    /// The length of the outermost dimension of a fixed-size array.
    fn fixed_length(dims: &[ArrayLength]) -> Option<u64> {
        match dims.last() {
            Some(ArrayLength::Fixed(len)) => len.to_u64(),
            _ => None,
        }
    }

    fn malloc<'a>(bin: &Binary<'a>, ty: &Type) -> PointerValue<'a> {
        let size = bin
            .builder
            .build_int_truncate(
                bin.llvm_type(ty).size_of().unwrap(),
                bin.context.i32_type(),
                "size",
            )
            .unwrap();
        bin.builder
            .build_call(
                bin.module.get_function("__malloc").unwrap(),
                &[size.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    fn member<'a>(
        bin: &Binary<'a>,
        ty: &Type,
        ptr: PointerValue<'a>,
        index: IntValue<'a>,
    ) -> PointerValue<'a> {
        unsafe {
            bin.builder
                .build_gep(
                    bin.llvm_type(ty),
                    ptr,
                    &[bin.context.i32_type().const_zero(), index],
                    "member",
                )
                .unwrap()
        }
    }

    /// Store a loaded value into a struct field or array element. Nested
    /// structs and fixed arrays are loaded as pointers, but live inline.
    fn store_member<'a>(
        bin: &Binary<'a>,
        ty: &Type,
        dest: PointerValue<'a>,
        value: BasicValueEnum<'a>,
    ) {
        let value = if ty.is_fixed_reference_type(bin.ns) {
            bin.builder
                .build_load(bin.llvm_type(ty), value.into_pointer_value(), "member")
                .unwrap()
        } else {
            value
        };
        bin.builder.build_store(dest, value).unwrap();
    }

    fn load_slots<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) -> BasicValueEnum<'a> {
        let ty = Self::storage_type(bin, ty);
        match &ty {
            Type::Struct(struct_ty) => {
                let new = Self::malloc(bin, &ty);
                for (i, field) in struct_ty.definition(bin.ns).fields.iter().enumerate() {
                    let value = self.load_slots(bin, &field.ty, slot, function);
                    let index = bin.context.i32_type().const_int(i as u64, false);
                    let dest = Self::member(bin, &ty, new, index);
                    Self::store_member(bin, &Self::storage_type(bin, &field.ty), dest, value);
                }
                new.into()
            }
            Type::Array(_, dims) if Self::fixed_length(dims).is_some() => {
                let len = Self::fixed_length(dims).unwrap();
                let elem_ty = Self::storage_type(bin, &ty.array_deref());
                let new = Self::malloc(bin, &ty);
                bin.emit_static_loop_with_int(
                    function,
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_int(len, false),
                    slot,
                    |index, slot| {
                        let value = self.load_slots(bin, &elem_ty, slot, function);
                        let dest = Self::member(bin, &ty, new, index);
                        Self::store_member(bin, &elem_ty, dest, value);
                    },
                );
                new.into()
            }
            _ => {
                Self::check_single_slot(bin, &ty);
                let word = Self::sload(bin, *slot);
                Self::next_slot(bin, slot);
                Self::from_slot_word(bin, &ty, word)
            }
        }
    }

    fn store_slots<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        value: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    ) {
        let ty = Self::storage_type(bin, ty);
        match &ty {
            Type::Struct(struct_ty) => {
                for (i, field) in struct_ty.definition(bin.ns).fields.iter().enumerate() {
                    let index = bin.context.i32_type().const_int(i as u64, false);
                    let src = Self::member(bin, &ty, value.into_pointer_value(), index);
                    self.store_slots(bin, &field.ty, slot, src.into(), function);
                }
            }
            Type::Array(_, dims) if Self::fixed_length(dims).is_some() => {
                let len = Self::fixed_length(dims).unwrap();
                let elem_ty = Self::storage_type(bin, &ty.array_deref());
                bin.emit_static_loop_with_int(
                    function,
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_int(len, false),
                    slot,
                    |index, slot| {
                        let src = Self::member(bin, &ty, value.into_pointer_value(), index);
                        self.store_slots(bin, &elem_ty, slot, src.into(), function);
                    },
                );
            }
            _ => {
                Self::check_single_slot(bin, &ty);
                let word = Self::to_slot_word(bin, &ty, value);
                Self::sstore(bin, *slot, word);
                Self::next_slot(bin, slot);
            }
        }
    }

    fn delete_slots<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        let ty = Self::storage_type(bin, ty);
        match &ty {
            Type::Struct(struct_ty) => {
                for field in &struct_ty.definition(bin.ns).fields {
                    self.delete_slots(bin, &field.ty, slot, function);
                }
            }
            Type::Array(_, dims) if Self::fixed_length(dims).is_some() => {
                let len = Self::fixed_length(dims).unwrap();
                let elem_ty = Self::storage_type(bin, &ty.array_deref());
                bin.emit_static_loop_with_int(
                    function,
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_int(len, false),
                    slot,
                    |_, slot| self.delete_slots(bin, &elem_ty, slot, function),
                );
            }
            // As in Solidity, deleting a struct leaves its mappings alone.
            Type::Mapping(..) => Self::next_slot(bin, slot),
            _ => {
                Self::check_single_slot(bin, &ty);
                let zero = bin.context.custom_width_int_type(256).const_zero();
                Self::sstore(bin, *slot, zero);
                Self::next_slot(bin, slot);
            }
        }
    }
}

pub(crate) fn declare_syscalls(bin: &mut Binary) {
    let ctx = bin.context;
    let i64_ty = ctx.i64_type();
    let i8_ptr_ty = ctx.ptr_type(AddressSpace::default());
    let void_ty = ctx.void_type();

    let sload_ty = void_ty.fn_type(
        &[
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i8_ptr_ty.into(),
        ],
        false,
    );
    let sload = bin.module.add_function("__sys_sload", sload_ty, None);
    sload.set_linkage(Linkage::External);

    let sstore_ty = void_ty.fn_type(
        &[
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
            i64_ty.into(),
        ],
        false,
    );
    let sstore = bin.module.add_function("__sys_sstore", sstore_ty, None);
    sstore.set_linkage(Linkage::External);

    let return_ty = void_ty.fn_type(&[i8_ptr_ty.into(), i64_ty.into()], false);
    let return_fn = bin.module.add_function("__sys_return", return_ty, None);
    return_fn.set_linkage(Linkage::External);

    let caller_ty = void_ty.fn_type(&[i8_ptr_ty.into()], false);
    let caller = bin.module.add_function("__sys_caller", caller_ty, None);
    caller.set_linkage(Linkage::External);

    let callvalue_ty = void_ty.fn_type(&[i8_ptr_ty.into()], false);
    let callvalue = bin
        .module
        .add_function("__sys_callvalue", callvalue_ty, None);
    callvalue.set_linkage(Linkage::External);

    let revert_ty = void_ty.fn_type(&[i8_ptr_ty.into(), i64_ty.into()], false);
    let revert = bin.module.add_function("__sys_revert", revert_ty, None);
    revert.set_linkage(Linkage::External);
}

impl<'a> TargetRuntime<'a> for RiscvTargetRuntime {
    fn get_storage_int(
        &self,
        bin: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let i256_ty = bin.context.custom_width_int_type(256);
        let slot = bin
            .builder
            .build_load(i256_ty, slot, "slot")
            .unwrap()
            .into_int_value();
        let word = Self::sload(bin, slot);
        bin.builder
            .build_int_truncate_or_bit_cast(word, ty, "storage_int")
            .unwrap()
    }

    fn storage_load(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        _slot_ty: Option<&Type>,
        function: FunctionValue<'a>,
        _storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        self.load_slots(bin, ty, slot, function)
    }

    fn storage_store(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        _existing: bool,
        slot: &mut IntValue<'a>,
        _slot_ty: Option<&Type>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        _storage_type: &Option<StorageType>,
    ) {
        self.store_slots(bin, ty, slot, dest, function);
    }

    fn return_abi_data<'b>(
        &self,
        bin: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        let len = data_len.into_int_value();
        let len_i64 = bin
            .builder
            .build_int_cast(len, bin.context.i64_type(), "len_cast")
            .unwrap();
        let return_fn = bin.module.get_function("__sys_return").unwrap();
        let args: Vec<BasicMetadataValueEnum> = vec![data.into(), len_i64.into()];
        let _ = bin.builder.build_call(return_fn, &args, "");
        bin.builder.build_unreachable().unwrap();
    }

    fn value_transferred<'b>(&self, contract: &Binary<'b>) -> IntValue<'b> {
        let i256_ty = contract.context.custom_width_int_type(256);
        i256_ty.const_zero()
    }

    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        self.delete_slots(bin, ty, slot, function);
    }

    fn set_storage_string(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        unimplemented!("set_storage_string")
    }

    fn get_storage_string(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!("get_storage_string")
    }

    fn set_storage_extfunc(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
        _dest_ty: BasicTypeEnum,
    ) {
        unimplemented!("set_storage_extfunc")
    }

    fn get_storage_extfunc(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!("get_storage_extfunc")
    }

    fn get_storage_bytes_subscript(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _index: IntValue<'a>,
        _loc: Loc,
    ) -> IntValue<'a> {
        unimplemented!("get_storage_bytes_subscript")
    }

    fn set_storage_bytes_subscript(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _index: IntValue<'a>,
        _value: IntValue<'a>,
        _loc: Loc,
    ) {
        unimplemented!("set_storage_bytes_subscript")
    }

    fn storage_subscript(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &Type,
        _slot: IntValue<'a>,
        _index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        unimplemented!("storage_subscript")
    }

    fn storage_push(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &Type,
        _slot: IntValue<'a>,
        _val: Option<BasicValueEnum<'a>>,
    ) -> BasicValueEnum<'a> {
        unimplemented!("storage_push")
    }

    fn storage_pop(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &Type,
        _slot: IntValue<'a>,
        _load: bool,
        _loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!("storage_pop")
    }

    fn storage_array_length(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _elem_ty: &Type,
    ) -> IntValue<'a> {
        unimplemented!("storage_array_length")
    }

    fn keccak256_hash(
        &self,
        _bin: &Binary<'a>,
        _src: PointerValue,
        _length: IntValue,
        _dest: PointerValue,
    ) {
        unimplemented!("keccak256_hash")
    }

    /// r55 has no debug output syscall.
    fn print<'b>(&self, _bin: &Binary<'b>, _string: PointerValue<'b>, _length: IntValue<'b>) {}

    fn return_empty_abi(&self, bin: &Binary) {
        let null = bin.context.ptr_type(AddressSpace::default()).const_null();
        let return_fn = bin.module.get_function("__sys_return").unwrap();
        let args: Vec<BasicMetadataValueEnum> =
            vec![null.into(), bin.context.i64_type().const_zero().into()];
        let _ = bin.builder.build_call(return_fn, &args, "");
        bin.builder.build_unreachable().unwrap();
    }

    fn return_code<'b>(&self, bin: &'b Binary, _ret: IntValue<'b>) {
        // r55 has no exit codes, only the Return and Revert syscalls.
        self.return_empty_abi(bin);
    }

    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        let len_i64 = bin
            .builder
            .build_int_cast(length, bin.context.i64_type(), "revert_len")
            .unwrap();
        let revert_fn = bin.module.get_function("__sys_revert").unwrap();
        let args: Vec<BasicMetadataValueEnum> = vec![data.into(), len_i64.into()];
        let _ = bin.builder.build_call(revert_fn, &args, "");
        bin.builder.build_unreachable().unwrap();
    }

    fn builtin_function(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _builtin_func: &crate::sema::ast::Function,
        _args: &[BasicMetadataValueEnum<'a>],
        _first_arg_type: Option<BasicTypeEnum>,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!("builtin_function")
    }

    fn builtin<'b>(
        &self,
        _bin: &Binary<'b>,
        _expr: &crate::codegen::Expression,
        _vartab: &HashMap<usize, Variable<'b>>,
        _function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        unimplemented!("builtin")
    }

    fn emit_event<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue<'b>,
        _data: BasicValueEnum<'b>,
        _topics: &[BasicValueEnum<'b>],
    ) {
        unimplemented!("emit_event")
    }

    fn external_call<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue<'b>,
        _success: Option<&mut BasicValueEnum<'b>>,
        _payload: PointerValue<'b>,
        _payload_len: IntValue<'b>,
        _address: Option<BasicValueEnum<'b>>,
        _contract_args: ContractArgs<'b>,
        _ty: CallTy,
        _loc: Loc,
    ) {
        unimplemented!("external_call")
    }

    fn create_contract<'b>(
        &mut self,
        _bin: &Binary<'b>,
        _function: FunctionValue<'b>,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        _address: PointerValue<'b>,
        _encoded_args: BasicValueEnum<'b>,
        _encoded_args_len: BasicValueEnum<'b>,
        _contract_args: ContractArgs<'b>,
        _loc: Loc,
    ) {
        unimplemented!("create_contract")
    }

    fn value_transfer<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
        _loc: Loc,
    ) {
        unimplemented!("value_transfer")
    }

    fn return_data<'b>(&self, _bin: &Binary<'b>, _function: FunctionValue<'b>) -> PointerValue<'b> {
        unimplemented!("return_data")
    }

    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: inkwell::values::ArrayValue<'b>) {
        unimplemented!("selfdestruct")
    }

    fn hash<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue<'b>,
        _hash: HashTy,
        _string: PointerValue<'b>,
        _length: IntValue<'b>,
    ) -> IntValue<'b> {
        unimplemented!("hash")
    }
}
