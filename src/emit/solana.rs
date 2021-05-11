use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use std::collections::HashMap;
use std::str;

use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress};
use inkwell::{context::Context, types::BasicTypeEnum};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use num_traits::ToPrimitive;
use tiny_keccak::{Hasher, Keccak};

use super::ethabiencoder;
use super::loop_builder::LoopBuilder;
use super::{Contract, ReturnCode, TargetRuntime, Variable};

pub struct SolanaTarget {
    abi: ethabiencoder::EthAbiDecoder,
    magic: u32,
}

// Implement the Solana target which uses BPF
impl SolanaTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Contract<'a> {
        // We need a magic number for our contract. This is used to check if the contract storage
        // account is initialized for the correct contract
        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        hasher.update(contract.name.as_bytes());
        hasher.finalize(&mut hash);
        let mut magic = [0u8; 4];

        magic.copy_from_slice(&hash[0..4]);

        let mut target = SolanaTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: true },
            magic: u32::from_le_bytes(magic),
        };

        let mut con = Contract::new(
            context,
            contract,
            ns,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        con.return_values
            .insert(ReturnCode::Success, context.i64_type().const_zero());
        con.return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        con.return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );

        // externals
        target.declare_externals(&mut con);

        target.emit_functions(&mut con);

        target.emit_dispatch(&mut con);

        con.internalize(&[
            "entrypoint",
            "sol_log_",
            "sol_alloc_free_",
            // This entry is produced by llvm due to merging of stdlib.bc with solidity llvm ir
            "sol_alloc_free_.1",
        ]);

        con
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let void_ty = contract.context.void_type();
        let u8_ptr = contract.context.i8_type().ptr_type(AddressSpace::Generic);
        let u64_ty = contract.context.i64_type();
        let u32_ty = contract.context.i32_type();
        let sol_bytes = contract
            .context
            .struct_type(&[u8_ptr.into(), u64_ty.into()], false)
            .ptr_type(AddressSpace::Generic);

        let function = contract.module.add_function(
            "sol_alloc_free_",
            u8_ptr.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = contract.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = contract.module.add_function(
            "sol_sha256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = contract.module.add_function(
            "sol_keccak256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);
    }

    /// Returns the SolAccountInfo of the executing contract
    fn contract_storage_account<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value();

        let ka_cur = contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        unsafe {
            contract.builder.build_gep(
                parameters,
                &[
                    contract.context.i32_type().const_int(0, false),
                    contract.context.i32_type().const_int(0, false),
                    ka_cur,
                ],
                "account",
            )
        }
    }

    /// Returns the account data of the executing contract
    fn contract_storage_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value();

        let ka_cur = contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        contract
            .builder
            .build_load(
                unsafe {
                    contract.builder.build_gep(
                        parameters,
                        &[
                            contract.context.i32_type().const_int(0, false),
                            contract.context.i32_type().const_int(0, false),
                            ka_cur,
                            contract.context.i32_type().const_int(3, false),
                        ],
                        "data",
                    )
                },
                "data",
            )
            .into_pointer_value()
    }

    /// Returns the account data length of the executing contract
    fn contract_storage_datalen<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value();

        let ka_cur = contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        contract
            .builder
            .build_load(
                unsafe {
                    contract.builder.build_gep(
                        parameters,
                        &[
                            contract.context.i32_type().const_int(0, false),
                            contract.context.i32_type().const_int(0, false),
                            ka_cur,
                            contract.context.i32_type().const_int(2, false),
                        ],
                        "data_len",
                    )
                },
                "data_len",
            )
            .into_int_value()
    }

    fn emit_dispatch(&mut self, contract: &mut Contract) {
        let initializer = self.emit_initializer(contract);

        let function = contract.module.get_function("solang_dispatch").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let sol_params = function.get_nth_param(0).unwrap().into_pointer_value();

        let input = contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(sol_params, 5, "input")
                    .unwrap(),
                "data",
            )
            .into_pointer_value();

        let input_len = contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(sol_params, 6, "input_len")
                    .unwrap(),
                "data_len",
            )
            .into_int_value();

        // load magic value of contract storage
        contract.parameters = Some(sol_params);

        let contract_data = self.contract_storage_data(contract);

        let magic_value_ptr = contract.builder.build_pointer_cast(
            contract_data,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "magic_value_ptr",
        );

        let magic_value = contract
            .builder
            .build_load(magic_value_ptr, "magic")
            .into_int_value();

        let function_block = contract
            .context
            .append_basic_block(function, "function_call");
        let constructor_block = contract
            .context
            .append_basic_block(function, "constructor_call");
        let badmagic_block = contract.context.append_basic_block(function, "bad_magic");

        // if the magic is zero it's a virgin contract
        // if the magic is our magic value, it's a function call
        // if the magic is another magic value, it is an error
        contract.builder.build_switch(
            magic_value,
            badmagic_block,
            &[
                (contract.context.i32_type().const_zero(), constructor_block),
                (
                    contract
                        .context
                        .i32_type()
                        .const_int(self.magic as u64, false),
                    function_block,
                ),
            ],
        );

        contract.builder.position_at_end(badmagic_block);

        contract.builder.build_return(Some(
            &contract.context.i64_type().const_int(4u64 << 32, false),
        ));

        // generate constructor code
        contract.builder.position_at_end(constructor_block);

        // do we have enough contract data
        let contract_data_len = self.contract_storage_datalen(contract);

        let fixed_fields_size = contract.contract.fixed_layout_size.to_u64().unwrap();

        let is_enough = contract.builder.build_int_compare(
            IntPredicate::UGE,
            contract_data_len,
            contract
                .context
                .i64_type()
                .const_int(fixed_fields_size, false),
            "is_enough",
        );

        let not_enough = contract.context.append_basic_block(function, "not_enough");
        let enough = contract.context.append_basic_block(function, "enough");

        contract
            .builder
            .build_conditional_branch(is_enough, enough, not_enough);

        contract.builder.position_at_end(not_enough);

        contract.builder.build_return(Some(
            &contract.context.i64_type().const_int(5u64 << 32, false),
        ));

        contract.builder.position_at_end(enough);

        // write our magic value to the contract
        contract.builder.build_store(
            magic_value_ptr,
            contract
                .context
                .i32_type()
                .const_int(self.magic as u64, false),
        );

        // write heap_offset.
        let heap_offset_ptr = unsafe {
            contract.builder.build_gep(
                magic_value_ptr,
                &[contract.context.i64_type().const_int(3, false)],
                "heap_offset",
            )
        };

        // align heap to 8 bytes
        let heap_offset = (fixed_fields_size + 7) & !7;

        contract.builder.build_store(
            heap_offset_ptr,
            contract.context.i32_type().const_int(heap_offset, false),
        );

        let arg_ty = initializer.get_type().get_param_types()[0].into_pointer_type();

        contract.builder.build_call(
            initializer,
            &[contract
                .builder
                .build_pointer_cast(sol_params, arg_ty, "")
                .into()],
            "",
        );

        // There is only one possible constructor
        let ret = if let Some((cfg_no, cfg)) = contract
            .contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor)
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, input, input_len, &cfg.params);

            let function = contract.functions[&cfg_no];
            let params_ty = function
                .get_type()
                .get_param_types()
                .last()
                .unwrap()
                .into_pointer_type();

            args.push(
                contract
                    .builder
                    .build_pointer_cast(sol_params, params_ty, "")
                    .into(),
            );

            contract
                .builder
                .build_call(function, &args, "")
                .try_as_basic_value()
                .left()
                .unwrap()
        } else {
            // return 0 for success
            contract.context.i64_type().const_int(0, false).into()
        };

        contract.builder.build_return(Some(&ret));

        // Generate function call dispatch
        contract.builder.position_at_end(function_block);

        let input = contract.builder.build_pointer_cast(
            input,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "input_ptr32",
        );

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            input,
            input_len,
            function,
            None,
            |_| false,
        );
    }

    /// Free contract storage and zero out
    fn storage_free<'b>(
        &self,
        contract: &Contract<'b>,
        ty: &ast::Type,
        data: PointerValue<'b>,
        slot: IntValue<'b>,
        function: FunctionValue<'b>,
        zero: bool,
    ) {
        if !zero && !ty.is_dynamic(contract.ns) {
            // nothing to do
            return;
        }

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = contract
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            contract.builder.build_call(
                contract.module.get_function("account_data_free").unwrap(),
                &[data.into(), offset.into()],
                "",
            );

            // account_data_alloc will return 0 if the string is length 0
            let new_offset = contract.context.i32_type().const_zero();

            contract.builder.build_store(offset_ptr, new_offset);
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // delete the existing storage
            let mut elem_slot = slot;

            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            if elem_ty.is_dynamic(contract.ns) || zero {
                let length = if let Some(length) = dim[0].as_ref() {
                    contract
                        .context
                        .i32_type()
                        .const_int(length.to_u64().unwrap(), false)
                } else {
                    elem_slot = contract
                        .builder
                        .build_load(offset_ptr, "offset")
                        .into_int_value();

                    self.storage_array_length(contract, function, slot, elem_ty)
                };

                let elem_size = elem_ty.size_of(contract.ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(contract, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(contract, "offset", slot.get_type(), elem_slot.into());

                let _ = builder.over(contract, contract.context.i32_type().const_zero(), length);

                let offset_val = offset_phi.into_int_value();

                let elem_ty = ty.array_deref();

                self.storage_free(
                    contract,
                    &elem_ty.deref_any(),
                    data,
                    offset_val,
                    function,
                    zero,
                );

                let offset_val = contract.builder.build_int_add(
                    offset_val,
                    contract.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(contract, "offset", offset_val.into());

                // done
                builder.finish(contract);
            }

            // if the array was dynamic, free the array itself
            if dim[0].is_none() {
                let slot = contract
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                contract.builder.build_call(
                    contract.module.get_function("account_data_free").unwrap(),
                    &[data.into(), slot.into()],
                    "",
                );

                // account_data_alloc will return 0 if the string is length 0
                let new_offset = contract.context.i32_type().const_zero();

                contract.builder.build_store(offset_ptr, new_offset);
            }
        } else if let ast::Type::Struct(struct_no) = ty {
            for (i, field) in contract.ns.structs[*struct_no].fields.iter().enumerate() {
                let field_offset = contract.ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                let offset = contract.builder.build_int_add(
                    slot,
                    contract.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                self.storage_free(contract, &field.ty, data, offset, function, zero);
            }
        } else {
            let ty = contract.llvm_type(ty);

            contract.builder.build_store(
                contract
                    .builder
                    .build_pointer_cast(member, ty.ptr_type(AddressSpace::Generic), ""),
                ty.into_int_type().const_zero(),
            );
        }
    }

    /// An entry in a sparse array or mapping
    fn sparse_entry<'b>(
        &self,
        contract: &Contract<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
    ) -> BasicTypeEnum<'b> {
        let key = if matches!(
            key_ty,
            ast::Type::String | ast::Type::DynamicBytes | ast::Type::Mapping(_, _)
        ) {
            contract.context.i32_type().into()
        } else {
            contract.llvm_type(key_ty)
        };

        contract
            .context
            .struct_type(
                &[
                    key,                                // key
                    contract.context.i32_type().into(), // next field
                    if value_ty.is_mapping() {
                        contract.context.i32_type().into()
                    } else {
                        contract.llvm_type(value_ty) // value
                    },
                ],
                false,
            )
            .into()
    }

    /// Generate sparse lookup
    fn sparse_lookup_function<'b>(
        &self,
        contract: &Contract<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
    ) -> FunctionValue<'b> {
        let function_name = format!(
            "sparse_lookup_{}_{}",
            key_ty.to_wasm_string(contract.ns),
            value_ty.to_wasm_string(contract.ns)
        );

        if let Some(function) = contract.module.get_function(&function_name) {
            return function;
        }

        // The function takes an offset (of the mapping or sparse array), the key which
        // is the index, and it should return an offset.
        let function_ty = contract.function_type(
            &[ast::Type::Uint(32), key_ty.clone()],
            &[ast::Type::Uint(32)],
        );

        let function =
            contract
                .module
                .add_function(&function_name, function_ty, Some(Linkage::Internal));

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let offset = function.get_nth_param(0).unwrap().into_int_value();
        let key = function.get_nth_param(1).unwrap();

        let entry_ty = self.sparse_entry(contract, key_ty, value_ty);
        let value_offset = unsafe {
            entry_ty
                .ptr_type(AddressSpace::Generic)
                .const_null()
                .const_gep(&[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(2, false),
                ])
                .const_to_int(contract.context.i32_type())
        };

        let data = self.contract_storage_data(contract);

        let member = unsafe { contract.builder.build_gep(data, &[offset], "data") };
        let offset_ptr = contract.builder.build_pointer_cast(
            member,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        // calculate the correct bucket. We have an prime number of
        let bucket = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            contract
                .builder
                .build_call(
                    contract.module.get_function("vector_hash").unwrap(),
                    &[key],
                    "hash",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if key_ty.bits(contract.ns) > 64 {
            contract.builder.build_int_truncate(
                key.into_int_value(),
                contract.context.i64_type(),
                "",
            )
        } else {
            key.into_int_value()
        };

        let bucket = contract.builder.build_int_unsigned_rem(
            bucket,
            bucket
                .get_type()
                .const_int(crate::sema::SOLANA_BUCKET_SIZE, false),
            "",
        );

        let first_offset_ptr = unsafe {
            contract
                .builder
                .build_gep(offset_ptr, &[bucket], "bucket_list")
        };

        // we should now loop until offset is zero or we found it
        let loop_entry = contract.context.append_basic_block(function, "loop_entry");
        let end_of_bucket = contract
            .context
            .append_basic_block(function, "end_of_bucket");
        let examine_bucket = contract
            .context
            .append_basic_block(function, "examine_bucket");
        let found_entry = contract.context.append_basic_block(function, "found_entry");
        let next_entry = contract.context.append_basic_block(function, "next_entry");

        // let's enter the loop
        contract.builder.build_unconditional_branch(loop_entry);

        contract.builder.position_at_end(loop_entry);

        // we are walking the bucket list via the offset ptr
        let offset_ptr_phi = contract.builder.build_phi(
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        offset_ptr_phi.add_incoming(&[(&first_offset_ptr, entry)]);

        // load the offset and check for zero (end of bucket list)
        let offset = contract
            .builder
            .build_load(
                offset_ptr_phi.as_basic_value().into_pointer_value(),
                "offset",
            )
            .into_int_value();

        let is_offset_zero = contract.builder.build_int_compare(
            IntPredicate::EQ,
            offset,
            offset.get_type().const_zero(),
            "offset_is_zero",
        );

        contract
            .builder
            .build_conditional_branch(is_offset_zero, end_of_bucket, examine_bucket);

        contract.builder.position_at_end(examine_bucket);

        // let's compare the key in this entry to the key we are looking for
        let member = unsafe { contract.builder.build_gep(data, &[offset], "data") };
        let entry_ptr = contract.builder.build_pointer_cast(
            member,
            entry_ty.ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let entry_key = contract
            .builder
            .build_load(
                unsafe {
                    contract.builder.build_gep(
                        entry_ptr,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "key_ptr",
                    )
                },
                "key",
            )
            .into_int_value();

        let matches = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            // entry_key is an offset
            let entry_data = unsafe { contract.builder.build_gep(data, &[entry_key], "data") };
            let entry_length = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_len").unwrap(),
                    &[data.into(), entry_key.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            contract
                .builder
                .build_call(
                    contract.module.get_function("__memcmp").unwrap(),
                    &[
                        entry_data.into(),
                        entry_length.into(),
                        contract.vector_bytes(key).into(),
                        contract.vector_len(key).into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            contract.builder.build_int_compare(
                IntPredicate::EQ,
                key.into_int_value(),
                entry_key,
                "matches",
            )
        };

        contract
            .builder
            .build_conditional_branch(matches, found_entry, next_entry);

        contract.builder.position_at_end(found_entry);

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        contract.builder.build_store(
            ret_offset,
            contract
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        contract
            .builder
            .build_return(Some(&contract.context.i64_type().const_zero()));

        contract.builder.position_at_end(next_entry);

        let offset_ptr = contract
            .builder
            .build_struct_gep(entry_ptr, 1, "offset_ptr")
            .unwrap();

        offset_ptr_phi.add_incoming(&[(&offset_ptr, next_entry)]);

        contract.builder.build_unconditional_branch(loop_entry);

        let offset_ptr = offset_ptr_phi.as_basic_value().into_pointer_value();

        contract.builder.position_at_end(end_of_bucket);

        let entry_length = entry_ty
            .size_of()
            .unwrap()
            .const_cast(contract.context.i32_type(), false);

        let account = self.contract_storage_account(contract);

        // account_data_alloc will return offset = 0 if the string is length 0
        let rc = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_alloc").unwrap(),
                &[account.into(), entry_length.into(), offset_ptr.into()],
                "rc",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = contract.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            contract.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = contract.context.append_basic_block(function, "rc_zero");

        contract
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        contract.builder.position_at_end(rc_not_zero);

        self.return_code(contract, rc);

        contract.builder.position_at_end(rc_zero);

        let offset = contract
            .builder
            .build_load(offset_ptr, "new_offset")
            .into_int_value();

        let member = unsafe { contract.builder.build_gep(data, &[offset], "data") };

        // Clear memory. The length argument to __bzero8 is in lengths of 8 bytes. We round up to the nearest
        // 8 byte, since account_data_alloc also rounds up to the nearest 8 byte when allocating.
        let length = contract.builder.build_int_unsigned_div(
            contract.builder.build_int_add(
                entry_length,
                contract.context.i32_type().const_int(7, false),
                "",
            ),
            contract.context.i32_type().const_int(8, false),
            "length_div_8",
        );

        contract.builder.build_call(
            contract.module.get_function("__bzero8").unwrap(),
            &[member.into(), length.into()],
            "zeroed",
        );

        let entry_ptr = contract.builder.build_pointer_cast(
            member,
            entry_ty.ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        // set key
        if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let new_string_length = contract.vector_len(key);
            let offset_ptr = contract
                .builder
                .build_struct_gep(entry_ptr, 0, "key_ptr")
                .unwrap();

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = contract.builder.build_int_compare(
                IntPredicate::EQ,
                rc,
                contract.context.i64_type().const_zero(),
                "is_rc_zero",
            );

            let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = contract.context.append_basic_block(function, "rc_zero");
            let memcpy = contract.context.append_basic_block(function, "memcpy");

            contract
                .builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

            contract.builder.position_at_end(rc_not_zero);

            self.return_code(
                contract,
                contract.context.i64_type().const_int(5u64 << 32, false),
            );

            contract.builder.position_at_end(rc_zero);

            let new_offset = contract.builder.build_load(offset_ptr, "new_offset");

            contract.builder.build_unconditional_branch(memcpy);

            contract.builder.position_at_end(memcpy);

            let offset_phi = contract
                .builder
                .build_phi(contract.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            contract.builder.build_call(
                contract.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    contract.vector_bytes(key).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else {
            let key_ptr = contract
                .builder
                .build_struct_gep(entry_ptr, 0, "key_ptr")
                .unwrap();

            contract.builder.build_store(key_ptr, key);
        };

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        contract.builder.build_store(
            ret_offset,
            contract
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        contract
            .builder
            .build_return(Some(&contract.context.i64_type().const_zero()));

        function
    }

    /// Do a lookup/subscript in a sparse array or mapping; this will call a function
    fn sparse_lookup<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        slot: IntValue<'b>,
        index: BasicValueEnum<'b>,
    ) -> IntValue<'b> {
        let offset = contract.build_alloca(function, contract.context.i32_type(), "offset");

        let current_block = contract.builder.get_insert_block().unwrap();

        let lookup = self.sparse_lookup_function(contract, key_ty, value_ty);

        contract.builder.position_at_end(current_block);

        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value();

        let rc = contract
            .builder
            .build_call(
                lookup,
                &[slot.into(), index, offset.into(), parameters.into()],
                "mapping_lookup_res",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // either load the result from offset or return failure
        let is_rc_zero = contract.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            rc.get_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = contract.context.append_basic_block(function, "rc_zero");

        contract
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        contract.builder.position_at_end(rc_not_zero);

        self.return_code(contract, rc);

        contract.builder.position_at_end(rc_zero);

        contract
            .builder
            .build_load(offset, "offset")
            .into_int_value()
    }
}

impl<'a> TargetRuntime<'a> for SolanaTarget {
    /// Solana does not use slot based-storage so override
    fn storage_delete(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
    ) {
        // contract storage is in 2nd account
        let data = self.contract_storage_data(contract);

        self.storage_free(contract, ty, data, *slot, function, true);
    }

    fn set_storage_extfunc(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        // unused
        unreachable!();
    }

    fn get_storage_string(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        // unused
        unreachable!();
    }

    fn get_storage_bytes_subscript(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(contract);

        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = contract.builder.build_pointer_cast(
            member,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = contract
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            contract
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let get_block = contract.context.append_basic_block(function, "in_range");
        let bang_block = contract.context.append_basic_block(function, "bang_block");

        contract
            .builder
            .build_conditional_branch(in_range, get_block, bang_block);

        contract.builder.position_at_end(bang_block);

        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );

        contract.builder.position_at_end(get_block);

        let offset = contract.builder.build_int_add(offset, index, "offset");

        let member = unsafe { contract.builder.build_gep(data, &[offset], "data") };

        contract.builder.build_load(member, "val").into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        contract: &Contract,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
    ) {
        let data = self.contract_storage_data(contract);

        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = contract.builder.build_pointer_cast(
            member,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = contract
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            contract
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let set_block = contract.context.append_basic_block(function, "in_range");
        let bang_block = contract.context.append_basic_block(function, "bang_block");

        contract
            .builder
            .build_conditional_branch(in_range, set_block, bang_block);

        contract.builder.position_at_end(bang_block);
        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );

        contract.builder.position_at_end(set_block);

        let offset = contract.builder.build_int_add(offset, index, "offset");

        let member = unsafe { contract.builder.build_gep(data, &[offset], "data") };

        contract.builder.build_store(member, val);
    }

    fn storage_subscript(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
    ) -> IntValue<'a> {
        let account = self.contract_storage_account(contract);

        if let ast::Type::Mapping(key, value) = ty.deref_any() {
            self.sparse_lookup(contract, function, key, value, slot, index)
        } else if ty.is_sparse_solana(contract.ns) {
            // sparse array
            let elem_ty = ty.storage_array_elem().deref_into();

            let key = ast::Type::Uint(256);

            self.sparse_lookup(contract, function, &key, &elem_ty, slot, index)
        } else {
            // 3rd member of account is data pointer
            let data = unsafe {
                contract.builder.build_gep(
                    account,
                    &[
                        contract.context.i32_type().const_zero(),
                        contract.context.i32_type().const_int(3, false),
                    ],
                    "data",
                )
            };

            let data = contract
                .builder
                .build_load(data, "data")
                .into_pointer_value();

            let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };
            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = contract
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let elem_ty = ty.storage_array_elem().deref_into();

            let elem_size = contract
                .context
                .i32_type()
                .const_int(elem_ty.size_of(contract.ns).to_u64().unwrap(), false);

            contract.builder.build_int_add(
                offset,
                contract
                    .builder
                    .build_int_mul(index.into_int_value(), elem_size, ""),
                "",
            )
        }
    }

    fn storage_push(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        val: BasicValueEnum<'a>,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(contract);
        let account = self.contract_storage_account(contract);

        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = contract.builder.build_pointer_cast(
            member,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = contract
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let member_size = contract
            .context
            .i32_type()
            .const_int(ty.size_of(contract.ns).to_u64().unwrap(), false);
        let new_length = contract
            .builder
            .build_int_add(length, member_size, "new_length");

        let rc = contract
            .builder
            .build_call(
                contract
                    .module
                    .get_function("account_data_realloc")
                    .unwrap(),
                &[
                    account.into(),
                    offset.into(),
                    new_length.into(),
                    offset_ptr.into(),
                ],
                "new_offset",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = contract.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            contract.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = contract.context.append_basic_block(function, "rc_zero");

        contract
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        contract.builder.position_at_end(rc_not_zero);

        self.return_code(
            contract,
            contract.context.i64_type().const_int(5u64 << 32, false),
        );

        contract.builder.position_at_end(rc_zero);

        let mut new_offset = contract.builder.build_int_add(
            contract
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value(),
            length,
            "",
        );

        self.storage_store(contract, ty, &mut new_offset, val, function);

        if ty.is_reference_type() {
            // Caller expects a reference to storage; note that storage_store() should not modify
            // new_offset even if the argument is mut
            new_offset.into()
        } else {
            val
        }
    }

    fn storage_pop(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(contract);
        let account = self.contract_storage_account(contract);

        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = contract.builder.build_pointer_cast(
            member,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = contract
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = contract.builder.build_int_compare(
            IntPredicate::NE,
            contract.context.i32_type().const_zero(),
            length,
            "index_in_range",
        );

        let bang_block = contract.context.append_basic_block(function, "bang_block");
        let retrieve_block = contract.context.append_basic_block(function, "in_range");

        contract
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        contract.builder.position_at_end(bang_block);
        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );

        let member_size = contract
            .context
            .i32_type()
            .const_int(ty.size_of(contract.ns).to_u64().unwrap(), false);

        contract.builder.position_at_end(retrieve_block);

        let new_length = contract
            .builder
            .build_int_sub(length, member_size, "new_length");

        let mut new_offset = contract.builder.build_int_add(offset, new_length, "");

        let val = self.storage_load(contract, ty, &mut new_offset, function);

        // delete existing storage -- pointers need to be freed
        //self.storage_free(contract, ty, account, data, new_offset, function, false);

        // we can assume pointer will stay the same after realloc to smaller size
        contract.builder.build_call(
            contract
                .module
                .get_function("account_data_realloc")
                .unwrap(),
            &[
                account.into(),
                offset.into(),
                new_length.into(),
                offset_ptr.into(),
            ],
            "new_offset",
        );

        val
    }

    fn storage_array_length(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        elem_ty: &ast::Type,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(contract);

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[slot], "data") };

        let offset = contract
            .builder
            .build_load(
                contract.builder.build_pointer_cast(
                    member,
                    contract.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                "offset",
            )
            .into_int_value();

        let member_size = contract
            .context
            .i32_type()
            .const_int(elem_ty.size_of(contract.ns).to_u64().unwrap(), false);

        let length_bytes = contract
            .builder
            .build_call(
                contract.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        contract
            .builder
            .build_int_unsigned_div(length_bytes, member_size, "")
    }

    fn get_storage_int(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ty: IntType<'a>,
    ) -> IntValue<'a> {
        // unused
        unreachable!();
    }

    /// Recursively load a type from contract storage. This overrides the default method
    /// in the trait, which is for chains with 256 bit storage keys.
    fn storage_load(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(contract);

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[*slot], "data") };

        match ty {
            ast::Type::String | ast::Type::DynamicBytes => {
                let offset = contract
                    .builder
                    .build_load(
                        contract.builder.build_pointer_cast(
                            member,
                            contract.context.i32_type().ptr_type(AddressSpace::Generic),
                            "",
                        ),
                        "offset",
                    )
                    .into_int_value();

                let string_length = contract
                    .builder
                    .build_call(
                        contract.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "free",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let string_data =
                    unsafe { contract.builder.build_gep(data, &[offset], "string_data") };

                contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[
                            string_length.into(),
                            contract.context.i32_type().const_int(1, false).into(),
                            string_data.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Type::Struct(struct_no) => {
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

                for (i, field) in contract.ns.structs[*struct_no].fields.iter().enumerate() {
                    let field_offset = contract.ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                    let mut offset = contract.builder.build_int_add(
                        *slot,
                        contract.context.i32_type().const_int(field_offset, false),
                        "field_offset",
                    );

                    let val = self.storage_load(contract, &field.ty, &mut offset, function);

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
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = contract.llvm_type(ty.deref_any());

                let dest;
                let length;
                let mut slot = *slot;

                if dim[0].is_some() {
                    // LLVMSizeOf() produces an i64 and malloc takes i32
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

                    dest = contract.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );
                    length = contract
                        .context
                        .i32_type()
                        .const_int(dim[0].as_ref().unwrap().to_u64().unwrap(), false);
                } else {
                    let elem_size = contract.builder.build_int_truncate(
                        contract
                            .context
                            .i32_type()
                            .const_int(elem_ty.size_of(contract.ns).to_u64().unwrap(), false),
                        contract.context.i32_type(),
                        "size_of",
                    );

                    length = self.storage_array_length(contract, function, slot, &elem_ty);

                    slot = contract
                        .builder
                        .build_load(
                            contract.builder.build_pointer_cast(
                                member,
                                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                                "",
                            ),
                            "offset",
                        )
                        .into_int_value();

                    dest = contract.vector_new(length, elem_size, None);
                };

                let elem_size = elem_ty.size_of(contract.ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(contract, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(contract, "offset", slot.get_type(), slot.into());

                let index =
                    builder.over(contract, contract.context.i32_type().const_zero(), length);

                let elem = contract.array_subscript(ty.deref_any(), dest, index);

                let elem_ty = ty.array_deref();

                let mut offset_val = offset_phi.into_int_value();

                let val =
                    self.storage_load(contract, &elem_ty.deref_memory(), &mut offset_val, function);

                contract.builder.build_store(elem, val);

                offset_val = contract.builder.build_int_add(
                    offset_val,
                    contract.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(contract, "offset", offset_val.into());

                // done
                builder.finish(contract);

                dest.into()
            }
            _ => contract.builder.build_load(
                contract.builder.build_pointer_cast(
                    member,
                    contract.llvm_type(ty).ptr_type(AddressSpace::Generic),
                    "",
                ),
                "",
            ),
        }
    }

    fn storage_store(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        val: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    ) {
        let data = self.contract_storage_data(contract);
        let account = self.contract_storage_account(contract);

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[*slot], "data") };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = contract
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let existing_string_length = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_len").unwrap(),
                    &[data.into(), offset.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let new_string_length = contract.vector_len(val);

            let allocation_necessary = contract.builder.build_int_compare(
                IntPredicate::NE,
                existing_string_length,
                new_string_length,
                "allocation_necessary",
            );

            let entry = contract.builder.get_insert_block().unwrap();

            let realloc = contract.context.append_basic_block(function, "realloc");
            let memcpy = contract.context.append_basic_block(function, "memcpy");

            contract
                .builder
                .build_conditional_branch(allocation_necessary, realloc, memcpy);

            contract.builder.position_at_end(realloc);

            // do not realloc since we're copying everything
            contract.builder.build_call(
                contract.module.get_function("account_data_free").unwrap(),
                &[data.into(), offset.into()],
                "free",
            );

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = contract.builder.build_int_compare(
                IntPredicate::EQ,
                rc,
                contract.context.i64_type().const_zero(),
                "is_rc_zero",
            );

            let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = contract.context.append_basic_block(function, "rc_zero");

            contract
                .builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

            contract.builder.position_at_end(rc_not_zero);

            self.return_code(
                contract,
                contract.context.i64_type().const_int(5u64 << 32, false),
            );

            contract.builder.position_at_end(rc_zero);

            let new_offset = contract.builder.build_load(offset_ptr, "new_offset");

            contract.builder.build_unconditional_branch(memcpy);

            contract.builder.position_at_end(memcpy);

            let offset_phi = contract
                .builder
                .build_phi(contract.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            contract.builder.build_call(
                contract.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    contract.vector_bytes(val).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // make sure any pointers are freed
            self.storage_free(contract, ty, data, *slot, function, false);

            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let length = if let Some(length) = dim[0].as_ref() {
                contract
                    .context
                    .i32_type()
                    .const_int(length.to_u64().unwrap(), false)
            } else {
                contract.vector_len(val)
            };

            let mut elem_slot = *slot;

            if dim[0].is_none() {
                // reallocate to the right size
                let member_size = contract
                    .context
                    .i32_type()
                    .const_int(elem_ty.size_of(contract.ns).to_u64().unwrap(), false);
                let new_length = contract
                    .builder
                    .build_int_mul(length, member_size, "new_length");
                let offset = contract
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                let rc = contract
                    .builder
                    .build_call(
                        contract
                            .module
                            .get_function("account_data_realloc")
                            .unwrap(),
                        &[
                            account.into(),
                            offset.into(),
                            new_length.into(),
                            offset_ptr.into(),
                        ],
                        "new_offset",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let is_rc_zero = contract.builder.build_int_compare(
                    IntPredicate::EQ,
                    rc,
                    contract.context.i64_type().const_zero(),
                    "is_rc_zero",
                );

                let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
                let rc_zero = contract.context.append_basic_block(function, "rc_zero");

                contract
                    .builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

                contract.builder.position_at_end(rc_not_zero);

                self.return_code(
                    contract,
                    contract.context.i64_type().const_int(5u64 << 32, false),
                );

                contract.builder.position_at_end(rc_zero);

                elem_slot = contract
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();
            }

            let elem_size = elem_ty.size_of(contract.ns).to_u64().unwrap();

            // loop over the array
            let mut builder = LoopBuilder::new(contract, function);

            // we need a phi for the offset
            let offset_phi =
                builder.add_loop_phi(contract, "offset", slot.get_type(), elem_slot.into());

            let index = builder.over(contract, contract.context.i32_type().const_zero(), length);

            let elem = contract.array_subscript(ty, val.into_pointer_value(), index);

            let mut offset_val = offset_phi.into_int_value();

            let elem_ty = ty.array_deref();

            self.storage_store(
                contract,
                &elem_ty.deref_any(),
                &mut offset_val,
                contract.builder.build_load(elem, "array_elem"),
                function,
            );

            offset_val = contract.builder.build_int_add(
                offset_val,
                contract.context.i32_type().const_int(elem_size, false),
                "new_offset",
            );

            // set the offset for the next iteration of the loop
            builder.set_loop_phi_value(contract, "offset", offset_val.into());

            // done
            builder.finish(contract);
        } else if let ast::Type::Struct(struct_no) = ty {
            for (i, field) in contract.ns.structs[*struct_no].fields.iter().enumerate() {
                let field_offset = contract.ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                let mut offset = contract.builder.build_int_add(
                    *slot,
                    contract.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                let elem = unsafe {
                    contract.builder.build_gep(
                        val.into_pointer_value(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(i as u64, false),
                        ],
                        &field.name,
                    )
                };

                // free any existing dynamic storage
                self.storage_free(contract, &field.ty, data, offset, function, false);

                self.storage_store(
                    contract,
                    &field.ty,
                    &mut offset,
                    contract.builder.build_load(elem, &field.name),
                    function,
                );
            }
        } else {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    member,
                    val.get_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                val,
            );
        }
    }

    /// sabre has no keccak256 host function, so call our implementation
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        contract.builder.build_call(
            contract.module.get_function("keccak256").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        src,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "src",
                    )
                    .into(),
                length.into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "dest",
                    )
                    .into(),
            ],
            "",
        );
    }

    fn return_empty_abi(&self, contract: &Contract) {
        let data = self.contract_storage_data(contract);

        let header_ptr = contract.builder.build_pointer_cast(
            data,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "header_ptr",
        );

        let data_len_ptr = unsafe {
            contract.builder.build_gep(
                header_ptr,
                &[contract.context.i64_type().const_int(1, false)],
                "data_len_ptr",
            )
        };

        let data_ptr = unsafe {
            contract.builder.build_gep(
                header_ptr,
                &[contract.context.i64_type().const_int(2, false)],
                "data_ptr",
            )
        };

        let offset = contract
            .builder
            .build_load(data_ptr, "offset")
            .into_int_value();

        contract.builder.build_call(
            contract.module.get_function("account_data_free").unwrap(),
            &[data.into(), offset.into()],
            "",
        );

        contract
            .builder
            .build_store(data_len_ptr, contract.context.i32_type().const_zero());

        contract
            .builder
            .build_store(data_ptr, contract.context.i32_type().const_zero());

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i64_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, _data: PointerValue<'b>, _length: IntValue) {
        // return data already filled in output contract

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i64_type().const_int(0, false)));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract, _data: PointerValue, _length: IntValue) {
        // the reason code should be null (and already printed)

        // return 1 for failure
        contract.builder.build_return(Some(
            &contract.context.i64_type().const_int(1u64 << 32, false),
        ));
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
    ) -> PointerValue<'b> {
        ethabiencoder::encode_to_vector(contract, function, packed, args, tys, true)
    }

    fn abi_encode(
        &self,
        contract: &Contract<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue<'a>,
        args: &[BasicValueEnum<'a>],
        tys: &[ast::Type],
    ) -> (PointerValue<'a>, IntValue<'a>) {
        debug_assert_eq!(args.len(), tys.len());

        let mut tys = tys.to_vec();

        let packed = if let Some(selector) = selector {
            tys.insert(0, ast::Type::Uint(32));
            vec![selector.into()]
        } else {
            vec![]
        };

        let encoder =
            ethabiencoder::EncoderBuilder::new(contract, function, load, &packed, args, &tys, true);

        let length = encoder.encoded_length();

        let data = self.contract_storage_data(contract);
        let account = self.contract_storage_account(contract);

        let header_ptr = contract.builder.build_pointer_cast(
            data,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "header_ptr",
        );

        let data_len_ptr = unsafe {
            contract.builder.build_gep(
                header_ptr,
                &[contract.context.i64_type().const_int(1, false)],
                "data_len_ptr",
            )
        };

        let data_offset_ptr = unsafe {
            contract.builder.build_gep(
                header_ptr,
                &[contract.context.i64_type().const_int(2, false)],
                "data_offset_ptr",
            )
        };

        let offset = contract
            .builder
            .build_load(data_offset_ptr, "offset")
            .into_int_value();

        let rc = contract
            .builder
            .build_call(
                contract
                    .module
                    .get_function("account_data_realloc")
                    .unwrap(),
                &[
                    account.into(),
                    offset.into(),
                    length.into(),
                    data_offset_ptr.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = contract.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            contract.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = contract.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = contract.context.append_basic_block(function, "rc_zero");

        contract
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        contract.builder.position_at_end(rc_not_zero);

        self.return_code(
            contract,
            contract.context.i64_type().const_int(5u64 << 32, false),
        );

        contract.builder.position_at_end(rc_zero);

        contract.builder.build_store(data_len_ptr, length);

        let offset = contract
            .builder
            .build_load(data_offset_ptr, "offset")
            .into_int_value();

        // step over that field, and cast to u8* for the buffer itself
        let output = contract.builder.build_pointer_cast(
            unsafe { contract.builder.build_gep(data, &[offset], "data_ptr") },
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "data_ptr",
        );

        encoder.finish(contract, function, output);

        (output, length)
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
    ) {
        self.abi
            .decode(contract, function, args, data, length, spec);
    }

    fn print(&self, contract: &Contract, string_ptr: PointerValue, string_len: IntValue) {
        let string_len64 =
            contract
                .builder
                .build_int_z_extend(string_len, contract.context.i64_type(), "");

        contract.builder.build_call(
            contract.module.get_function("sol_log_").unwrap(),
            &[string_ptr.into(), string_len64.into()],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        _constructor_no: Option<usize>,
        _address: PointerValue<'b>,
        _args: &[BasicValueEnum],
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
    ) {
        unimplemented!();
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        _ty: ast::CallTy,
    ) {
        debug_assert!(address.is_none());

        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap();

        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("external_call").unwrap(),
                &[payload.into(), payload_len.into(), parameters],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            contract.context.i64_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // do not abort execution; return success or not in success variable
            *success = is_success.into();
        } else {
            let success_block = contract.context.append_basic_block(function, "success");
            let bail_block = contract.context.append_basic_block(function, "bail");

            contract
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            contract.builder.position_at_end(bail_block);

            // should we log "call failed?"
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
        }
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let parameters = contract
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap()
            .into_pointer_value();

        // return the account that returned the value
        contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_struct_gep(parameters, 3, "ka_last_called")
                    .unwrap(),
                "data",
            )
            .into_pointer_value()
    }

    fn return_code<'b>(&self, contract: &'b Contract, ret: IntValue<'b>) {
        contract.builder.build_return(Some(&ret));
    }

    /// Value received
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        contract.value_type().const_zero()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, _contract: &Contract<'b>, _addr: IntValue<'b>) {
        unimplemented!();
    }

    /// Send event
    fn send_event<'b>(
        &self,
        _contract: &Contract<'b>,
        _event_no: usize,
        _data: PointerValue<'b>,
        _data_len: IntValue<'b>,
        _topics: Vec<(PointerValue<'b>, IntValue<'b>)>,
    ) {
        // Solana does not implement events, ignore for now
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        contract: &Contract<'b>,
        expr: &ast::Expression,
        _vartab: &HashMap<usize, Variable<'b>>,
        _function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        match expr {
            ast::Expression::Builtin(_, _, ast::Builtin::Timestamp, _) => {
                let parameters = contract
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap()
                    .get_last_param()
                    .unwrap();

                contract
                    .builder
                    .build_call(
                        contract.module.get_function("sol_timestamp").unwrap(),
                        &[parameters],
                        "timestamp",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::GetAddress, _) => {
                let parameters = contract
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap()
                    .get_last_param()
                    .unwrap()
                    .into_pointer_value();

                let account_id = contract
                    .builder
                    .build_load(
                        contract
                            .builder
                            .build_struct_gep(parameters, 4, "account_id")
                            .unwrap(),
                        "account_id",
                    )
                    .into_pointer_value();

                let value = contract
                    .builder
                    .build_alloca(contract.address_type(), "self_address");

                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                account_id,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                value,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(contract.ns.address_length as u64, false)
                            .into(),
                    ],
                    "",
                );

                contract.builder.build_load(value, "self_address")
            }
            _ => unimplemented!(),
        }
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        contract: &Contract<'b>,
        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
    ) -> IntValue<'b> {
        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("sol_keccak256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("sol_sha256", 32),
            _ => unreachable!(),
        };

        let res = contract.builder.build_array_alloca(
            contract.context.i8_type(),
            contract.context.i32_type().const_int(hashlen, false),
            "res",
        );

        if hash == HashTy::Ripemd160 {
            contract.builder.build_call(
                contract.module.get_function(fname).unwrap(),
                &[input.into(), input_len.into(), res.into()],
                "hash",
            );
        } else {
            let u8_ptr = contract.context.i8_type().ptr_type(AddressSpace::Generic);
            let u64_ty = contract.context.i64_type();

            let sol_bytes = contract
                .context
                .struct_type(&[u8_ptr.into(), u64_ty.into()], false);
            let array = contract.builder.build_alloca(sol_bytes, "sol_bytes");

            contract.builder.build_store(
                contract
                    .builder
                    .build_struct_gep(array, 0, "input")
                    .unwrap(),
                input,
            );

            contract.builder.build_store(
                contract
                    .builder
                    .build_struct_gep(array, 1, "input_len")
                    .unwrap(),
                contract
                    .builder
                    .build_int_z_extend(input_len, u64_ty, "input_len"),
            );

            contract.builder.build_call(
                contract.module.get_function(fname).unwrap(),
                &[
                    array.into(),
                    contract.context.i32_type().const_int(1, false).into(),
                    res.into(),
                ],
                "hash",
            );
        }

        // bytes32 needs to reverse bytes
        let temp = contract
            .builder
            .build_alloca(contract.llvm_type(&ast::Type::Bytes(hashlen as u8)), "hash");

        contract.builder.build_call(
            contract.module.get_function("__beNtoleN").unwrap(),
            &[
                res.into(),
                contract
                    .builder
                    .build_pointer_cast(
                        temp,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        contract.builder.build_load(temp, "hash").into_int_value()
    }
}
