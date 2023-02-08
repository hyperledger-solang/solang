// SPDX-License-Identifier: Apache-2.0

use crate::emit::binary::Binary;
use crate::emit::storage::StorageSlot;
use crate::emit::substrate::{log_return_code, SubstrateTarget};
use crate::emit::TargetRuntime;
use crate::emit_context;
use crate::sema::ast::{ArrayLength, Namespace, Type};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{ArrayValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

impl StorageSlot for SubstrateTarget {
    fn set_storage(
        &self,
        binary: &Binary,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    ) {
        emit_context!(binary);

        let dest_size = if dest_ty.is_array_type() {
            dest_ty
                .into_array_type()
                .size_of()
                .expect("array should be fixed size")
                .const_cast(binary.context.i32_type(), false)
        } else {
            dest_ty
                .into_int_type()
                .size_of()
                .const_cast(binary.context.i32_type(), false)
        };

        let ret = seal_set_storage!(
            slot.into(),
            i32_const!(32).into(),
            dest.into(),
            dest_size.into()
        );

        log_return_code(binary, "seal_set_storage", ret);
    }

    fn get_storage_address<'a>(
        &self,
        binary: &Binary<'a>,
        slot: PointerValue<'a>,
        ns: &Namespace,
    ) -> ArrayValue<'a> {
        emit_context!(binary);

        let (scratch_buf, scratch_len) = scratch_buf!();

        binary
            .builder
            .build_store(scratch_len, i32_const!(ns.address_length as u64));

        let exists = seal_get_storage!(
            slot.into(),
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
                    .build_load(binary.address_type(ns), scratch_buf, "address")
                    .into_array_value(),
                binary.address_type(ns).const_zero(),
                "retrieved_address",
            )
            .into_array_value()
    }

    fn storage_delete_single_slot(&self, binary: &Binary, slot: PointerValue) {
        emit_context!(binary);

        let ret = call!("seal_clear_storage", &[slot.into(), i32_const!(32).into()])
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        log_return_code(binary, "seal_clear_storage", ret);
    }

    fn storage_load_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        emit_context!(bin);

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
                    let new = call!("__malloc", &[size.into()])
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    bin.emit_static_loop_with_int(
                        function,
                        bin.context.i64_type().const_zero(),
                        bin.context.i64_type().const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = unsafe {
                                bin.builder.build_gep(
                                    llvm_ty,
                                    new,
                                    &[i32_zero!(), index],
                                    "index_access",
                                )
                            };

                            let val =
                                self.storage_load_slot(bin, &ty, slot, slot_ptr, function, ns);

                            let val = if ty.deref_memory().is_fixed_reference_type() {
                                let load_ty = bin.llvm_type(ty.deref_any(), ns);
                                bin.builder
                                    .build_load(load_ty, val.into_pointer_value(), "elem")
                            } else {
                                val
                            };

                            bin.builder.build_store(elem, val);
                        },
                    );

                    new.into()
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
                        bin.context.i8_type().ptr_type(AddressSpace::default()),
                        "invalid",
                    );

                    let dest = call!("vector_new", &[size.into(), elem_size.into(), init.into()])
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
                        .build_load(slot.get_type(), slot_ptr, "elem_slot")
                        .into_int_value();

                    bin.emit_loop_cond_first_with_int(
                        function,
                        i32_zero!(),
                        size,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = bin.array_subscript(ty, dest, elem_no, ns);

                            let entry =
                                self.storage_load_slot(bin, elem_ty, slot, slot_ptr, function, ns);

                            let entry = if elem_ty.deref_memory().is_fixed_reference_type() {
                                bin.builder.build_load(
                                    bin.llvm_type(elem_ty.deref_memory(), ns),
                                    entry.into_pointer_value(),
                                    "elem",
                                )
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

                let new = call!("__malloc", &[size.into()])
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                for (i, field) in str_ty.definition(ns).fields.iter().enumerate() {
                    let val = self.storage_load_slot(bin, &field.ty, slot, slot_ptr, function, ns);

                    let elem = unsafe {
                        bin.builder.build_gep(
                            llvm_ty,
                            new,
                            &[i32_zero!(), i32_const!(i as u64)],
                            field.name_as_str(),
                        )
                    };

                    let val = if field.ty.deref_memory().is_fixed_reference_type() {
                        let load_ty = bin.llvm_type(field.ty.deref_memory(), ns);
                        bin.builder.build_load(
                            load_ty,
                            val.into_pointer_value(),
                            field.name_as_str(),
                        )
                    } else {
                        val
                    };

                    bin.builder.build_store(elem, val);
                }

                new.into()
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
                        bin.llvm_type(ty.deref_any(), ns)
                            .ptr_type(AddressSpace::default()),
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

                let ret = self.get_storage_address(bin, slot_ptr, ns);

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

    fn storage_store_slot<'a>(
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
                                    bin.llvm_type(ty.deref_any(), ns),
                                    dest.into_pointer_value(),
                                    &[bin.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            if elem_ty.is_reference_type(ns)
                                && !elem_ty.deref_memory().is_fixed_reference_type()
                            {
                                let load_ty =
                                    bin.llvm_type(elem_ty, ns).ptr_type(AddressSpace::default());
                                elem = bin
                                    .builder
                                    .build_load(load_ty, elem, "")
                                    .into_pointer_value();
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

                    self.set_storage(bin, slot_ptr, new_slot, bin.llvm_type(&slot_ty, ns));

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
                        .build_load(
                            bin.llvm_type(&slot_ty, ns).into_int_type(),
                            new_slot,
                            "elem_slot",
                        )
                        .into_int_value();

                    bin.emit_loop_cond_first_with_int(
                        function,
                        bin.context.i32_type().const_zero(),
                        len,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = bin.builder.build_int_mul(elem_no, elem_size, "");

                            let mut elem = unsafe {
                                bin.builder.build_gep(
                                    bin.llvm_type(ty.deref_any(), ns),
                                    dest.into_pointer_value(),
                                    &[
                                        bin.context.i32_type().const_zero(),
                                        bin.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            if elem_ty.is_reference_type(ns)
                                && !elem_ty.deref_memory().is_fixed_reference_type()
                            {
                                elem = bin
                                    .builder
                                    .build_load(llvm_elem_ty, elem, "")
                                    .into_pointer_value();
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
                            bin.llvm_type(ty.deref_any(), ns),
                            dest.into_pointer_value(),
                            &[
                                bin.context.i32_type().const_zero(),
                                bin.context.i32_type().const_int(i as u64, false),
                            ],
                            field.name_as_str(),
                        )
                    };

                    if field.ty.is_reference_type(ns) && !field.ty.is_fixed_reference_type() {
                        let load_ty = bin
                            .llvm_type(&field.ty, ns)
                            .ptr_type(AddressSpace::default());
                        elem = bin
                            .builder
                            .build_load(load_ty, elem, field.name_as_str())
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

                self.set_storage_extfunc(
                    bin,
                    function,
                    slot_ptr,
                    dest.into_pointer_value(),
                    bin.llvm_type(ty, ns),
                );
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

                self.set_storage(bin, slot_ptr, m, ptr_ty.as_basic_type_enum());
            }
            Type::Address(_) | Type::Contract(_) => {
                if dest.is_pointer_value() {
                    bin.builder.build_store(slot_ptr, *slot);

                    self.set_storage(
                        bin,
                        slot_ptr,
                        dest.into_pointer_value(),
                        bin.llvm_type(ty, ns),
                    );
                } else {
                    let address = bin.builder.build_alloca(bin.address_type(ns), "address");

                    bin.builder.build_store(address, dest.into_array_value());

                    bin.builder.build_store(slot_ptr, *slot);

                    self.set_storage(
                        bin,
                        slot_ptr,
                        address,
                        bin.address_type(ns).as_basic_type_enum(),
                    );
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
                self.set_storage(bin, slot_ptr, dest, bin.llvm_type(ty.deref_any(), ns));
            }
        }
    }

    fn storage_delete_slot<'a>(
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

                    let mut entry_slot = bin
                        .builder
                        .build_load(slot.get_type(), buf, "entry_slot")
                        .into_int_value();

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

                self.storage_delete_single_slot(bin, slot_ptr);
            }
        }
    }
}
