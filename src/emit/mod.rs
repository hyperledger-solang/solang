// SPDX-License-Identifier: Apache-2.0

use crate::codegen::{Builtin, Expression};
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{
    ArrayLength, CallTy, Contract, FormatArg, Function, Namespace, Parameter, StringLocation,
    StructType, Type,
};
use solang_parser::pt;
// use solang_parser::pt::Loc;
use std::convert::TryFrom;
use std::fmt;
use std::str;

use num_bigint::{BigInt, Sign};
use num_traits::One;
use num_traits::ToPrimitive;
use std::collections::{HashMap, VecDeque};

use crate::Target;
use inkwell::debug_info::AsDIScope;
use inkwell::debug_info::DISubprogram;
use inkwell::debug_info::DIType;
use inkwell::module::Linkage;
use inkwell::targets::TargetTriple;
use inkwell::types::{BasicType, IntType, StringRadix};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, CallableValue, FunctionValue, IntValue,
    PhiValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use solang_parser::pt::{CodeLocation, Loc};

pub mod binary;
mod ethabiencoder;
mod loop_builder;
pub mod solana;
pub mod substrate;

use crate::codegen::{
    cfg::{ControlFlowGraph, HashTy, Instr, InternalCallTy},
    vartable::Storage,
};
use crate::emit::binary::Binary;

#[derive(Clone)]
pub struct Variable<'a> {
    value: BasicValueEnum<'a>,
}

#[derive(Clone, Copy)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "add",
                Self::Subtract => "sub",
                Self::Multiply => "mul",
            }
        )
    }
}

pub trait TargetRuntime<'a> {
    fn abi_decode<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[Parameter],
        ns: &Namespace,
    );

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
    ) -> (PointerValue<'a>, IntValue<'a>);

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[Type],
        ns: &Namespace,
    ) -> PointerValue<'b>;

    fn set_storage(
        &self,
        _bin: &Binary,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_int(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a>;

    fn get_storage_address(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ns: &Namespace,
    ) -> ArrayValue<'a> {
        unimplemented!();
    }

    // Bytes and string have special storage layout
    fn set_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    );
    fn get_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a>;
    fn set_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    );
    fn get_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &Namespace,
    ) -> PointerValue<'a>;
    fn get_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a>;
    fn set_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
    );
    fn storage_subscript(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &Type,
        _slot: IntValue<'a>,
        _index: BasicValueEnum<'a>,
        _ns: &Namespace,
    ) -> IntValue<'a> {
        // not need for slot-based storage chains
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
    ) -> BasicValueEnum<'a>;
    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &Namespace,
    ) -> Option<BasicValueEnum<'a>>;
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
    );

    /// Prints a string
    fn print(&self, bin: &Binary, string: PointerValue, length: IntValue);

    /// Return success without any result
    fn return_empty_abi(&self, bin: &Binary);

    /// Return failure code
    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>);

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, bin: &'b Binary, data: PointerValue<'b>, length: IntValue);

    /// Return failure without any result
    fn assert_failure<'b>(&self, bin: &'b Binary, data: PointerValue, length: IntValue);

    fn builtin_function(
        &self,
        _binary: &Binary<'a>,
        _func: &Function,
        _args: &[BasicMetadataValueEnum<'a>],
        _ns: &Namespace,
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
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
        space: Option<IntValue<'b>>,
        ns: &Namespace,
    );

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
        ty: CallTy,
        ns: &Namespace,
    );

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
    ) -> BasicValueEnum<'b>;

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b>;

    /// Return the value we received
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &Namespace) -> IntValue<'b>;

    /// Terminate execution, destroy bin and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &Namespace);

    /// Crypto Hash
    fn hash<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        string: PointerValue<'b>,
        length: IntValue<'b>,
        ns: &Namespace,
    ) -> IntValue<'b>;

    /// Integer to prefix events with
    fn event_id<'b>(
        &self,
        _bin: &Binary<'b>,
        _contract: &Contract,
        _event_no: usize,
    ) -> Option<IntValue<'b>> {
        None
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
    );

    /// Helper functions which need access to the trait

    /// If we receive a value transfer, and we are "payable", abort with revert
    fn abort_if_value_transfer(&self, binary: &Binary, function: FunctionValue, ns: &Namespace) {
        if ns.target != Target::Solana {
            let value = self.value_transferred(binary, ns);

            let got_value = binary.builder.build_int_compare(
                IntPredicate::NE,
                value,
                binary.value_type(ns).const_zero(),
                "is_value_transfer",
            );

            let not_value_transfer = binary
                .context
                .append_basic_block(function, "not_value_transfer");
            let abort_value_transfer = binary
                .context
                .append_basic_block(function, "abort_value_transfer");

            binary.builder.build_conditional_branch(
                got_value,
                abort_value_transfer,
                not_value_transfer,
            );

            binary.builder.position_at_end(abort_value_transfer);

            self.assert_failure(
                binary,
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                binary.context.i32_type().const_zero(),
            );

            binary.builder.position_at_end(not_value_transfer);
        }
    }

    /// Recursively load a type from bin storage
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

    /// Recursively load a type from bin storage for slot based bin storage
    fn storage_load_slot(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        match ty {
            Type::Ref(ty) => self.storage_load_slot(bin, ty, slot, slot_ptr, function, ns),
            Type::Array(elem_ty, dim) => {
                if let Some(ArrayLength::Fixed(d)) = dim.last() {
                    let llvm_ty = bin.llvm_type(ty.deref_any(), ns);
                    // LLVMSizeOf() produces an i64
                    let size = bin.builder.build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        bin.context.i32_type(),
                        "size_of",
                    );

                    let ty = ty.array_deref();
                    let new = bin
                        .builder
                        .build_call(
                            bin.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let dest = bin.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );

                    bin.emit_static_loop_with_int(
                        function,
                        bin.context.i64_type().const_zero(),
                        bin.context.i64_type().const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = unsafe {
                                bin.builder.build_gep(
                                    dest,
                                    &[bin.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val =
                                self.storage_load_slot(bin, &ty, slot, slot_ptr, function, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type() {
                                bin.builder.build_load(val.into_pointer_value(), "elem")
                            } else {
                                val
                            };

                            bin.builder.build_store(elem, val);
                        },
                    );

                    dest.into()
                } else {
                    // iterate over dynamic array
                    let slot_ty = Type::Uint(256);

                    let size = bin.builder.build_int_truncate(
                        self.storage_load_slot(bin, &slot_ty, slot, slot_ptr, function, ns)
                            .into_int_value(),
                        bin.context.i32_type(),
                        "size",
                    );

                    let llvm_elem_ty = bin.llvm_field_ty(elem_ty, ns);

                    let elem_size = bin.builder.build_int_truncate(
                        llvm_elem_ty.size_of().unwrap(),
                        bin.context.i32_type(),
                        "size_of",
                    );
                    let init = bin.builder.build_int_to_ptr(
                        bin.context.i32_type().const_all_ones(),
                        bin.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    let dest = bin
                        .builder
                        .build_call(
                            bin.module.get_function("vector_new").unwrap(),
                            &[size.into(), elem_size.into(), init.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // get the slot for the elements
                    // this hashes in-place
                    self.keccak256_hash(
                        bin,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(bin.context.i32_type(), false),
                        slot_ptr,
                        ns,
                    );

                    let mut elem_slot = bin
                        .builder
                        .build_load(slot_ptr, "elem_slot")
                        .into_int_value();

                    bin.emit_loop_cond_first_with_int(
                        function,
                        bin.context.i32_type().const_zero(),
                        size,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = bin.array_subscript(ty, dest, elem_no, ns);

                            let entry =
                                self.storage_load_slot(bin, elem_ty, slot, slot_ptr, function, ns);

                            let entry = if elem_ty.deref_memory().is_fixed_reference_type() {
                                bin.builder.build_load(entry.into_pointer_value(), "elem")
                            } else {
                                entry
                            };

                            bin.builder.build_store(elem, entry);
                        },
                    );
                    // load
                    dest.into()
                }
            }
            Type::Struct(str_ty) => {
                let llvm_ty = bin.llvm_type(ty.deref_any(), ns);
                // LLVMSizeOf() produces an i64
                let size = bin.builder.build_int_truncate(
                    llvm_ty.size_of().unwrap(),
                    bin.context.i32_type(),
                    "size_of",
                );

                let new = bin
                    .builder
                    .build_call(
                        bin.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let dest = bin.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    "dest",
                );

                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let val = self.storage_load_slot(bin, &field.ty, slot, slot_ptr, function, ns);

                    let elem = unsafe {
                        bin.builder.build_gep(
                            dest,
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    let val = if field.ty.deref_memory().is_fixed_reference_type() {
                        bin.builder
                            .build_load(val.into_pointer_value(), field.name_as_str())
                    } else {
                        val
                    };

                    bin.builder.build_store(elem, val);
                }

                dest.into()
            }
            Type::String | Type::DynamicBytes => {
                bin.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_string(bin, function, slot_ptr);

                *slot = bin.builder.build_int_add(
                    *slot,
                    bin.number_literal(256, &BigInt::one(), ns),
                    "string",
                );

                ret.into()
            }
            Type::InternalFunction { .. } => {
                bin.builder.build_store(slot_ptr, *slot);

                let ptr_ty = bin
                    .context
                    .custom_width_int_type(ns.target.ptr_size() as u32);

                let ret = self.get_storage_int(bin, function, slot_ptr, ptr_ty);

                bin.builder
                    .build_int_to_ptr(
                        ret,
                        bin.llvm_type(ty.deref_any(), ns).into_pointer_type(),
                        "",
                    )
                    .into()
            }
            Type::ExternalFunction { .. } => {
                bin.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_extfunc(bin, function, slot_ptr, ns);

                *slot = bin.builder.build_int_add(
                    *slot,
                    bin.number_literal(256, &BigInt::one(), ns),
                    "string",
                );

                ret.into()
            }
            Type::Address(_) | Type::Contract(_) => {
                bin.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_address(bin, function, slot_ptr, ns);

                *slot = bin.builder.build_int_add(
                    *slot,
                    bin.number_literal(256, &BigInt::one(), ns),
                    "string",
                );

                ret.into()
            }
            _ => {
                bin.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_int(
                    bin,
                    function,
                    slot_ptr,
                    bin.llvm_type(ty.deref_any(), ns).into_int_type(),
                );

                *slot = bin.builder.build_int_add(
                    *slot,
                    bin.number_literal(256, &BigInt::one(), ns),
                    "int",
                );

                ret.into()
            }
        }
    }

    /// Recursively store a type to bin storage
    fn storage_store(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        _existing: bool,
        slot: &mut IntValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        let slot_ptr = bin.builder.build_alloca(slot.get_type(), "slot");

        self.storage_store_slot(bin, ty, slot, slot_ptr, dest, function, ns)
    }

    /// Recursively store a type to bin storage for slot-based bin storage
    fn storage_store_slot(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        match ty.deref_any() {
            Type::Array(elem_ty, dim) => {
                if let Some(ArrayLength::Fixed(d)) = dim.last() {
                    bin.emit_static_loop_with_int(
                        function,
                        bin.context.i64_type().const_zero(),
                        bin.context.i64_type().const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let mut elem = unsafe {
                                bin.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[bin.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            if elem_ty.is_reference_type(ns)
                                && !elem_ty.deref_memory().is_fixed_reference_type()
                            {
                                elem = bin.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store_slot(
                                bin,
                                elem_ty,
                                slot,
                                slot_ptr,
                                elem.into(),
                                function,
                                ns,
                            );

                            if !elem_ty.is_reference_type(ns) {
                                *slot = bin.builder.build_int_add(
                                    *slot,
                                    bin.number_literal(256, &elem_ty.storage_slots(ns), ns),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // get the length of the our in-memory array
                    let len = bin.vector_len(dest);

                    let slot_ty = Type::Uint(256);

                    // details about our array elements
                    let llvm_elem_ty = bin.llvm_field_ty(elem_ty, ns);
                    let elem_size = bin.builder.build_int_truncate(
                        llvm_elem_ty.size_of().unwrap(),
                        bin.context.i32_type(),
                        "size_of",
                    );

                    // the previous length of the storage array
                    // we need this to clear any elements
                    let previous_size = bin.builder.build_int_truncate(
                        self.storage_load_slot(bin, &slot_ty, slot, slot_ptr, function, ns)
                            .into_int_value(),
                        bin.context.i32_type(),
                        "previous_size",
                    );

                    let new_slot = bin
                        .builder
                        .build_alloca(bin.llvm_type(&slot_ty, ns).into_int_type(), "new");

                    // set new length
                    bin.builder.build_store(
                        new_slot,
                        bin.builder.build_int_z_extend(
                            len,
                            bin.llvm_type(&slot_ty, ns).into_int_type(),
                            "",
                        ),
                    );

                    self.set_storage(bin, function, slot_ptr, new_slot);

                    self.keccak256_hash(
                        bin,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(bin.context.i32_type(), false),
                        new_slot,
                        ns,
                    );

                    let mut elem_slot = bin
                        .builder
                        .build_load(new_slot, "elem_slot")
                        .into_int_value();

                    bin.emit_loop_cond_first_with_int(
                        function,
                        bin.context.i32_type().const_zero(),
                        len,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = bin.builder.build_int_mul(elem_no, elem_size, "");

                            let data = unsafe {
                                bin.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[
                                        bin.context.i32_type().const_zero(),
                                        bin.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let mut elem = bin.builder.build_pointer_cast(
                                data,
                                llvm_elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            if elem_ty.is_reference_type(ns)
                                && !elem_ty.deref_memory().is_fixed_reference_type()
                            {
                                elem = bin.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store_slot(
                                bin,
                                elem_ty,
                                slot,
                                slot_ptr,
                                elem.into(),
                                function,
                                ns,
                            );

                            if !elem_ty.is_reference_type(ns) {
                                *slot = bin.builder.build_int_add(
                                    *slot,
                                    bin.number_literal(256, &elem_ty.storage_slots(ns), ns),
                                    "",
                                );
                            }
                        },
                    );

                    // we've populated the array with the new values; if the new array is shorter
                    // than the previous, clear out the trailing elements
                    bin.emit_loop_cond_first_with_int(
                        function,
                        len,
                        previous_size,
                        &mut elem_slot,
                        |_: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(bin, elem_ty, slot, slot_ptr, function, ns);

                            if !elem_ty.is_reference_type(ns) {
                                *slot = bin.builder.build_int_add(
                                    *slot,
                                    bin.number_literal(256, &elem_ty.storage_slots(ns), ns),
                                    "",
                                );
                            }
                        },
                    );
                }
            }
            Type::Struct(str_ty) => {
                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let mut elem = unsafe {
                        bin.builder.build_gep(
                            dest.into_pointer_value(),
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    if field.ty.is_reference_type(ns) && !field.ty.is_fixed_reference_type() {
                        elem = bin
                            .builder
                            .build_load(elem, field.name_as_str())
                            .into_pointer_value();
                    }

                    self.storage_store_slot(
                        bin,
                        &field.ty,
                        slot,
                        slot_ptr,
                        elem.into(),
                        function,
                        ns,
                    );

                    if !field.ty.is_reference_type(ns)
                        || matches!(field.ty, Type::String | Type::DynamicBytes)
                    {
                        *slot = bin.builder.build_int_add(
                            *slot,
                            bin.number_literal(256, &field.ty.storage_slots(ns), ns),
                            field.name_as_str(),
                        );
                    }
                }
            }
            Type::String | Type::DynamicBytes => {
                bin.builder.build_store(slot_ptr, *slot);

                self.set_storage_string(bin, function, slot_ptr, dest);
            }
            Type::ExternalFunction { .. } => {
                bin.builder.build_store(slot_ptr, *slot);

                self.set_storage_extfunc(bin, function, slot_ptr, dest.into_pointer_value());
            }
            Type::InternalFunction { .. } => {
                let ptr_ty = bin
                    .context
                    .custom_width_int_type(ns.target.ptr_size() as u32);

                let m = bin.build_alloca(function, ptr_ty, "");

                bin.builder.build_store(
                    m,
                    bin.builder.build_ptr_to_int(
                        dest.into_pointer_value(),
                        ptr_ty,
                        "function_pointer",
                    ),
                );

                bin.builder.build_store(slot_ptr, *slot);

                self.set_storage(bin, function, slot_ptr, m);
            }
            Type::Address(_) | Type::Contract(_) => {
                if dest.is_pointer_value() {
                    bin.builder.build_store(slot_ptr, *slot);

                    self.set_storage(bin, function, slot_ptr, dest.into_pointer_value());
                } else {
                    let address = bin.builder.build_alloca(bin.address_type(ns), "address");

                    bin.builder.build_store(address, dest.into_array_value());

                    bin.builder.build_store(slot_ptr, *slot);

                    self.set_storage(bin, function, slot_ptr, address);
                }
            }
            _ => {
                bin.builder.build_store(slot_ptr, *slot);

                let dest = if dest.is_int_value() {
                    let m = bin.build_alloca(function, dest.get_type(), "");
                    bin.builder.build_store(m, dest);

                    m
                } else {
                    dest.into_pointer_value()
                };

                // TODO ewasm allocates 32 bytes here, even though we have just
                // allocated test. This can be folded into one allocation, if llvm
                // does not already fold it into one.
                self.set_storage(bin, function, slot_ptr, dest);
            }
        }
    }

    // Clear a particlar storage slot (slot-based storage chains should implement)
    fn storage_delete_single_slot(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue,
    ) {
        unimplemented!();
    }

    /// Recursively clear bin storage. The default implementation is for slot-based bin storage
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

    /// Recursively clear bin storage for slot-based bin storage
    fn storage_delete_slot(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        match ty.deref_any() {
            Type::Array(_, dim) => {
                let ty = ty.array_deref();

                if let Some(ArrayLength::Fixed(d)) = dim.last() {
                    bin.emit_static_loop_with_int(
                        function,
                        bin.context.i64_type().const_zero(),
                        bin.context.i64_type().const_int(d.to_u64().unwrap(), false),
                        slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(bin, &ty, slot, slot_ptr, function, ns);

                            if !ty.is_reference_type(ns) {
                                *slot = bin.builder.build_int_add(
                                    *slot,
                                    bin.number_literal(256, &ty.storage_slots(ns), ns),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // dynamic length array.
                    // load length
                    bin.builder.build_store(slot_ptr, *slot);

                    let slot_ty = bin.context.custom_width_int_type(256);

                    let buf = bin.builder.build_alloca(slot_ty, "buf");

                    let length = self.get_storage_int(bin, function, slot_ptr, slot_ty);

                    // we need to hash the length slot in order to get the slot of the first
                    // entry of the array
                    self.keccak256_hash(
                        bin,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(bin.context.i32_type(), false),
                        buf,
                        ns,
                    );

                    let mut entry_slot = bin.builder.build_load(buf, "entry_slot").into_int_value();

                    // now loop from first slot to first slot + length
                    bin.emit_loop_cond_first_with_int(
                        function,
                        length.get_type().const_zero(),
                        length,
                        &mut entry_slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(bin, &ty, slot, slot_ptr, function, ns);

                            if !ty.is_reference_type(ns) {
                                *slot = bin.builder.build_int_add(
                                    *slot,
                                    bin.number_literal(256, &ty.storage_slots(ns), ns),
                                    "",
                                );
                            }
                        },
                    );

                    // clear length itself
                    self.storage_delete_slot(bin, &Type::Uint(256), slot, slot_ptr, function, ns);
                }
            }
            Type::Struct(str_ty) => {
                for (_, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    self.storage_delete_slot(bin, &field.ty, slot, slot_ptr, function, ns);

                    if !field.ty.is_reference_type(ns)
                        || matches!(field.ty, Type::String | Type::DynamicBytes)
                    {
                        *slot = bin.builder.build_int_add(
                            *slot,
                            bin.number_literal(256, &field.ty.storage_slots(ns), ns),
                            field.name_as_str(),
                        );
                    }
                }
            }
            Type::Mapping(..) => {
                // nothing to do, step over it
            }
            _ => {
                bin.builder.build_store(slot_ptr, *slot);

                self.storage_delete_single_slot(bin, function, slot_ptr);
            }
        }
    }

    /// The expression function recursively emits code for expressions. The BasicEnumValue it
    /// returns depends on the context; if it is simple integer, bool or bytes32 expression, the value
    /// is an Intvalue. For references to arrays, it is a PointerValue to the array. For references
    /// to storage, it is the storage slot. The references types are dereferenced by the Expression::Load()
    /// and Expression::StorageLoad() expression types.
    fn expression(
        &self,
        bin: &Binary<'a>,
        e: &Expression,
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        match e {
            Expression::FunctionArg(_, _, pos) => function.get_nth_param(*pos as u32).unwrap(),
            Expression::BoolLiteral(_, val) => {
                bin.context.bool_type().const_int(*val as u64, false).into()
            }
            Expression::NumberLiteral(_, Type::Address(_), val) => {
                // address can be negative; "address(-1)" is 0xffff...
                let mut bs = val.to_signed_bytes_be();

                // make sure it's no more than 32
                if bs.len() > ns.address_length {
                    // remove leading bytes
                    for _ in 0..bs.len() - ns.address_length {
                        bs.remove(0);
                    }
                } else {
                    // insert leading bytes
                    let val = if val.sign() == Sign::Minus { 0xff } else { 0 };

                    for _ in 0..ns.address_length - bs.len() {
                        bs.insert(0, val);
                    }
                }

                let address = bs
                    .iter()
                    .map(|b| bin.context.i8_type().const_int(*b as u64, false))
                    .collect::<Vec<IntValue>>();

                bin.context.i8_type().const_array(&address).into()
            }
            Expression::NumberLiteral(_, ty, n) => {
                bin.number_literal(ty.bits(ns) as u32, n, ns).into()
            }
            Expression::StructLiteral(_, ty, exprs) => {
                let struct_ty = bin.llvm_type(ty, ns);

                let s = bin
                    .builder
                    .build_call(
                        bin.module.get_function("__malloc").unwrap(),
                        &[struct_ty
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let s = bin.builder.build_pointer_cast(
                    s,
                    struct_ty.ptr_type(AddressSpace::Generic),
                    "struct_literal",
                );

                for (i, expr) in exprs.iter().enumerate() {
                    let elemptr = unsafe {
                        bin.builder.build_gep(
                            s,
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            "struct member",
                        )
                    };

                    let elem = self.expression(bin, expr, vartab, function, ns);

                    let elem = if expr.ty().is_fixed_reference_type() {
                        bin.builder.build_load(elem.into_pointer_value(), "elem")
                    } else {
                        elem
                    };

                    bin.builder.build_store(elemptr, elem);
                }

                s.into()
            }
            Expression::BytesLiteral(_, _, bs) => {
                let ty = bin.context.custom_width_int_type((bs.len() * 8) as u32);

                // hex"11223344" should become i32 0x11223344
                let s = hex::encode(bs);

                ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                    .unwrap()
                    .into()
            }
            Expression::CodeLiteral(_, bin_no, runtime) => {
                let codegen_bin = &ns.contracts[*bin_no];

                let target_bin = Binary::build(
                    bin.context,
                    codegen_bin,
                    ns,
                    "",
                    bin.opt,
                    bin.math_overflow_check,
                    bin.generate_debug_info,
                );

                let code = if *runtime && target_bin.runtime.is_some() {
                    target_bin
                        .runtime
                        .unwrap()
                        .code(Generate::Linked)
                        .expect("compile should succeeed")
                } else {
                    target_bin
                        .code(Generate::Linked)
                        .expect("compile should succeeed")
                };

                let size = bin.context.i32_type().const_int(code.len() as u64, false);

                let elem_size = bin.context.i32_type().const_int(1, false);

                let init = bin.emit_global_string(
                    &format!(
                        "code_{}_{}",
                        if *runtime { "runtime" } else { "deployer" },
                        &codegen_bin.name
                    ),
                    &code,
                    true,
                );

                bin.builder
                    .build_call(
                        bin.module.get_function("vector_new").unwrap(),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::Add(_, _, unchecked, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                if bin.math_overflow_check && !*unchecked {
                    let signed = l.ty().is_signed_int();
                    self.build_binary_op_with_overflow_check(
                        bin,
                        function,
                        left,
                        right,
                        BinaryOp::Add,
                        signed,
                    )
                    .into()
                } else {
                    bin.builder.build_int_add(left, right, "").into()
                }
            }
            Expression::Subtract(_, _, unchecked, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                if bin.math_overflow_check && !*unchecked {
                    let signed = l.ty().is_signed_int();
                    self.build_binary_op_with_overflow_check(
                        bin,
                        function,
                        left,
                        right,
                        BinaryOp::Subtract,
                        signed,
                    )
                    .into()
                } else {
                    bin.builder.build_int_sub(left, right, "").into()
                }
            }
            Expression::Multiply(_, res_ty, unchecked, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                self.mul(
                    bin,
                    function,
                    *unchecked,
                    left,
                    right,
                    res_ty.is_signed_int(),
                )
                .into()
            }
            Expression::UnsignedDivide(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("udivmod{}", div_bits);

                    let f = bin
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = bin.context.custom_width_int_type(div_bits);

                    let dividend = bin.build_alloca(function, ty, "dividend");
                    let divisor = bin.build_alloca(function, ty, "divisor");
                    let rem = bin.build_alloca(function, ty, "remainder");
                    let quotient = bin.build_alloca(function, ty, "quotient");

                    bin.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder.build_int_z_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    bin.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder.build_int_z_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = bin
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = bin.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = bin.context.append_basic_block(function, "success");
                    let bail_block = bin.context.append_basic_block(function, "bail");
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    bin.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        bin,
                        bin.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        bin.context.i32_type().const_zero(),
                    );

                    bin.builder.position_at_end(success_block);

                    let quotient = bin
                        .builder
                        .build_load(quotient, "quotient")
                        .into_int_value();

                    if bits < div_bits {
                        bin.builder
                            .build_int_truncate(quotient, left.get_type(), "")
                    } else {
                        quotient
                    }
                    .into()
                } else {
                    bin.builder.build_int_unsigned_div(left, right, "").into()
                }
            }
            Expression::SignedDivide(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("sdivmod{}", div_bits);

                    let f = bin
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = bin.context.custom_width_int_type(div_bits);

                    let dividend = bin.build_alloca(function, ty, "dividend");
                    let divisor = bin.build_alloca(function, ty, "divisor");
                    let rem = bin.build_alloca(function, ty, "remainder");
                    let quotient = bin.build_alloca(function, ty, "quotient");

                    bin.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder.build_int_s_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    bin.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder.build_int_s_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = bin
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = bin.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = bin.context.append_basic_block(function, "success");
                    let bail_block = bin.context.append_basic_block(function, "bail");
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    bin.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        bin,
                        bin.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        bin.context.i32_type().const_zero(),
                    );

                    bin.builder.position_at_end(success_block);

                    let quotient = bin
                        .builder
                        .build_load(quotient, "quotient")
                        .into_int_value();

                    if bits < div_bits {
                        bin.builder
                            .build_int_truncate(quotient, left.get_type(), "")
                    } else {
                        quotient
                    }
                    .into()
                } else if ns.target == Target::Solana {
                    // no signed div on BPF; do abs udev and then negate if needed
                    let left_negative = bin.builder.build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    );

                    let left = bin
                        .builder
                        .build_select(
                            left_negative,
                            bin.builder.build_int_neg(left, "signed_left"),
                            left,
                            "left_abs",
                        )
                        .into_int_value();

                    let right_negative = bin.builder.build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    );

                    let right = bin
                        .builder
                        .build_select(
                            right_negative,
                            bin.builder.build_int_neg(right, "signed_right"),
                            right,
                            "right_abs",
                        )
                        .into_int_value();

                    let res = bin.builder.build_int_unsigned_div(left, right, "");

                    let negate_result =
                        bin.builder
                            .build_xor(left_negative, right_negative, "negate_result");

                    bin.builder.build_select(
                        negate_result,
                        bin.builder.build_int_neg(res, "unsigned_res"),
                        res,
                        "res",
                    )
                } else {
                    bin.builder.build_int_signed_div(left, right, "").into()
                }
            }
            Expression::UnsignedModulo(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("udivmod{}", div_bits);

                    let f = bin
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = bin.context.custom_width_int_type(div_bits);

                    let dividend = bin.build_alloca(function, ty, "dividend");
                    let divisor = bin.build_alloca(function, ty, "divisor");
                    let rem = bin.build_alloca(function, ty, "remainder");
                    let quotient = bin.build_alloca(function, ty, "quotient");

                    bin.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder.build_int_z_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    bin.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder.build_int_z_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = bin
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = bin.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = bin.context.append_basic_block(function, "success");
                    let bail_block = bin.context.append_basic_block(function, "bail");
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    bin.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        bin,
                        bin.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        bin.context.i32_type().const_zero(),
                    );

                    bin.builder.position_at_end(success_block);

                    let rem = bin.builder.build_load(rem, "urem").into_int_value();

                    if bits < div_bits {
                        bin.builder.build_int_truncate(
                            rem,
                            bin.context.custom_width_int_type(bits),
                            "",
                        )
                    } else {
                        rem
                    }
                    .into()
                } else {
                    bin.builder.build_int_unsigned_rem(left, right, "").into()
                }
            }
            Expression::SignedModulo(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("sdivmod{}", div_bits);

                    let f = bin
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = bin.context.custom_width_int_type(div_bits);

                    let dividend = bin.build_alloca(function, ty, "dividend");
                    let divisor = bin.build_alloca(function, ty, "divisor");
                    let rem = bin.build_alloca(function, ty, "remainder");
                    let quotient = bin.build_alloca(function, ty, "quotient");

                    bin.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            bin.builder.build_int_s_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    bin.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            bin.builder.build_int_s_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = bin
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "sdiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = bin.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        bin.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = bin.context.append_basic_block(function, "success");
                    let bail_block = bin.context.append_basic_block(function, "bail");
                    bin.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    bin.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        bin,
                        bin.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        bin.context.i32_type().const_zero(),
                    );

                    bin.builder.position_at_end(success_block);

                    let rem = bin.builder.build_load(rem, "srem").into_int_value();

                    if bits < div_bits {
                        bin.builder.build_int_truncate(
                            rem,
                            bin.context.custom_width_int_type(bits),
                            "",
                        )
                    } else {
                        rem
                    }
                    .into()
                } else if ns.target == Target::Solana {
                    // no signed rem on BPF; do abs udev and then negate if needed
                    let left_negative = bin.builder.build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    );

                    let left = bin.builder.build_select(
                        left_negative,
                        bin.builder.build_int_neg(left, "signed_left"),
                        left,
                        "left_abs",
                    );

                    let right_negative = bin.builder.build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    );

                    let right = bin.builder.build_select(
                        right_negative,
                        bin.builder.build_int_neg(right, "signed_right"),
                        right,
                        "right_abs",
                    );

                    let res = bin.builder.build_int_unsigned_rem(
                        left.into_int_value(),
                        right.into_int_value(),
                        "",
                    );

                    bin.builder.build_select(
                        left_negative,
                        bin.builder.build_int_neg(res, "unsigned_res"),
                        res,
                        "res",
                    )
                } else {
                    bin.builder.build_int_signed_rem(left, right, "").into()
                }
            }
            Expression::Power(_, res_ty, unchecked, l, r) => {
                let left = self.expression(bin, l, vartab, function, ns);
                let right = self.expression(bin, r, vartab, function, ns);

                let bits = left.into_int_value().get_type().get_bit_width();
                let o = bin.build_alloca(function, left.get_type(), "");
                let f = self.power(bin, *unchecked, bits, res_ty.is_signed_int(), o);

                // If the function returns zero, then the operation was successful.
                let error_return = bin
                    .builder
                    .build_call(f, &[left.into(), right.into(), o.into()], "power")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                // Load the result pointer
                let res = bin.builder.build_load(o, "");

                if *unchecked || ns.target != Target::Solana {
                    // If target is Substrate, we don't neet to check on the return of function for ovf.
                    // If ovf happens in Substrate target, execution will hit an unreachable instruction.
                    res
                } else {
                    // In Solana, a return other than zero will abort execution. We need to check if power() returned a zero or not.
                    let error_block = bin.context.append_basic_block(function, "error");
                    let return_block = bin.context.append_basic_block(function, "return_block");

                    let error_ret = bin.builder.build_int_compare(
                        IntPredicate::NE,
                        error_return.into_int_value(),
                        error_return.get_type().const_zero().into_int_value(),
                        "",
                    );

                    bin.builder
                        .build_conditional_branch(error_ret, error_block, return_block);
                    bin.builder.position_at_end(error_block);

                    self.assert_failure(
                        bin,
                        bin.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        bin.context.i32_type().const_zero(),
                    );

                    bin.builder.position_at_end(return_block);

                    res
                }
            }
            Expression::Equal(_, l, r) => {
                if l.ty().is_address() {
                    let mut res = bin.context.bool_type().const_int(1, false);
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_array_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_array_value();

                    // TODO: Address should be passed around as pointer. Once this is done, we can replace
                    // this with a call to address_equal()
                    for index in 0..ns.address_length {
                        let l = bin
                            .builder
                            .build_extract_value(left, index as u32, "left")
                            .unwrap()
                            .into_int_value();
                        let r = bin
                            .builder
                            .build_extract_value(right, index as u32, "right")
                            .unwrap()
                            .into_int_value();

                        res = bin.builder.build_and(
                            res,
                            bin.builder.build_int_compare(IntPredicate::EQ, l, r, ""),
                            "cmp",
                        );
                    }

                    res.into()
                } else {
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_int_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_int_value();

                    bin.builder
                        .build_int_compare(IntPredicate::EQ, left, right, "")
                        .into()
                }
            }
            Expression::NotEqual(_, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::NE, left, right, "")
                    .into()
            }
            Expression::SignedMore(_, l, r) | Expression::UnsignedMore(_, l, r) => {
                if l.ty().is_address() {
                    self.compare_address(bin, l, r, IntPredicate::SGT, vartab, function, ns)
                        .into()
                } else {
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_int_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_int_value();

                    bin.builder
                        .build_int_compare(
                            if matches!(e, Expression::SignedMore(..)) {
                                IntPredicate::SGT
                            } else {
                                IntPredicate::UGT
                            },
                            left,
                            right,
                            "",
                        )
                        .into()
                }
            }
            Expression::MoreEqual(_, l, r) => {
                if l.ty().is_address() {
                    self.compare_address(bin, l, r, IntPredicate::SGE, vartab, function, ns)
                        .into()
                } else {
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_int_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_int_value();

                    bin.builder
                        .build_int_compare(
                            if l.ty().is_signed_int() {
                                IntPredicate::SGE
                            } else {
                                IntPredicate::UGE
                            },
                            left,
                            right,
                            "",
                        )
                        .into()
                }
            }
            Expression::SignedLess(_, l, r) | Expression::UnsignedLess(_, l, r) => {
                if l.ty().is_address() {
                    self.compare_address(bin, l, r, IntPredicate::SLT, vartab, function, ns)
                        .into()
                } else {
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_int_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_int_value();

                    bin.builder
                        .build_int_compare(
                            if matches!(e, Expression::SignedLess(..)) {
                                IntPredicate::SLT
                            } else {
                                IntPredicate::ULT
                            },
                            left,
                            right,
                            "",
                        )
                        .into()
                }
            }
            Expression::LessEqual(_, l, r) => {
                if l.ty().is_address() {
                    self.compare_address(bin, l, r, IntPredicate::SLE, vartab, function, ns)
                        .into()
                } else {
                    let left = self
                        .expression(bin, l, vartab, function, ns)
                        .into_int_value();
                    let right = self
                        .expression(bin, r, vartab, function, ns)
                        .into_int_value();

                    bin.builder
                        .build_int_compare(
                            if l.ty().is_signed_int() {
                                IntPredicate::SLE
                            } else {
                                IntPredicate::ULE
                            },
                            left,
                            right,
                            "",
                        )
                        .into()
                }
            }
            Expression::Variable(_, _, s) => vartab[s].value,
            Expression::GetRef(_, _, expr) => {
                let address = self
                    .expression(bin, expr, vartab, function, ns)
                    .into_array_value();

                let stack = bin.build_alloca(function, address.get_type(), "address");

                bin.builder.build_store(stack, address);

                stack.into()
            }
            Expression::Load(_, ty, e) => {
                let ptr = self
                    .expression(bin, e, vartab, function, ns)
                    .into_pointer_value();

                let value = bin.builder.build_load(ptr, "");

                if ty.is_reference_type(ns) && !ty.is_fixed_reference_type() {
                    // if the pointer is null, it needs to be allocated
                    let allocation_needed = bin
                        .builder
                        .build_is_null(value.into_pointer_value(), "allocation_needed");

                    let allocate = bin.context.append_basic_block(function, "allocate");
                    let already_allocated = bin
                        .context
                        .append_basic_block(function, "already_allocated");

                    bin.builder.build_conditional_branch(
                        allocation_needed,
                        allocate,
                        already_allocated,
                    );

                    let entry = bin.builder.get_insert_block().unwrap();

                    bin.builder.position_at_end(allocate);

                    // allocate a new struct
                    let ty = e.ty();

                    let llvm_ty = bin.llvm_type(ty.deref_memory(), ns);

                    let new_struct = bin
                        .builder
                        .build_call(
                            bin.module.get_function("__malloc").unwrap(),
                            &[llvm_ty
                                .size_of()
                                .unwrap()
                                .const_cast(bin.context.i32_type(), false)
                                .into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let new_struct = bin.builder.build_pointer_cast(
                        new_struct,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        &format!("new_{}", ty.to_string(ns)),
                    );

                    bin.builder.build_store(ptr, new_struct);

                    bin.builder.build_unconditional_branch(already_allocated);

                    bin.builder.position_at_end(already_allocated);

                    // insert phi node
                    let combined_struct_ptr = bin.builder.build_phi(
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        &format!("ptr_{}", ty.to_string(ns)),
                    );

                    combined_struct_ptr.add_incoming(&[(&value, entry), (&new_struct, allocate)]);

                    combined_struct_ptr.as_basic_value()
                } else {
                    value
                }
            }

            Expression::ZeroExt(_, t, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();
                let ty = bin.llvm_type(t, ns);

                bin.builder
                    .build_int_z_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::UnaryMinus(_, _, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_int_neg(e, "").into()
            }
            Expression::SignExt(_, t, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();
                let ty = bin.llvm_type(t, ns);

                bin.builder
                    .build_int_s_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Trunc(_, t, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();
                let ty = bin.llvm_type(t, ns);

                bin.builder
                    .build_int_truncate(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Cast(_, to, e) => {
                let from = e.ty();

                let e = self.expression(bin, e, vartab, function, ns);

                self.runtime_cast(bin, function, &from, to, e, ns)
            }
            Expression::BytesCast(_, Type::Bytes(_), Type::DynamicBytes, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();

                let size = e.get_type().get_bit_width() / 8;
                let size = bin.context.i32_type().const_int(size as u64, false);
                let elem_size = bin.context.i32_type().const_int(1, false);

                // Swap the byte order
                let bytes_ptr = bin.build_alloca(function, e.get_type(), "bytes_ptr");
                bin.builder.build_store(bytes_ptr, e);
                let bytes_ptr = bin.builder.build_pointer_cast(
                    bytes_ptr,
                    bin.context.i8_type().ptr_type(AddressSpace::Generic),
                    "bytes_ptr",
                );
                let init = bin.builder.build_pointer_cast(
                    bin.build_alloca(function, e.get_type(), "init"),
                    bin.context.i8_type().ptr_type(AddressSpace::Generic),
                    "init",
                );
                bin.builder.build_call(
                    bin.module.get_function("__leNtobeN").unwrap(),
                    &[bytes_ptr.into(), init.into(), size.into()],
                    "",
                );

                bin.builder
                    .build_call(
                        bin.module.get_function("vector_new").unwrap(),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::BytesCast(_, Type::DynamicBytes, Type::Bytes(n), e) => {
                let array = self.expression(bin, e, vartab, function, ns);

                let len = bin.vector_len(array);

                // Check if equal to n
                let is_equal_to_n = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    len,
                    bin.context.i32_type().const_int(*n as u64, false),
                    "is_equal_to_n",
                );
                let cast = bin.context.append_basic_block(function, "cast");
                let error = bin.context.append_basic_block(function, "error");
                bin.builder
                    .build_conditional_branch(is_equal_to_n, cast, error);

                bin.builder.position_at_end(error);
                self.assert_failure(
                    bin,
                    bin.context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .const_null(),
                    bin.context.i32_type().const_zero(),
                );

                bin.builder.position_at_end(cast);
                let bytes_ptr = bin.vector_bytes(array);

                // Switch byte order
                let ty = bin.context.custom_width_int_type(*n as u32 * 8);
                let le_bytes_ptr = bin.build_alloca(function, ty, "le_bytes");

                bin.builder.build_call(
                    bin.module.get_function("__beNtoleN").unwrap(),
                    &[
                        bytes_ptr.into(),
                        bin.builder
                            .build_pointer_cast(
                                le_bytes_ptr,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "le_bytes_ptr",
                            )
                            .into(),
                        len.into(),
                    ],
                    "",
                );
                bin.builder.build_load(le_bytes_ptr, "bytes")
            }
            Expression::Not(_, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();

                bin.builder
                    .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                    .into()
            }
            Expression::Complement(_, _, e) => {
                let e = self
                    .expression(bin, e, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_not(e, "").into()
            }
            Expression::BitwiseOr(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_or(left, right, "").into()
            }
            Expression::BitwiseAnd(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseXor(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_xor(left, right, "").into()
            }
            Expression::ShiftLeft(_, _, l, r) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder.build_left_shift(left, right, "").into()
            }
            Expression::ShiftRight(_, _, l, r, signed) => {
                let left = self
                    .expression(bin, l, vartab, function, ns)
                    .into_int_value();
                let right = self
                    .expression(bin, r, vartab, function, ns)
                    .into_int_value();

                bin.builder
                    .build_right_shift(left, right, *signed, "")
                    .into()
            }
            Expression::Subscript(_, elem_ty, ty, a, i) => {
                if ty.is_storage_bytes() {
                    let index = self
                        .expression(bin, i, vartab, function, ns)
                        .into_int_value();
                    let slot = self
                        .expression(bin, a, vartab, function, ns)
                        .into_int_value();
                    self.get_storage_bytes_subscript(bin, function, slot, index)
                        .into()
                } else if ty.is_contract_storage() {
                    let array = self
                        .expression(bin, a, vartab, function, ns)
                        .into_int_value();
                    let index = self.expression(bin, i, vartab, function, ns);

                    self.storage_subscript(bin, function, ty, array, index, ns)
                        .into()
                } else if elem_ty.is_builtin_struct() == Some(StructType::AccountInfo) {
                    let array = self
                        .expression(bin, a, vartab, function, ns)
                        .into_pointer_value();
                    let index = self
                        .expression(bin, i, vartab, function, ns)
                        .into_int_value();

                    unsafe {
                        bin.builder
                            .build_gep(array, &[index], "account_info")
                            .into()
                    }
                } else if ty.is_dynamic_memory() {
                    let array = self.expression(bin, a, vartab, function, ns);

                    let ty = bin.llvm_field_ty(elem_ty, ns);

                    let mut array_index = self
                        .expression(bin, i, vartab, function, ns)
                        .into_int_value();

                    // bounds checking already done; we can down-cast if necessary
                    if array_index.get_type().get_bit_width() > 32 {
                        array_index = bin.builder.build_int_truncate(
                            array_index,
                            bin.context.i32_type(),
                            "index",
                        );
                    }

                    let index = bin.builder.build_int_mul(
                        array_index,
                        ty.into_pointer_type()
                            .get_element_type()
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false),
                        "",
                    );

                    let elem = unsafe {
                        bin.builder
                            .build_gep(bin.vector_bytes(array), &[index], "index_access")
                    };

                    bin.builder
                        .build_pointer_cast(elem, ty.into_pointer_type(), "elem")
                        .into()
                } else {
                    let array = self
                        .expression(bin, a, vartab, function, ns)
                        .into_pointer_value();
                    let index = self
                        .expression(bin, i, vartab, function, ns)
                        .into_int_value();

                    unsafe {
                        bin.builder
                            .build_gep(
                                array,
                                &[bin.context.i32_type().const_zero(), index],
                                "index_access",
                            )
                            .into()
                    }
                }
            }
            Expression::StructMember(_, _, a, _)
                if a.ty().is_builtin_struct() == Some(StructType::AccountInfo) =>
            {
                self.builtin(bin, e, vartab, function, ns)
            }
            Expression::StructMember(_, _, a, i) => {
                let struct_ptr = self
                    .expression(bin, a, vartab, function, ns)
                    .into_pointer_value();

                bin.builder
                    .build_struct_gep(struct_ptr, *i as u32, "struct member")
                    .unwrap()
                    .into()
            }
            Expression::ConstArrayLiteral(_, _, dims, exprs) => {
                // For const arrays (declared with "constant" keyword, we should create a global constant
                let mut dims = dims.iter();

                let exprs = exprs
                    .iter()
                    .map(|e| {
                        self.expression(bin, e, vartab, function, ns)
                            .into_int_value()
                    })
                    .collect::<Vec<IntValue>>();
                let ty = exprs[0].get_type();

                let top_size = *dims.next().unwrap();

                // Create a vector of ArrayValues
                let mut arrays = exprs
                    .chunks(top_size as usize)
                    .map(|a| ty.const_array(a))
                    .collect::<Vec<ArrayValue>>();

                let mut ty = ty.array_type(top_size);

                // for each dimension, split the array into futher arrays
                for d in dims {
                    ty = ty.array_type(*d);

                    arrays = arrays
                        .chunks(*d as usize)
                        .map(|a| ty.const_array(a))
                        .collect::<Vec<ArrayValue>>();
                }

                // We actually end up with an array with a single entry

                // now we've created the type, and the const array. Put it into a global
                let gv =
                    bin.module
                        .add_global(ty, Some(AddressSpace::Generic), "const_array_literal");

                gv.set_linkage(Linkage::Internal);

                gv.set_initializer(&arrays[0]);
                gv.set_constant(true);

                gv.as_pointer_value().into()
            }
            Expression::ArrayLiteral(_, ty, dims, exprs) => {
                // non-const array literals should alloca'ed and each element assigned
                let ty = bin.llvm_type(ty, ns);

                let p = bin
                    .builder
                    .build_call(
                        bin.module.get_function("__malloc").unwrap(),
                        &[ty.size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false)
                            .into()],
                        "array_literal",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let array = bin.builder.build_pointer_cast(
                    p.into_pointer_value(),
                    ty.ptr_type(AddressSpace::Generic),
                    "array_literal",
                );

                for (i, expr) in exprs.iter().enumerate() {
                    let mut ind = vec![bin.context.i32_type().const_zero()];

                    let mut e = i as u32;

                    for d in dims {
                        ind.insert(1, bin.context.i32_type().const_int((e % *d).into(), false));

                        e /= *d;
                    }

                    let elemptr =
                        unsafe { bin.builder.build_gep(array, &ind, &format!("elemptr{}", i)) };

                    let elem = self.expression(bin, expr, vartab, function, ns);

                    let elem = if expr.ty().is_fixed_reference_type() {
                        bin.builder.build_load(elem.into_pointer_value(), "elem")
                    } else {
                        elem
                    };

                    bin.builder.build_store(elemptr, elem);
                }

                array.into()
            }
            Expression::AllocDynamicArray(_, ty, size, init) => {
                if matches!(ty, Type::Slice(_)) {
                    let init = init.as_ref().unwrap();

                    let data = bin.emit_global_string("const_string", init, true);

                    bin.llvm_type(ty, ns)
                        .into_struct_type()
                        .const_named_struct(&[
                            data.into(),
                            bin.context
                                .custom_width_int_type(ns.target.ptr_size().into())
                                .const_int(init.len() as u64, false)
                                .into(),
                        ])
                        .into()
                } else {
                    let elem = match ty {
                        Type::Slice(_) | Type::String | Type::DynamicBytes => Type::Bytes(1),
                        _ => ty.array_elem(),
                    };

                    let size = self
                        .expression(bin, size, vartab, function, ns)
                        .into_int_value();

                    let elem_size = bin
                        .llvm_type(&elem, ns)
                        .size_of()
                        .unwrap()
                        .const_cast(bin.context.i32_type(), false);

                    bin.vector_new(size, elem_size, init.as_ref()).into()
                }
            }
            Expression::Builtin(_, _, Builtin::ArrayLength, args)
                if args[0].ty().array_deref().is_builtin_struct().is_none() =>
            {
                let array = self.expression(bin, &args[0], vartab, function, ns);

                bin.vector_len(array).into()
            }
            Expression::Builtin(_, returns, Builtin::ReadFromBuffer, args) => {
                let v = self.expression(bin, &args[0], vartab, function, ns);
                let offset = self
                    .expression(bin, &args[1], vartab, function, ns)
                    .into_int_value();

                let data = bin.vector_bytes(v);

                let start = unsafe { bin.builder.build_gep(data, &[offset], "start") };

                if let Type::Bytes(n) = &returns[0] {
                    let store = bin.build_alloca(
                        function,
                        bin.context.custom_width_int_type(*n as u32 * 8),
                        "stack",
                    );
                    bin.builder.build_call(
                        bin.module.get_function("__beNtoleN").unwrap(),
                        &[
                            bin.builder
                                .build_pointer_cast(
                                    start,
                                    bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            bin.builder
                                .build_pointer_cast(
                                    store,
                                    bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            bin.context.i32_type().const_int(*n as u64, false).into(),
                        ],
                        "",
                    );
                    bin.builder.build_load(store, &format!("bytes{}", *n))
                } else {
                    let start = bin.builder.build_pointer_cast(
                        start,
                        bin.llvm_type(&returns[0], ns)
                            .ptr_type(AddressSpace::Generic),
                        "start",
                    );

                    bin.builder.build_load(start, "value")
                }
            }
            Expression::Keccak256(_, _, exprs) => {
                let mut length = bin.context.i32_type().const_zero();
                let mut values: Vec<(BasicValueEnum, IntValue, Type)> = Vec::new();

                // first we need to calculate the length of the buffer and get the types/lengths
                for e in exprs {
                    let v = self.expression(bin, e, vartab, function, ns);

                    let len = match e.ty() {
                        Type::DynamicBytes | Type::String => bin.vector_len(v),
                        _ => v
                            .get_type()
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false),
                    };

                    length = bin.builder.build_int_add(length, len, "");

                    values.push((v, len, e.ty()));
                }

                //  now allocate a buffer
                let src =
                    bin.builder
                        .build_array_alloca(bin.context.i8_type(), length, "keccak_src");

                // fill in all the fields
                let mut offset = bin.context.i32_type().const_zero();

                for (v, len, ty) in values {
                    let elem = unsafe { bin.builder.build_gep(src, &[offset], "elem") };

                    offset = bin.builder.build_int_add(offset, len, "");

                    match ty {
                        Type::DynamicBytes | Type::String => {
                            let data = bin.vector_bytes(v);

                            bin.builder.build_call(
                                bin.module.get_function("__memcpy").unwrap(),
                                &[
                                    elem.into(),
                                    bin.builder
                                        .build_pointer_cast(
                                            data,
                                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                            "data",
                                        )
                                        .into(),
                                    len.into(),
                                ],
                                "",
                            );
                        }
                        _ => {
                            let elem = bin.builder.build_pointer_cast(
                                elem,
                                v.get_type().ptr_type(AddressSpace::Generic),
                                "",
                            );

                            bin.builder.build_store(elem, v);
                        }
                    }
                }
                let dst = bin
                    .builder
                    .build_alloca(bin.context.custom_width_int_type(256), "keccak_dst");

                self.keccak256_hash(bin, src, length, dst, ns);

                bin.builder.build_load(dst, "keccak256_hash")
            }
            Expression::StringCompare(_, l, r) => {
                let (left, left_len) = self.string_location(bin, l, vartab, function, ns);
                let (right, right_len) = self.string_location(bin, r, vartab, function, ns);

                bin.builder
                    .build_call(
                        bin.module.get_function("__memcmp").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::StringConcat(_, _, l, r) => {
                let (left, left_len) = self.string_location(bin, l, vartab, function, ns);
                let (right, right_len) = self.string_location(bin, r, vartab, function, ns);

                bin.builder
                    .build_call(
                        bin.module.get_function("concat").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::ReturnData(_) => self.return_data(bin, function).into(),
            Expression::StorageArrayLength { array, elem_ty, .. } => {
                let slot = self
                    .expression(bin, array, vartab, function, ns)
                    .into_int_value();

                self.storage_array_length(bin, function, slot, elem_ty, ns)
                    .into()
            }
            Expression::AbiEncode {
                tys, packed, args, ..
            } => self
                .abi_encode_to_vector(
                    bin,
                    function,
                    &packed
                        .iter()
                        .map(|a| self.expression(bin, a, vartab, function, ns))
                        .collect::<Vec<BasicValueEnum>>(),
                    &args
                        .iter()
                        .map(|a| self.expression(bin, a, vartab, function, ns))
                        .collect::<Vec<BasicValueEnum>>(),
                    tys,
                    ns,
                )
                .into(),
            Expression::Builtin(_, _, Builtin::Signature, _) if ns.target != Target::Solana => {
                // need to byte-reverse selector
                let selector = bin.build_alloca(function, bin.context.i32_type(), "selector");

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                bin.builder.build_call(
                    bin.module.get_function("__beNtoleN").unwrap(),
                    &[
                        bin.builder
                            .build_pointer_cast(
                                bin.selector.as_pointer_value(),
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        bin.builder
                            .build_pointer_cast(
                                selector,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        bin.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                bin.builder.build_load(selector, "selector")
            }
            Expression::Builtin(_, _, Builtin::AddMod, args) => {
                let arith_ty = bin.context.custom_width_int_type(512);
                let res_ty = bin.context.custom_width_int_type(256);

                let x = self
                    .expression(bin, &args[0], vartab, function, ns)
                    .into_int_value();
                let y = self
                    .expression(bin, &args[1], vartab, function, ns)
                    .into_int_value();
                let k = self
                    .expression(bin, &args[2], vartab, function, ns)
                    .into_int_value();
                let dividend = bin.builder.build_int_add(
                    bin.builder.build_int_z_extend(x, arith_ty, "wide_x"),
                    bin.builder.build_int_z_extend(y, arith_ty, "wide_y"),
                    "x_plus_y",
                );

                let divisor = bin.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let pdividend = bin.build_alloca(function, arith_ty, "dividend");
                let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
                let rem = bin.build_alloca(function, arith_ty, "remainder");
                let quotient = bin.build_alloca(function, arith_ty, "quotient");

                bin.builder.build_store(pdividend, dividend);
                bin.builder.build_store(pdivisor, divisor);

                let ret = bin
                    .builder
                    .build_call(
                        bin.module.get_function("udivmod512").unwrap(),
                        &[
                            pdividend.into(),
                            pdivisor.into(),
                            rem.into(),
                            quotient.into(),
                        ],
                        "quotient",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // On Solana the return type is 64 bit
                let ret: BasicValueEnum = bin
                    .builder
                    .build_int_z_extend(
                        ret,
                        bin.return_values[&ReturnCode::Success].get_type(),
                        "ret",
                    )
                    .into();

                bin.builder.build_return(Some(&ret));
                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(quotient, "quotient")
                    .into_int_value();

                bin.builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::Builtin(_, _, Builtin::MulMod, args) => {
                let arith_ty = bin.context.custom_width_int_type(512);
                let res_ty = bin.context.custom_width_int_type(256);

                let x = self
                    .expression(bin, &args[0], vartab, function, ns)
                    .into_int_value();
                let y = self
                    .expression(bin, &args[1], vartab, function, ns)
                    .into_int_value();
                let x_m = bin.build_alloca(function, arith_ty, "x_m");
                let y_m = bin.build_alloca(function, arith_ty, "x_y");
                let x_times_y_m = bin.build_alloca(function, arith_ty, "x_times_y_m");

                bin.builder
                    .build_store(x_m, bin.builder.build_int_z_extend(x, arith_ty, "wide_x"));
                bin.builder
                    .build_store(y_m, bin.builder.build_int_z_extend(y, arith_ty, "wide_y"));

                bin.builder.build_call(
                    bin.module.get_function("__mul32").unwrap(),
                    &[
                        bin.builder
                            .build_pointer_cast(
                                x_m,
                                bin.context.i32_type().ptr_type(AddressSpace::Generic),
                                "left",
                            )
                            .into(),
                        bin.builder
                            .build_pointer_cast(
                                y_m,
                                bin.context.i32_type().ptr_type(AddressSpace::Generic),
                                "right",
                            )
                            .into(),
                        bin.builder
                            .build_pointer_cast(
                                x_times_y_m,
                                bin.context.i32_type().ptr_type(AddressSpace::Generic),
                                "output",
                            )
                            .into(),
                        bin.context.i32_type().const_int(512 / 32, false).into(),
                    ],
                    "",
                );
                let k = self
                    .expression(bin, &args[2], vartab, function, ns)
                    .into_int_value();
                let dividend = bin.builder.build_load(x_times_y_m, "x_t_y");

                let divisor = bin.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let pdividend = bin.build_alloca(function, arith_ty, "dividend");
                let pdivisor = bin.build_alloca(function, arith_ty, "divisor");
                let rem = bin.build_alloca(function, arith_ty, "remainder");
                let quotient = bin.build_alloca(function, arith_ty, "quotient");

                bin.builder.build_store(pdividend, dividend);
                bin.builder.build_store(pdivisor, divisor);

                let ret = bin
                    .builder
                    .build_call(
                        bin.module.get_function("udivmod512").unwrap(),
                        &[
                            pdividend.into(),
                            pdivisor.into(),
                            rem.into(),
                            quotient.into(),
                        ],
                        "quotient",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let success = bin.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    bin.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = bin.context.append_basic_block(function, "success");
                let bail_block = bin.context.append_basic_block(function, "bail");
                bin.builder
                    .build_conditional_branch(success, success_block, bail_block);

                bin.builder.position_at_end(bail_block);

                // On Solana the return type is 64 bit
                let ret: BasicValueEnum = bin
                    .builder
                    .build_int_z_extend(
                        ret,
                        bin.return_values[&ReturnCode::Success].get_type(),
                        "ret",
                    )
                    .into();

                bin.builder.build_return(Some(&ret));

                bin.builder.position_at_end(success_block);

                let quotient = bin
                    .builder
                    .build_load(quotient, "quotient")
                    .into_int_value();

                bin.builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::Builtin(_, _, hash @ Builtin::Ripemd160, args)
            | Expression::Builtin(_, _, hash @ Builtin::Keccak256, args)
            | Expression::Builtin(_, _, hash @ Builtin::Blake2_128, args)
            | Expression::Builtin(_, _, hash @ Builtin::Blake2_256, args)
            | Expression::Builtin(_, _, hash @ Builtin::Sha256, args) => {
                let v = self.expression(bin, &args[0], vartab, function, ns);

                let hash = match hash {
                    Builtin::Ripemd160 => HashTy::Ripemd160,
                    Builtin::Sha256 => HashTy::Sha256,
                    Builtin::Keccak256 => HashTy::Keccak256,
                    Builtin::Blake2_128 => HashTy::Blake2_128,
                    Builtin::Blake2_256 => HashTy::Blake2_256,
                    _ => unreachable!(),
                };

                self.hash(
                    bin,
                    function,
                    hash,
                    bin.vector_bytes(v),
                    bin.vector_len(v),
                    ns,
                )
                .into()
            }
            Expression::Builtin(..) => self.builtin(bin, e, vartab, function, ns),
            Expression::InternalFunctionCfg(cfg_no) => bin.functions[cfg_no]
                .as_global_value()
                .as_pointer_value()
                .into(),
            Expression::FormatString(_, args) => {
                self.format_string(bin, args, vartab, function, ns)
            }

            Expression::AdvancePointer {
                pointer,
                bytes_offset,
            } => {
                let pointer = if pointer.ty().is_dynamic_memory() {
                    bin.vector_bytes(self.expression(bin, pointer, vartab, function, ns))
                } else {
                    self.expression(bin, pointer, vartab, function, ns)
                        .into_pointer_value()
                };
                let offset = self
                    .expression(bin, bytes_offset, vartab, function, ns)
                    .into_int_value();
                let advanced = unsafe { bin.builder.build_gep(pointer, &[offset], "adv_pointer") };

                advanced.into()
            }

            Expression::RationalNumberLiteral(..)
            | Expression::List(..)
            | Expression::Undefined(..)
            | Expression::Poison
            | Expression::BytesCast(..) => {
                unreachable!("should not exist in cfg")
            }
        }
    }

    fn compare_address(
        &self,
        binary: &Binary<'a>,
        left: &Expression,
        right: &Expression,
        op: inkwell::IntPredicate,
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) -> IntValue<'a> {
        let l = self
            .expression(binary, left, vartab, function, ns)
            .into_array_value();
        let r = self
            .expression(binary, right, vartab, function, ns)
            .into_array_value();

        let left = binary.build_alloca(function, binary.address_type(ns), "left");
        let right = binary.build_alloca(function, binary.address_type(ns), "right");

        binary.builder.build_store(left, l);
        binary.builder.build_store(right, r);

        let res = binary
            .builder
            .build_call(
                binary.module.get_function("__memcmp_ord").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            left,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            right,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    binary
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64, false)
                        .into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        binary
            .builder
            .build_int_compare(op, res, binary.context.i32_type().const_zero(), "")
    }

    /// Load a string from expression or create global
    fn string_location(
        &self,
        bin: &Binary<'a>,
        location: &StringLocation<Expression>,
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        match location {
            StringLocation::CompileTime(literal) => (
                bin.emit_global_string("const_string", literal, true),
                bin.context
                    .i32_type()
                    .const_int(literal.len() as u64, false),
            ),
            StringLocation::RunTime(e) => {
                let v = self.expression(bin, e, vartab, function, ns);

                (bin.vector_bytes(v), bin.vector_len(v))
            }
        }
    }

    fn runtime_cast(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        from: &Type,
        to: &Type,
        val: BasicValueEnum<'a>,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        if matches!(from, Type::Address(_) | Type::Contract(_))
            && matches!(to, Type::Address(_) | Type::Contract(_))
        {
            // no conversion needed
            val
        } else if let Type::Address(_) = to {
            let llvm_ty = bin.llvm_type(from, ns);

            let src = bin.build_alloca(function, llvm_ty, "dest");

            bin.builder.build_store(src, val.into_int_value());

            let dest = bin.build_alloca(function, bin.address_type(ns), "address");

            let len = bin
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false);

            bin.builder.build_call(
                bin.module.get_function("__leNtobeN").unwrap(),
                &[
                    bin.builder
                        .build_pointer_cast(
                            src,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "address_ptr",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            dest,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "dest_ptr",
                        )
                        .into(),
                    len.into(),
                ],
                "",
            );

            bin.builder.build_load(dest, "val")
        } else if let Type::Address(_) = from {
            let llvm_ty = bin.llvm_type(to, ns);

            let src = bin.build_alloca(function, bin.address_type(ns), "address");

            bin.builder.build_store(src, val.into_array_value());

            let dest = bin.build_alloca(function, llvm_ty, "dest");

            let len = bin
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false);

            bin.builder.build_call(
                bin.module.get_function("__beNtoleN").unwrap(),
                &[
                    bin.builder
                        .build_pointer_cast(
                            src,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "address_ptr",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            dest,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "dest_ptr",
                        )
                        .into(),
                    len.into(),
                ],
                "",
            );

            bin.builder.build_load(dest, "val")
        } else if matches!(from, Type::Bool) && matches!(to, Type::Int(_) | Type::Uint(_)) {
            bin.builder
                .build_int_cast(
                    val.into_int_value(),
                    bin.llvm_type(to, ns).into_int_type(),
                    "bool_to_int_cast",
                )
                .into()
        } else if from.is_reference_type(ns) && matches!(to, Type::Uint(_)) {
            bin.builder
                .build_ptr_to_int(
                    val.into_pointer_value(),
                    bin.llvm_type(to, ns).into_int_type(),
                    "ptr_to_int",
                )
                .into()
        } else if matches!((from, to), (Type::DynamicBytes, Type::Slice(_))) {
            let slice = bin.build_alloca(function, bin.llvm_type(to, ns), "slice");

            let data = bin.vector_bytes(val);

            let data_ptr = bin.builder.build_struct_gep(slice, 0, "data").unwrap();

            bin.builder.build_store(data_ptr, data);

            let len =
                bin.builder
                    .build_int_z_extend(bin.vector_len(val), bin.context.i64_type(), "len");

            let len_ptr = bin.builder.build_struct_gep(slice, 1, "len").unwrap();

            bin.builder.build_store(len_ptr, len);

            bin.builder.build_load(slice, "slice")
        } else {
            val
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn emit_cfg(
        &mut self,
        bin: &mut Binary<'a>,
        contract: &Contract,
        cfg: &ControlFlowGraph,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        let dibuilder = &bin.dibuilder;
        let compile_unit = &bin.compile_unit;
        let file = compile_unit.get_file();
        let mut di_func_scope: Option<DISubprogram<'_>> = None;

        if bin.generate_debug_info {
            let return_type = function.get_type().get_return_type();
            match return_type {
                None => {}
                Some(return_type) => {
                    let return_type_size = return_type.size_of().unwrap();
                    let size = return_type_size.get_type().get_bit_width();
                    let mut type_name = "size_".to_owned();
                    type_name.push_str(&size.to_string());
                    let di_flags = if cfg.public {
                        inkwell::debug_info::DIFlagsConstants::PUBLIC
                    } else {
                        inkwell::debug_info::DIFlagsConstants::PRIVATE
                    };

                    let di_return_type = dibuilder
                        .create_basic_type(&type_name, size as u64, 0x00, di_flags)
                        .unwrap();
                    let param_types = function.get_type().get_param_types();
                    let di_param_types: Vec<DIType<'_>> = param_types
                        .iter()
                        .map(|typ| {
                            let mut param_tname = "size_".to_owned();
                            let param_size = typ.size_of().unwrap().get_type().get_bit_width();
                            param_tname.push_str(&size.to_string());
                            dibuilder
                                .create_basic_type(&param_tname, param_size as u64, 0x00, di_flags)
                                .unwrap()
                                .as_type()
                        })
                        .collect();
                    let di_func_type = dibuilder.create_subroutine_type(
                        file,
                        Some(di_return_type.as_type()),
                        di_param_types.as_slice(),
                        di_flags,
                    );

                    let func_loc = cfg.blocks[0].instr.first().unwrap().loc();
                    let line_num = if let Loc::File(file_offset, offset, _) = func_loc {
                        let (line, _) = ns.files[file_offset].offset_to_line_column(offset);
                        line
                    } else {
                        0
                    };

                    di_func_scope = Some(dibuilder.create_function(
                        compile_unit.as_debug_info_scope(),
                        function.get_name().to_str().unwrap(),
                        None,
                        file,
                        line_num.try_into().unwrap(),
                        di_func_type,
                        true,
                        true,
                        line_num.try_into().unwrap(),
                        di_flags,
                        false,
                    ));
                    function.set_subprogram(di_func_scope.unwrap());
                }
            }
        }

        // recurse through basic blocks
        struct BasicBlock<'a> {
            bb: inkwell::basic_block::BasicBlock<'a>,
            phis: HashMap<usize, PhiValue<'a>>,
        }

        struct Work<'b> {
            block_no: usize,
            vars: HashMap<usize, Variable<'b>>,
        }

        let mut blocks: HashMap<usize, BasicBlock> = HashMap::new();

        fn create_block<'a>(
            block_no: usize,
            bin: &Binary<'a>,
            cfg: &ControlFlowGraph,
            function: FunctionValue,
            ns: &Namespace,
        ) -> BasicBlock<'a> {
            let cfg_bb = &cfg.blocks[block_no];
            let mut phis = HashMap::new();

            let bb = bin.context.append_basic_block(function, &cfg_bb.name);

            bin.builder.position_at_end(bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    let ty = bin.llvm_var_ty(&cfg.vars[v].ty, ns);

                    phis.insert(*v, bin.builder.build_phi(ty, &cfg.vars[v].id.name));
                }
            }

            BasicBlock { bb, phis }
        }

        let mut work = VecDeque::new();

        blocks.insert(0, create_block(0, bin, cfg, function, ns));

        // On Solana, the last argument is the accounts
        if ns.target == Target::Solana {
            bin.parameters = Some(function.get_last_param().unwrap().into_pointer_value());
        }

        // Create all the stack variables
        let mut vars = HashMap::new();

        for (no, v) in &cfg.vars {
            match v.storage {
                Storage::Local if v.ty.is_reference_type(ns) && !v.ty.is_contract_storage() => {
                    // a null pointer means an empty, zero'ed thing, be it string, struct or array
                    let value = bin
                        .llvm_type(&v.ty, ns)
                        .ptr_type(AddressSpace::Generic)
                        .const_null()
                        .into();

                    vars.insert(*no, Variable { value });
                }
                Storage::Local if v.ty.is_contract_storage() => {
                    vars.insert(
                        *no,
                        Variable {
                            value: bin
                                .llvm_type(&ns.storage_type(), ns)
                                .into_int_type()
                                .const_zero()
                                .into(),
                        },
                    );
                }
                Storage::Constant(_) | Storage::Contract(_) if v.ty.is_reference_type(ns) => {
                    // This needs a placeholder
                    vars.insert(
                        *no,
                        Variable {
                            value: bin.context.bool_type().get_undef().into(),
                        },
                    );
                }
                Storage::Local | Storage::Contract(_) | Storage::Constant(_) => {
                    let ty = bin.llvm_type(&v.ty, ns);
                    vars.insert(
                        *no,
                        Variable {
                            value: if ty.is_pointer_type() {
                                ty.into_pointer_type().const_zero().into()
                            } else if ty.is_array_type() {
                                ty.into_array_type().const_zero().into()
                            } else if ty.is_int_type() {
                                ty.into_int_type().const_zero().into()
                            } else {
                                ty.into_struct_type().const_zero().into()
                            },
                        },
                    );
                }
            }
        }

        work.push_back(Work { block_no: 0, vars });

        while let Some(mut w) = work.pop_front() {
            let bb = blocks.get(&w.block_no).unwrap();

            bin.builder.position_at_end(bb.bb);

            for (v, phi) in bb.phis.iter() {
                w.vars.get_mut(v).unwrap().value = (*phi).as_basic_value();
            }

            for ins in &cfg.blocks[w.block_no].instr {
                if bin.generate_debug_info {
                    let debug_loc = ins.loc();
                    if let Loc::File(file_offset, offset, _) = debug_loc {
                        let (line, col) = ns.files[file_offset].offset_to_line_column(offset);
                        let debug_loc = dibuilder.create_debug_location(
                            bin.context,
                            line as u32,
                            col as u32,
                            di_func_scope.unwrap().as_debug_info_scope(),
                            None,
                        );
                        bin.builder
                            .set_current_debug_location(bin.context, debug_loc);
                    }
                }
                match ins {
                    Instr::Nop => (),
                    Instr::Return { value } if value.is_empty() => {
                        bin.builder
                            .build_return(Some(&bin.return_values[&ReturnCode::Success]));
                    }
                    Instr::Return { value } => {
                        let returns_offset = cfg.params.len();
                        for (i, val) in value.iter().enumerate() {
                            let arg = function.get_nth_param((returns_offset + i) as u32).unwrap();
                            let retval = self.expression(bin, val, &w.vars, function, ns);

                            bin.builder.build_store(arg.into_pointer_value(), retval);
                        }

                        bin.builder
                            .build_return(Some(&bin.return_values[&ReturnCode::Success]));
                    }
                    Instr::Set { res, expr, .. } => {
                        if let Expression::Undefined(expr_type) = expr {
                            // If the variable has been declared as undefined, but we can
                            // initialize it with a default value
                            if let Some(default_expr) = expr_type.default(ns) {
                                w.vars.get_mut(res).unwrap().value =
                                    self.expression(bin, &default_expr, &w.vars, function, ns);
                            }
                        } else {
                            w.vars.get_mut(res).unwrap().value =
                                self.expression(bin, expr, &w.vars, function, ns);
                        }
                    }
                    Instr::Branch { block: dest } => {
                        let pos = bin.builder.get_insert_block().unwrap();

                        if !blocks.contains_key(dest) {
                            blocks.insert(*dest, create_block(*dest, bin, cfg, function, ns));
                            work.push_back(Work {
                                block_no: *dest,
                                vars: w.vars.clone(),
                            });
                        }

                        let bb = blocks.get(dest).unwrap();

                        for (v, phi) in bb.phis.iter() {
                            phi.add_incoming(&[(&w.vars[v].value, pos)]);
                        }

                        bin.builder.position_at_end(pos);
                        bin.builder.build_unconditional_branch(bb.bb);
                    }
                    Instr::Store { dest, data } => {
                        let value_ref = self.expression(bin, data, &w.vars, function, ns);
                        let dest_ref = self
                            .expression(bin, dest, &w.vars, function, ns)
                            .into_pointer_value();
                        bin.builder.build_store(dest_ref, value_ref);
                    }
                    Instr::BranchCond {
                        cond,
                        true_block: true_,
                        false_block: false_,
                    } => {
                        let cond = self.expression(bin, cond, &w.vars, function, ns);

                        let pos = bin.builder.get_insert_block().unwrap();

                        let bb_true = {
                            if !blocks.contains_key(true_) {
                                blocks.insert(*true_, create_block(*true_, bin, cfg, function, ns));
                                work.push_back(Work {
                                    block_no: *true_,
                                    vars: w.vars.clone(),
                                });
                            }

                            let bb = blocks.get(true_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                phi.add_incoming(&[(&w.vars[v].value, pos)]);
                            }

                            bb.bb
                        };

                        let bb_false = {
                            if !blocks.contains_key(false_) {
                                blocks
                                    .insert(*false_, create_block(*false_, bin, cfg, function, ns));
                                work.push_back(Work {
                                    block_no: *false_,
                                    vars: w.vars.clone(),
                                });
                            }

                            let bb = blocks.get(false_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                phi.add_incoming(&[(&w.vars[v].value, pos)]);
                            }

                            bb.bb
                        };

                        bin.builder.position_at_end(pos);
                        bin.builder.build_conditional_branch(
                            cond.into_int_value(),
                            bb_true,
                            bb_false,
                        );
                    }
                    Instr::LoadStorage { res, ty, storage } => {
                        let mut slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();

                        w.vars.get_mut(res).unwrap().value =
                            self.storage_load(bin, ty, &mut slot, function, ns);
                    }
                    Instr::ClearStorage { ty, storage } => {
                        let mut slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();

                        self.storage_delete(bin, ty, &mut slot, function, ns);
                    }
                    Instr::SetStorage { ty, value, storage } => {
                        let value = self.expression(bin, value, &w.vars, function, ns);

                        let mut slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();

                        self.storage_store(bin, ty, true, &mut slot, value, function, ns);
                    }
                    Instr::SetStorageBytes {
                        storage,
                        value,
                        offset,
                    } => {
                        let value = self.expression(bin, value, &w.vars, function, ns);

                        let slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();
                        let offset = self
                            .expression(bin, offset, &w.vars, function, ns)
                            .into_int_value();

                        self.set_storage_bytes_subscript(
                            bin,
                            function,
                            slot,
                            offset,
                            value.into_int_value(),
                        );
                    }
                    Instr::PushStorage {
                        res,
                        ty,
                        storage,
                        value,
                    } => {
                        let val = value
                            .as_ref()
                            .map(|expr| self.expression(bin, expr, &w.vars, function, ns));
                        let slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();

                        w.vars.get_mut(res).unwrap().value =
                            self.storage_push(bin, function, ty, slot, val, ns);
                    }
                    Instr::PopStorage { res, ty, storage } => {
                        let slot = self
                            .expression(bin, storage, &w.vars, function, ns)
                            .into_int_value();

                        let value = self.storage_pop(bin, function, ty, slot, res.is_some(), ns);

                        if let Some(res) = res {
                            w.vars.get_mut(res).unwrap().value = value.unwrap();
                        }
                    }
                    Instr::PushMemory {
                        res,
                        ty,
                        array,
                        value,
                    } => {
                        let arr = w.vars[array].value;

                        let llvm_ty = bin.llvm_type(ty, ns);
                        let elem_ty = ty.array_elem();

                        // Calculate total size for reallocation
                        let llvm_elem_ty = bin.llvm_field_ty(&elem_ty, ns);
                        let elem_size = llvm_elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false);
                        let len = bin.vector_len(arr);
                        let new_len = bin.builder.build_int_add(
                            len,
                            bin.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = bin
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false);
                        let size = bin.builder.build_int_mul(elem_size, new_len, "");
                        let size = bin.builder.build_int_add(size, vec_size, "");

                        // Reallocate and reassign the array pointer
                        let new = bin
                            .builder
                            .build_call(
                                bin.module.get_function("__realloc").unwrap(),
                                &[
                                    bin.builder
                                        .build_pointer_cast(
                                            arr.into_pointer_value(),
                                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                            "a",
                                        )
                                        .into(),
                                    size.into(),
                                ],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        let dest = bin.builder.build_pointer_cast(
                            new,
                            llvm_ty.ptr_type(AddressSpace::Generic),
                            "dest",
                        );
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Store the value into the last element
                        let slot_ptr = unsafe {
                            bin.builder.build_gep(
                                dest,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_int(2, false),
                                    bin.builder.build_int_mul(len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let value = self.expression(bin, value, &w.vars, function, ns);
                        let elem_ptr = bin.builder.build_pointer_cast(
                            slot_ptr,
                            llvm_elem_ty.ptr_type(AddressSpace::Generic),
                            "element pointer",
                        );
                        let value = if elem_ty.is_fixed_reference_type() {
                            w.vars.get_mut(res).unwrap().value = elem_ptr.into();
                            bin.builder.build_load(value.into_pointer_value(), "elem")
                        } else {
                            w.vars.get_mut(res).unwrap().value = value;
                            value
                        };
                        bin.builder.build_store(elem_ptr, value);

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            bin.builder.build_gep(
                                dest,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = bin.builder.build_pointer_cast(
                            len_ptr,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        bin.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            bin.builder.build_gep(
                                dest,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = bin.builder.build_pointer_cast(
                            size_ptr,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        bin.builder.build_store(size_field, new_len);
                    }
                    Instr::PopMemory { res, ty, array } => {
                        let a = w.vars[array].value.into_pointer_value();
                        let len = unsafe {
                            bin.builder.build_gep(
                                a,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_zero(),
                                ],
                                "a_len",
                            )
                        };
                        let len = bin.builder.build_load(len, "a_len").into_int_value();

                        // First check if the array is empty
                        let is_array_empty = bin.builder.build_int_compare(
                            IntPredicate::EQ,
                            len,
                            bin.context.i32_type().const_zero(),
                            "is_array_empty",
                        );
                        let error = bin.context.append_basic_block(function, "error");
                        let pop = bin.context.append_basic_block(function, "pop");
                        bin.builder
                            .build_conditional_branch(is_array_empty, error, pop);

                        bin.builder.position_at_end(error);
                        self.assert_failure(
                            bin,
                            bin.context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            bin.context.i32_type().const_zero(),
                        );

                        bin.builder.position_at_end(pop);
                        let llvm_ty = bin.llvm_type(ty, ns);

                        let elem_ty = ty.array_elem();
                        let llvm_elem_ty = bin.llvm_field_ty(&elem_ty, ns);

                        // Calculate total size for reallocation
                        let elem_size = llvm_elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false);
                        let new_len = bin.builder.build_int_sub(
                            len,
                            bin.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = bin
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(bin.context.i32_type(), false);
                        let size = bin.builder.build_int_mul(elem_size, new_len, "");
                        let size = bin.builder.build_int_add(size, vec_size, "");

                        // Get the pointer to the last element and return it
                        let slot_ptr = unsafe {
                            bin.builder.build_gep(
                                a,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_int(2, false),
                                    bin.builder.build_int_mul(new_len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let slot_ptr = bin.builder.build_pointer_cast(
                            slot_ptr,
                            llvm_elem_ty.ptr_type(AddressSpace::Generic),
                            "slot_ptr",
                        );
                        if elem_ty.is_fixed_reference_type() {
                            w.vars.get_mut(res).unwrap().value = slot_ptr.into();
                        } else {
                            let ret_val = bin.builder.build_load(slot_ptr, "");
                            w.vars.get_mut(res).unwrap().value = ret_val;
                        }

                        // Reallocate and reassign the array pointer
                        let a = bin.builder.build_pointer_cast(
                            a,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "a",
                        );
                        let new = bin
                            .builder
                            .build_call(
                                bin.module.get_function("__realloc").unwrap(),
                                &[a.into(), size.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        let dest = bin.builder.build_pointer_cast(
                            new,
                            llvm_ty.ptr_type(AddressSpace::Generic),
                            "dest",
                        );
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            bin.builder.build_gep(
                                dest,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = bin.builder.build_pointer_cast(
                            len_ptr,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        bin.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            bin.builder.build_gep(
                                dest,
                                &[
                                    bin.context.i32_type().const_zero(),
                                    bin.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = bin.builder.build_pointer_cast(
                            size_ptr,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        bin.builder.build_store(size_field, new_len);
                    }
                    Instr::AssertFailure { expr: None } => {
                        self.assert_failure(
                            bin,
                            bin.context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            bin.context.i32_type().const_zero(),
                        );
                    }
                    Instr::AssertFailure { expr: Some(expr) } => {
                        let v = self.expression(bin, expr, &w.vars, function, ns);

                        let selector = 0x08c3_79a0u32;

                        let (data, len) = self.abi_encode(
                            bin,
                            Some(bin.context.i32_type().const_int(selector as u64, false)),
                            false,
                            function,
                            &[v],
                            &[Type::String],
                            ns,
                        );

                        self.assert_failure(bin, data, len);
                    }
                    Instr::Print { expr } => {
                        let expr = self.expression(bin, expr, &w.vars, function, ns);

                        self.print(bin, bin.vector_bytes(expr), bin.vector_len(expr));
                    }
                    Instr::Call {
                        res,
                        call: InternalCallTy::Static { cfg_no },
                        args,
                        ..
                    } => {
                        let f = &contract.cfg[*cfg_no];

                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(bin, p, &w.vars, function, ns).into())
                            .collect::<Vec<BasicMetadataValueEnum>>();

                        if !res.is_empty() {
                            for v in f.returns.iter() {
                                parms.push(if ns.target == Target::Solana {
                                    bin.build_alloca(
                                        function,
                                        bin.llvm_var_ty(&v.ty, ns),
                                        v.name_as_str(),
                                    )
                                    .into()
                                } else {
                                    bin.builder
                                        .build_alloca(bin.llvm_var_ty(&v.ty, ns), v.name_as_str())
                                        .into()
                                });
                            }
                        }

                        if let Some(parameters) = bin.parameters {
                            parms.push(parameters.into());
                        }

                        let ret = bin
                            .builder
                            .build_call(bin.functions[cfg_no], &parms, "")
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        let success = bin.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            bin.return_values[&ReturnCode::Success],
                            "success",
                        );

                        let success_block = bin.context.append_basic_block(function, "success");
                        let bail_block = bin.context.append_basic_block(function, "bail");
                        bin.builder
                            .build_conditional_branch(success, success_block, bail_block);

                        bin.builder.position_at_end(bail_block);

                        bin.builder.build_return(Some(&ret));
                        bin.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, v) in f.returns.iter().enumerate() {
                                let val = bin.builder.build_load(
                                    parms[args.len() + i].into_pointer_value(),
                                    v.name_as_str(),
                                );

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value()
                                    && !(v.ty.is_reference_type(ns)
                                        || matches!(v.ty, Type::ExternalFunction { .. }))
                                {
                                    bin.builder.build_store(dest.into_pointer_value(), val);
                                } else {
                                    w.vars.get_mut(&res[i]).unwrap().value = val;
                                }
                            }
                        }
                    }
                    Instr::Call {
                        res,
                        call: InternalCallTy::Builtin { ast_func_no },
                        args,
                        ..
                    } => {
                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(bin, p, &w.vars, function, ns).into())
                            .collect::<Vec<BasicMetadataValueEnum>>();

                        let func = &ns.functions[*ast_func_no];

                        if !res.is_empty() {
                            for v in func.returns.iter() {
                                parms.push(if ns.target == Target::Solana {
                                    bin.build_alloca(
                                        function,
                                        bin.llvm_var_ty(&v.ty, ns),
                                        v.name_as_str(),
                                    )
                                    .into()
                                } else {
                                    bin.builder
                                        .build_alloca(bin.llvm_var_ty(&v.ty, ns), v.name_as_str())
                                        .into()
                                });
                            }
                        }

                        let ret = self.builtin_function(bin, func, &parms, ns);

                        let success = bin.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            bin.return_values[&ReturnCode::Success],
                            "success",
                        );

                        let success_block = bin.context.append_basic_block(function, "success");
                        let bail_block = bin.context.append_basic_block(function, "bail");
                        bin.builder
                            .build_conditional_branch(success, success_block, bail_block);

                        bin.builder.position_at_end(bail_block);

                        bin.builder.build_return(Some(&ret));
                        bin.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, v) in func.returns.iter().enumerate() {
                                let val = bin.builder.build_load(
                                    parms[args.len() + i].into_pointer_value(),
                                    v.name_as_str(),
                                );

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value()
                                    && !(v.ty.is_reference_type(ns)
                                        || matches!(v.ty, Type::ExternalFunction { .. }))
                                {
                                    bin.builder.build_store(dest.into_pointer_value(), val);
                                } else {
                                    w.vars.get_mut(&res[i]).unwrap().value = val;
                                }
                            }
                        }
                    }
                    Instr::Call {
                        res,
                        call: InternalCallTy::Dynamic(call_expr),
                        args,
                        ..
                    } => {
                        let ty = call_expr.ty();

                        let returns = if let Type::InternalFunction { returns, .. } = ty.deref_any()
                        {
                            returns
                        } else {
                            panic!("should be Type::InternalFunction type");
                        };

                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(bin, p, &w.vars, function, ns).into())
                            .collect::<Vec<BasicMetadataValueEnum>>();

                        if !res.is_empty() {
                            for ty in returns.iter() {
                                parms.push(
                                    bin.build_alloca(function, bin.llvm_var_ty(ty, ns), "")
                                        .into(),
                                );
                            }
                        }

                        // on Solana, we need to pass the accounts parameter around
                        if let Some(parameters) = bin.parameters {
                            parms.push(parameters.into());
                        }

                        let callable = CallableValue::try_from(
                            self.expression(bin, call_expr, &w.vars, function, ns)
                                .into_pointer_value(),
                        )
                        .unwrap();

                        let ret = bin
                            .builder
                            .build_call(callable, &parms, "")
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        let success = bin.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            bin.return_values[&ReturnCode::Success],
                            "success",
                        );

                        let success_block = bin.context.append_basic_block(function, "success");
                        let bail_block = bin.context.append_basic_block(function, "bail");
                        bin.builder
                            .build_conditional_branch(success, success_block, bail_block);

                        bin.builder.position_at_end(bail_block);

                        bin.builder.build_return(Some(&ret));
                        bin.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, ty) in returns.iter().enumerate() {
                                let val = bin
                                    .builder
                                    .build_load(parms[args.len() + i].into_pointer_value(), "");

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value() && !ty.is_reference_type(ns) {
                                    bin.builder.build_store(dest.into_pointer_value(), val);
                                } else {
                                    w.vars.get_mut(&res[i]).unwrap().value = val;
                                }
                            }
                        }
                    }
                    Instr::Constructor {
                        success,
                        res,
                        contract_no,
                        constructor_no,
                        args,
                        value,
                        gas,
                        salt,
                        space,
                    } => {
                        let args = &args
                            .iter()
                            .map(|a| self.expression(bin, a, &w.vars, function, ns))
                            .collect::<Vec<BasicValueEnum>>();

                        let address = bin.build_alloca(function, bin.address_type(ns), "address");

                        let gas = self
                            .expression(bin, gas, &w.vars, function, ns)
                            .into_int_value();
                        let value = value.as_ref().map(|v| {
                            self.expression(bin, v, &w.vars, function, ns)
                                .into_int_value()
                        });
                        let salt = salt.as_ref().map(|v| {
                            self.expression(bin, v, &w.vars, function, ns)
                                .into_int_value()
                        });
                        let space = space.as_ref().map(|v| {
                            self.expression(bin, v, &w.vars, function, ns)
                                .into_int_value()
                        });

                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        self.create_contract(
                            bin,
                            function,
                            success,
                            *contract_no,
                            *constructor_no,
                            bin.builder.build_pointer_cast(
                                address,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            args,
                            gas,
                            value,
                            salt,
                            space,
                            ns,
                        );

                        w.vars.get_mut(res).unwrap().value =
                            bin.builder.build_load(address, "address");
                    }
                    Instr::ExternalCall {
                        success,
                        address,
                        payload,
                        value,
                        gas,
                        callty,
                        accounts,
                        seeds,
                    } => {
                        let gas = self
                            .expression(bin, gas, &w.vars, function, ns)
                            .into_int_value();
                        let value = self
                            .expression(bin, value, &w.vars, function, ns)
                            .into_int_value();
                        let payload_ty = payload.ty();
                        let payload = self.expression(bin, payload, &w.vars, function, ns);

                        let address = if let Some(address) = address {
                            let address = self.expression(bin, address, &w.vars, function, ns);

                            let addr = bin.build_array_alloca(
                                function,
                                bin.context.i8_type(),
                                bin.context
                                    .i32_type()
                                    .const_int(ns.address_length as u64, false),
                                "address",
                            );

                            bin.builder.build_store(
                                bin.builder.build_pointer_cast(
                                    addr,
                                    address.get_type().ptr_type(AddressSpace::Generic),
                                    "address",
                                ),
                                address,
                            );

                            Some(addr)
                        } else {
                            None
                        };

                        let accounts = if let Some(accounts) = accounts {
                            let ty = accounts.ty();

                            let expr = self.expression(bin, accounts, &w.vars, function, ns);

                            if let Some(n) = ty.array_length() {
                                let accounts = expr.into_pointer_value();
                                let len =
                                    bin.context.i32_type().const_int(n.to_u64().unwrap(), false);

                                Some((accounts, len))
                            } else {
                                let addr = bin.vector_bytes(expr);
                                let len = bin.vector_len(expr);
                                Some((addr, len))
                            }
                        } else {
                            None
                        };

                        let (payload_ptr, payload_len) = if payload_ty == Type::DynamicBytes {
                            (bin.vector_bytes(payload), bin.vector_len(payload))
                        } else {
                            let ptr = payload.into_pointer_value();
                            let len = ptr
                                .get_type()
                                .get_element_type()
                                .size_of()
                                .unwrap()
                                .const_cast(bin.context.i32_type(), false);

                            (ptr, len)
                        };

                        let seeds = if let Some(seeds) = seeds {
                            let len = seeds.ty().array_length().unwrap().to_u64().unwrap();
                            let seeds_ty = bin.llvm_type(
                                &Type::Slice(Box::new(Type::Slice(Box::new(Type::Bytes(1))))),
                                ns,
                            );

                            let output_seeds = bin.build_array_alloca(
                                function,
                                seeds_ty,
                                bin.context.i64_type().const_int(len, false),
                                "seeds",
                            );

                            if let Expression::ArrayLiteral(_, _, _, exprs) = seeds {
                                for i in 0..len {
                                    let val = self.expression(
                                        bin,
                                        &exprs[i as usize],
                                        &w.vars,
                                        function,
                                        ns,
                                    );

                                    let seed_count = val
                                        .get_type()
                                        .into_pointer_type()
                                        .get_element_type()
                                        .into_array_type()
                                        .len();

                                    let dest = unsafe {
                                        bin.builder.build_gep(
                                            output_seeds,
                                            &[
                                                bin.context.i32_type().const_int(i, false),
                                                bin.context.i32_type().const_zero(),
                                            ],
                                            "dest",
                                        )
                                    };

                                    let val = bin.builder.build_pointer_cast(
                                        val.into_pointer_value(),
                                        dest.get_type().get_element_type().into_pointer_type(),
                                        "seeds",
                                    );

                                    bin.builder.build_store(dest, val);

                                    let dest = unsafe {
                                        bin.builder.build_gep(
                                            output_seeds,
                                            &[
                                                bin.context.i32_type().const_int(i, false),
                                                bin.context.i32_type().const_int(1, false),
                                            ],
                                            "dest",
                                        )
                                    };

                                    let val =
                                        bin.context.i64_type().const_int(seed_count as u64, false);

                                    bin.builder.build_store(dest, val);
                                }
                            }

                            Some((output_seeds, bin.context.i64_type().const_int(len, false)))
                        } else {
                            None
                        };

                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        self.external_call(
                            bin,
                            function,
                            success,
                            payload_ptr,
                            payload_len,
                            address,
                            gas,
                            value,
                            accounts,
                            seeds,
                            callty.clone(),
                            ns,
                        );
                    }
                    Instr::ValueTransfer {
                        success,
                        address,
                        value,
                    } => {
                        let value = self
                            .expression(bin, value, &w.vars, function, ns)
                            .into_int_value();
                        let address = self
                            .expression(bin, address, &w.vars, function, ns)
                            .into_array_value();

                        let addr = bin.build_alloca(function, bin.address_type(ns), "address");

                        bin.builder.build_store(
                            bin.builder.build_pointer_cast(
                                addr,
                                address.get_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            address,
                        );
                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        self.value_transfer(
                            bin,
                            function,
                            success,
                            bin.builder.build_pointer_cast(
                                addr,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            value,
                            ns,
                        );
                    }
                    Instr::AbiDecode {
                        res,
                        selector,
                        exception_block: exception,
                        tys,
                        data,
                    } => {
                        let v = self.expression(bin, data, &w.vars, function, ns);

                        let mut data = bin.vector_bytes(v);

                        let mut data_len = bin.vector_len(v);

                        if let Some(selector) = selector {
                            let exception = exception.unwrap();

                            let pos = bin.builder.get_insert_block().unwrap();

                            blocks.entry(exception).or_insert({
                                work.push_back(Work {
                                    block_no: exception,
                                    vars: w.vars.clone(),
                                });

                                create_block(exception, bin, cfg, function, ns)
                            });

                            bin.builder.position_at_end(pos);

                            let exception_block = blocks.get(&exception).unwrap();

                            let has_selector = bin.builder.build_int_compare(
                                IntPredicate::UGT,
                                data_len,
                                bin.context.i32_type().const_int(4, false),
                                "has_selector",
                            );

                            let ok1 = bin.context.append_basic_block(function, "ok1");

                            bin.builder.build_conditional_branch(
                                has_selector,
                                ok1,
                                exception_block.bb,
                            );
                            bin.builder.position_at_end(ok1);

                            let selector_data = bin
                                .builder
                                .build_load(
                                    bin.builder.build_pointer_cast(
                                        data,
                                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                                        "selector",
                                    ),
                                    "selector",
                                )
                                .into_int_value();

                            let selector = if ns.target.is_substrate() {
                                *selector
                            } else {
                                selector.to_be()
                            };

                            let correct_selector = bin.builder.build_int_compare(
                                IntPredicate::EQ,
                                selector_data,
                                bin.context.i32_type().const_int(selector as u64, false),
                                "correct_selector",
                            );

                            let ok2 = bin.context.append_basic_block(function, "ok2");

                            bin.builder.build_conditional_branch(
                                correct_selector,
                                ok2,
                                exception_block.bb,
                            );

                            bin.builder.position_at_end(ok2);

                            data_len = bin.builder.build_int_sub(
                                data_len,
                                bin.context.i32_type().const_int(4, false),
                                "data_len",
                            );

                            data = unsafe {
                                bin.builder.build_gep(
                                    bin.builder.build_pointer_cast(
                                        data,
                                        bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                        "data",
                                    ),
                                    &[bin.context.i32_type().const_int(4, false)],
                                    "data",
                                )
                            };
                        }

                        let mut returns = Vec::new();

                        self.abi_decode(bin, function, &mut returns, data, data_len, tys, ns);

                        for (i, ret) in returns.into_iter().enumerate() {
                            w.vars.get_mut(&res[i]).unwrap().value = ret;
                        }
                    }
                    Instr::Unreachable => {
                        // Nothing to do; unreachable instruction should have already been inserteds
                    }
                    Instr::SelfDestruct { recipient } => {
                        let recipient = self
                            .expression(bin, recipient, &w.vars, function, ns)
                            .into_array_value();

                        self.selfdestruct(bin, recipient, ns);
                    }
                    Instr::EmitEvent {
                        event_no,
                        data,
                        data_tys,
                        topics,
                        topic_tys,
                    } => {
                        let data = data
                            .iter()
                            .map(|a| self.expression(bin, a, &w.vars, function, ns))
                            .collect::<Vec<BasicValueEnum>>();

                        let topics = topics
                            .iter()
                            .map(|a| self.expression(bin, a, &w.vars, function, ns))
                            .collect::<Vec<BasicValueEnum>>();

                        self.emit_event(
                            bin, contract, function, *event_no, &data, data_tys, &topics,
                            topic_tys, ns,
                        );
                    }
                    Instr::WriteBuffer { buf, offset, value } => {
                        let v = self.expression(bin, buf, &w.vars, function, ns);
                        let data = bin.vector_bytes(v);

                        let offset = self
                            .expression(bin, offset, &w.vars, function, ns)
                            .into_int_value();
                        let emit_value = self.expression(bin, value, &w.vars, function, ns);

                        let start = unsafe { bin.builder.build_gep(data, &[offset], "start") };

                        let is_bytes = if let Type::Bytes(n) = value.ty() {
                            n
                        } else {
                            0
                        };

                        if is_bytes > 1 {
                            let value_ptr = bin.build_alloca(
                                function,
                                emit_value.into_int_value().get_type(),
                                &format!("bytes{}", is_bytes),
                            );
                            bin.builder
                                .build_store(value_ptr, emit_value.into_int_value());
                            bin.builder.build_call(
                                bin.module.get_function("__leNtobeN").unwrap(),
                                &[
                                    bin.builder
                                        .build_pointer_cast(
                                            value_ptr,
                                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                            "store",
                                        )
                                        .into(),
                                    bin.builder
                                        .build_pointer_cast(
                                            start,
                                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                            "dest",
                                        )
                                        .into(),
                                    bin.context
                                        .i32_type()
                                        .const_int(is_bytes as u64, false)
                                        .into(),
                                ],
                                "",
                            );
                        } else {
                            let start = bin.builder.build_pointer_cast(
                                start,
                                emit_value.get_type().ptr_type(AddressSpace::Generic),
                                "start",
                            );

                            bin.builder.build_store(start, emit_value);
                        }
                    }
                    Instr::MemCopy {
                        source: from,
                        destination: to,
                        bytes,
                    } => {
                        let src = if from.ty().is_dynamic_memory() {
                            bin.vector_bytes(self.expression(bin, from, &w.vars, function, ns))
                        } else {
                            self.expression(bin, from, &w.vars, function, ns)
                                .into_pointer_value()
                        };

                        let dest = if to.ty().is_dynamic_memory() {
                            bin.vector_bytes(self.expression(bin, to, &w.vars, function, ns))
                        } else {
                            self.expression(bin, to, &w.vars, function, ns)
                                .into_pointer_value()
                        };

                        let size = self.expression(bin, bytes, &w.vars, function, ns);

                        if matches!(bytes, Expression::NumberLiteral(..)) {
                            let _ =
                                bin.builder
                                    .build_memcpy(dest, 1, src, 1, size.into_int_value());
                        } else {
                            bin.builder.build_call(
                                bin.module.get_function("__memcpy").unwrap(),
                                &[dest.into(), src.into(), size.into()],
                                "",
                            );
                        }
                    }
                }
                bin.builder.unset_current_debug_location();
                dibuilder.finalize();
            }
        }
    }

    /// Create function dispatch based on abi encoded argsdata. The dispatcher loads the leading function selector,
    /// and dispatches based on that. If no function matches this, or no selector is in the argsdata, then fallback
    /// code is executed. This is either a fallback block provided to this function, or it automatically dispatches
    /// to the fallback function or receive function, if any.
    fn emit_function_dispatch<F>(
        &self,
        bin: &Binary<'a>,
        contract: &Contract,
        ns: &Namespace,
        function_ty: pt::FunctionTy,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        functions: &HashMap<usize, FunctionValue<'a>>,
        fallback: Option<inkwell::basic_block::BasicBlock>,
        nonpayable: F,
    ) where
        F: Fn(&ControlFlowGraph) -> bool,
    {
        // create start function
        let no_function_matched = match fallback {
            Some(block) => block,
            None => bin
                .context
                .append_basic_block(function, "no_function_matched"),
        };

        let switch_block = bin.context.append_basic_block(function, "switch");

        let not_fallback = bin.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            argslen.get_type().const_int(4, false),
            "",
        );

        bin.builder
            .build_conditional_branch(not_fallback, switch_block, no_function_matched);

        bin.builder.position_at_end(switch_block);

        let fid = bin
            .builder
            .build_load(argsdata, "function_selector")
            .into_int_value();

        if ns.target != Target::Solana {
            // TODO: solana does not support bss, so different solution is needed
            bin.builder
                .build_store(bin.selector.as_pointer_value(), fid);
        }

        // step over the function selector
        let argsdata = unsafe {
            bin.builder.build_gep(
                argsdata,
                &[bin.context.i32_type().const_int(1, false)],
                "argsdata",
            )
        };

        let argslen =
            bin.builder
                .build_int_sub(argslen, argslen.get_type().const_int(4, false), "argslen");

        let mut cases = Vec::new();

        for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
            if cfg.ty != function_ty || !cfg.public {
                continue;
            }

            self.add_dispatch_case(
                bin,
                cfg,
                ns,
                &mut cases,
                argsdata,
                argslen,
                function,
                functions[&cfg_no],
                &nonpayable,
            );
        }

        bin.builder.position_at_end(switch_block);

        bin.builder.build_switch(fid, no_function_matched, &cases);

        if fallback.is_some() {
            return; // caller will generate fallback code
        }

        // emit fallback code
        bin.builder.position_at_end(no_function_matched);

        let fallback = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Fallback);

        let receive = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Receive);

        if fallback.is_none() && receive.is_none() {
            // no need to check value transferred; we will abort either way
            self.return_code(bin, bin.return_values[&ReturnCode::FunctionSelectorInvalid]);

            return;
        }

        if ns.target == Target::Solana {
            match fallback {
                Some((cfg_no, _)) => {
                    let args = if ns.target == Target::Solana {
                        vec![function.get_last_param().unwrap().into()]
                    } else {
                        vec![]
                    };

                    bin.builder.build_call(functions[&cfg_no], &args, "");

                    self.return_empty_abi(bin);
                }
                None => {
                    self.return_code(bin, bin.context.i32_type().const_int(2, false));
                }
            }
        } else {
            let got_value = if bin.function_abort_value_transfers {
                bin.context.bool_type().const_zero()
            } else {
                let value = self.value_transferred(bin, ns);

                bin.builder.build_int_compare(
                    IntPredicate::NE,
                    value,
                    bin.value_type(ns).const_zero(),
                    "is_value_transfer",
                )
            };

            let fallback_block = bin.context.append_basic_block(function, "fallback");
            let receive_block = bin.context.append_basic_block(function, "receive");

            bin.builder
                .build_conditional_branch(got_value, receive_block, fallback_block);

            bin.builder.position_at_end(fallback_block);

            match fallback {
                Some((cfg_no, _)) => {
                    let args = if ns.target == Target::Solana {
                        vec![function.get_last_param().unwrap().into()]
                    } else {
                        vec![]
                    };

                    bin.builder.build_call(functions[&cfg_no], &args, "");

                    self.return_empty_abi(bin);
                }
                None => {
                    self.return_code(bin, bin.context.i32_type().const_int(2, false));
                }
            }

            bin.builder.position_at_end(receive_block);

            match receive {
                Some((cfg_no, _)) => {
                    let args = if ns.target == Target::Solana {
                        vec![function.get_last_param().unwrap().into()]
                    } else {
                        vec![]
                    };

                    bin.builder.build_call(functions[&cfg_no], &args, "");

                    self.return_empty_abi(bin);
                }
                None => {
                    self.return_code(bin, bin.context.i32_type().const_int(2, false));
                }
            }
        }
    }

    ///Add single case for emit_function_dispatch
    fn add_dispatch_case<F>(
        &self,
        bin: &Binary<'a>,
        f: &ControlFlowGraph,
        ns: &Namespace,
        cases: &mut Vec<(
            inkwell::values::IntValue<'a>,
            inkwell::basic_block::BasicBlock<'a>,
        )>,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        dest: inkwell::values::FunctionValue<'a>,
        nonpayable: &F,
    ) where
        F: Fn(&ControlFlowGraph) -> bool,
    {
        let bb = bin.context.append_basic_block(function, "");

        bin.builder.position_at_end(bb);

        if nonpayable(f) {
            self.abort_if_value_transfer(bin, function, ns);
        }

        let mut args = Vec::new();

        // insert abi decode
        self.abi_decode(bin, function, &mut args, argsdata, argslen, &f.params, ns);

        // add return values as pointer arguments at the end
        if !f.returns.is_empty() {
            for v in f.returns.iter() {
                args.push(if !v.ty.is_reference_type(ns) {
                    bin.build_alloca(function, bin.llvm_type(&v.ty, ns), v.name_as_str())
                        .into()
                } else {
                    bin.build_alloca(
                        function,
                        bin.llvm_type(&v.ty, ns).ptr_type(AddressSpace::Generic),
                        v.name_as_str(),
                    )
                    .into()
                });
            }
        }

        if ns.target == Target::Solana {
            let params_ty = dest
                .get_type()
                .get_param_types()
                .last()
                .unwrap()
                .into_pointer_type();

            args.push(
                bin.builder
                    .build_pointer_cast(
                        function.get_last_param().unwrap().into_pointer_value(),
                        params_ty,
                        "",
                    )
                    .into(),
            );
        }

        let meta_args: Vec<BasicMetadataValueEnum> = args.iter().map(|arg| (*arg).into()).collect();

        let ret = bin
            .builder
            .build_call(dest, &meta_args, "")
            .try_as_basic_value()
            .left()
            .unwrap();

        let success = bin.builder.build_int_compare(
            IntPredicate::EQ,
            ret.into_int_value(),
            bin.return_values[&ReturnCode::Success],
            "success",
        );

        let success_block = bin.context.append_basic_block(function, "success");
        let bail_block = bin.context.append_basic_block(function, "bail");

        bin.builder
            .build_conditional_branch(success, success_block, bail_block);

        bin.builder.position_at_end(success_block);

        if f.returns.is_empty() {
            // return ABI of length 0
            self.return_empty_abi(bin);
        } else {
            let tys: Vec<Type> = f.returns.iter().map(|p| p.ty.clone()).collect();

            let (data, length) = self.abi_encode(
                bin,
                None,
                true,
                function,
                &args[f.params.len()..f.params.len() + f.returns.len()],
                &tys,
                ns,
            );

            self.return_abi(bin, data, length);
        }

        bin.builder.position_at_end(bail_block);

        self.return_code(bin, ret.into_int_value());

        cases.push((
            bin.context.i32_type().const_int(
                u32::from_le_bytes(f.selector.as_slice().try_into().unwrap()) as u64,
                false,
            ),
            bb,
        ));
    }

    /// Emit the bin storage initializers
    fn emit_initializer(
        &mut self,
        bin: &mut Binary<'a>,
        contract: &Contract,
        ns: &Namespace,
    ) -> FunctionValue<'a> {
        let function_ty = bin.function_type(&[], &[], ns);

        let function = bin.module.add_function(
            &format!("sol::{}::storage_initializers", contract.name),
            function_ty,
            Some(Linkage::Internal),
        );

        let cfg = &contract.cfg[contract.initializer.unwrap()];

        self.emit_cfg(bin, contract, cfg, function, ns);

        function
    }

    /// Emit all functions, constructors, fallback and receiver
    fn emit_functions(&mut self, bin: &mut Binary<'a>, contract: &Contract, ns: &Namespace) {
        let mut defines = Vec::new();

        for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
            if !cfg.is_placeholder() {
                let ftype = bin.function_type(
                    &cfg.params
                        .iter()
                        .map(|p| p.ty.clone())
                        .collect::<Vec<Type>>(),
                    &cfg.returns
                        .iter()
                        .map(|p| p.ty.clone())
                        .collect::<Vec<Type>>(),
                    ns,
                );

                assert_eq!(bin.module.get_function(&cfg.name), None);

                let func_decl = bin
                    .module
                    .add_function(&cfg.name, ftype, Some(Linkage::Internal));

                bin.functions.insert(cfg_no, func_decl);

                defines.push((func_decl, cfg));
            }
        }

        for (func_decl, cfg) in defines {
            self.emit_cfg(bin, contract, cfg, func_decl, ns);
        }
    }

    /// Implement "...{}...{}".format(a, b)
    fn format_string(
        &self,
        bin: &Binary<'a>,
        args: &[(FormatArg, Expression)],
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        // first we need to calculate the space we need
        let mut length = bin.context.i32_type().const_zero();

        let mut evaluated_arg = Vec::new();

        evaluated_arg.resize(args.len(), None);

        for (i, (spec, arg)) in args.iter().enumerate() {
            let len = if *spec == FormatArg::StringLiteral {
                if let Expression::BytesLiteral(_, _, bs) = arg {
                    bin.context.i32_type().const_int(bs.len() as u64, false)
                } else {
                    unreachable!();
                }
            } else {
                match arg.ty() {
                    // bool: "true" or "false"
                    Type::Bool => bin.context.i32_type().const_int(5, false),
                    // hex encode bytes
                    Type::Contract(_) | Type::Address(_) => bin
                        .context
                        .i32_type()
                        .const_int(ns.address_length as u64 * 2, false),
                    Type::Bytes(size) => bin.context.i32_type().const_int(size as u64 * 2, false),
                    Type::String => {
                        let val = self.expression(bin, arg, vartab, function, ns);

                        evaluated_arg[i] = Some(val);

                        bin.vector_len(val)
                    }
                    Type::DynamicBytes => {
                        let val = self.expression(bin, arg, vartab, function, ns);

                        evaluated_arg[i] = Some(val);

                        // will be hex encoded, so double
                        let len = bin.vector_len(val);

                        bin.builder.build_int_add(len, len, "hex_len")
                    }
                    Type::Uint(bits) if *spec == FormatArg::Hex => {
                        bin.context.i32_type().const_int(bits as u64 / 8 + 2, false)
                    }
                    Type::Int(bits) if *spec == FormatArg::Hex => {
                        bin.context.i32_type().const_int(bits as u64 / 8 + 3, false)
                    }
                    Type::Uint(bits) if *spec == FormatArg::Binary => {
                        bin.context.i32_type().const_int(bits as u64 + 2, false)
                    }
                    Type::Int(bits) if *spec == FormatArg::Binary => {
                        bin.context.i32_type().const_int(bits as u64 + 3, false)
                    }
                    // bits / 3 is a rough over-estimate of how many decimals we need
                    Type::Uint(bits) if *spec == FormatArg::Default => {
                        bin.context.i32_type().const_int(bits as u64 / 3, false)
                    }
                    Type::Int(bits) if *spec == FormatArg::Default => {
                        bin.context.i32_type().const_int(bits as u64 / 3 + 1, false)
                    }
                    Type::Enum(enum_no) => bin
                        .context
                        .i32_type()
                        .const_int(ns.enums[enum_no].ty.bits(ns) as u64 / 3, false),
                    _ => unimplemented!(),
                }
            };

            length = bin.builder.build_int_add(length, len, "");
        }

        // allocate the string and
        let vector = bin.vector_new(length, bin.context.i32_type().const_int(1, false), None);

        let output_start = bin.vector_bytes(vector.into());

        // now encode each of the arguments
        let mut output = output_start;

        // format it
        for (i, (spec, arg)) in args.iter().enumerate() {
            if *spec == FormatArg::StringLiteral {
                if let Expression::BytesLiteral(_, _, bs) = arg {
                    let s = bin.emit_global_string("format_arg", bs, true);
                    let len = bin.context.i32_type().const_int(bs.len() as u64, false);

                    bin.builder.build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    );

                    output = unsafe { bin.builder.build_gep(output, &[len], "") };
                }
            } else {
                let val = evaluated_arg[i]
                    .unwrap_or_else(|| self.expression(bin, arg, vartab, function, ns));
                let arg_ty = arg.ty();

                match arg_ty {
                    Type::Bool => {
                        let len = bin
                            .builder
                            .build_select(
                                val.into_int_value(),
                                bin.context.i32_type().const_int(4, false),
                                bin.context.i32_type().const_int(5, false),
                                "bool_length",
                            )
                            .into_int_value();

                        let s = bin.builder.build_select(
                            val.into_int_value(),
                            bin.emit_global_string("bool_true", b"true", true),
                            bin.emit_global_string("bool_false", b"false", true),
                            "bool_value",
                        );

                        bin.builder.build_call(
                            bin.module.get_function("__memcpy").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        output = unsafe { bin.builder.build_gep(output, &[len], "") };
                    }
                    Type::String => {
                        let s = bin.vector_bytes(val);
                        let len = bin.vector_len(val);

                        bin.builder.build_call(
                            bin.module.get_function("__memcpy").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        output = unsafe { bin.builder.build_gep(output, &[len], "") };
                    }
                    Type::DynamicBytes => {
                        let s = bin.vector_bytes(val);
                        let len = bin.vector_len(val);

                        bin.builder.build_call(
                            bin.module.get_function("hex_encode").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { bin.builder.build_gep(output, &[hex_len], "") };
                    }
                    Type::Address(_) | Type::Contract(_) => {
                        // for Solana/Substrate, we should encode in base58
                        let buf = bin.build_alloca(function, bin.address_type(ns), "address");

                        bin.builder.build_store(buf, val.into_array_value());

                        let len = bin
                            .context
                            .i32_type()
                            .const_int(ns.address_length as u64, false);

                        let s = bin.builder.build_pointer_cast(
                            buf,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "address_bytes",
                        );

                        bin.builder.build_call(
                            bin.module.get_function("hex_encode").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { bin.builder.build_gep(output, &[hex_len], "") };
                    }
                    Type::Bytes(size) => {
                        let buf = bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "bytesN");

                        bin.builder.build_store(buf, val.into_int_value());

                        let len = bin.context.i32_type().const_int(size as u64, false);

                        let s = bin.builder.build_pointer_cast(
                            buf,
                            bin.context.i8_type().ptr_type(AddressSpace::Generic),
                            "bytes",
                        );

                        bin.builder.build_call(
                            bin.module.get_function("hex_encode_rev").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = bin.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { bin.builder.build_gep(output, &[hex_len], "") };
                    }
                    Type::Enum(_) => {
                        let val = bin.builder.build_int_z_extend(
                            val.into_int_value(),
                            bin.context.i64_type(),
                            "val_64bits",
                        );

                        output = bin
                            .builder
                            .build_call(
                                bin.module.get_function("uint2dec").unwrap(),
                                &[output.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                    Type::Uint(bits) => {
                        if *spec == FormatArg::Default && bits <= 64 {
                            let val = if bits == 64 {
                                val.into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    bin.context.i64_type(),
                                    "val_64bits",
                                )
                            };

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint2dec").unwrap(),
                                    &[output.into(), val.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else if *spec == FormatArg::Default && bits <= 128 {
                            let val = if bits == 128 {
                                val.into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    bin.context.custom_width_int_type(128),
                                    "val_128bits",
                                )
                            };

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint128dec").unwrap(),
                                    &[output.into(), val.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else if *spec == FormatArg::Default {
                            let val = if bits == 256 {
                                val.into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    bin.context.custom_width_int_type(256),
                                    "val_256bits",
                                )
                            };

                            let pval = bin.build_alloca(
                                function,
                                bin.context.custom_width_int_type(256),
                                "int",
                            );

                            bin.builder.build_store(pval, val);

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint256dec").unwrap(),
                                    &[output.into(), pval.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else {
                            let buf =
                                bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "uint");

                            bin.builder.build_store(buf, val.into_int_value());

                            let s = bin.builder.build_pointer_cast(
                                buf,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "uint",
                            );

                            let len = bin.context.i32_type().const_int(bits as u64 / 8, false);

                            let func_name = if *spec == FormatArg::Hex {
                                "uint2hex"
                            } else {
                                "uint2bin"
                            };

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function(func_name).unwrap(),
                                    &[output.into(), s.into(), len.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        }
                    }
                    Type::Int(bits) => {
                        let val = val.into_int_value();

                        let is_negative = bin.builder.build_int_compare(
                            IntPredicate::SLT,
                            val,
                            val.get_type().const_zero(),
                            "negative",
                        );

                        let entry = bin.builder.get_insert_block().unwrap();
                        let positive = bin.context.append_basic_block(function, "int_positive");
                        let negative = bin.context.append_basic_block(function, "int_negative");

                        bin.builder
                            .build_conditional_branch(is_negative, negative, positive);

                        bin.builder.position_at_end(negative);

                        // add "-" to output and negate our val
                        bin.builder.build_store(
                            output,
                            bin.context.i8_type().const_int('-' as u64, false),
                        );

                        let minus_len = bin.context.i32_type().const_int(1, false);

                        let neg_data = unsafe { bin.builder.build_gep(output, &[minus_len], "") };
                        let neg_val = bin.builder.build_int_neg(val, "negative_int");

                        bin.builder.build_unconditional_branch(positive);

                        bin.builder.position_at_end(positive);

                        let data_phi = bin.builder.build_phi(output.get_type(), "data");
                        let val_phi = bin.builder.build_phi(val.get_type(), "val");

                        data_phi.add_incoming(&[(&neg_data, negative), (&output, entry)]);
                        val_phi.add_incoming(&[(&neg_val, negative), (&val, entry)]);

                        if *spec == FormatArg::Default && bits <= 64 {
                            let val = if bits == 64 {
                                val_phi.as_basic_value().into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    bin.context.i64_type(),
                                    "val_64bits",
                                )
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint2dec").unwrap(),
                                    &[output_after_minus.into(), val.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else if *spec == FormatArg::Default && bits <= 128 {
                            let val = if bits == 128 {
                                val_phi.as_basic_value().into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    bin.context.custom_width_int_type(128),
                                    "val_128bits",
                                )
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint128dec").unwrap(),
                                    &[output_after_minus.into(), val.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else if *spec == FormatArg::Default {
                            let val = if bits == 256 {
                                val_phi.as_basic_value().into_int_value()
                            } else {
                                bin.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    bin.context.custom_width_int_type(256),
                                    "val_256bits",
                                )
                            };

                            let pval = bin.build_alloca(
                                function,
                                bin.context.custom_width_int_type(256),
                                "int",
                            );

                            bin.builder.build_store(pval, val);

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function("uint256dec").unwrap(),
                                    &[output_after_minus.into(), pval.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else {
                            let buf = bin.build_alloca(function, bin.llvm_type(&arg_ty, ns), "int");

                            bin.builder
                                .build_store(buf, val_phi.as_basic_value().into_int_value());

                            let s = bin.builder.build_pointer_cast(
                                buf,
                                bin.context.i8_type().ptr_type(AddressSpace::Generic),
                                "int",
                            );

                            let len = bin.context.i32_type().const_int(bits as u64 / 8, false);

                            let func_name = if *spec == FormatArg::Hex {
                                "uint2hex"
                            } else {
                                "uint2bin"
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = bin
                                .builder
                                .build_call(
                                    bin.module.get_function(func_name).unwrap(),
                                    &[output_after_minus.into(), s.into(), len.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        }

        // write the final length into the vector
        let length = bin.builder.build_int_sub(
            bin.builder
                .build_ptr_to_int(output, bin.context.i32_type(), "end"),
            bin.builder
                .build_ptr_to_int(output_start, bin.context.i32_type(), "begin"),
            "datalength",
        );

        let data_len = unsafe {
            bin.builder.build_gep(
                vector,
                &[
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_zero(),
                ],
                "data_len",
            )
        };

        bin.builder.build_store(data_len, length);

        vector.into()
    }

    // Signed overflow detection is handled by the following steps:
    // 1- Do an unsigned multiplication first, This step will check if the generated value will fit in N bits. (unsigned overflow)
    // 2- Get the result, and negate it if needed.
    // 3- Check for signed overflow, by checking for an unexpected change in the sign of the result.
    fn signed_ovf_detect(
        &self,
        bin: &Binary<'a>,
        mul_ty: IntType<'a>,
        mul_bits: u32,
        left: IntValue<'a>,
        right: IntValue<'a>,
        bits: u32,
        function: FunctionValue<'a>,
    ) -> IntValue<'a> {
        // We check for signed overflow based on the facts:
        //  - * - = +
        //  + * + = +
        //  - * + = - (if op1 and op2 != 0)
        // if one of the operands is zero, discard the last rule.
        let left_negative = bin.builder.build_int_compare(
            IntPredicate::SLT,
            left,
            left.get_type().const_zero(),
            "left_negative",
        );

        let left_abs = bin
            .builder
            .build_select(
                left_negative,
                bin.builder.build_int_neg(left, "signed_left"),
                left,
                "left_abs",
            )
            .into_int_value();

        let right_negative = bin.builder.build_int_compare(
            IntPredicate::SLT,
            right,
            right.get_type().const_zero(),
            "right_negative",
        );

        let right_abs = bin
            .builder
            .build_select(
                right_negative,
                bin.builder.build_int_neg(right, "signed_right"),
                right,
                "right_abs",
            )
            .into_int_value();

        let l = bin.build_alloca(function, mul_ty, "");
        let r = bin.build_alloca(function, mul_ty, "");
        let o = bin.build_alloca(function, mul_ty, "");

        bin.builder
            .build_store(l, bin.builder.build_int_z_extend(left_abs, mul_ty, ""));
        bin.builder
            .build_store(r, bin.builder.build_int_z_extend(right_abs, mul_ty, ""));

        let return_val = bin.builder.build_call(
            bin.module.get_function("__mul32_with_builtin_ovf").unwrap(),
            &[
                bin.builder
                    .build_pointer_cast(
                        l,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "left",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        r,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "right",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        o,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "output",
                    )
                    .into(),
                bin.context
                    .i32_type()
                    .const_int(mul_bits as u64 / 32, false)
                    .into(),
            ],
            "",
        );

        let res = bin.builder.build_load(o, "mul");
        let ovf_any_type = if mul_bits != bits {
            // If there are any set bits, then there is an overflow.
            let check_ovf = bin.builder.build_right_shift(
                res.into_int_value(),
                mul_ty.const_int((bits).into(), false),
                false,
                "",
            );
            bin.builder.build_int_compare(
                IntPredicate::NE,
                check_ovf,
                check_ovf.get_type().const_zero(),
                "",
            )
        } else {
            // If no size extension took place, there is no overflow in most significant N bits
            bin.context.bool_type().const_zero()
        };

        let negate_result = bin
            .builder
            .build_xor(left_negative, right_negative, "negate_result");

        let res = bin.builder.build_select(
            negate_result,
            bin.builder
                .build_int_neg(res.into_int_value(), "unsigned_res"),
            res.into_int_value(),
            "res",
        );

        let error_block = bin.context.append_basic_block(function, "error");
        let return_block = bin.context.append_basic_block(function, "return_block");

        // Extract sign bit of the operands and the result
        let left_sign_bit = self.extract_sign_bit(bin, left, left.get_type());
        let right_sign_bit = self.extract_sign_bit(bin, right, right.get_type());
        let res_sign_bit = if mul_bits == bits {
            // If no extension took place, get the leftmost bit(sign bit).
            self.extract_sign_bit(bin, res.into_int_value(), res.into_int_value().get_type())
        } else {
            // If extension took place, truncate the result to the type of the operands then extract the leftmost bit(sign bit).
            self.extract_sign_bit(
                bin,
                bin.builder
                    .build_int_truncate(res.into_int_value(), left.get_type(), ""),
                left.get_type(),
            )
        };

        let value_fits_n_bits = bin.builder.build_not(
            bin.builder.build_or(
                return_val
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value(),
                ovf_any_type,
                "",
            ),
            "",
        );

        let left_is_zero =
            bin.builder
                .build_int_compare(IntPredicate::EQ, left, left.get_type().const_zero(), "");
        let right_is_zero = bin.builder.build_int_compare(
            IntPredicate::EQ,
            right,
            right.get_type().const_zero(),
            "",
        );

        // If one of the operands is zero
        let mul_by_zero = bin.builder.build_or(left_is_zero, right_is_zero, "");

        // Will resolve to one if signs are differnet
        let different_signs = bin.builder.build_xor(left_sign_bit, right_sign_bit, "");

        let not_ok_operation = bin
            .builder
            .build_not(bin.builder.build_xor(different_signs, res_sign_bit, ""), "");

        // Here, we disregard the last rule mentioned above by oring with mul_by_zero
        bin.builder.build_conditional_branch(
            bin.builder.build_and(
                bin.builder.build_or(not_ok_operation, mul_by_zero, ""),
                value_fits_n_bits,
                "",
            ),
            return_block,
            error_block,
        );

        bin.builder.position_at_end(error_block);

        self.assert_failure(
            bin,
            bin.context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            bin.context.i32_type().const_zero(),
        );

        bin.builder.position_at_end(return_block);

        bin.builder
            .build_int_truncate(res.into_int_value(), left.get_type(), "")
    }

    // Call void __mul32 and return the result.
    fn call_mul32_without_ovf(
        &self,
        bin: &Binary<'a>,
        l: PointerValue<'a>,
        r: PointerValue<'a>,
        o: PointerValue<'a>,
        mul_bits: u32,
        mul_type: IntType<'a>,
    ) -> IntValue<'a> {
        bin.builder.build_call(
            bin.module.get_function("__mul32").unwrap(),
            &[
                bin.builder
                    .build_pointer_cast(
                        l,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "left",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        r,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "right",
                    )
                    .into(),
                bin.builder
                    .build_pointer_cast(
                        o,
                        bin.context.i32_type().ptr_type(AddressSpace::Generic),
                        "output",
                    )
                    .into(),
                bin.context
                    .i32_type()
                    .const_int(mul_bits as u64 / 32, false)
                    .into(),
            ],
            "",
        );

        let res = bin.builder.build_load(o, "mul");

        bin.builder
            .build_int_truncate(res.into_int_value(), mul_type, "")
    }

    // Utility function to extract the sign bit of an IntValue
    fn extract_sign_bit(
        &self,
        bin: &Binary<'a>,
        operand: IntValue<'a>,
        int_type: IntType<'a>,
    ) -> IntValue<'a> {
        let n_bits_to_shift = int_type.get_bit_width() - 1;
        let val_to_shift = int_type.const_int(n_bits_to_shift as u64, false);
        let shifted = bin
            .builder
            .build_right_shift(operand, val_to_shift, false, "");
        bin.builder
            .build_int_truncate(shifted, bin.context.bool_type(), "")
    }

    // Emit a multiply for any width with or without overflow checking
    fn mul(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        unchecked: bool,
        left: IntValue<'a>,
        right: IntValue<'a>,
        signed: bool,
    ) -> IntValue<'a> {
        let bits = left.get_type().get_bit_width();

        if bits > 64 {
            // Round up the number of bits to the next 32
            let mul_bits = (bits + 31) & !31;
            let mul_ty = bin.context.custom_width_int_type(mul_bits);

            // Round up bits
            let l = bin.build_alloca(function, mul_ty, "");
            let r = bin.build_alloca(function, mul_ty, "");
            let o = bin.build_alloca(function, mul_ty, "");

            if mul_bits == bits {
                bin.builder.build_store(l, left);
                bin.builder.build_store(r, right);
            }
            // LLVM-IR can handle multiplication of sizes up to 64 bits. If the size is larger, we need to implement our own mutliplication function.
            // We divide the operands into sizes of 32 bits (check __mul32 in stdlib/bigint.c documentation).
            // If the size is not divisble by 32, we extend it to the next 32 bits. For example, int72 will be extended to int96.
            // Here, We zext the operands to the nearest 32 bits. zext is called instead of sext because we need to do unsigned multiplication by default.
            // It will not matter in terms of mul without overflow, because we always truncate the result to the bit size of the operands.
            // In mul with overflow however, it is needed so that overflow can be detected if the most significant bits of the result are not zeros.
            else {
                bin.builder
                    .build_store(l, bin.builder.build_int_z_extend(left, mul_ty, ""));
                bin.builder
                    .build_store(r, bin.builder.build_int_z_extend(right, mul_ty, ""));
            }

            // If unchecked, and no overflow flag in command line arguments.
            if unchecked && !bin.math_overflow_check {
                return self.call_mul32_without_ovf(bin, l, r, o, mul_bits, left.get_type());
            }

            if signed {
                return self.signed_ovf_detect(bin, mul_ty, mul_bits, left, right, bits, function);
            }

            // Unsigned overflow detection Approach:
            // If the size is a multiple of 32, we call __mul32_with_builtin_ovf and it returns an overflow flag (check __mul32_with_builtin_ovf in stdlib/bigint.c documentation)
            // If that is not the case, some extra work has to be done. We have to check the extended bits for any set bits. If there is any, an overflow occured.
            // For example, if we have uint72, it will be extended to uint96. __mul32 with ovf will raise an ovf flag if the result overflows 96 bits, not 72.
            // We account for that by checking the extended leftmost bits. In the example mentioned, they will be 96-72=24 bits.
            let return_val = bin.builder.build_call(
                bin.module.get_function("__mul32_with_builtin_ovf").unwrap(),
                &[
                    bin.builder
                        .build_pointer_cast(
                            l,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            r,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    bin.builder
                        .build_pointer_cast(
                            o,
                            bin.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    bin.context
                        .i32_type()
                        .const_int(mul_bits as u64 / 32, false)
                        .into(),
                ],
                "ovf",
            );

            let res = bin.builder.build_load(o, "mul");

            let error_block = bin.context.append_basic_block(function, "error");
            let return_block = bin.context.append_basic_block(function, "return_block");

            // If the operands were extended to nearest 32 bit size, check the most significant N bits, where N equals bit width after extension minus original bit width.
            let ovf_any_type = if mul_bits != bits {
                // If there are any set bits, then there is an overflow.
                let check_ovf = bin.builder.build_right_shift(
                    res.into_int_value(),
                    mul_ty.const_int((bits).into(), false),
                    false,
                    "",
                );
                bin.builder.build_int_compare(
                    IntPredicate::NE,
                    check_ovf,
                    check_ovf.get_type().const_zero(),
                    "",
                )
            } else {
                // If no size extension took place, there is no overflow in most significant N bits
                bin.context.bool_type().const_zero()
            };

            // Until this point, we only checked the extended bits for ovf. But mul ovf can take place any where from bit size to double bit size.
            // For example: If we have uint72, it will be extended to uint96. We only checked the most significant 24 bits for overflow, which can happen up to 72*2=144 bits.
            // bool __mul32_with_builtin_ovf takes care of overflowing bits beyond 96.
            // What is left now is to or these two ovf flags, and check if any one of them is set. If so, an overflow occured.
            let lowbit = bin.builder.build_int_truncate(
                bin.builder.build_or(
                    ovf_any_type,
                    return_val
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value(),
                    "",
                ),
                bin.context.bool_type(),
                "bit",
            );

            // If ovf, raise an error, else return the result.
            bin.builder
                .build_conditional_branch(lowbit, error_block, return_block);

            bin.builder.position_at_end(error_block);

            self.assert_failure(
                bin,
                bin.context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                bin.context.i32_type().const_zero(),
            );

            bin.builder.position_at_end(return_block);

            bin.builder
                .build_int_truncate(res.into_int_value(), left.get_type(), "")
        } else if bin.math_overflow_check && !unchecked {
            self.build_binary_op_with_overflow_check(
                bin,
                function,
                left,
                right,
                BinaryOp::Multiply,
                signed,
            )
        } else {
            bin.builder.build_int_mul(left, right, "")
        }
    }

    fn power(
        &self,
        bin: &Binary<'a>,
        unchecked: bool,
        bits: u32,
        signed: bool,
        o: PointerValue<'a>,
    ) -> FunctionValue<'a> {
        /*
            int ipow(int base, int exp)
            {
                int result = 1;
                for (;;)
                {
                    if (exp & 1)
                        result *= base;
                    exp >>= 1;
                    if (!exp)
                        break;
                    base *= base;
                }
                return result;
            }
        */
        let name = format!(
            "__{}power{}{}",
            if signed { 's' } else { 'u' },
            bits,
            if unchecked { "unchecked" } else { "" }
        );
        let ty = bin.context.custom_width_int_type(bits);

        if let Some(f) = bin.module.get_function(&name) {
            return f;
        }

        let pos = bin.builder.get_insert_block().unwrap();

        // __upower(base, exp)
        let function = bin.module.add_function(
            &name,
            ty.fn_type(&[ty.into(), ty.into(), o.get_type().into()], false),
            None,
        );

        let entry = bin.context.append_basic_block(function, "entry");
        let loop_block = bin.context.append_basic_block(function, "loop");
        let multiply = bin.context.append_basic_block(function, "multiply");
        let nomultiply = bin.context.append_basic_block(function, "nomultiply");
        let done = bin.context.append_basic_block(function, "done");
        let notdone = bin.context.append_basic_block(function, "notdone");

        bin.builder.position_at_end(entry);

        bin.builder.build_unconditional_branch(loop_block);

        bin.builder.position_at_end(loop_block);
        let base = bin.builder.build_phi(ty, "base");
        base.add_incoming(&[(&function.get_nth_param(0).unwrap(), entry)]);

        let exp = bin.builder.build_phi(ty, "exp");
        exp.add_incoming(&[(&function.get_nth_param(1).unwrap(), entry)]);

        let result = bin.builder.build_phi(ty, "result");
        result.add_incoming(&[(&ty.const_int(1, false), entry)]);

        let lowbit = bin.builder.build_int_truncate(
            exp.as_basic_value().into_int_value(),
            bin.context.bool_type(),
            "bit",
        );

        bin.builder
            .build_conditional_branch(lowbit, multiply, nomultiply);

        bin.builder.position_at_end(multiply);

        let result2 = self.mul(
            bin,
            function,
            unchecked,
            result.as_basic_value().into_int_value(),
            base.as_basic_value().into_int_value(),
            signed,
        );

        let multiply = bin.builder.get_insert_block().unwrap();

        bin.builder.build_unconditional_branch(nomultiply);
        bin.builder.position_at_end(nomultiply);

        let result3 = bin.builder.build_phi(ty, "result");
        result3.add_incoming(&[(&result.as_basic_value(), loop_block), (&result2, multiply)]);

        let exp2 = bin.builder.build_right_shift(
            exp.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "exp",
        );
        let zero = bin
            .builder
            .build_int_compare(IntPredicate::EQ, exp2, ty.const_zero(), "zero");

        bin.builder.build_conditional_branch(zero, done, notdone);
        bin.builder.position_at_end(done);

        // If successful operation, load the result in the output pointer then return zero.
        bin.builder.build_store(
            function.get_nth_param(2).unwrap().into_pointer_value(),
            result3.as_basic_value(),
        );
        bin.builder
            .build_return(Some(&bin.context.i64_type().const_zero()));

        bin.builder.position_at_end(notdone);

        let base2 = self.mul(
            bin,
            function,
            unchecked,
            base.as_basic_value().into_int_value(),
            base.as_basic_value().into_int_value(),
            signed,
        );

        let notdone = bin.builder.get_insert_block().unwrap();

        base.add_incoming(&[(&base2, notdone)]);
        result.add_incoming(&[(&result3.as_basic_value(), notdone)]);
        exp.add_incoming(&[(&exp2, notdone)]);

        bin.builder.build_unconditional_branch(loop_block);

        bin.builder.position_at_end(pos);

        function
    }
    /// Convenience function for generating binary operations with overflow checking.
    fn build_binary_op_with_overflow_check(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        left: IntValue<'a>,
        right: IntValue<'a>,
        op: BinaryOp,
        signed: bool,
    ) -> IntValue<'a> {
        let ret_ty = bin.context.struct_type(
            &[
                left.get_type().into(),
                bin.context.custom_width_int_type(1).into(),
            ],
            false,
        );
        let binop = bin.llvm_overflow(ret_ty.into(), left.get_type(), signed, op);

        let op_res = bin
            .builder
            .build_call(binop, &[left.into(), right.into()], "res")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_struct_value();

        let overflow = bin
            .builder
            .build_extract_value(op_res, 1, "overflow")
            .unwrap()
            .into_int_value();

        let success_block = bin.context.append_basic_block(function, "success");
        let error_block = bin.context.append_basic_block(function, "error");

        bin.builder
            .build_conditional_branch(overflow, error_block, success_block);

        bin.builder.position_at_end(error_block);

        self.assert_failure(
            bin,
            bin.context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            bin.context.i32_type().const_zero(),
        );

        bin.builder.position_at_end(success_block);

        bin.builder
            .build_extract_value(op_res, 0, "res")
            .unwrap()
            .into_int_value()
    }
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) enum ReturnCode {
    Success,
    FunctionSelectorInvalid,
    AbiEncodingInvalid,
}

#[derive(PartialEq, Eq)]
pub enum Generate {
    Object,
    Assembly,
    Linked,
}

impl Target {
    /// LLVM Target name
    fn llvm_target_name(&self) -> &'static str {
        if *self == Target::Solana {
            "bpfel"
        } else {
            "wasm32"
        }
    }

    /// LLVM Target triple
    fn llvm_target_triple(&self) -> TargetTriple {
        TargetTriple::create(if *self == Target::Solana {
            "bpfel-unknown-unknown"
        } else {
            "wasm32-unknown-unknown-wasm"
        })
    }

    /// LLVM Target triple
    fn llvm_features(&self) -> &'static str {
        if *self == Target::Solana {
            "+solana"
        } else {
            ""
        }
    }
}
