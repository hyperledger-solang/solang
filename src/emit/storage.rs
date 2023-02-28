// SPDX-License-Identifier: Apache-2.0

use crate::emit::binary::Binary;
use crate::sema::ast::{Namespace, Type};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{ArrayValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};

/// This trait species the methods for managing storage on slot based environments
pub(super) trait StorageSlot {
    fn set_storage(
        &self,
        binary: &Binary,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    );

    fn get_storage_address<'a>(
        &self,
        binary: &Binary<'a>,
        slot: PointerValue<'a>,
        ns: &Namespace,
    ) -> ArrayValue<'a>;

    /// Clear a particlar storage slot (slot-based storage chains should implement)
    fn storage_delete_single_slot(&self, binary: &Binary, slot: PointerValue);

    /// Recursively load a type from storage for slot based storage
    fn storage_load_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
        ns: &Namespace,
    ) -> BasicValueEnum<'a>;

    /// Recursively store a type to storage for slot-based storage
    fn storage_store_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    );

    /// Recursively clear bin storage for slot-based storage
    fn storage_delete_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    );
}
