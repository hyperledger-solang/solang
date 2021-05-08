use crate::parser::pt;
use crate::sema::ast;
use crate::sema::ast::{Builtin, Expression, FormatArg, StringLocation};
use std::cell::RefCell;
use std::ffi::CStr;
use std::fmt;
use std::path::Path;
use std::str;

use num_bigint::BigInt;
use num_traits::One;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::collections::VecDeque;

use crate::Target;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode, TargetTriple};
use inkwell::types::{BasicType, BasicTypeEnum, FunctionType, IntType, StringRadix};
use inkwell::values::{
    ArrayValue, BasicValueEnum, FunctionValue, GlobalValue, IntValue, PhiValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

mod ethabiencoder;
mod ewasm;
mod generic;
mod loop_builder;
mod sabre;
mod solana;
mod substrate;

use crate::codegen::cfg::{ControlFlowGraph, HashTy, Instr, InternalCallTy, Storage};
use crate::linker::link;

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = {
        inkwell::targets::Target::initialize_webassembly(&Default::default());
        inkwell::targets::Target::initialize_bpf(&Default::default());
    };
}

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
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
    );

    /// Abi encode with optional four bytes selector. The load parameter should be set if the args are
    /// pointers to data, not the actual data  itself.
    fn abi_encode(
        &self,
        contract: &Contract<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue<'a>,
        args: &[BasicValueEnum<'a>],
        tys: &[ast::Type],
    ) -> (PointerValue<'a>, IntValue<'a>);

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
    ) -> PointerValue<'b>;

    fn set_storage(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_int(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a>;

    // Bytes and string have special storage layout
    fn set_storage_string(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    );
    fn get_storage_string(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a>;
    fn set_storage_extfunc(
        &self,
        contract: &Contract,
        function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    );
    fn get_storage_extfunc(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a>;
    fn get_storage_bytes_subscript(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a>;
    fn set_storage_bytes_subscript(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
    );
    fn storage_subscript(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue<'a>,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
        _index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        // not need for slot-based storage chains
        unimplemented!();
    }
    fn storage_push(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        val: BasicValueEnum<'a>,
    ) -> BasicValueEnum<'a>;
    fn storage_pop(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
    ) -> BasicValueEnum<'a>;
    fn storage_array_length(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _elem_ty: &ast::Type,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    /// keccak256 hash
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    );

    /// Prints a string
    fn print(&self, contract: &Contract, string: PointerValue, length: IntValue);

    /// Return success without any result
    fn return_empty_abi(&self, contract: &Contract);

    /// Return failure code
    fn return_code<'b>(&self, contract: &'b Contract, ret: IntValue<'b>);

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue);

    /// Return failure without any result
    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, length: IntValue);

    /// Calls constructor
    fn create_contract<'b>(
        &mut self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
    );

    /// call external function
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        ty: ast::CallTy,
    );

    /// send value to address
    fn value_transfer<'b>(
        &self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
    ) {
        unimplemented!();
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        contract: &Contract<'b>,
        expr: &Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b>;

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b>;

    /// Return the value we received
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b>;

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, contract: &Contract<'b>, addr: IntValue<'b>);

    /// Crypto Hash
    fn hash<'b>(
        &self,
        contract: &Contract<'b>,
        hash: HashTy,
        string: PointerValue<'b>,
        length: IntValue<'b>,
    ) -> IntValue<'b>;

    /// Integer to prefix events with
    fn event_id<'b>(&self, _contract: &Contract<'b>, _event_no: usize) -> Option<IntValue<'b>> {
        None
    }

    /// Send event
    fn send_event<'b>(
        &self,
        contract: &Contract<'b>,
        event_no: usize,
        data: PointerValue<'b>,
        data_len: IntValue<'b>,
        topics: Vec<(PointerValue<'b>, IntValue<'b>)>,
    );

    /// Helper functions which need access to the trait

    /// If we receive a value transfer, and we are "payable", abort with revert
    fn abort_if_value_transfer(&self, contract: &Contract, function: FunctionValue) {
        let value = self.value_transferred(&contract);

        let got_value = contract.builder.build_int_compare(
            IntPredicate::NE,
            value,
            contract.value_type().const_zero(),
            "is_value_transfer",
        );

        let not_value_transfer = contract
            .context
            .append_basic_block(function, "not_value_transfer");
        let abort_value_transfer = contract
            .context
            .append_basic_block(function, "abort_value_transfer");

        contract.builder.build_conditional_branch(
            got_value,
            abort_value_transfer,
            not_value_transfer,
        );

        contract.builder.position_at_end(abort_value_transfer);

        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );

        contract.builder.position_at_end(not_value_transfer);
    }

    /// Recursively load a type from contract storage
    fn storage_load(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue,
    ) -> BasicValueEnum<'a> {
        // The storage slot is an i256 accessed through a pointer, so we need
        // to store it
        let slot_ptr = contract.builder.build_alloca(slot.get_type(), "slot");

        self.storage_load_slot(contract, ty, slot, slot_ptr, function)
    }

    /// Recursively load a type from contract storage for slot based contract storage
    fn storage_load_slot(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
    ) -> BasicValueEnum<'a> {
        match ty {
            ast::Type::Ref(ty) => self.storage_load_slot(contract, ty, slot, slot_ptr, function),
            ast::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let llvm_ty = contract.llvm_type(ty.deref_any());
                    // LLVMSizeOf() produces an i64
                    let size = contract.builder.build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        contract.context.i32_type(),
                        "size_of",
                    );

                    let ty = ty.array_deref();
                    let new = contract
                        .builder
                        .build_call(
                            contract.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let dest = contract.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );

                    contract.emit_static_loop_with_int(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = unsafe {
                                contract.builder.build_gep(
                                    dest,
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val =
                                self.storage_load_slot(contract, &ty, slot, slot_ptr, function);

                            contract.builder.build_store(elem, val);
                        },
                    );

                    dest.into()
                } else {
                    // iterate over dynamic array
                    let slot_ty = ast::Type::Uint(256);

                    let size = contract.builder.build_int_truncate(
                        self.storage_load_slot(contract, &slot_ty, slot, slot_ptr, function)
                            .into_int_value(),
                        contract.context.i32_type(),
                        "size",
                    );

                    let elem_ty = contract.llvm_type(&ty.array_elem());
                    let elem_size = contract.builder.build_int_truncate(
                        elem_ty.size_of().unwrap(),
                        contract.context.i32_type(),
                        "size_of",
                    );
                    let init = contract.builder.build_int_to_ptr(
                        contract.context.i32_type().const_all_ones(),
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    let dest = contract
                        .builder
                        .build_call(
                            contract.module.get_function("vector_new").unwrap(),
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
                        contract,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(contract.context.i32_type(), false),
                        slot_ptr,
                    );

                    let mut elem_slot = contract
                        .builder
                        .build_load(slot_ptr, "elem_slot")
                        .into_int_value();

                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        size,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = contract.builder.build_int_mul(elem_no, elem_size, "");

                            let entry = self.storage_load_slot(
                                contract,
                                &ty.array_elem(),
                                slot,
                                slot_ptr,
                                function,
                            );

                            let data = unsafe {
                                contract.builder.build_gep(
                                    dest,
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            contract.builder.build_store(
                                contract.builder.build_pointer_cast(
                                    data,
                                    elem_ty.ptr_type(AddressSpace::Generic),
                                    "entry",
                                ),
                                entry,
                            );
                        },
                    );
                    // load
                    dest.into()
                }
            }
            ast::Type::Struct(n) => {
                let llvm_ty = contract.llvm_type(ty.deref_any());
                // LLVMSizeOf() produces an i64
                let size = contract.builder.build_int_truncate(
                    llvm_ty.size_of().unwrap(),
                    contract.context.i32_type(),
                    "size_of",
                );

                let new = contract
                    .builder
                    .build_call(
                        contract.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let dest = contract.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    "dest",
                );

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let val = self.storage_load_slot(contract, &field.ty, slot, slot_ptr, function);

                    let elem = unsafe {
                        contract.builder.build_gep(
                            dest,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    contract.builder.build_store(elem, val);
                }

                dest.into()
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                contract.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_string(contract, function, slot_ptr);

                *slot = contract.builder.build_int_add(
                    *slot,
                    contract.number_literal(256, &BigInt::one()),
                    "string",
                );

                ret.into()
            }
            ast::Type::InternalFunction { .. } => {
                contract.builder.build_store(slot_ptr, *slot);

                let ptr_ty = contract
                    .context
                    .custom_width_int_type(contract.ns.target.ptr_size() as u32);

                let ret = self.get_storage_int(contract, function, slot_ptr, ptr_ty);

                contract
                    .builder
                    .build_int_to_ptr(
                        ret,
                        contract.llvm_type(ty.deref_any()).into_pointer_type(),
                        "",
                    )
                    .into()
            }
            ast::Type::ExternalFunction { .. } => {
                contract.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_extfunc(contract, function, slot_ptr);

                *slot = contract.builder.build_int_add(
                    *slot,
                    contract.number_literal(256, &BigInt::one()),
                    "string",
                );

                ret.into()
            }
            _ => {
                contract.builder.build_store(slot_ptr, *slot);

                let ret = self.get_storage_int(
                    contract,
                    function,
                    slot_ptr,
                    contract.llvm_type(ty.deref_any()).into_int_type(),
                );

                *slot = contract.builder.build_int_add(
                    *slot,
                    contract.number_literal(256, &BigInt::one()),
                    "int",
                );

                ret.into()
            }
        }
    }

    /// Recursively store a type to contract storage
    fn storage_store(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    ) {
        let slot_ptr = contract.builder.build_alloca(slot.get_type(), "slot");

        self.storage_store_slot(contract, ty, slot, slot_ptr, dest, function)
    }

    /// Recursively store a type to contract storage for slot-based contract storage
    fn storage_store_slot(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    ) {
        match ty.deref_any() {
            ast::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let ty = ty.array_deref();

                    contract.emit_static_loop_with_int(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let mut elem = unsafe {
                                contract.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            if ty.is_reference_type() {
                                elem = contract.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store_slot(
                                contract,
                                &ty,
                                slot,
                                slot_ptr,
                                elem.into(),
                                function,
                            );

                            if !ty.is_reference_type() {
                                *slot = contract.builder.build_int_add(
                                    *slot,
                                    contract.number_literal(256, &ty.storage_slots(contract.ns)),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // get the lenght of the our in-memory array
                    let len = contract
                        .builder
                        .build_load(
                            unsafe {
                                contract.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_zero(),
                                    ],
                                    "array_len",
                                )
                            },
                            "array_len",
                        )
                        .into_int_value();

                    let slot_ty = ast::Type::Uint(256);

                    // details about our array elements
                    let elem_ty = contract.llvm_type(&ty.array_elem());
                    let elem_size = contract.builder.build_int_truncate(
                        elem_ty.size_of().unwrap(),
                        contract.context.i32_type(),
                        "size_of",
                    );

                    // the previous length of the storage array
                    // we need this to clear any elements
                    let previous_size = contract.builder.build_int_truncate(
                        self.storage_load_slot(contract, &slot_ty, slot, slot_ptr, function)
                            .into_int_value(),
                        contract.context.i32_type(),
                        "previous_size",
                    );

                    let new_slot = contract
                        .builder
                        .build_alloca(contract.llvm_type(&slot_ty).into_int_type(), "new");

                    // set new length
                    contract.builder.build_store(
                        new_slot,
                        contract.builder.build_int_z_extend(
                            len,
                            contract.llvm_type(&slot_ty).into_int_type(),
                            "",
                        ),
                    );

                    self.set_storage(contract, function, slot_ptr, new_slot);

                    self.keccak256_hash(
                        contract,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(contract.context.i32_type(), false),
                        new_slot,
                    );

                    let mut elem_slot = contract
                        .builder
                        .build_load(new_slot, "elem_slot")
                        .into_int_value();

                    let ty = ty.array_deref();

                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = contract.builder.build_int_mul(elem_no, elem_size, "");

                            let data = unsafe {
                                contract.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let mut elem = contract.builder.build_pointer_cast(
                                data,
                                elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            if ty.is_reference_type() {
                                elem = contract.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store_slot(
                                contract,
                                &ty,
                                slot,
                                slot_ptr,
                                elem.into(),
                                function,
                            );

                            if !ty.is_reference_type() {
                                *slot = contract.builder.build_int_add(
                                    *slot,
                                    contract.number_literal(256, &ty.storage_slots(contract.ns)),
                                    "",
                                );
                            }
                        },
                    );

                    // we've populated the array with the new values; if the new array is shorter
                    // than the previous, clear out the trailing elements
                    contract.emit_loop_cond_first_with_int(
                        function,
                        len,
                        previous_size,
                        &mut elem_slot,
                        |_: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(contract, &ty, slot, slot_ptr, function);

                            if !ty.is_reference_type() {
                                *slot = contract.builder.build_int_add(
                                    *slot,
                                    contract.number_literal(256, &ty.storage_slots(contract.ns)),
                                    "",
                                );
                            }
                        },
                    );
                }
            }
            ast::Type::Struct(n) => {
                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let mut elem = unsafe {
                        contract.builder.build_gep(
                            dest.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    if field.ty.is_reference_type() {
                        elem = contract
                            .builder
                            .build_load(elem, &field.name)
                            .into_pointer_value();
                    }

                    self.storage_store_slot(
                        contract,
                        &field.ty,
                        slot,
                        slot_ptr,
                        elem.into(),
                        function,
                    );

                    if !field.ty.is_reference_type() {
                        *slot = contract.builder.build_int_add(
                            *slot,
                            contract.number_literal(256, &field.ty.storage_slots(contract.ns)),
                            &field.name,
                        );
                    }
                }
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                contract.builder.build_store(slot_ptr, *slot);

                self.set_storage_string(contract, function, slot_ptr, dest);
            }
            ast::Type::ExternalFunction { .. } => {
                contract.builder.build_store(slot_ptr, *slot);

                self.set_storage_extfunc(contract, function, slot_ptr, dest.into_pointer_value());
            }
            ast::Type::InternalFunction { .. } => {
                let ptr_ty = contract
                    .context
                    .custom_width_int_type(contract.ns.target.ptr_size() as u32);

                let m = contract.builder.build_alloca(ptr_ty, "");

                contract.builder.build_store(
                    m,
                    contract.builder.build_ptr_to_int(
                        dest.into_pointer_value(),
                        ptr_ty,
                        "function_pointer",
                    ),
                );

                contract.builder.build_store(slot_ptr, *slot);

                self.set_storage(contract, function, slot_ptr, m);
            }
            _ => {
                contract.builder.build_store(slot_ptr, *slot);

                let dest = if dest.is_int_value() {
                    let m = contract.builder.build_alloca(dest.get_type(), "");
                    contract.builder.build_store(m, dest);

                    m
                } else {
                    dest.into_pointer_value()
                };

                // TODO ewasm allocates 32 bytes here, even though we have just
                // allocated test. This can be folded into one allocation, if llvm
                // does not already fold it into one.
                self.set_storage(contract, function, slot_ptr, dest);
            }
        }
    }

    // Clear a particlar storage slot (slot-based storage chains should implement)
    fn storage_delete_single_slot(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
    ) {
        unimplemented!();
    }

    /// Recursively clear contract storage. The default implementation is for slot-based contract storage
    fn storage_delete(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        let slot_ptr = contract.builder.build_alloca(slot.get_type(), "slot");

        self.storage_delete_slot(contract, ty, slot, slot_ptr, function);
    }

    /// Recursively clear contract storage for slot-based contract storage
    fn storage_delete_slot(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
    ) {
        match ty.deref_any() {
            ast::Type::Array(_, dim) => {
                let ty = ty.array_deref();

                if let Some(d) = &dim[0] {
                    contract.emit_static_loop_with_int(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(contract, &ty, slot, slot_ptr, function);

                            if !ty.is_reference_type() {
                                *slot = contract.builder.build_int_add(
                                    *slot,
                                    contract.number_literal(256, &ty.storage_slots(contract.ns)),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // dynamic length array.
                    // load length
                    contract.builder.build_store(slot_ptr, *slot);

                    let slot_ty = contract.context.custom_width_int_type(256);

                    let buf = contract.builder.build_alloca(slot_ty, "buf");

                    let length = self.get_storage_int(contract, function, slot_ptr, slot_ty);

                    // we need to hash the length slot in order to get the slot of the first
                    // entry of the array
                    self.keccak256_hash(
                        contract,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(contract.context.i32_type(), false),
                        buf,
                    );

                    let mut entry_slot = contract
                        .builder
                        .build_load(buf, "entry_slot")
                        .into_int_value();

                    // now loop from first slot to first slot + length
                    contract.emit_loop_cond_first_with_int(
                        function,
                        length.get_type().const_zero(),
                        length,
                        &mut entry_slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_delete_slot(contract, &ty, slot, slot_ptr, function);

                            if !ty.is_reference_type() {
                                *slot = contract.builder.build_int_add(
                                    *slot,
                                    contract.number_literal(256, &ty.storage_slots(contract.ns)),
                                    "",
                                );
                            }
                        },
                    );

                    // clear length itself
                    self.storage_delete_slot(
                        contract,
                        &ast::Type::Uint(256),
                        slot,
                        slot_ptr,
                        function,
                    );
                }
            }
            ast::Type::Struct(n) => {
                for (_, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    self.storage_delete_slot(contract, &field.ty, slot, slot_ptr, function);

                    if !field.ty.is_reference_type() {
                        *slot = contract.builder.build_int_add(
                            *slot,
                            contract.number_literal(256, &field.ty.storage_slots(contract.ns)),
                            &field.name,
                        );
                    }
                }
            }
            ast::Type::Mapping(_, _) => {
                // nothing to do, step over it
            }
            _ => {
                contract.builder.build_store(slot_ptr, *slot);

                self.storage_delete_single_slot(contract, function, slot_ptr);
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
        contract: &Contract<'a>,
        e: &Expression,
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
    ) -> BasicValueEnum<'a> {
        match e {
            Expression::FunctionArg(_, _, pos) => function.get_nth_param(*pos as u32).unwrap(),
            Expression::BoolLiteral(_, val) => contract
                .context
                .bool_type()
                .const_int(*val as u64, false)
                .into(),
            Expression::NumberLiteral(_, ty, n) => contract
                .number_literal(ty.bits(contract.ns) as u32, n)
                .into(),
            Expression::StructLiteral(_, ty, exprs) => {
                let struct_ty = contract.llvm_type(ty);

                let s = contract
                    .builder
                    .build_call(
                        contract.module.get_function("__malloc").unwrap(),
                        &[struct_ty
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let s = contract.builder.build_pointer_cast(
                    s,
                    struct_ty.ptr_type(AddressSpace::Generic),
                    "struct_literal",
                );

                for (i, f) in exprs.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            s,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            "struct member",
                        )
                    };

                    contract
                        .builder
                        .build_store(elem, self.expression(contract, f, vartab, function));
                }

                s.into()
            }
            Expression::BytesLiteral(_, _, bs) => {
                let ty = contract
                    .context
                    .custom_width_int_type((bs.len() * 8) as u32);

                // hex"11223344" should become i32 0x11223344
                let s = hex::encode(bs);

                ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                    .unwrap()
                    .into()
            }
            Expression::CodeLiteral(_, contract_no, runtime) => {
                let codegen_contract = &contract.ns.contracts[*contract_no];

                let target_contract = Contract::build(
                    contract.context,
                    &codegen_contract,
                    contract.ns,
                    "",
                    contract.opt,
                    contract.math_overflow_check,
                );

                let code = if *runtime && target_contract.runtime.is_some() {
                    target_contract
                        .runtime
                        .unwrap()
                        .code(true)
                        .expect("compile should succeeed")
                } else {
                    target_contract.code(true).expect("compile should succeeed")
                };

                let size = contract
                    .context
                    .i32_type()
                    .const_int(code.len() as u64, false);

                let elem_size = contract.context.i32_type().const_int(1, false);

                let init = contract.emit_global_string(
                    &format!(
                        "code_{}_{}",
                        if *runtime { "runtime" } else { "deployer" },
                        &codegen_contract.name
                    ),
                    &code,
                    true,
                );

                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                contract
                    .builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::Add(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                if contract.math_overflow_check {
                    let signed = l.ty().is_signed_int();
                    self.build_binary_op_with_overflow_check(
                        contract,
                        function,
                        left,
                        right,
                        BinaryOp::Add,
                        signed,
                    )
                    .into()
                } else {
                    contract.builder.build_int_add(left, right, "").into()
                }
            }
            Expression::Subtract(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                if contract.math_overflow_check {
                    let signed = l.ty().is_signed_int();
                    self.build_binary_op_with_overflow_check(
                        contract,
                        function,
                        left,
                        right,
                        BinaryOp::Subtract,
                        signed,
                    )
                    .into()
                } else {
                    contract.builder.build_int_sub(left, right, "").into()
                }
            }
            Expression::Multiply(_, res_ty, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                self.mul(contract, function, left, right, res_ty.is_signed_int())
                    .into()
            }
            Expression::Divide(_, _, l, r) if !l.ty().is_signed_int() => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("udivmod{}", div_bits);

                    let f = contract
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = contract.context.custom_width_int_type(div_bits);

                    let dividend = contract.build_alloca(function, ty, "dividend");
                    let divisor = contract.build_alloca(function, ty, "divisor");
                    let rem = contract.build_alloca(function, ty, "remainder");
                    let quotient = contract.build_alloca(function, ty, "quotient");

                    contract.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            contract.builder.build_int_z_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    contract.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            contract.builder.build_int_z_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = contract
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = contract.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        contract.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = contract.context.append_basic_block(function, "success");
                    let bail_block = contract.context.append_basic_block(function, "bail");
                    contract
                        .builder
                        .build_conditional_branch(success, success_block, bail_block);

                    contract.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        contract,
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        contract.context.i32_type().const_zero(),
                    );

                    contract.builder.position_at_end(success_block);

                    let quotient = contract
                        .builder
                        .build_load(quotient, "quotient")
                        .into_int_value();

                    if bits < div_bits {
                        contract
                            .builder
                            .build_int_truncate(quotient, left.get_type(), "")
                    } else {
                        quotient
                    }
                    .into()
                } else {
                    contract
                        .builder
                        .build_int_unsigned_div(left, right, "")
                        .into()
                }
            }
            Expression::Divide(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("sdivmod{}", div_bits);

                    let f = contract
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = contract.context.custom_width_int_type(div_bits);

                    let dividend = contract.build_alloca(function, ty, "dividend");
                    let divisor = contract.build_alloca(function, ty, "divisor");
                    let rem = contract.build_alloca(function, ty, "remainder");
                    let quotient = contract.build_alloca(function, ty, "quotient");

                    contract.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            contract.builder.build_int_s_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    contract.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            contract.builder.build_int_s_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = contract
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = contract.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        contract.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = contract.context.append_basic_block(function, "success");
                    let bail_block = contract.context.append_basic_block(function, "bail");
                    contract
                        .builder
                        .build_conditional_branch(success, success_block, bail_block);

                    contract.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        contract,
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        contract.context.i32_type().const_zero(),
                    );

                    contract.builder.position_at_end(success_block);

                    let quotient = contract
                        .builder
                        .build_load(quotient, "quotient")
                        .into_int_value();

                    if bits < div_bits {
                        contract
                            .builder
                            .build_int_truncate(quotient, left.get_type(), "")
                    } else {
                        quotient
                    }
                    .into()
                } else if contract.ns.target == Target::Solana {
                    // no signed div on BPF; do abs udev and then negate if needed
                    let left_negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    );

                    let left = contract
                        .builder
                        .build_select(
                            left_negative,
                            contract.builder.build_int_neg(left, "signed_left"),
                            left,
                            "left_abs",
                        )
                        .into_int_value();

                    let right_negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    );

                    let right = contract
                        .builder
                        .build_select(
                            right_negative,
                            contract.builder.build_int_neg(right, "signed_right"),
                            right,
                            "right_abs",
                        )
                        .into_int_value();

                    let res = contract.builder.build_int_unsigned_div(left, right, "");

                    let negate_result =
                        contract
                            .builder
                            .build_xor(left_negative, right_negative, "negate_result");

                    contract.builder.build_select(
                        negate_result,
                        contract.builder.build_int_neg(res, "unsigned_res"),
                        res,
                        "res",
                    )
                } else {
                    contract
                        .builder
                        .build_int_signed_div(left, right, "")
                        .into()
                }
            }
            Expression::Modulo(_, _, l, r) if !l.ty().is_signed_int() => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("udivmod{}", div_bits);

                    let f = contract
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = contract.context.custom_width_int_type(div_bits);

                    let dividend = contract.build_alloca(function, ty, "dividend");
                    let divisor = contract.build_alloca(function, ty, "divisor");
                    let rem = contract.build_alloca(function, ty, "remainder");
                    let quotient = contract.build_alloca(function, ty, "quotient");

                    contract.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            contract.builder.build_int_z_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    contract.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            contract.builder.build_int_z_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = contract
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "udiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = contract.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        contract.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = contract.context.append_basic_block(function, "success");
                    let bail_block = contract.context.append_basic_block(function, "bail");
                    contract
                        .builder
                        .build_conditional_branch(success, success_block, bail_block);

                    contract.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        contract,
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        contract.context.i32_type().const_zero(),
                    );

                    contract.builder.position_at_end(success_block);

                    let rem = contract.builder.build_load(rem, "urem").into_int_value();

                    if bits < div_bits {
                        contract.builder.build_int_truncate(
                            rem,
                            contract.context.custom_width_int_type(bits),
                            "",
                        )
                    } else {
                        rem
                    }
                    .into()
                } else {
                    contract
                        .builder
                        .build_int_unsigned_rem(left, right, "")
                        .into()
                }
            }
            Expression::Modulo(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let div_bits = if bits <= 128 { 128 } else { 256 };

                    let name = format!("sdivmod{}", div_bits);

                    let f = contract
                        .module
                        .get_function(&name)
                        .expect("div function missing");

                    let ty = contract.context.custom_width_int_type(div_bits);

                    let dividend = contract.build_alloca(function, ty, "dividend");
                    let divisor = contract.build_alloca(function, ty, "divisor");
                    let rem = contract.build_alloca(function, ty, "remainder");
                    let quotient = contract.build_alloca(function, ty, "quotient");

                    contract.builder.build_store(
                        dividend,
                        if bits < div_bits {
                            contract.builder.build_int_s_extend(left, ty, "")
                        } else {
                            left
                        },
                    );

                    contract.builder.build_store(
                        divisor,
                        if bits < div_bits {
                            contract.builder.build_int_s_extend(right, ty, "")
                        } else {
                            right
                        },
                    );

                    let ret = contract
                        .builder
                        .build_call(
                            f,
                            &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
                            "sdiv",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = contract.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        contract.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = contract.context.append_basic_block(function, "success");
                    let bail_block = contract.context.append_basic_block(function, "bail");
                    contract
                        .builder
                        .build_conditional_branch(success, success_block, bail_block);

                    contract.builder.position_at_end(bail_block);

                    // throw division by zero error should be an assert
                    self.assert_failure(
                        contract,
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .const_null(),
                        contract.context.i32_type().const_zero(),
                    );

                    contract.builder.position_at_end(success_block);

                    let rem = contract.builder.build_load(rem, "srem").into_int_value();

                    if bits < div_bits {
                        contract.builder.build_int_truncate(
                            rem,
                            contract.context.custom_width_int_type(bits),
                            "",
                        )
                    } else {
                        rem
                    }
                    .into()
                } else if contract.ns.target == Target::Solana {
                    // no signed rem on BPF; do abs udev and then negate if needed
                    let left_negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        left,
                        left.get_type().const_zero(),
                        "left_negative",
                    );

                    let left = contract.builder.build_select(
                        left_negative,
                        contract.builder.build_int_neg(left, "signed_left"),
                        left,
                        "left_abs",
                    );

                    let right_negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        right,
                        right.get_type().const_zero(),
                        "right_negative",
                    );

                    let right = contract.builder.build_select(
                        right_negative,
                        contract.builder.build_int_neg(right, "signed_right"),
                        right,
                        "right_abs",
                    );

                    let res = contract.builder.build_int_unsigned_rem(
                        left.into_int_value(),
                        right.into_int_value(),
                        "",
                    );

                    contract.builder.build_select(
                        left_negative,
                        contract.builder.build_int_neg(res, "unsigned_res"),
                        res,
                        "res",
                    )
                } else {
                    contract
                        .builder
                        .build_int_signed_rem(left, right, "")
                        .into()
                }
            }
            Expression::Power(_, res_ty, l, r) => {
                let left = self.expression(contract, l, vartab, function);
                let right = self.expression(contract, r, vartab, function);

                let bits = left.into_int_value().get_type().get_bit_width();

                let f = self.power(contract, bits, res_ty.is_signed_int());

                contract
                    .builder
                    .build_call(f, &[left, right], "power")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::Equal(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_int_compare(IntPredicate::EQ, left, right, "")
                    .into()
            }
            Expression::NotEqual(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_int_compare(IntPredicate::NE, left, right, "")
                    .into()
            }
            Expression::More(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_int_compare(
                        if l.ty().is_signed_int() {
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
            Expression::MoreEqual(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
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
            Expression::Less(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_int_compare(
                        if l.ty().is_signed_int() {
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
            Expression::LessEqual(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
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
            Expression::Variable(_, _, s) => vartab[s].value,
            Expression::Load(_, ty, e) => {
                let ptr = self
                    .expression(contract, e, vartab, function)
                    .into_pointer_value();

                let value = contract.builder.build_load(ptr, "");

                if ty.is_reference_type() {
                    // if the pointer is null, it needs to be allocated
                    let allocation_needed = contract
                        .builder
                        .build_is_null(value.into_pointer_value(), "allocation_needed");

                    let allocate = contract.context.append_basic_block(function, "allocate");
                    let already_allocated = contract
                        .context
                        .append_basic_block(function, "already_allocated");

                    contract.builder.build_conditional_branch(
                        allocation_needed,
                        allocate,
                        already_allocated,
                    );

                    let entry = contract.builder.get_insert_block().unwrap();

                    contract.builder.position_at_end(allocate);

                    // allocate a new struct
                    let ty = e.ty();

                    let llvm_ty = contract.llvm_type(&ty.deref_memory());

                    let new_struct = contract
                        .builder
                        .build_call(
                            contract.module.get_function("__malloc").unwrap(),
                            &[llvm_ty
                                .size_of()
                                .unwrap()
                                .const_cast(contract.context.i32_type(), false)
                                .into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let new_struct = contract.builder.build_pointer_cast(
                        new_struct,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        &format!("new_{}", ty.to_string(contract.ns)),
                    );

                    contract.builder.build_store(ptr, new_struct);

                    contract
                        .builder
                        .build_unconditional_branch(already_allocated);

                    contract.builder.position_at_end(already_allocated);

                    // insert phi node
                    let combined_struct_ptr = contract.builder.build_phi(
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        &format!("ptr_{}", ty.to_string(contract.ns)),
                    );

                    combined_struct_ptr.add_incoming(&[(&value, entry), (&new_struct, allocate)]);

                    combined_struct_ptr.as_basic_value()
                } else {
                    value
                }
            }
            Expression::StorageLoad(_, _, _) => {
                // this should be covered by the Instr::LoadStorage instruction; if we reach this
                // point the code generation is broken.
                unreachable!();
            }
            Expression::ZeroExt(_, t, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();
                let ty = contract.llvm_type(t);

                contract
                    .builder
                    .build_int_z_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::UnaryMinus(_, _, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();

                contract.builder.build_int_neg(e, "").into()
            }
            Expression::SignExt(_, t, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();
                let ty = contract.llvm_type(t);

                contract
                    .builder
                    .build_int_s_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Trunc(_, t, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();
                let ty = contract.llvm_type(t);

                contract
                    .builder
                    .build_int_truncate(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Cast(_, _, e) => self.expression(contract, e, vartab, function),
            Expression::BytesCast(_, ast::Type::Bytes(_), ast::Type::DynamicBytes, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();

                let size = e.get_type().get_bit_width() / 8;
                let size = contract.context.i32_type().const_int(size as u64, false);
                let elem_size = contract.context.i32_type().const_int(1, false);

                // Swap the byte order
                let bytes_ptr = contract.builder.build_alloca(e.get_type(), "bytes_ptr");
                contract.builder.build_store(bytes_ptr, e);
                let bytes_ptr = contract.builder.build_pointer_cast(
                    bytes_ptr,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "bytes_ptr",
                );
                let init = contract.builder.build_pointer_cast(
                    contract.builder.build_alloca(e.get_type(), "init"),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "init",
                );
                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[bytes_ptr.into(), init.into(), size.into()],
                    "",
                );

                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                contract
                    .builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::BytesCast(_, ast::Type::DynamicBytes, ast::Type::Bytes(n), e) => {
                let array = self.expression(contract, e, vartab, function);

                let len = contract.vector_len(array);

                // Check if equal to n
                let is_equal_to_n = contract.builder.build_int_compare(
                    IntPredicate::EQ,
                    len,
                    contract.context.i32_type().const_int(*n as u64, false),
                    "is_equal_to_n",
                );
                let cast = contract.context.append_basic_block(function, "cast");
                let error = contract.context.append_basic_block(function, "error");
                contract
                    .builder
                    .build_conditional_branch(is_equal_to_n, cast, error);

                contract.builder.position_at_end(error);
                self.assert_failure(
                    &contract,
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .const_null(),
                    contract.context.i32_type().const_zero(),
                );

                contract.builder.position_at_end(cast);
                let bytes_ptr = contract.vector_bytes(array);

                // Switch byte order
                let ty = contract.context.custom_width_int_type(*n as u32 * 8);
                let le_bytes_ptr = contract.builder.build_alloca(ty, "le_bytes");

                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        bytes_ptr.into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                le_bytes_ptr,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "le_bytes_ptr",
                            )
                            .into(),
                        len.into(),
                    ],
                    "",
                );
                contract.builder.build_load(le_bytes_ptr, "bytes")
            }
            Expression::Not(_, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                    .into()
            }
            Expression::Complement(_, _, e) => {
                let e = self
                    .expression(contract, e, vartab, function)
                    .into_int_value();

                contract.builder.build_not(e, "").into()
            }
            Expression::Or(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_or(left, right, "").into()
            }
            Expression::And(_, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseOr(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_or(left, right, "").into()
            }
            Expression::BitwiseAnd(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseXor(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_xor(left, right, "").into()
            }
            Expression::ShiftLeft(_, _, l, r) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_left_shift(left, right, "").into()
            }
            Expression::ShiftRight(_, _, l, r, signed) => {
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract
                    .builder
                    .build_right_shift(left, right, *signed, "")
                    .into()
            }
            Expression::Subscript(_, ty, a, i) => {
                if ty.is_contract_storage() {
                    let array = self
                        .expression(contract, a, vartab, function)
                        .into_int_value();
                    let index = self.expression(contract, i, vartab, function);

                    self.storage_subscript(contract, function, ty, array, index)
                        .into()
                } else {
                    let array = self
                        .expression(contract, a, vartab, function)
                        .into_pointer_value();
                    let index = self
                        .expression(contract, i, vartab, function)
                        .into_int_value();

                    unsafe {
                        contract
                            .builder
                            .build_gep(
                                array,
                                &[contract.context.i32_type().const_zero(), index],
                                "index_access",
                            )
                            .into()
                    }
                }
            }
            Expression::StorageBytesSubscript(_, a, i) => {
                let index = self
                    .expression(contract, i, vartab, function)
                    .into_int_value();
                let slot = self
                    .expression(contract, a, vartab, function)
                    .into_int_value();
                self.get_storage_bytes_subscript(&contract, function, slot, index)
                    .into()
            }
            Expression::DynamicArraySubscript(_, elem_ty, a, i) => {
                let array = self.expression(contract, a, vartab, function);

                let ty = contract.llvm_var(elem_ty);

                let mut array_index = self
                    .expression(contract, i, vartab, function)
                    .into_int_value();

                // bounds checking already done; we can down-cast if necessary
                if array_index.get_type().get_bit_width() > 32 {
                    array_index = contract.builder.build_int_truncate(
                        array_index,
                        contract.context.i32_type(),
                        "index",
                    );
                }

                let index = contract.builder.build_int_mul(
                    array_index,
                    ty.into_pointer_type()
                        .get_element_type()
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false),
                    "",
                );

                let elem = unsafe {
                    contract.builder.build_gep(
                        contract.vector_bytes(array),
                        &[index],
                        "index_access",
                    )
                };

                contract
                    .builder
                    .build_pointer_cast(elem, ty.into_pointer_type(), "elem")
                    .into()
            }
            Expression::StructMember(_, _, a, i) => {
                let struct_ptr = self
                    .expression(contract, a, vartab, function)
                    .into_pointer_value();

                unsafe {
                    contract
                        .builder
                        .build_gep(
                            struct_ptr,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(*i as u64, false),
                            ],
                            "struct member",
                        )
                        .into()
                }
            }
            Expression::Ternary(_, _, c, l, r) => {
                let cond = self
                    .expression(contract, c, vartab, function)
                    .into_int_value();
                let left = self
                    .expression(contract, l, vartab, function)
                    .into_int_value();
                let right = self
                    .expression(contract, r, vartab, function)
                    .into_int_value();

                contract.builder.build_select(cond, left, right, "")
            }
            Expression::ConstArrayLiteral(_, _, dims, exprs) => {
                // For const arrays (declared with "constant" keyword, we should create a global constant
                let mut dims = dims.iter();

                let exprs = exprs
                    .iter()
                    .map(|e| {
                        self.expression(contract, e, vartab, function)
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
                let gv = contract.module.add_global(
                    ty,
                    Some(AddressSpace::Generic),
                    "const_array_literal",
                );

                gv.set_linkage(Linkage::Internal);

                gv.set_initializer(&arrays[0]);
                gv.set_constant(true);

                gv.as_pointer_value().into()
            }
            Expression::ArrayLiteral(_, ty, dims, exprs) => {
                // non-const array literals should alloca'ed and each element assigned
                let ty = contract.llvm_type(ty);

                let p = contract
                    .builder
                    .build_call(
                        contract.module.get_function("__malloc").unwrap(),
                        &[ty.size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false)
                            .into()],
                        "array_literal",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let array = contract.builder.build_pointer_cast(
                    p.into_pointer_value(),
                    ty.ptr_type(AddressSpace::Generic),
                    "array_literal",
                );

                for (i, expr) in exprs.iter().enumerate() {
                    let mut ind = vec![contract.context.i32_type().const_zero()];

                    let mut e = i as u32;

                    for d in dims {
                        ind.insert(
                            1,
                            contract
                                .context
                                .i32_type()
                                .const_int((e % *d).into(), false),
                        );

                        e /= *d;
                    }

                    let elemptr = unsafe {
                        contract
                            .builder
                            .build_gep(array, &ind, &format!("elemptr{}", i))
                    };

                    contract
                        .builder
                        .build_store(elemptr, self.expression(contract, expr, vartab, function));
                }

                array.into()
            }
            Expression::AllocDynamicArray(_, ty, size, init) => {
                if *ty == ast::Type::Slice {
                    let init = init.as_ref().unwrap();

                    let data = contract.emit_global_string("const_string", init, true);

                    contract
                        .llvm_type(ty)
                        .into_struct_type()
                        .const_named_struct(&[
                            data.into(),
                            contract
                                .context
                                .i32_type()
                                .const_int(init.len() as u64, false)
                                .into(),
                        ])
                        .into()
                } else {
                    let elem = match ty {
                        ast::Type::Slice | ast::Type::String | ast::Type::DynamicBytes => {
                            ast::Type::Bytes(1)
                        }
                        _ => ty.array_elem(),
                    };

                    let size = self
                        .expression(contract, size, vartab, function)
                        .into_int_value();

                    let elem_size = contract
                        .llvm_type(&elem)
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

                    contract.vector_new(size, elem_size, init.as_ref()).into()
                }
            }
            Expression::DynamicArrayLength(_, a) => {
                let array = self.expression(contract, a, vartab, function);

                contract.vector_len(array).into()
            }
            Expression::Keccak256(_, _, exprs) => {
                let mut length = contract.context.i32_type().const_zero();
                let mut values: Vec<(BasicValueEnum, IntValue, ast::Type)> = Vec::new();

                // first we need to calculate the length of the buffer and get the types/lengths
                for e in exprs {
                    let v = self.expression(contract, &e, vartab, function);

                    let len = match e.ty() {
                        ast::Type::DynamicBytes | ast::Type::String => {
                            // field 0 is the length
                            let array_len = unsafe {
                                contract.builder.build_gep(
                                    v.into_pointer_value(),
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_zero(),
                                    ],
                                    "array_len",
                                )
                            };

                            contract
                                .builder
                                .build_load(array_len, "array_len")
                                .into_int_value()
                        }
                        _ => v
                            .get_type()
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false),
                    };

                    length = contract.builder.build_int_add(length, len, "");

                    values.push((v, len, e.ty()));
                }

                //  now allocate a buffer
                let src = contract.builder.build_array_alloca(
                    contract.context.i8_type(),
                    length,
                    "keccak_src",
                );

                // fill in all the fields
                let mut offset = contract.context.i32_type().const_zero();

                for (v, len, ty) in values {
                    let elem = unsafe { contract.builder.build_gep(src, &[offset], "elem") };

                    offset = contract.builder.build_int_add(offset, len, "");

                    match ty {
                        ast::Type::DynamicBytes | ast::Type::String => {
                            let data = unsafe {
                                contract.builder.build_gep(
                                    v.into_pointer_value(),
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_int(2, false),
                                    ],
                                    "",
                                )
                            };

                            contract.builder.build_call(
                                contract.module.get_function("__memcpy").unwrap(),
                                &[
                                    elem.into(),
                                    contract
                                        .builder
                                        .build_pointer_cast(
                                            data,
                                            contract
                                                .context
                                                .i8_type()
                                                .ptr_type(AddressSpace::Generic),
                                            "data",
                                        )
                                        .into(),
                                    len.into(),
                                ],
                                "",
                            );
                        }
                        _ => {
                            let elem = contract.builder.build_pointer_cast(
                                elem,
                                v.get_type().ptr_type(AddressSpace::Generic),
                                "",
                            );

                            contract.builder.build_store(elem, v);
                        }
                    }
                }
                let dst = contract
                    .builder
                    .build_alloca(contract.context.custom_width_int_type(256), "keccak_dst");

                self.keccak256_hash(&contract, src, length, dst);

                contract.builder.build_load(dst, "keccak256_hash")
            }
            Expression::StringCompare(_, l, r) => {
                let (left, left_len) = self.string_location(contract, l, vartab, function);
                let (right, right_len) = self.string_location(contract, r, vartab, function);

                contract
                    .builder
                    .build_call(
                        contract.module.get_function("__memcmp").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::StringConcat(_, _, l, r) => {
                let (left, left_len) = self.string_location(contract, l, vartab, function);
                let (right, right_len) = self.string_location(contract, r, vartab, function);

                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("concat").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                contract
                    .builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::ReturnData(_) => self.return_data(contract).into(),
            Expression::StorageArrayLength { array, elem_ty, .. } => {
                let slot = self
                    .expression(contract, array, vartab, function)
                    .into_int_value();

                self.storage_array_length(contract, function, slot, elem_ty)
                    .into()
            }
            Expression::AbiEncode {
                tys, packed, args, ..
            } => self
                .abi_encode_to_vector(
                    contract,
                    function,
                    &packed
                        .iter()
                        .map(|a| self.expression(contract, &a, vartab, function))
                        .collect::<Vec<BasicValueEnum>>(),
                    &args
                        .iter()
                        .map(|a| self.expression(contract, &a, vartab, function))
                        .collect::<Vec<BasicValueEnum>>(),
                    tys,
                )
                .into(),
            Expression::Builtin(_, _, Builtin::Calldata, _)
                if contract.ns.target != Target::Substrate =>
            {
                contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[
                            contract.builder.build_load(
                                contract.calldata_len.as_pointer_value(),
                                "calldata_len",
                            ),
                            contract.context.i32_type().const_int(1, false).into(),
                            contract.builder.build_load(
                                contract.calldata_data.as_pointer_value(),
                                "calldata_data",
                            ),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::Builtin(_, _, Builtin::Signature, _) => {
                // need to byte-reverse selector
                let selector = contract
                    .builder
                    .build_alloca(contract.context.i32_type(), "selector");

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                contract.selector.as_pointer_value(),
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                selector,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                contract.builder.build_load(selector, "selector")
            }
            Expression::Builtin(_, _, Builtin::AddMod, args) => {
                let arith_ty = contract.context.custom_width_int_type(512);
                let res_ty = contract.context.custom_width_int_type(256);

                let x = self
                    .expression(contract, &args[0], vartab, function)
                    .into_int_value();
                let y = self
                    .expression(contract, &args[1], vartab, function)
                    .into_int_value();
                let k = self
                    .expression(contract, &args[2], vartab, function)
                    .into_int_value();
                let dividend = contract.builder.build_int_add(
                    contract.builder.build_int_z_extend(x, arith_ty, "wide_x"),
                    contract.builder.build_int_z_extend(y, arith_ty, "wide_y"),
                    "x_plus_y",
                );

                let divisor = contract.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let pdividend = contract.builder.build_alloca(arith_ty, "dividend");
                let pdivisor = contract.builder.build_alloca(arith_ty, "divisor");
                let rem = contract.builder.build_alloca(arith_ty, "remainder");
                let quotient = contract.builder.build_alloca(arith_ty, "quotient");

                contract.builder.build_store(pdividend, dividend);
                contract.builder.build_store(pdivisor, divisor);

                let ret = contract
                    .builder
                    .build_call(
                        contract.module.get_function("udivmod512").unwrap(),
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

                let success = contract.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    contract.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = contract.context.append_basic_block(function, "success");
                let bail_block = contract.context.append_basic_block(function, "bail");
                contract
                    .builder
                    .build_conditional_branch(success, success_block, bail_block);

                contract.builder.position_at_end(bail_block);

                // On Solana the return type is 64 bit
                let ret: BasicValueEnum = contract
                    .builder
                    .build_int_z_extend(
                        ret,
                        contract.return_values[&ReturnCode::Success].get_type(),
                        "ret",
                    )
                    .into();

                contract.builder.build_return(Some(&ret));
                contract.builder.position_at_end(success_block);

                let quotient = contract
                    .builder
                    .build_load(quotient, "quotient")
                    .into_int_value();

                contract
                    .builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::Builtin(_, _, Builtin::MulMod, args) => {
                let arith_ty = contract.context.custom_width_int_type(512);
                let res_ty = contract.context.custom_width_int_type(256);

                let x = self
                    .expression(contract, &args[0], vartab, function)
                    .into_int_value();
                let y = self
                    .expression(contract, &args[1], vartab, function)
                    .into_int_value();
                let x_m = contract.builder.build_alloca(arith_ty, "x_m");
                let y_m = contract.builder.build_alloca(arith_ty, "x_y");
                let x_times_y_m = contract.builder.build_alloca(arith_ty, "x_times_y_m");

                contract.builder.build_store(
                    x_m,
                    contract.builder.build_int_z_extend(x, arith_ty, "wide_x"),
                );
                contract.builder.build_store(
                    y_m,
                    contract.builder.build_int_z_extend(y, arith_ty, "wide_y"),
                );

                contract.builder.build_call(
                    contract.module.get_function("__mul32").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                x_m,
                                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                                "left",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                y_m,
                                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                                "right",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                x_times_y_m,
                                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                                "output",
                            )
                            .into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(512 / 32, false)
                            .into(),
                    ],
                    "",
                );
                let k = self
                    .expression(contract, &args[2], vartab, function)
                    .into_int_value();
                let dividend = contract.builder.build_load(x_times_y_m, "x_t_y");

                let divisor = contract.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let pdividend = contract.builder.build_alloca(arith_ty, "dividend");
                let pdivisor = contract.builder.build_alloca(arith_ty, "divisor");
                let rem = contract.builder.build_alloca(arith_ty, "remainder");
                let quotient = contract.builder.build_alloca(arith_ty, "quotient");

                contract.builder.build_store(pdividend, dividend);
                contract.builder.build_store(pdivisor, divisor);

                let ret = contract
                    .builder
                    .build_call(
                        contract.module.get_function("udivmod512").unwrap(),
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

                let success = contract.builder.build_int_compare(
                    IntPredicate::EQ,
                    ret,
                    contract.context.i32_type().const_zero(),
                    "success",
                );

                let success_block = contract.context.append_basic_block(function, "success");
                let bail_block = contract.context.append_basic_block(function, "bail");
                contract
                    .builder
                    .build_conditional_branch(success, success_block, bail_block);

                contract.builder.position_at_end(bail_block);

                // On Solana the return type is 64 bit
                let ret: BasicValueEnum = contract
                    .builder
                    .build_int_z_extend(
                        ret,
                        contract.return_values[&ReturnCode::Success].get_type(),
                        "ret",
                    )
                    .into();

                contract.builder.build_return(Some(&ret));

                contract.builder.position_at_end(success_block);

                let quotient = contract
                    .builder
                    .build_load(quotient, "quotient")
                    .into_int_value();

                contract
                    .builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::ExternalFunction {
                ty,
                address,
                function_no,
                ..
            } => {
                let address = self
                    .expression(contract, address, vartab, function)
                    .into_int_value();

                let selector = contract.ns.functions[*function_no].selector();

                assert!(matches!(ty, ast::Type::ExternalFunction { .. }));

                let ty = contract.llvm_type(&ty);

                let ef = contract
                    .builder
                    .build_call(
                        contract.module.get_function("__malloc").unwrap(),
                        &[ty.into_pointer_type()
                            .get_element_type()
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ef = contract.builder.build_pointer_cast(
                    ef,
                    ty.into_pointer_type(),
                    "function_type",
                );

                let address_member = unsafe {
                    contract.builder.build_gep(
                        ef,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "address",
                    )
                };

                contract.builder.build_store(address_member, address);

                let selector_member = unsafe {
                    contract.builder.build_gep(
                        ef,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(1, false),
                        ],
                        "selector",
                    )
                };

                contract.builder.build_store(
                    selector_member,
                    contract
                        .context
                        .i32_type()
                        .const_int(selector as u64, false),
                );

                ef.into()
            }
            Expression::Builtin(_, _, Builtin::ExternalFunctionSelector, args) => {
                let ef = self
                    .expression(contract, &args[0], vartab, function)
                    .into_pointer_value();

                let selector_member = unsafe {
                    contract.builder.build_gep(
                        ef,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(1, false),
                        ],
                        "selector",
                    )
                };

                contract.builder.build_load(selector_member, "selector")
            }
            Expression::Builtin(_, _, Builtin::ExternalFunctionAddress, args) => {
                let ef = self
                    .expression(contract, &args[0], vartab, function)
                    .into_pointer_value();

                let selector_member = unsafe {
                    contract.builder.build_gep(
                        ef,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "address",
                    )
                };

                contract.builder.build_load(selector_member, "address")
            }
            Expression::Builtin(_, _, hash @ Builtin::Ripemd160, args)
            | Expression::Builtin(_, _, hash @ Builtin::Keccak256, args)
            | Expression::Builtin(_, _, hash @ Builtin::Blake2_128, args)
            | Expression::Builtin(_, _, hash @ Builtin::Blake2_256, args)
            | Expression::Builtin(_, _, hash @ Builtin::Sha256, args) => {
                let v = self.expression(contract, &args[0], vartab, function);

                let hash = match hash {
                    Builtin::Ripemd160 => HashTy::Ripemd160,
                    Builtin::Sha256 => HashTy::Sha256,
                    Builtin::Keccak256 => HashTy::Keccak256,
                    Builtin::Blake2_128 => HashTy::Blake2_128,
                    Builtin::Blake2_256 => HashTy::Blake2_256,
                    _ => unreachable!(),
                };

                self.hash(
                    &contract,
                    hash,
                    contract.vector_bytes(v),
                    contract.vector_len(v),
                )
                .into()
            }
            Expression::Builtin(_, _, _, _) => self.builtin(contract, e, vartab, function),
            Expression::InternalFunctionCfg(cfg_no) => contract.functions[cfg_no]
                .as_global_value()
                .as_pointer_value()
                .into(),
            Expression::FormatString(_, args) => {
                self.format_string(contract, args, vartab, function)
            }
            _ => panic!("{:?} not implemented", e),
        }
    }

    /// Load a string from expression or create global
    fn string_location(
        &self,
        contract: &Contract<'a>,
        location: &StringLocation,
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        match location {
            StringLocation::CompileTime(literal) => (
                contract.emit_global_string("const_string", literal, true),
                contract
                    .context
                    .i32_type()
                    .const_int(literal.len() as u64, false),
            ),
            StringLocation::RunTime(e) => {
                let v = self.expression(contract, e, vartab, function);

                (contract.vector_bytes(v), contract.vector_len(v))
            }
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn emit_cfg(
        &mut self,
        contract: &mut Contract<'a>,
        cfg: &'a ControlFlowGraph,
        function: FunctionValue<'a>,
    ) {
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
            contract: &Contract<'a>,
            cfg: &ControlFlowGraph,
            function: FunctionValue,
        ) -> BasicBlock<'a> {
            let cfg_bb = &cfg.blocks[block_no];
            let mut phis = HashMap::new();

            let bb = contract.context.append_basic_block(function, &cfg_bb.name);

            contract.builder.position_at_end(bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    let ty = contract.llvm_var(&cfg.vars[v].ty);

                    phis.insert(*v, contract.builder.build_phi(ty, &cfg.vars[v].id.name));
                }
            }

            BasicBlock { bb, phis }
        }

        let mut work = VecDeque::new();

        blocks.insert(0, create_block(0, contract, cfg, function));

        // On Solana, the last argument is the accounts
        if contract.ns.target == Target::Solana {
            contract.parameters = Some(function.get_last_param().unwrap().into_pointer_value());
        }

        // Create all the stack variables
        let mut vars = HashMap::new();

        for (no, v) in &cfg.vars {
            match v.storage {
                Storage::Local if v.ty.is_reference_type() && !v.ty.is_contract_storage() => {
                    // a null pointer means an empty, zero'ed thing, be it string, struct or array
                    let value = contract
                        .llvm_type(&v.ty)
                        .ptr_type(AddressSpace::Generic)
                        .const_null()
                        .into();

                    vars.insert(*no, Variable { value });
                }
                Storage::Local if v.ty.is_contract_storage() => {
                    vars.insert(
                        *no,
                        Variable {
                            value: contract
                                .llvm_type(&contract.ns.storage_type())
                                .into_int_type()
                                .const_zero()
                                .into(),
                        },
                    );
                }
                Storage::Constant(_) | Storage::Contract(_) if v.ty.is_reference_type() => {
                    // This needs a placeholder
                    vars.insert(
                        *no,
                        Variable {
                            value: contract.context.bool_type().get_undef().into(),
                        },
                    );
                }
                Storage::Local | Storage::Contract(_) | Storage::Constant(_) => {
                    let ty = contract.llvm_type(&v.ty);
                    vars.insert(
                        *no,
                        Variable {
                            value: if ty.is_pointer_type() {
                                ty.into_pointer_type().const_zero().into()
                            } else {
                                ty.into_int_type().const_zero().into()
                            },
                        },
                    );
                }
            }
        }

        work.push_back(Work { block_no: 0, vars });

        while let Some(mut w) = work.pop_front() {
            let bb = blocks.get(&w.block_no).unwrap();

            contract.builder.position_at_end(bb.bb);

            for (v, phi) in bb.phis.iter() {
                w.vars.get_mut(v).unwrap().value = (*phi).as_basic_value();
            }

            for ins in &cfg.blocks[w.block_no].instr {
                match ins {
                    Instr::Nop => (),
                    Instr::Return { value } if value.is_empty() => {
                        contract
                            .builder
                            .build_return(Some(&contract.return_values[&ReturnCode::Success]));
                    }
                    Instr::Return { value } => {
                        let returns_offset = cfg.params.len();
                        for (i, val) in value.iter().enumerate() {
                            let arg = function.get_nth_param((returns_offset + i) as u32).unwrap();
                            let retval = self.expression(contract, val, &w.vars, function);

                            contract
                                .builder
                                .build_store(arg.into_pointer_value(), retval);
                        }
                        contract
                            .builder
                            .build_return(Some(&contract.return_values[&ReturnCode::Success]));
                    }
                    Instr::Set { res, expr, .. } => {
                        let value_ref = self.expression(contract, expr, &w.vars, function);

                        w.vars.get_mut(res).unwrap().value = value_ref;
                    }
                    Instr::Branch { block: dest } => {
                        let pos = contract.builder.get_insert_block().unwrap();

                        if !blocks.contains_key(&dest) {
                            blocks.insert(*dest, create_block(*dest, contract, cfg, function));
                            work.push_back(Work {
                                block_no: *dest,
                                vars: w.vars.clone(),
                            });
                        }

                        let bb = blocks.get(dest).unwrap();

                        for (v, phi) in bb.phis.iter() {
                            phi.add_incoming(&[(&w.vars[v].value, pos)]);
                        }

                        contract.builder.position_at_end(pos);
                        contract.builder.build_unconditional_branch(bb.bb);
                    }
                    Instr::Store { dest, pos } => {
                        let value_ref = w.vars[pos].value;
                        let dest_ref = self
                            .expression(contract, dest, &w.vars, function)
                            .into_pointer_value();

                        contract.builder.build_store(dest_ref, value_ref);
                    }
                    Instr::BranchCond {
                        cond,
                        true_block: true_,
                        false_block: false_,
                    } => {
                        let cond = self.expression(contract, cond, &w.vars, function);

                        let pos = contract.builder.get_insert_block().unwrap();

                        let bb_true = {
                            if !blocks.contains_key(&true_) {
                                blocks
                                    .insert(*true_, create_block(*true_, contract, cfg, function));
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
                            if !blocks.contains_key(&false_) {
                                blocks.insert(
                                    *false_,
                                    create_block(*false_, contract, cfg, function),
                                );
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

                        contract.builder.position_at_end(pos);
                        contract.builder.build_conditional_branch(
                            cond.into_int_value(),
                            bb_true,
                            bb_false,
                        );
                    }
                    Instr::LoadStorage { res, ty, storage } => {
                        let mut slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();

                        w.vars.get_mut(res).unwrap().value =
                            self.storage_load(contract, ty, &mut slot, function);
                    }
                    Instr::ClearStorage { ty, storage } => {
                        let mut slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();

                        self.storage_delete(contract, ty, &mut slot, function);
                    }
                    Instr::SetStorage { ty, value, storage } => {
                        let value = self.expression(contract, value, &w.vars, function);

                        let mut slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();

                        self.storage_store(contract, ty, &mut slot, value, function);
                    }
                    Instr::SetStorageBytes {
                        storage,
                        value,
                        offset,
                    } => {
                        let value = self.expression(contract, value, &w.vars, function);

                        let slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();
                        let offset = self
                            .expression(contract, offset, &w.vars, function)
                            .into_int_value();

                        self.set_storage_bytes_subscript(
                            contract,
                            function,
                            slot,
                            offset,
                            value.into_int_value(),
                        );
                    }
                    Instr::PushStorage {
                        res,
                        storage,
                        value,
                    } => {
                        let val = self.expression(contract, value, &w.vars, function);
                        let slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();

                        w.vars.get_mut(res).unwrap().value =
                            self.storage_push(&contract, function, &value.ty(), slot, val);
                    }
                    Instr::PopStorage { res, ty, storage } => {
                        let slot = self
                            .expression(contract, storage, &w.vars, function)
                            .into_int_value();

                        let value = self.storage_pop(&contract, function, ty, slot);

                        w.vars.get_mut(res).unwrap().value = value;
                    }
                    Instr::PushMemory {
                        res,
                        ty,
                        array,
                        value,
                    } => {
                        let a = w.vars[array].value.into_pointer_value();
                        let len = unsafe {
                            contract.builder.build_gep(
                                a,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "array_len",
                            )
                        };
                        let a = contract.builder.build_pointer_cast(
                            a,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "a",
                        );
                        let llvm_ty = contract.llvm_type(ty);

                        // Calculate total size for reallocation
                        let elem_ty = match ty {
                            ast::Type::Array(..) => match contract.llvm_type(&ty.array_elem()) {
                                elem @ BasicTypeEnum::StructType(_) => {
                                    // We don't store structs directly in the array, instead we store references to structs
                                    elem.ptr_type(AddressSpace::Generic).as_basic_type_enum()
                                }
                                elem => elem,
                            },
                            ast::Type::DynamicBytes => contract.context.i8_type().into(),
                            _ => unreachable!(),
                        };
                        let elem_size = elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false);
                        let len = contract
                            .builder
                            .build_load(len, "array_len")
                            .into_int_value();
                        let new_len = contract.builder.build_int_add(
                            len,
                            contract.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false);
                        let size = contract.builder.build_int_mul(elem_size, new_len, "");
                        let size = contract.builder.build_int_add(size, vec_size, "");

                        // Reallocate and reassign the array pointer
                        let new = contract
                            .builder
                            .build_call(
                                contract.module.get_function("__realloc").unwrap(),
                                &[a.into(), size.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        let dest = contract.builder.build_pointer_cast(
                            new,
                            llvm_ty.ptr_type(AddressSpace::Generic),
                            "dest",
                        );
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Store the value into the last element
                        let slot_ptr = unsafe {
                            contract.builder.build_gep(
                                dest,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(2, false),
                                    contract.builder.build_int_mul(len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let value = self.expression(contract, value, &w.vars, function);
                        let elem_ptr = contract.builder.build_pointer_cast(
                            slot_ptr,
                            elem_ty.ptr_type(AddressSpace::Generic),
                            "element pointer",
                        );
                        contract.builder.build_store(elem_ptr, value);
                        w.vars.get_mut(res).unwrap().value = value;

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            contract.builder.build_gep(
                                dest,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = contract.builder.build_pointer_cast(
                            len_ptr,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        contract.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            contract.builder.build_gep(
                                dest,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = contract.builder.build_pointer_cast(
                            size_ptr,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        contract.builder.build_store(size_field, new_len);
                    }
                    Instr::PopMemory { res, ty, array } => {
                        let a = w.vars[array].value.into_pointer_value();
                        let len = unsafe {
                            contract.builder.build_gep(
                                a,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "a_len",
                            )
                        };
                        let len = contract.builder.build_load(len, "a_len").into_int_value();

                        // First check if the array is empty
                        let is_array_empty = contract.builder.build_int_compare(
                            IntPredicate::EQ,
                            len,
                            contract.context.i32_type().const_zero(),
                            "is_array_empty",
                        );
                        let error = contract.context.append_basic_block(function, "error");
                        let pop = contract.context.append_basic_block(function, "pop");
                        contract
                            .builder
                            .build_conditional_branch(is_array_empty, error, pop);

                        contract.builder.position_at_end(error);
                        self.assert_failure(
                            contract,
                            contract
                                .context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            contract.context.i32_type().const_zero(),
                        );

                        contract.builder.position_at_end(pop);
                        let llvm_ty = contract.llvm_type(ty);

                        // Calculate total size for reallocation
                        let elem_ty = match ty {
                            ast::Type::Array(..) => match contract.llvm_type(&ty.array_elem()) {
                                elem @ BasicTypeEnum::StructType(_) => {
                                    // We don't store structs directly in the array, instead we store references to structs
                                    elem.ptr_type(AddressSpace::Generic).as_basic_type_enum()
                                }
                                elem => elem,
                            },
                            ast::Type::DynamicBytes => contract.context.i8_type().into(),
                            _ => unreachable!(),
                        };
                        let elem_size = elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false);
                        let new_len = contract.builder.build_int_sub(
                            len,
                            contract.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(contract.context.i32_type(), false);
                        let size = contract.builder.build_int_mul(elem_size, new_len, "");
                        let size = contract.builder.build_int_add(size, vec_size, "");

                        // Get the pointer to the last element and return it
                        let slot_ptr = unsafe {
                            contract.builder.build_gep(
                                a,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(2, false),
                                    contract.builder.build_int_mul(new_len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let slot_ptr = contract.builder.build_pointer_cast(
                            slot_ptr,
                            elem_ty.ptr_type(AddressSpace::Generic),
                            "slot_ptr",
                        );
                        let ret_val = contract.builder.build_load(slot_ptr, "");
                        w.vars.get_mut(res).unwrap().value = ret_val;

                        // Reallocate and reassign the array pointer
                        let a = contract.builder.build_pointer_cast(
                            a,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "a",
                        );
                        let new = contract
                            .builder
                            .build_call(
                                contract.module.get_function("__realloc").unwrap(),
                                &[a.into(), size.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        let dest = contract.builder.build_pointer_cast(
                            new,
                            llvm_ty.ptr_type(AddressSpace::Generic),
                            "dest",
                        );
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            contract.builder.build_gep(
                                dest,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = contract.builder.build_pointer_cast(
                            len_ptr,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        contract.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            contract.builder.build_gep(
                                dest,
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = contract.builder.build_pointer_cast(
                            size_ptr,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        contract.builder.build_store(size_field, new_len);
                    }
                    Instr::AssertFailure { expr: None } => {
                        self.assert_failure(
                            contract,
                            contract
                                .context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            contract.context.i32_type().const_zero(),
                        );
                    }
                    Instr::AssertFailure { expr: Some(expr) } => {
                        let v = self.expression(contract, expr, &w.vars, function);

                        let selector = 0x08c3_79a0u32;

                        let (data, len) = self.abi_encode(
                            contract,
                            Some(
                                contract
                                    .context
                                    .i32_type()
                                    .const_int(selector as u64, false),
                            ),
                            false,
                            function,
                            &[v],
                            &[ast::Type::String],
                        );

                        self.assert_failure(contract, data, len);
                    }
                    Instr::Print { expr } => {
                        let expr = self.expression(contract, expr, &w.vars, function);

                        self.print(
                            &contract,
                            contract.vector_bytes(expr),
                            contract.vector_len(expr),
                        );
                    }
                    Instr::Call {
                        res,
                        call: InternalCallTy::Static(cfg_no),
                        args,
                        ..
                    } => {
                        let f = &contract.contract.cfg[*cfg_no];

                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(contract, p, &w.vars, function))
                            .collect::<Vec<BasicValueEnum>>();

                        if !res.is_empty() {
                            for v in f.returns.iter() {
                                parms.push(
                                    contract
                                        .builder
                                        .build_alloca(contract.llvm_var(&v.ty), &v.name)
                                        .into(),
                                );
                            }
                        }

                        if let Some(parameters) = contract.parameters {
                            parms.push(parameters.into());
                        }

                        let ret = contract
                            .builder
                            .build_call(contract.functions[cfg_no], &parms, "")
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        let success = contract.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            contract.return_values[&ReturnCode::Success],
                            "success",
                        );

                        let success_block =
                            contract.context.append_basic_block(function, "success");
                        let bail_block = contract.context.append_basic_block(function, "bail");
                        contract.builder.build_conditional_branch(
                            success,
                            success_block,
                            bail_block,
                        );

                        contract.builder.position_at_end(bail_block);

                        contract.builder.build_return(Some(&ret));
                        contract.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, v) in f.returns.iter().enumerate() {
                                let val = contract.builder.build_load(
                                    parms[args.len() + i].into_pointer_value(),
                                    &v.name,
                                );

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value()
                                    && !(v.ty.is_reference_type()
                                        || matches!(v.ty, ast::Type::ExternalFunction { .. }))
                                {
                                    contract.builder.build_store(dest.into_pointer_value(), val);
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

                        let returns =
                            if let ast::Type::InternalFunction { returns, .. } = ty.deref_any() {
                                returns
                            } else {
                                panic!("should be Type::InternalFunction type");
                            };

                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(contract, p, &w.vars, function))
                            .collect::<Vec<BasicValueEnum>>();

                        // on Solana, we need to pass the accounts parameter around
                        if let Some(parameters) = contract.parameters {
                            parms.push(parameters.into());
                        }

                        if !res.is_empty() {
                            for ty in returns.iter() {
                                parms.push(
                                    contract
                                        .builder
                                        .build_alloca(contract.llvm_var(ty), "")
                                        .into(),
                                );
                            }
                        }

                        let ret = contract
                            .builder
                            .build_call(
                                self.expression(contract, call_expr, &w.vars, function)
                                    .into_pointer_value(),
                                &parms,
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        let success = contract.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            contract.context.i32_type().const_zero(),
                            "success",
                        );

                        let success_block =
                            contract.context.append_basic_block(function, "success");
                        let bail_block = contract.context.append_basic_block(function, "bail");
                        contract.builder.build_conditional_branch(
                            success,
                            success_block,
                            bail_block,
                        );

                        contract.builder.position_at_end(bail_block);

                        contract.builder.build_return(Some(&ret));
                        contract.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, ty) in returns.iter().enumerate() {
                                let val = contract
                                    .builder
                                    .build_load(parms[args.len() + i].into_pointer_value(), "");

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value() && !ty.is_reference_type() {
                                    contract.builder.build_store(dest.into_pointer_value(), val);
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
                    } => {
                        let args = &args
                            .iter()
                            .map(|a| self.expression(contract, &a, &w.vars, function))
                            .collect::<Vec<BasicValueEnum>>();

                        let address = contract
                            .builder
                            .build_alloca(contract.address_type(), "address");

                        let gas = self
                            .expression(contract, gas, &w.vars, function)
                            .into_int_value();
                        let value = value.as_ref().map(|v| {
                            self.expression(contract, &v, &w.vars, function)
                                .into_int_value()
                        });
                        let salt = salt.as_ref().map(|v| {
                            self.expression(contract, &v, &w.vars, function)
                                .into_int_value()
                        });

                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        self.create_contract(
                            &contract,
                            function,
                            success,
                            *contract_no,
                            *constructor_no,
                            contract.builder.build_pointer_cast(
                                address,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            args,
                            gas,
                            value,
                            salt,
                        );

                        w.vars.get_mut(res).unwrap().value =
                            contract.builder.build_load(address, "address");
                    }
                    Instr::ExternalCall {
                        success,
                        address,
                        payload,
                        value,
                        gas,
                        callty,
                    } => {
                        let gas = self
                            .expression(contract, gas, &w.vars, function)
                            .into_int_value();
                        let value = self
                            .expression(contract, value, &w.vars, function)
                            .into_int_value();
                        let payload = self.expression(contract, payload, &w.vars, function);

                        let address = if let Some(address) = address {
                            let address = self.expression(contract, address, &w.vars, function);

                            let addr = contract.builder.build_array_alloca(
                                contract.context.i8_type(),
                                contract
                                    .context
                                    .i32_type()
                                    .const_int(contract.ns.address_length as u64, false),
                                "address",
                            );

                            contract.builder.build_store(
                                contract.builder.build_pointer_cast(
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

                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        self.external_call(
                            contract,
                            function,
                            success,
                            contract.vector_bytes(payload),
                            contract.vector_len(payload),
                            address,
                            gas,
                            value,
                            callty.clone(),
                        );
                    }
                    Instr::ValueTransfer {
                        success,
                        address,
                        value,
                    } => {
                        let value = self
                            .expression(contract, value, &w.vars, function)
                            .into_int_value();
                        let address = self
                            .expression(contract, address, &w.vars, function)
                            .into_int_value();

                        let addr = contract.builder.build_array_alloca(
                            contract.context.i8_type(),
                            contract
                                .context
                                .i32_type()
                                .const_int(contract.ns.address_length as u64, false),
                            "address",
                        );

                        contract.builder.build_store(
                            contract.builder.build_pointer_cast(
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

                        self.value_transfer(contract, function, success, addr, value);
                    }
                    Instr::AbiDecode {
                        res,
                        selector,
                        exception_block: exception,
                        tys,
                        data,
                    } => {
                        let v = self.expression(contract, data, &w.vars, function);

                        let mut data = contract.vector_bytes(v);

                        let mut data_len = contract.vector_len(v);

                        if let Some(selector) = selector {
                            let exception = exception.unwrap();

                            let pos = contract.builder.get_insert_block().unwrap();

                            blocks.entry(exception).or_insert({
                                work.push_back(Work {
                                    block_no: exception,
                                    vars: w.vars.clone(),
                                });

                                create_block(exception, contract, cfg, function)
                            });

                            contract.builder.position_at_end(pos);

                            let exception_block = blocks.get(&exception).unwrap();

                            let has_selector = contract.builder.build_int_compare(
                                IntPredicate::UGT,
                                data_len,
                                contract.context.i32_type().const_int(4, false),
                                "has_selector",
                            );

                            let ok1 = contract.context.append_basic_block(function, "ok1");

                            contract.builder.build_conditional_branch(
                                has_selector,
                                ok1,
                                exception_block.bb,
                            );
                            contract.builder.position_at_end(ok1);

                            let selector_data = contract
                                .builder
                                .build_load(
                                    contract.builder.build_pointer_cast(
                                        data,
                                        contract.context.i32_type().ptr_type(AddressSpace::Generic),
                                        "selector",
                                    ),
                                    "selector",
                                )
                                .into_int_value();

                            let selector = if contract.ns.target == Target::Substrate {
                                *selector
                            } else {
                                selector.to_be()
                            };

                            let correct_selector = contract.builder.build_int_compare(
                                IntPredicate::EQ,
                                selector_data,
                                contract
                                    .context
                                    .i32_type()
                                    .const_int(selector as u64, false),
                                "correct_selector",
                            );

                            let ok2 = contract.context.append_basic_block(function, "ok2");

                            contract.builder.build_conditional_branch(
                                correct_selector,
                                ok2,
                                exception_block.bb,
                            );

                            contract.builder.position_at_end(ok2);

                            data_len = contract.builder.build_int_sub(
                                data_len,
                                contract.context.i32_type().const_int(4, false),
                                "data_len",
                            );

                            data = unsafe {
                                contract.builder.build_gep(
                                    contract.builder.build_pointer_cast(
                                        data,
                                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                        "data",
                                    ),
                                    &[contract.context.i32_type().const_int(4, false)],
                                    "data",
                                )
                            };
                        }

                        let mut returns = Vec::new();

                        self.abi_decode(contract, function, &mut returns, data, data_len, &tys);

                        for (i, ret) in returns.into_iter().enumerate() {
                            w.vars.get_mut(&res[i]).unwrap().value = ret;
                        }
                    }
                    Instr::Unreachable => {
                        contract.builder.build_unreachable();
                    }
                    Instr::SelfDestruct { recipient } => {
                        let recipient = self
                            .expression(contract, recipient, &w.vars, function)
                            .into_int_value();

                        self.selfdestruct(contract, recipient);
                    }
                    Instr::EmitEvent {
                        event_no,
                        data,
                        data_tys,
                        topics,
                        topic_tys,
                    } => {
                        let data_tys: Vec<ast::Type> =
                            data_tys.iter().map(|p| p.ty.clone()).collect();

                        let (data_ptr, data_len) = self.abi_encode(
                            contract,
                            self.event_id(contract, *event_no),
                            false,
                            function,
                            &data
                                .iter()
                                .map(|a| self.expression(contract, &a, &w.vars, function))
                                .collect::<Vec<BasicValueEnum>>(),
                            &data_tys,
                        );

                        let mut encoded = Vec::new();

                        for (i, topic) in topics.iter().enumerate() {
                            encoded.push(self.abi_encode(
                                contract,
                                None,
                                false,
                                function,
                                &[self.expression(contract, topic, &w.vars, function)],
                                &[topic_tys[i].ty.clone()],
                            ));
                        }

                        self.send_event(contract, *event_no, data_ptr, data_len, encoded);
                    }
                }
            }
        }
    }

    /// Create function dispatch based on abi encoded argsdata. The dispatcher loads the leading function selector,
    /// and dispatches based on that. If no function matches this, or no selector is in the argsdata, then fallback
    /// code is executed. This is either a fallback block provided to this function, or it automatically dispatches
    /// to the fallback function or receive function, if any.
    fn emit_function_dispatch<F>(
        &self,
        contract: &Contract<'a>,
        function_ty: pt::FunctionTy,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        fallback: Option<inkwell::basic_block::BasicBlock>,
        nonpayable: F,
    ) where
        F: Fn(&ControlFlowGraph) -> bool,
    {
        // create start function
        let no_function_matched = match fallback {
            Some(block) => block,
            None => contract
                .context
                .append_basic_block(function, "no_function_matched"),
        };

        let switch_block = contract.context.append_basic_block(function, "switch");

        let not_fallback = contract.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            argslen.get_type().const_int(4, false),
            "",
        );

        contract
            .builder
            .build_conditional_branch(not_fallback, switch_block, no_function_matched);

        contract.builder.position_at_end(switch_block);

        let fid = contract
            .builder
            .build_load(argsdata, "function_selector")
            .into_int_value();

        if contract.ns.target != Target::Solana {
            // TODO: solana does not support bss, so different solution is needed
            contract
                .builder
                .build_store(contract.selector.as_pointer_value(), fid);
        }

        // step over the function selector
        let argsdata = unsafe {
            contract.builder.build_gep(
                argsdata,
                &[contract.context.i32_type().const_int(1, false)],
                "argsdata",
            )
        };

        let argslen = contract.builder.build_int_sub(
            argslen,
            argslen.get_type().const_int(4, false),
            "argslen",
        );

        let mut cases = Vec::new();

        for (cfg_no, cfg) in contract.contract.cfg.iter().enumerate() {
            if cfg.ty != function_ty || !cfg.public {
                continue;
            }

            self.add_dispatch_case(
                contract,
                cfg,
                &mut cases,
                argsdata,
                argslen,
                function,
                contract.functions[&cfg_no],
                &nonpayable,
            );
        }

        contract.builder.position_at_end(switch_block);

        contract
            .builder
            .build_switch(fid, no_function_matched, &cases);

        if fallback.is_some() {
            return; // caller will generate fallback code
        }

        // emit fallback code
        contract.builder.position_at_end(no_function_matched);

        let fallback = contract
            .contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Fallback);

        let receive = contract
            .contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Receive);

        if fallback.is_none() && receive.is_none() {
            // no need to check value transferred; we will abort either way
            self.return_code(
                contract,
                contract.return_values[&ReturnCode::FunctionSelectorInvalid],
            );

            return;
        }

        let got_value = if contract.function_abort_value_transfers {
            contract.context.bool_type().const_zero()
        } else {
            let value = self.value_transferred(contract);

            contract.builder.build_int_compare(
                IntPredicate::NE,
                value,
                contract.value_type().const_zero(),
                "is_value_transfer",
            )
        };

        let fallback_block = contract.context.append_basic_block(function, "fallback");
        let receive_block = contract.context.append_basic_block(function, "receive");

        contract
            .builder
            .build_conditional_branch(got_value, receive_block, fallback_block);

        contract.builder.position_at_end(fallback_block);

        match fallback {
            Some((cfg_no, _)) => {
                contract
                    .builder
                    .build_call(contract.functions[&cfg_no], &[], "");

                self.return_empty_abi(contract);
            }
            None => {
                self.return_code(contract, contract.context.i32_type().const_int(2, false));
            }
        }

        contract.builder.position_at_end(receive_block);

        match receive {
            Some((cfg_no, _)) => {
                contract
                    .builder
                    .build_call(contract.functions[&cfg_no], &[], "");

                self.return_empty_abi(contract);
            }
            None => {
                self.return_code(contract, contract.context.i32_type().const_int(2, false));
            }
        }
    }

    ///Add single case for emit_function_dispatch
    fn add_dispatch_case<F>(
        &self,
        contract: &Contract<'a>,
        f: &ControlFlowGraph,
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
        let bb = contract.context.append_basic_block(function, "");

        contract.builder.position_at_end(bb);

        if nonpayable(f) {
            self.abort_if_value_transfer(contract, function);
        }

        let mut args = Vec::new();

        // insert abi decode
        self.abi_decode(&contract, function, &mut args, argsdata, argslen, &f.params);

        // add return values as pointer arguments at the end
        if !f.returns.is_empty() {
            for v in f.returns.iter() {
                args.push(if !v.ty.is_reference_type() {
                    contract
                        .build_alloca(function, contract.llvm_type(&v.ty), &v.name)
                        .into()
                } else {
                    contract
                        .build_alloca(
                            function,
                            contract.llvm_type(&v.ty).ptr_type(AddressSpace::Generic),
                            &v.name,
                        )
                        .into()
                });
            }
        }

        if contract.ns.target == Target::Solana {
            let params_ty = dest
                .get_type()
                .get_param_types()
                .last()
                .unwrap()
                .into_pointer_type();

            args.push(
                contract
                    .builder
                    .build_pointer_cast(
                        function.get_last_param().unwrap().into_pointer_value(),
                        params_ty,
                        "",
                    )
                    .into(),
            );
        }

        let ret = contract
            .builder
            .build_call(dest, &args, "")
            .try_as_basic_value()
            .left()
            .unwrap();

        let success = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret.into_int_value(),
            contract.return_values[&ReturnCode::Success],
            "success",
        );

        let success_block = contract.context.append_basic_block(function, "success");
        let bail_block = contract.context.append_basic_block(function, "bail");

        contract
            .builder
            .build_conditional_branch(success, success_block, bail_block);

        contract.builder.position_at_end(success_block);

        if f.returns.is_empty() {
            // return ABI of length 0
            self.return_empty_abi(&contract);
        } else {
            let tys: Vec<ast::Type> = f.returns.iter().map(|p| p.ty.clone()).collect();

            let (data, length) = self.abi_encode(
                &contract,
                None,
                true,
                function,
                &args[f.params.len()..f.params.len() + f.returns.len()],
                &tys,
            );

            self.return_abi(&contract, data, length);
        }

        contract.builder.position_at_end(bail_block);

        self.return_code(contract, ret.into_int_value());

        cases.push((
            contract
                .context
                .i32_type()
                .const_int(f.selector.to_be() as u64, false),
            bb,
        ));
    }

    /// Emit the contract storage initializers
    fn emit_initializer(&mut self, contract: &mut Contract<'a>) -> FunctionValue<'a> {
        let function_ty = contract.function_type(&[], &[]);

        let function = contract.module.add_function(
            "storage_initializers",
            function_ty,
            Some(Linkage::Internal),
        );

        let cfg = &contract.contract.cfg[contract.contract.initializer.unwrap()];

        self.emit_cfg(contract, cfg, function);

        function
    }

    /// Emit all functions, constructors, fallback and receiver
    fn emit_functions(&mut self, contract: &mut Contract<'a>) {
        let mut defines = Vec::new();

        for (cfg_no, cfg) in contract.contract.cfg.iter().enumerate() {
            if !cfg.is_placeholder() {
                let ftype = contract.function_type(
                    &cfg.params
                        .iter()
                        .map(|p| p.ty.clone())
                        .collect::<Vec<ast::Type>>(),
                    &cfg.returns
                        .iter()
                        .map(|p| p.ty.clone())
                        .collect::<Vec<ast::Type>>(),
                );

                let func_decl =
                    contract
                        .module
                        .add_function(&cfg.name, ftype, Some(Linkage::Internal));

                contract.functions.insert(cfg_no, func_decl);

                defines.push((func_decl, cfg));
            }
        }

        for (func_decl, cfg) in defines {
            self.emit_cfg(contract, cfg, func_decl);
        }
    }

    /// Implement "...{}...{}".format(a, b)
    fn format_string(
        &self,
        contract: &Contract<'a>,
        args: &[(FormatArg, Expression)],
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
    ) -> BasicValueEnum<'a> {
        // first we need to calculate the space we need
        let mut length = contract.context.i32_type().const_zero();

        let mut evaluated_arg = Vec::new();

        evaluated_arg.resize(args.len(), None);

        for (i, (spec, arg)) in args.iter().enumerate() {
            let len = if *spec == FormatArg::StringLiteral {
                if let Expression::BytesLiteral(_, _, bs) = arg {
                    contract
                        .context
                        .i32_type()
                        .const_int(bs.len() as u64, false)
                } else {
                    unreachable!();
                }
            } else {
                match arg.ty() {
                    // bool: "true" or "false"
                    ast::Type::Bool => contract.context.i32_type().const_int(5, false),
                    // hex encode bytes
                    ast::Type::Contract(_) | ast::Type::Address(_) => contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.address_length as u64 * 2, false),
                    ast::Type::Bytes(size) => contract
                        .context
                        .i32_type()
                        .const_int(size as u64 * 2, false),
                    ast::Type::String => {
                        let val = self.expression(contract, arg, vartab, function);

                        evaluated_arg[i] = Some(val);

                        contract.vector_len(val)
                    }
                    ast::Type::DynamicBytes => {
                        let val = self.expression(contract, arg, vartab, function);

                        evaluated_arg[i] = Some(val);

                        // will be hex encoded, so double
                        let len = contract.vector_len(val);

                        contract.builder.build_int_add(len, len, "hex_len")
                    }
                    ast::Type::Uint(bits) if *spec == FormatArg::Hex => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 / 8 + 2, false),
                    ast::Type::Int(bits) if *spec == FormatArg::Hex => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 / 8 + 3, false),
                    ast::Type::Uint(bits) if *spec == FormatArg::Binary => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 + 2, false),
                    ast::Type::Int(bits) if *spec == FormatArg::Binary => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 + 3, false),
                    // bits / 3 is a rough over-estimate of how many decimals we need
                    ast::Type::Uint(bits) if *spec == FormatArg::Default => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 / 3, false),
                    ast::Type::Int(bits) if *spec == FormatArg::Default => contract
                        .context
                        .i32_type()
                        .const_int(bits as u64 / 3 + 1, false),
                    ast::Type::Enum(enum_no) => contract.context.i32_type().const_int(
                        contract.ns.enums[enum_no].ty.bits(contract.ns) as u64 / 3,
                        false,
                    ),
                    _ => unimplemented!(),
                }
            };

            length = contract.builder.build_int_add(length, len, "");
        }

        // allocate the string and
        let vector = contract.vector_new(
            length,
            contract.context.i32_type().const_int(1, false),
            None,
        );

        let output_start = contract.vector_bytes(vector.into());

        // now encode each of the arguments
        let mut output = output_start;

        // format it
        for (i, (spec, arg)) in args.iter().enumerate() {
            if *spec == FormatArg::StringLiteral {
                if let Expression::BytesLiteral(_, _, bs) = arg {
                    let s = contract.emit_global_string("format_arg", &bs, true);
                    let len = contract
                        .context
                        .i32_type()
                        .const_int(bs.len() as u64, false);

                    contract.builder.build_call(
                        contract.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    );

                    output = unsafe { contract.builder.build_gep(output, &[len], "") };
                }
            } else {
                let val = evaluated_arg[i]
                    .unwrap_or_else(|| self.expression(contract, arg, vartab, function));
                let arg_ty = arg.ty();

                match arg_ty {
                    ast::Type::Bool => {
                        let len = contract
                            .builder
                            .build_select(
                                val.into_int_value(),
                                contract.context.i32_type().const_int(4, false),
                                contract.context.i32_type().const_int(5, false),
                                "bool_length",
                            )
                            .into_int_value();

                        let s = contract.builder.build_select(
                            val.into_int_value(),
                            contract.emit_global_string("bool_true", b"true", true),
                            contract.emit_global_string("bool_false", b"false", true),
                            "bool_value",
                        );

                        contract.builder.build_call(
                            contract.module.get_function("__memcpy").unwrap(),
                            &[output.into(), s, len.into()],
                            "",
                        );

                        output = unsafe { contract.builder.build_gep(output, &[len], "") };
                    }
                    ast::Type::String => {
                        let s = contract.vector_bytes(val);
                        let len = contract.vector_len(val);

                        contract.builder.build_call(
                            contract.module.get_function("__memcpy").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        output = unsafe { contract.builder.build_gep(output, &[len], "") };
                    }
                    ast::Type::DynamicBytes => {
                        let s = contract.vector_bytes(val);
                        let len = contract.vector_len(val);

                        contract.builder.build_call(
                            contract.module.get_function("hex_encode").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = contract.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { contract.builder.build_gep(output, &[hex_len], "") };
                    }
                    ast::Type::Address(_) | ast::Type::Contract(_) => {
                        let buf =
                            contract.build_alloca(function, contract.address_type(), "address");

                        contract.builder.build_store(buf, val.into_int_value());

                        let len = contract
                            .context
                            .i32_type()
                            .const_int(contract.ns.address_length as u64, false);

                        let s = contract.builder.build_pointer_cast(
                            buf,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "address_bytes",
                        );

                        contract.builder.build_call(
                            contract.module.get_function("hex_encode").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = contract.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { contract.builder.build_gep(output, &[hex_len], "") };
                    }
                    ast::Type::Bytes(size) => {
                        let buf =
                            contract.build_alloca(function, contract.llvm_type(&arg_ty), "bytesN");

                        contract.builder.build_store(buf, val.into_int_value());

                        let len = contract.context.i32_type().const_int(size as u64, false);

                        let s = contract.builder.build_pointer_cast(
                            buf,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "bytes",
                        );

                        contract.builder.build_call(
                            contract.module.get_function("hex_encode_rev").unwrap(),
                            &[output.into(), s.into(), len.into()],
                            "",
                        );

                        let hex_len = contract.builder.build_int_add(len, len, "hex_len");

                        output = unsafe { contract.builder.build_gep(output, &[hex_len], "") };
                    }
                    ast::Type::Enum(_) => {
                        let val = contract.builder.build_int_z_extend(
                            val.into_int_value(),
                            contract.context.i64_type(),
                            "val_64bits",
                        );

                        output = contract
                            .builder
                            .build_call(
                                contract.module.get_function("uint2dec").unwrap(),
                                &[output.into(), val.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                    ast::Type::Uint(bits) => {
                        if *spec == FormatArg::Default && bits <= 64 {
                            let val = if bits == 64 {
                                val.into_int_value()
                            } else {
                                contract.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    contract.context.i64_type(),
                                    "val_64bits",
                                )
                            };

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint2dec").unwrap(),
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
                                contract.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    contract.context.custom_width_int_type(128),
                                    "val_128bits",
                                )
                            };

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint128dec").unwrap(),
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
                                contract.builder.build_int_z_extend(
                                    val.into_int_value(),
                                    contract.context.custom_width_int_type(256),
                                    "val_256bits",
                                )
                            };

                            let pval = contract.build_alloca(
                                function,
                                contract.context.custom_width_int_type(256),
                                "int",
                            );

                            contract.builder.build_store(pval, val);

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint256dec").unwrap(),
                                    &[output.into(), pval.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else {
                            let buf = contract.build_alloca(
                                function,
                                contract.llvm_type(&arg_ty),
                                "uint",
                            );

                            contract.builder.build_store(buf, val.into_int_value());

                            let s = contract.builder.build_pointer_cast(
                                buf,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "uint",
                            );

                            let len = contract
                                .context
                                .i32_type()
                                .const_int(bits as u64 / 8, false);

                            let func_name = if *spec == FormatArg::Hex {
                                "uint2hex"
                            } else {
                                "uint2bin"
                            };

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function(&func_name).unwrap(),
                                    &[output.into(), s.into(), len.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        }
                    }
                    ast::Type::Int(bits) => {
                        let val = val.into_int_value();

                        let is_negative = contract.builder.build_int_compare(
                            IntPredicate::SLT,
                            val,
                            val.get_type().const_zero(),
                            "negative",
                        );

                        let entry = contract.builder.get_insert_block().unwrap();
                        let positive = contract
                            .context
                            .append_basic_block(function, "int_positive");
                        let negative = contract
                            .context
                            .append_basic_block(function, "int_negative");

                        contract
                            .builder
                            .build_conditional_branch(is_negative, negative, positive);

                        contract.builder.position_at_end(negative);

                        // add "-" to output and negate our val
                        contract.builder.build_store(
                            output,
                            contract.context.i8_type().const_int('-' as u64, false),
                        );

                        let minus_len = contract.context.i32_type().const_int(1, false);

                        let neg_data =
                            unsafe { contract.builder.build_gep(output, &[minus_len], "") };
                        let neg_val = contract.builder.build_int_neg(val, "negative_int");

                        contract.builder.build_unconditional_branch(positive);

                        contract.builder.position_at_end(positive);

                        let data_phi = contract.builder.build_phi(output.get_type(), "data");
                        let val_phi = contract.builder.build_phi(val.get_type(), "val");

                        data_phi.add_incoming(&[(&neg_data, negative), (&output, entry)]);
                        val_phi.add_incoming(&[(&neg_val, negative), (&val, entry)]);

                        if *spec == FormatArg::Default && bits <= 64 {
                            let val = if bits == 64 {
                                val_phi.as_basic_value().into_int_value()
                            } else {
                                contract.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    contract.context.i64_type(),
                                    "val_64bits",
                                )
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint2dec").unwrap(),
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
                                contract.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    contract.context.custom_width_int_type(128),
                                    "val_128bits",
                                )
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint128dec").unwrap(),
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
                                contract.builder.build_int_z_extend(
                                    val_phi.as_basic_value().into_int_value(),
                                    contract.context.custom_width_int_type(256),
                                    "val_256bits",
                                )
                            };

                            let pval = contract.build_alloca(
                                function,
                                contract.context.custom_width_int_type(256),
                                "int",
                            );

                            contract.builder.build_store(pval, val);

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function("uint256dec").unwrap(),
                                    &[output_after_minus.into(), pval.into()],
                                    "",
                                )
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        } else {
                            let buf =
                                contract.build_alloca(function, contract.llvm_type(&arg_ty), "int");

                            contract
                                .builder
                                .build_store(buf, val_phi.as_basic_value().into_int_value());

                            let s = contract.builder.build_pointer_cast(
                                buf,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "int",
                            );

                            let len = contract
                                .context
                                .i32_type()
                                .const_int(bits as u64 / 8, false);

                            let func_name = if *spec == FormatArg::Hex {
                                "uint2hex"
                            } else {
                                "uint2bin"
                            };

                            let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                            output = contract
                                .builder
                                .build_call(
                                    contract.module.get_function(&func_name).unwrap(),
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
        let length = contract.builder.build_int_sub(
            contract
                .builder
                .build_ptr_to_int(output, contract.context.i32_type(), "end"),
            contract
                .builder
                .build_ptr_to_int(output_start, contract.context.i32_type(), "begin"),
            "datalength",
        );

        let data_len = unsafe {
            contract.builder.build_gep(
                vector,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_zero(),
                ],
                "data_len",
            )
        };

        contract.builder.build_store(data_len, length);

        vector.into()
    }

    // emit a multiply for any width with or without overflow checking
    fn mul(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        left: IntValue<'a>,
        right: IntValue<'a>,
        signed: bool,
    ) -> IntValue<'a> {
        let bits = left.get_type().get_bit_width();

        if bits > 64 {
            // round up the number of bits to the next 32
            let mul_bits = (bits + 31) & !31;
            let mul_ty = contract.context.custom_width_int_type(mul_bits);

            // round up bits
            let l = contract.build_alloca(function, mul_ty, "");
            let r = contract.build_alloca(function, mul_ty, "");
            let o = contract.build_alloca(function, mul_ty, "");

            if mul_bits == bits {
                contract.builder.build_store(l, left);
                contract.builder.build_store(r, right);
            } else if signed {
                contract
                    .builder
                    .build_store(l, contract.builder.build_int_s_extend(left, mul_ty, ""));
                contract
                    .builder
                    .build_store(r, contract.builder.build_int_s_extend(right, mul_ty, ""));
            } else {
                contract
                    .builder
                    .build_store(l, contract.builder.build_int_z_extend(left, mul_ty, ""));
                contract
                    .builder
                    .build_store(r, contract.builder.build_int_z_extend(right, mul_ty, ""));
            }

            contract.builder.build_call(
                contract.module.get_function("__mul32").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            l,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            r,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            o,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    contract
                        .context
                        .i32_type()
                        .const_int(mul_bits as u64 / 32, false)
                        .into(),
                ],
                "",
            );

            let res = contract.builder.build_load(o, "mul");

            if mul_bits == bits {
                res.into_int_value()
            } else {
                contract
                    .builder
                    .build_int_truncate(res.into_int_value(), left.get_type(), "")
            }
        } else if contract.math_overflow_check {
            self.build_binary_op_with_overflow_check(
                contract,
                function,
                left,
                right,
                BinaryOp::Multiply,
                signed,
            )
        } else {
            contract.builder.build_int_mul(left, right, "")
        }
    }

    fn power(&self, contract: &Contract<'a>, bits: u32, signed: bool) -> FunctionValue<'a> {
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
        let name = format!("__{}power{}", if signed { 's' } else { 'u' }, bits);
        let ty = contract.context.custom_width_int_type(bits);

        if let Some(f) = contract.module.get_function(&name) {
            return f;
        }

        let pos = contract.builder.get_insert_block().unwrap();

        // __upower(base, exp)
        let function =
            contract
                .module
                .add_function(&name, ty.fn_type(&[ty.into(), ty.into()], false), None);

        let entry = contract.context.append_basic_block(function, "entry");
        let loop_block = contract.context.append_basic_block(function, "loop");
        let multiply = contract.context.append_basic_block(function, "multiply");
        let nomultiply = contract.context.append_basic_block(function, "nomultiply");
        let done = contract.context.append_basic_block(function, "done");
        let notdone = contract.context.append_basic_block(function, "notdone");

        contract.builder.position_at_end(entry);

        contract.builder.build_unconditional_branch(loop_block);

        contract.builder.position_at_end(loop_block);
        let base = contract.builder.build_phi(ty, "base");
        base.add_incoming(&[(&function.get_nth_param(0).unwrap(), entry)]);

        let exp = contract.builder.build_phi(ty, "exp");
        exp.add_incoming(&[(&function.get_nth_param(1).unwrap(), entry)]);

        let result = contract.builder.build_phi(ty, "result");
        result.add_incoming(&[(&ty.const_int(1, false), entry)]);

        let lowbit = contract.builder.build_int_truncate(
            exp.as_basic_value().into_int_value(),
            contract.context.bool_type(),
            "bit",
        );

        contract
            .builder
            .build_conditional_branch(lowbit, multiply, nomultiply);

        contract.builder.position_at_end(multiply);

        let result2 = self.mul(
            contract,
            function,
            result.as_basic_value().into_int_value(),
            base.as_basic_value().into_int_value(),
            signed,
        );

        contract.builder.build_unconditional_branch(nomultiply);
        contract.builder.position_at_end(nomultiply);

        let result3 = contract.builder.build_phi(ty, "result");
        result3.add_incoming(&[(&result.as_basic_value(), loop_block), (&result2, multiply)]);

        let exp2 = contract.builder.build_right_shift(
            exp.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "exp",
        );
        let zero =
            contract
                .builder
                .build_int_compare(IntPredicate::EQ, exp2, ty.const_zero(), "zero");

        contract
            .builder
            .build_conditional_branch(zero, done, notdone);

        contract.builder.position_at_end(done);

        contract
            .builder
            .build_return(Some(&result3.as_basic_value()));

        contract.builder.position_at_end(notdone);

        let base2 = self.mul(
            contract,
            function,
            base.as_basic_value().into_int_value(),
            base.as_basic_value().into_int_value(),
            signed,
        );

        base.add_incoming(&[(&base2, notdone)]);
        result.add_incoming(&[(&result3.as_basic_value(), notdone)]);
        exp.add_incoming(&[(&exp2, notdone)]);

        contract.builder.build_unconditional_branch(loop_block);

        contract.builder.position_at_end(pos);

        function
    }

    /// Convenience function for generating binary operations with overflow checking.
    fn build_binary_op_with_overflow_check(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        left: IntValue<'a>,
        right: IntValue<'a>,
        op: BinaryOp,
        signed: bool,
    ) -> IntValue<'a> {
        let ret_ty = contract.context.struct_type(
            &[
                left.get_type().into(),
                contract.context.custom_width_int_type(1).into(),
            ],
            false,
        );
        let binop = contract.llvm_overflow(ret_ty.into(), left.get_type(), signed, op);

        let op_res = contract
            .builder
            .build_call(binop, &[left.into(), right.into()], "res")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_struct_value();

        let overflow = contract
            .builder
            .build_extract_value(op_res, 1, "overflow")
            .unwrap()
            .into_int_value();

        let success_block = contract.context.append_basic_block(function, "success");
        let error_block = contract.context.append_basic_block(function, "error");

        contract
            .builder
            .build_conditional_branch(overflow, error_block, success_block);

        contract.builder.position_at_end(error_block);

        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );

        contract.builder.position_at_end(success_block);

        contract
            .builder
            .build_extract_value(op_res, 0, "res")
            .unwrap()
            .into_int_value()
    }
}
pub struct Contract<'a> {
    pub name: String,
    pub module: Module<'a>,
    pub runtime: Option<Box<Contract<'a>>>,
    function_abort_value_transfers: bool,
    constructor_abort_value_transfers: bool,
    math_overflow_check: bool,
    builder: Builder<'a>,
    context: &'a Context,
    contract: &'a ast::Contract,
    ns: &'a ast::Namespace,
    functions: HashMap<usize, FunctionValue<'a>>,
    code: RefCell<Vec<u8>>,
    opt: OptimizationLevel,
    code_size: RefCell<Option<IntValue<'a>>>,
    selector: GlobalValue<'a>,
    calldata_data: GlobalValue<'a>,
    calldata_len: GlobalValue<'a>,
    scratch_len: Option<GlobalValue<'a>>,
    scratch: Option<GlobalValue<'a>>,
    parameters: Option<PointerValue<'a>>,
    return_values: HashMap<ReturnCode, IntValue<'a>>,
}

#[derive(PartialEq, Eq, Hash)]
enum ReturnCode {
    Success,
    FunctionSelectorInvalid,
    AbiEncodingInvalid,
}

impl<'a> Contract<'a> {
    /// Build the LLVM IR for a contract
    pub fn build(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Self {
        match ns.target {
            Target::Substrate => substrate::SubstrateTarget::build(
                context,
                contract,
                ns,
                filename,
                opt,
                math_overflow_check,
            ),
            Target::Ewasm => {
                ewasm::EwasmTarget::build(context, contract, ns, filename, opt, math_overflow_check)
            }
            Target::Sabre => {
                sabre::SabreTarget::build(context, contract, ns, filename, opt, math_overflow_check)
            }
            Target::Generic => generic::GenericTarget::build(
                context,
                contract,
                ns,
                filename,
                opt,
                math_overflow_check,
            ),
            Target::Solana => solana::SolanaTarget::build(
                context,
                contract,
                ns,
                filename,
                opt,
                math_overflow_check,
            ),
        }
    }

    /// Compile the contract and return the code as bytes. The result is
    /// cached, since this function can be called multiple times (e.g. one for
    /// each time a contract of this type is created).
    /// Pass our module to llvm for optimization and compilation
    pub fn code(&self, linking: bool) -> Result<Vec<u8>, String> {
        // return cached result if available
        if !self.code.borrow().is_empty() {
            return Ok(self.code.borrow().clone());
        }

        match self.opt {
            OptimizationLevel::Default | OptimizationLevel::Aggressive => {
                let pass_manager = PassManager::create(());

                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_function_inlining_pass();
                pass_manager.add_global_dce_pass();
                pass_manager.add_constant_merge_pass();

                pass_manager.run_on(&self.module);
            }
            _ => {}
        }

        let target =
            inkwell::targets::Target::from_name(self.ns.target.llvm_target_name()).unwrap();

        let target_machine = target
            .create_target_machine(
                &self.ns.target.llvm_target_triple(),
                "",
                self.ns.target.llvm_features(),
                self.opt,
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap();

        loop {
            // we need to loop here to support ewasm deployer. It needs to know the size
            // of itself. Note that in webassembly, the constants are LEB128 encoded so
            // patching the length might actually change the length. So we need to loop
            // until it is right.

            // The correct solution is to make ewasm less insane.
            match target_machine.write_to_memory_buffer(&self.module, FileType::Object) {
                Ok(out) => {
                    let slice = out.as_slice();

                    if linking {
                        let bs = link(slice, &self.contract.name, self.ns.target);

                        if !self.patch_code_size(bs.len() as u64) {
                            self.code.replace(bs.to_vec());

                            return Ok(bs.to_vec());
                        }
                    } else {
                        self.code.replace(slice.to_vec());

                        return Ok(slice.to_vec());
                    }
                }
                Err(s) => {
                    return Err(s.to_string());
                }
            }
        }
    }

    /// Mark all functions as internal unless they're in the export_list. This helps the
    /// llvm globaldce pass eliminate unnecessary functions and reduce the wasm output.
    fn internalize(&self, export_list: &[&str]) {
        let mut func = self.module.get_first_function();

        // FIXME: these functions are called from code generated by lowering into wasm,
        // so eliminating them now will cause link errors. Either we should prevent these
        // calls from being done in the first place or do dce at link time
        let mut export_list = export_list.to_vec();
        export_list.push("__ashlti3");
        export_list.push("__lshrti3");
        export_list.push("__ashrti3");

        while let Some(f) = func {
            let name = f.get_name().to_str().unwrap();

            if !name.starts_with("llvm.") && export_list.iter().all(|e| e != &name) {
                f.set_linkage(Linkage::Internal);
            }

            func = f.get_next_function();
        }
    }

    pub fn bitcode(&self, path: &Path) {
        self.module.write_bitcode_to_path(path);
    }

    pub fn dump_llvm(&self, path: &Path) -> Result<(), String> {
        if let Err(s) = self.module.print_to_file(path) {
            return Err(s.to_string());
        }

        Ok(())
    }

    pub fn new(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
        runtime: Option<Box<Contract<'a>>>,
    ) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let triple = ns.target.llvm_target_triple();
        let module = context.create_module(&contract.name);

        module.set_triple(&triple);
        module.set_source_file_name(filename);

        // stdlib
        let intr = load_stdlib(&context, &ns.target);
        module.link_in_module(intr).unwrap();

        // if there is no payable function, fallback or receive then abort all value transfers at the top
        // note that receive() is always payable so this just checkes for presence.
        let function_abort_value_transfers = !contract.functions.iter().any(|function_no| {
            let f = &ns.functions[*function_no];
            !f.is_constructor() && f.is_payable()
        });

        let constructor_abort_value_transfers = !contract.functions.iter().any(|function_no| {
            let f = &ns.functions[*function_no];
            f.is_constructor() && f.is_payable()
        });

        let selector =
            module.add_global(context.i32_type(), Some(AddressSpace::Generic), "selector");
        selector.set_linkage(Linkage::Internal);
        selector.set_initializer(&context.i32_type().const_zero());

        let calldata_len = module.add_global(
            context.i32_type(),
            Some(AddressSpace::Generic),
            "calldata_len",
        );
        calldata_len.set_linkage(Linkage::Internal);
        calldata_len.set_initializer(&context.i32_type().const_zero());

        let calldata_data = module.add_global(
            context.i8_type().ptr_type(AddressSpace::Generic),
            Some(AddressSpace::Generic),
            "calldata_data",
        );
        calldata_data.set_linkage(Linkage::Internal);
        calldata_data.set_initializer(
            &context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_zero(),
        );

        let mut return_values = HashMap::new();

        return_values.insert(ReturnCode::Success, context.i32_type().const_zero());
        return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i32_type().const_int(3, false),
        );
        return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i32_type().const_int(2, false),
        );

        Contract {
            name: contract.name.to_owned(),
            module,
            runtime,
            function_abort_value_transfers,
            constructor_abort_value_transfers,
            math_overflow_check,
            builder: context.create_builder(),
            context,
            contract,
            ns,
            functions: HashMap::new(),
            code: RefCell::new(Vec::new()),
            opt,
            code_size: RefCell::new(None),
            selector,
            calldata_data,
            calldata_len,
            scratch: None,
            scratch_len: None,
            parameters: None,
            return_values,
        }
    }

    /// llvm value type, as in chain currency (usually 128 bits int)
    fn value_type(&self) -> IntType<'a> {
        self.context
            .custom_width_int_type(self.ns.value_length as u32 * 8)
    }

    /// llvm address type
    fn address_type(&self) -> IntType<'a> {
        self.context
            .custom_width_int_type(self.ns.address_length as u32 * 8)
    }

    /// Creates global string in the llvm module with initializer
    ///
    fn emit_global_string(&self, name: &str, data: &[u8], constant: bool) -> PointerValue<'a> {
        let ty = self.context.i8_type().array_type(data.len() as u32);

        let gv = self
            .module
            .add_global(ty, Some(AddressSpace::Generic), name);

        gv.set_linkage(Linkage::Internal);

        gv.set_initializer(&self.context.const_string(data, false));

        if constant {
            gv.set_constant(true);
            gv.set_unnamed_addr(true);
        }

        self.builder.build_pointer_cast(
            gv.as_pointer_value(),
            self.context.i8_type().ptr_type(AddressSpace::Generic),
            name,
        )
    }

    /// Wrapper for alloca. Ensures that the alloca is done on the first basic block.
    /// If alloca is not on the first basic block, llvm will get to llvm_unreachable
    /// for the BPF target.
    fn build_alloca<T: BasicType<'a>>(
        &self,
        function: inkwell::values::FunctionValue<'a>,
        ty: T,
        name: &str,
    ) -> PointerValue<'a> {
        let entry = function
            .get_first_basic_block()
            .expect("function missing entry block");
        let current = self.builder.get_insert_block().unwrap();

        if let Some(instr) = entry.get_first_instruction() {
            self.builder.position_before(&instr);
        } else {
            self.builder.position_at_end(entry);
        }

        let res = self.builder.build_alloca(ty, name);

        self.builder.position_at_end(current);

        res
    }

    /// Emit a loop from `from` to `to`. The closure exists to insert the body of the loop; the closure
    /// gets the loop variable passed to it as an IntValue, and a userdata PointerValue
    pub fn emit_static_loop_with_pointer<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut PointerValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut PointerValue<'a>),
    {
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(body);
        self.builder.position_at_end(body);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_pointer_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        // add loop body
        insert_body(loop_var, &mut data);

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, next, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        let body = self.builder.get_insert_block().unwrap();
        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.position_at_end(done);

        *data_ref = data;
    }

    /// Emit a loop from `from` to `to`. The closure exists to insert the body of the loop; the closure
    /// gets the loop variable passed to it as an IntValue, and a userdata IntValue
    pub fn emit_static_loop_with_int<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut IntValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut IntValue<'a>),
    {
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(body);
        self.builder.position_at_end(body);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_int_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        // add loop body
        insert_body(loop_var, &mut data);

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, next, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        let body = self.builder.get_insert_block().unwrap();
        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.position_at_end(done);

        *data_ref = data;
    }

    /// Emit a loop from `from` to `to`, checking the condition _before_ the body.
    pub fn emit_loop_cond_first_with_int<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut IntValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut IntValue<'a>),
    {
        let cond = self.context.append_basic_block(function, "cond");
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(cond);
        self.builder.position_at_end(cond);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_int_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, loop_var, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        self.builder.position_at_end(body);
        // add loop body
        insert_body(loop_var, &mut data);

        let body = self.builder.get_insert_block().unwrap();

        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.build_unconditional_branch(cond);

        self.builder.position_at_end(done);

        *data_ref = data_phi.as_basic_value().into_int_value();
    }

    /// Emit a loop from `from` to `to`, checking the condition _before_ the body.
    pub fn emit_loop_cond_first_with_pointer<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut PointerValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut PointerValue<'a>),
    {
        let cond = self.context.append_basic_block(function, "cond");
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(cond);
        self.builder.position_at_end(cond);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_pointer_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, loop_var, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        self.builder.position_at_end(body);
        // add loop body
        insert_body(loop_var, &mut data);

        let body = self.builder.get_insert_block().unwrap();

        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.build_unconditional_branch(cond);

        self.builder.position_at_end(done);

        *data_ref = data_phi.as_basic_value().into_pointer_value();
    }

    /// Convert a BigInt number to llvm const value
    fn number_literal(&self, bits: u32, n: &BigInt) -> IntValue<'a> {
        let ty = self.context.custom_width_int_type(bits);
        let s = n.to_string();

        ty.const_int_from_string(&s, StringRadix::Decimal).unwrap()
    }

    /// Emit function prototype
    fn function_type(&self, params: &[ast::Type], returns: &[ast::Type]) -> FunctionType<'a> {
        // function parameters
        let mut args = params
            .iter()
            .map(|ty| self.llvm_var(&ty))
            .collect::<Vec<BasicTypeEnum>>();

        // add return values
        for ty in returns {
            args.push(if ty.is_reference_type() && !ty.is_contract_storage() {
                self.llvm_type(&ty)
                    .ptr_type(AddressSpace::Generic)
                    .ptr_type(AddressSpace::Generic)
                    .into()
            } else {
                self.llvm_type(&ty).ptr_type(AddressSpace::Generic).into()
            });
        }

        // On Solana, we need to pass around the accounts
        if self.ns.target == Target::Solana {
            args.push(
                self.module
                    .get_struct_type("struct.SolParameters")
                    .unwrap()
                    .ptr_type(AddressSpace::Generic)
                    .as_basic_type_enum(),
            );
        }

        // Solana return type should be 64 bit, 32 bit on wasm
        self.return_values[&ReturnCode::Success]
            .get_type()
            .fn_type(&args, false)
    }

    // Create the llvm intrinsic for counting leading zeros
    pub fn llvm_ctlz(&self, bit: u32) -> FunctionValue<'a> {
        let name = format!("llvm.ctlz.i{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module.add_function(
            &name,
            ty.fn_type(&[ty.into(), self.context.bool_type().into()], false),
            None,
        )
    }

    // Create the llvm intrinsic for bswap
    pub fn llvm_bswap(&self, bit: u32) -> FunctionValue<'a> {
        let name = format!("llvm.bswap.i{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module
            .add_function(&name, ty.fn_type(&[ty.into()], false), None)
    }

    // Create the llvm intrinsic for overflows
    pub fn llvm_overflow(
        &self,
        ret_ty: BasicTypeEnum<'a>,
        ty: IntType<'a>,
        signed: bool,
        op: BinaryOp,
    ) -> FunctionValue<'a> {
        let bit = ty.get_bit_width();
        let name = format!(
            "llvm.{}{}.with.overflow.i{}",
            if signed { "s" } else { "u" },
            op,
            bit,
        );

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module
            .add_function(&name, ret_ty.fn_type(&[ty.into(), ty.into()], false), None)
    }

    /// Return the llvm type for a variable holding the type, not the type itself
    fn llvm_var(&self, ty: &ast::Type) -> BasicTypeEnum<'a> {
        let llvm_ty = self.llvm_type(ty);
        match ty.deref_memory() {
            ast::Type::Struct(_)
            | ast::Type::Array(_, _)
            | ast::Type::DynamicBytes
            | ast::Type::String => llvm_ty.ptr_type(AddressSpace::Generic).as_basic_type_enum(),
            _ => llvm_ty,
        }
    }

    /// Default empty value
    fn default_value(&self, ty: &ast::Type) -> BasicValueEnum<'a> {
        let llvm_ty = self.llvm_var(ty);

        // const_zero() on BasicTypeEnum yet. Should be coming to inkwell soon
        if llvm_ty.is_pointer_type() {
            llvm_ty.into_pointer_type().const_null().into()
        } else {
            llvm_ty.into_int_type().const_zero().into()
        }
    }

    /// Return the llvm type for the resolved type.
    fn llvm_type(&self, ty: &ast::Type) -> BasicTypeEnum<'a> {
        match ty {
            ast::Type::Bool => BasicTypeEnum::IntType(self.context.bool_type()),
            ast::Type::Int(n) | ast::Type::Uint(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32))
            }
            ast::Type::Value => BasicTypeEnum::IntType(
                self.context
                    .custom_width_int_type(self.ns.value_length as u32 * 8),
            ),
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                BasicTypeEnum::IntType(self.address_type())
            }
            ast::Type::Bytes(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32 * 8))
            }
            ast::Type::Enum(n) => self.llvm_type(&self.ns.enums[*n].ty),
            ast::Type::String | ast::Type::DynamicBytes => {
                self.module.get_struct_type("struct.vector").unwrap().into()
            }
            ast::Type::Array(base_ty, dims) => {
                let ty = self.llvm_var(base_ty);

                let mut dims = dims.iter();

                let mut aty = match dims.next().unwrap() {
                    Some(d) => ty.array_type(d.to_u32().unwrap()),
                    None => return self.module.get_struct_type("struct.vector").unwrap().into(),
                };

                for dim in dims {
                    match dim {
                        Some(d) => aty = aty.array_type(d.to_u32().unwrap()),
                        None => {
                            return self.module.get_struct_type("struct.vector").unwrap().into()
                        }
                    }
                }

                BasicTypeEnum::ArrayType(aty)
            }
            ast::Type::Struct(n) => self
                .context
                .struct_type(
                    &self.ns.structs[*n]
                        .fields
                        .iter()
                        .map(|f| self.llvm_var(&f.ty))
                        .collect::<Vec<BasicTypeEnum>>(),
                    false,
                )
                .as_basic_type_enum(),
            ast::Type::Mapping(_, _) => unreachable!(),
            ast::Type::Ref(r) => self
                .llvm_type(r)
                .ptr_type(AddressSpace::Generic)
                .as_basic_type_enum(),
            ast::Type::StorageRef(_) => self.llvm_type(&self.ns.storage_type()),
            ast::Type::InternalFunction {
                params, returns, ..
            } => {
                let ftype = self.function_type(params, returns);

                BasicTypeEnum::PointerType(ftype.ptr_type(AddressSpace::Generic))
            }
            ast::Type::ExternalFunction { .. } => {
                let address = self.llvm_type(&ast::Type::Address(false));
                let selector = self.llvm_type(&ast::Type::Uint(32));

                BasicTypeEnum::PointerType(
                    self.context
                        .struct_type(&[address, selector], false)
                        .ptr_type(AddressSpace::Generic),
                )
            }
            ast::Type::Slice => BasicTypeEnum::StructType(
                self.context.struct_type(
                    &[
                        self.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(),
                        self.context.i32_type().into(),
                    ],
                    false,
                ),
            ),
            _ => unreachable!(),
        }
    }

    /// ewasm deployer needs to know what its own code size is, so we compile once to
    /// get the size, patch in the value and then recompile.
    fn patch_code_size(&self, code_size: u64) -> bool {
        let current_size = {
            let current_size_opt = self.code_size.borrow();

            if let Some(current_size) = *current_size_opt {
                if code_size == current_size.get_zero_extended_constant().unwrap() {
                    return false;
                }

                current_size
            } else {
                return false;
            }
        };

        let new_size = self.context.i32_type().const_int(code_size, false);

        current_size.replace_all_uses_with(new_size);

        self.code_size.replace(Some(new_size));

        true
    }

    /// Allocate vector
    fn vector_new(
        &self,
        size: IntValue<'a>,
        elem_size: IntValue<'a>,
        init: Option<&Vec<u8>>,
    ) -> PointerValue<'a> {
        if let Some(init) = init {
            if init.is_empty() {
                return self
                    .module
                    .get_struct_type("struct.vector")
                    .unwrap()
                    .ptr_type(AddressSpace::Generic)
                    .const_null();
            }
        }

        let init = match init {
            None => self.builder.build_int_to_ptr(
                self.context.i32_type().const_all_ones(),
                self.context.i8_type().ptr_type(AddressSpace::Generic),
                "invalid",
            ),
            Some(s) => self.emit_global_string("const_string", s, true),
        };

        let v = self
            .builder
            .build_call(
                self.module.get_function("vector_new").unwrap(),
                &[size.into(), elem_size.into(), init.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        self.builder.build_pointer_cast(
            v.into_pointer_value(),
            self.module
                .get_struct_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "vector",
        )
    }

    /// Number of element in a vector
    fn vector_len(&self, vector: BasicValueEnum<'a>) -> IntValue<'a> {
        if vector.is_struct_value() {
            // slice
            let slice = vector.into_struct_value();

            self.builder
                .build_extract_value(slice, 1, "slice_len")
                .unwrap()
                .into_int_value()
        } else {
            let struct_ty = vector
                .into_pointer_value()
                .get_type()
                .get_element_type()
                .into_struct_type();
            let name = struct_ty.get_name().unwrap();

            if name == CStr::from_bytes_with_nul(b"struct.SolAccountInfo\0").unwrap() {
                // load the data pointer
                let data = self
                    .builder
                    .build_load(
                        self.builder
                            .build_struct_gep(vector.into_pointer_value(), 3, "data")
                            .unwrap(),
                        "data",
                    )
                    .into_pointer_value();

                // get the offset of the return data
                let header_ptr = self.builder.build_pointer_cast(
                    data,
                    self.context.i32_type().ptr_type(AddressSpace::Generic),
                    "header_ptr",
                );

                let data_len_ptr = unsafe {
                    self.builder.build_gep(
                        header_ptr,
                        &[self.context.i64_type().const_int(1, false)],
                        "data_len_ptr",
                    )
                };

                self.builder
                    .build_load(data_len_ptr, "len")
                    .into_int_value()
            } else {
                // field 0 is the length
                let vector = vector.into_pointer_value();

                let len = unsafe {
                    self.builder.build_gep(
                        vector,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_zero(),
                        ],
                        "vector_len",
                    )
                };

                self.builder
                    .build_select(
                        self.builder.build_is_null(vector, "vector_is_null"),
                        self.context.i32_type().const_zero(),
                        self.builder.build_load(len, "vector_len").into_int_value(),
                        "length",
                    )
                    .into_int_value()
            }
        }
    }

    /// Return the pointer to the actual bytes in the vector
    fn vector_bytes(&self, vector: BasicValueEnum<'a>) -> PointerValue<'a> {
        if vector.is_struct_value() {
            // slice
            let slice = vector.into_struct_value();

            self.builder
                .build_extract_value(slice, 0, "slice_data")
                .unwrap()
                .into_pointer_value()
        } else {
            // Get the type name of the struct we are point to
            let struct_ty = vector
                .into_pointer_value()
                .get_type()
                .get_element_type()
                .into_struct_type();
            let name = struct_ty.get_name().unwrap();

            if name == CStr::from_bytes_with_nul(b"struct.SolAccountInfo\0").unwrap() {
                // load the data pointer
                let data = self
                    .builder
                    .build_load(
                        self.builder
                            .build_struct_gep(vector.into_pointer_value(), 3, "data")
                            .unwrap(),
                        "data",
                    )
                    .into_pointer_value();

                // get the offset of the return data
                let header_ptr = self.builder.build_pointer_cast(
                    data,
                    self.context.i32_type().ptr_type(AddressSpace::Generic),
                    "header_ptr",
                );

                let data_ptr = unsafe {
                    self.builder.build_gep(
                        header_ptr,
                        &[self.context.i64_type().const_int(2, false)],
                        "data_ptr",
                    )
                };

                let offset = self.builder.build_load(data_ptr, "offset").into_int_value();

                let v = unsafe { self.builder.build_gep(data, &[offset], "data") };

                self.builder.build_pointer_cast(
                    v,
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "data",
                )
            } else {
                let data = unsafe {
                    self.builder.build_gep(
                        vector.into_pointer_value(),
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_int(2, false),
                        ],
                        "data",
                    )
                };

                self.builder.build_pointer_cast(
                    data,
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "data",
                )
            }
        }
    }

    /// Dereference an array
    fn array_subscript(
        &self,
        array_ty: &ast::Type,
        array: PointerValue<'a>,
        index: IntValue<'a>,
    ) -> PointerValue<'a> {
        match array_ty {
            ast::Type::Array(_, dim) => {
                if dim[0].is_some() {
                    // fixed size array
                    unsafe {
                        self.builder.build_gep(
                            array,
                            &[self.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                    }
                } else {
                    let elem_ty = array_ty.array_deref();
                    let llvm_elem_ty = self.llvm_var(&elem_ty);

                    // dynamic length array or vector
                    let index = self.builder.build_int_mul(
                        index,
                        llvm_elem_ty
                            .into_pointer_type()
                            .get_element_type()
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false),
                        "",
                    );

                    let elem = unsafe {
                        self.builder.build_gep(
                            array,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(2, false),
                                index,
                            ],
                            "index_access",
                        )
                    };

                    self.builder
                        .build_pointer_cast(elem, llvm_elem_ty.into_pointer_type(), "elem")
                }
            }
            _ => unreachable!(),
        }
    }
}

static BPF_IR: [&[u8]; 5] = [
    include_bytes!("../../stdlib/bpf/stdlib.bc"),
    include_bytes!("../../stdlib/bpf/bigint.bc"),
    include_bytes!("../../stdlib/bpf/format.bc"),
    include_bytes!("../../stdlib/bpf/solana.bc"),
    include_bytes!("../../stdlib/bpf/ripemd160.bc"),
];

static WASM_IR: [&[u8]; 4] = [
    include_bytes!("../../stdlib/wasm/stdlib.bc"),
    include_bytes!("../../stdlib/wasm/wasmheap.bc"),
    include_bytes!("../../stdlib/wasm/bigint.bc"),
    include_bytes!("../../stdlib/wasm/format.bc"),
];

static KECCAK256_IR: &[u8] = include_bytes!("../../stdlib/wasm/keccak256.bc");
static RIPEMD160_IR: &[u8] = include_bytes!("../../stdlib/wasm/ripemd160.bc");
static SUBSTRATE_IR: &[u8] = include_bytes!("../../stdlib/wasm/substrate.bc");

/// Return the stdlib as parsed llvm module. The solidity standard library is hardcoded into
/// the solang library
fn load_stdlib<'a>(context: &'a Context, target: &Target) -> Module<'a> {
    if *target == Target::Solana {
        let memory = MemoryBuffer::create_from_memory_range(&BPF_IR[0], "bpf_bc");

        let module = Module::parse_bitcode_from_buffer(&memory, context).unwrap();

        for bc in BPF_IR.iter().skip(1) {
            let memory = MemoryBuffer::create_from_memory_range(bc, "bpf_bc");

            module
                .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
                .unwrap();
        }

        return module;
    }

    let memory = MemoryBuffer::create_from_memory_range(&WASM_IR[0], "wasm_bc");

    let module = Module::parse_bitcode_from_buffer(&memory, context).unwrap();

    for bc in WASM_IR.iter().skip(1) {
        let memory = MemoryBuffer::create_from_memory_range(bc, "wasm_bc");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    }

    if Target::Substrate == *target {
        let memory = MemoryBuffer::create_from_memory_range(SUBSTRATE_IR, "substrate");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();

        // substrate does not provide ripemd160
        let memory = MemoryBuffer::create_from_memory_range(RIPEMD160_IR, "ripemd160");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    } else if Target::Ewasm != *target {
        // Substrate and Ewasm provides a keccak256 (sha3) host function, others do not
        let memory = MemoryBuffer::create_from_memory_range(KECCAK256_IR, "sha3");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    }

    module
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

    /// File extension
    pub fn file_extension(&self) -> &'static str {
        match self {
            // Solana uses ELF dynamic shared object (BPF)
            Target::Solana => "so",
            // Generic target produces object file for linking
            Target::Generic => "o",
            // Everything else generates webassembly
            _ => "wasm",
        }
    }

    /// Size of a pointer in bytes
    pub fn ptr_size(&self) -> usize {
        if *self == Target::Solana {
            // Solana is BPF, which is 64 bit
            64
        } else {
            // All others are WebAssembly in 32 bit mode
            32
        }
    }
}
