use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use crate::Target;
use std::collections::HashMap;
use std::str;

use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress};
use inkwell::{context::Context, types::BasicTypeEnum};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use num_traits::ToPrimitive;

use super::ethabiencoder;
use super::loop_builder::LoopBuilder;
use super::{Binary, ReturnCode, TargetRuntime, Variable};

pub struct SolanaTarget {
    abi: ethabiencoder::EthAbiDecoder,
    magic: u32,
}

pub struct Contract<'a> {
    magic: u32,
    contract: &'a ast::Contract,
    ns: &'a ast::Namespace,
    storage_initializer: FunctionValue<'a>,
    constructor: Option<(FunctionValue<'a>, &'a Vec<ast::Parameter>)>,
    functions: HashMap<usize, FunctionValue<'a>>,
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
    ) -> Binary<'a> {
        let mut target = SolanaTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: true },
            magic: contract.selector(),
        };

        let mut binary = Binary::new(
            context,
            Target::Solana,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        binary
            .return_values
            .insert(ReturnCode::Success, context.i64_type().const_zero());
        binary.return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        binary.return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        // externals
        target.declare_externals(&mut binary);

        target.emit_functions(&mut binary, contract, ns);

        let storage_initializer = target.emit_initializer(&mut binary, contract, ns);

        let constructor = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor && cfg.public)
            .map(|(cfg_no, cfg)| (binary.functions[&cfg_no], &cfg.params));

        let mut functions = HashMap::new();

        std::mem::swap(&mut functions, &mut binary.functions);

        target.emit_dispatch(
            &mut binary,
            &[Contract {
                magic: target.magic,
                contract,
                ns,
                storage_initializer,
                constructor,
                functions,
            }],
        );

        binary.internalize(&[
            "entrypoint",
            "sol_log_",
            "sol_alloc_free_",
            // This entry is produced by llvm due to merging of stdlib.bc with solidity llvm ir
            "sol_alloc_free_.1",
        ]);

        binary
    }

    /// Build a bundle of contracts from the same namespace
    pub fn build_bundle<'a>(
        context: &'a Context,
        namespaces: &'a [ast::Namespace],
        filename: &str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Binary<'a> {
        let mut target = SolanaTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: true },
            magic: 0,
        };

        let mut binary = Binary::new(
            context,
            Target::Solana,
            "bundle",
            filename,
            opt,
            math_overflow_check,
            None,
        );

        binary
            .return_values
            .insert(ReturnCode::Success, context.i64_type().const_zero());
        binary.return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );
        binary.return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i64_type().const_int(2u64 << 32, false),
        );

        // externals
        target.declare_externals(&mut binary);

        let mut contracts: Vec<Contract> = Vec::new();

        for ns in namespaces {
            for contract in &ns.contracts {
                // We need a magic number for our contract.
                target.magic = contract.selector();

                // Ignore abstract contracts or contract names we have already seen
                if !contract.is_concrete() || contracts.iter().any(|c| c.magic == target.magic) {
                    continue;
                }

                target.emit_functions(&mut binary, contract, ns);

                let storage_initializer = target.emit_initializer(&mut binary, contract, ns);

                let constructor = contract
                    .cfg
                    .iter()
                    .enumerate()
                    .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor && cfg.public)
                    .map(|(cfg_no, cfg)| (binary.functions[&cfg_no], &cfg.params));

                let mut functions = HashMap::new();

                std::mem::swap(&mut functions, &mut binary.functions);

                contracts.push(Contract {
                    magic: target.magic,
                    ns,
                    contract,
                    storage_initializer,
                    constructor,
                    functions,
                });

                binary.functions.drain();
            }
        }

        target.emit_dispatch(&mut binary, &contracts);

        binary.internalize(&[
            "entrypoint",
            "sol_log_",
            "sol_alloc_free_",
            // This entry is produced by llvm due to merging of stdlib.bc with solidity llvm ir
            "sol_alloc_free_.1",
        ]);

        binary
    }

    fn declare_externals(&self, binary: &mut Binary) {
        let void_ty = binary.context.void_type();
        let u8_ptr = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        let u64_ty = binary.context.i64_type();
        let u32_ty = binary.context.i32_type();
        let sol_bytes = binary
            .context
            .struct_type(&[u8_ptr.into(), u64_ty.into()], false)
            .ptr_type(AddressSpace::Generic);

        let function = binary.module.add_function(
            "sol_alloc_free_",
            u8_ptr.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_sha256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_keccak256",
            void_ty.fn_type(&[sol_bytes.into(), u32_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_set_return_data",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = binary.module.add_function(
            "sol_get_return_data",
            u64_ty.fn_type(&[u8_ptr.into(), u64_ty.into(), u8_ptr.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let fields = binary.context.opaque_struct_type("SolLogDataField");

        fields.set_body(&[u8_ptr.into(), u64_ty.into()], false);

        let function = binary.module.add_function(
            "sol_log_data",
            void_ty.fn_type(
                &[fields.ptr_type(AddressSpace::Generic).into(), u64_ty.into()],
                false,
            ),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);
    }

    /// Returns the SolAccountInfo of the executing binary
    fn contract_storage_account<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(binary);

        let ka_cur = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        let account_info = unsafe {
            binary.builder.build_gep(
                parameters,
                &[
                    binary.context.i32_type().const_int(0, false),
                    binary.context.i32_type().const_int(0, false),
                    ka_cur,
                ],
                "account",
            )
        };

        binary.builder.build_pointer_cast(
            account_info,
            binary
                .module
                .get_struct_type("struct.SolAccountInfo")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "account_info",
        )
    }

    /// Get the pointer to SolParameters
    fn sol_parameters<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        let parameters = binary
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_last_param()
            .unwrap();

        binary.builder.build_pointer_cast(
            parameters.into_pointer_value(),
            binary
                .module
                .get_struct_type("struct.SolParameters")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "parameters",
        )
    }

    /// Returns the account data of the executing binary
    fn contract_storage_data<'b>(&self, binary: &Binary<'b>) -> PointerValue<'b> {
        let parameters = self.sol_parameters(binary);

        let ka_cur = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        binary
            .builder
            .build_load(
                unsafe {
                    binary.builder.build_gep(
                        parameters,
                        &[
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                            ka_cur,
                            binary.context.i32_type().const_int(3, false),
                        ],
                        "data",
                    )
                },
                "data",
            )
            .into_pointer_value()
    }

    /// Returns the account data length of the executing binary
    fn contract_storage_datalen<'b>(&self, binary: &Binary<'b>) -> IntValue<'b> {
        let parameters = self.sol_parameters(binary);

        let ka_cur = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(parameters, 2, "ka_cur")
                    .unwrap(),
                "ka_cur",
            )
            .into_int_value();

        binary
            .builder
            .build_load(
                unsafe {
                    binary.builder.build_gep(
                        parameters,
                        &[
                            binary.context.i32_type().const_int(0, false),
                            binary.context.i32_type().const_int(0, false),
                            ka_cur,
                            binary.context.i32_type().const_int(2, false),
                        ],
                        "data_len",
                    )
                },
                "data_len",
            )
            .into_int_value()
    }

    fn emit_dispatch<'b>(&mut self, binary: &mut Binary<'b>, contracts: &[Contract<'b>]) {
        let function = binary.module.get_function("solang_dispatch").unwrap();

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        let sol_params = function.get_nth_param(0).unwrap().into_pointer_value();

        let input = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(sol_params, 5, "input")
                    .unwrap(),
                "data",
            )
            .into_pointer_value();

        let input_len = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(sol_params, 6, "input_len")
                    .unwrap(),
                "data_len",
            )
            .into_int_value();

        // load magic value of binary storage
        binary.parameters = Some(sol_params);

        let storage_data = self.contract_storage_data(binary);

        let magic_value_ptr = binary.builder.build_pointer_cast(
            storage_data,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "magic_value_ptr",
        );

        let magic_value = binary
            .builder
            .build_load(magic_value_ptr, "magic")
            .into_int_value();

        let constructor_block = binary
            .context
            .append_basic_block(function, "constructor_call");
        let badmagic_block = binary.context.append_basic_block(function, "bad_magic");

        // if the magic is zero it's a virgin binary
        // if the magic is our magic value, it's a function call
        // if the magic is another magic value, it is an error

        // Generate function call dispatch
        let function_block = binary.builder.get_insert_block().unwrap();

        let mut cases = vec![(binary.context.i32_type().const_zero(), constructor_block)];

        let input = binary.builder.build_pointer_cast(
            input,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "input_ptr32",
        );

        let dispatch_function_ty = binary.context.i64_type().fn_type(
            &[
                input.get_type().into(),
                input_len.get_type().into(),
                sol_params.get_type().into(),
            ],
            false,
        );

        for contract in contracts {
            let dispatch_function = binary.module.add_function(
                &format!("dispatch_{}", contract.contract.name),
                dispatch_function_ty,
                None,
            );

            let entry = binary
                .context
                .append_basic_block(dispatch_function, "entry");

            binary.builder.position_at_end(entry);

            self.emit_function_dispatch(
                binary,
                contract.contract,
                contract.ns,
                pt::FunctionTy::Function,
                dispatch_function
                    .get_nth_param(0)
                    .unwrap()
                    .into_pointer_value(),
                dispatch_function.get_nth_param(1).unwrap().into_int_value(),
                dispatch_function,
                &contract.functions,
                None,
                |func| func.nonpayable,
            );

            let function_block = binary
                .context
                .append_basic_block(function, &format!("function_{}", contract.contract.name));

            binary.builder.position_at_end(function_block);

            let rc = binary
                .builder
                .build_call(
                    dispatch_function,
                    &[input.into(), input_len.into(), sol_params.into()],
                    "res",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            binary.builder.build_return(Some(&rc));

            cases.push((
                binary
                    .context
                    .i32_type()
                    .const_int(contract.magic as u64, false),
                function_block,
            ));
        }

        binary.builder.position_at_end(badmagic_block);

        binary.builder.build_return(Some(
            &binary.context.i64_type().const_int(4u64 << 32, false),
        ));

        binary.builder.position_at_end(function_block);

        let input = binary.builder.build_pointer_cast(
            input,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "input_ptr32",
        );

        binary
            .builder
            .build_switch(magic_value, badmagic_block, &cases);

        // generate constructor code
        let mut cases = Vec::new();

        binary.builder.position_at_end(constructor_block);

        let contract_data_len = self.contract_storage_datalen(binary);

        for contract in contracts {
            let constructor_block = binary
                .context
                .append_basic_block(function, &format!("constructor_{}", contract.contract.name));

            binary.builder.position_at_end(constructor_block);

            cases.push((
                binary
                    .context
                    .i32_type()
                    .const_int(contract.magic as u64, false),
                constructor_block,
            ));

            // do we have enough binary data
            let fixed_fields_size = contract.contract.fixed_layout_size.to_u64().unwrap();

            let is_enough = binary.builder.build_int_compare(
                IntPredicate::UGE,
                contract_data_len,
                binary
                    .context
                    .i64_type()
                    .const_int(fixed_fields_size, false),
                "is_enough",
            );

            let not_enough = binary.context.append_basic_block(function, "not_enough");
            let enough = binary.context.append_basic_block(function, "enough");

            binary
                .builder
                .build_conditional_branch(is_enough, enough, not_enough);

            binary.builder.position_at_end(not_enough);

            binary.builder.build_return(Some(
                &binary.context.i64_type().const_int(5u64 << 32, false),
            ));

            binary.builder.position_at_end(enough);

            // write our magic value to the binary
            binary.builder.build_store(
                magic_value_ptr,
                binary
                    .context
                    .i32_type()
                    .const_int(contract.magic as u64, false),
            );

            // write heap_offset.
            let heap_offset_ptr = unsafe {
                binary.builder.build_gep(
                    magic_value_ptr,
                    &[binary.context.i64_type().const_int(3, false)],
                    "heap_offset",
                )
            };

            // align heap to 8 bytes
            let heap_offset = (fixed_fields_size + 7) & !7;

            binary.builder.build_store(
                heap_offset_ptr,
                binary.context.i32_type().const_int(heap_offset, false),
            );

            let arg_ty =
                contract.storage_initializer.get_type().get_param_types()[0].into_pointer_type();

            binary.builder.build_call(
                contract.storage_initializer,
                &[binary
                    .builder
                    .build_pointer_cast(sol_params, arg_ty, "")
                    .into()],
                "",
            );

            // is there a not a payable constructor
            if !contract.contract.functions.iter().any(|function_no| {
                let f = &contract.ns.functions[*function_no];
                f.is_constructor() && f.is_payable()
            }) {
                self.abort_if_value_transfer(binary, function, contract.ns);
            }

            // There is only one possible constructor
            let ret = if let Some((constructor_function, params)) = contract.constructor {
                let mut args = Vec::new();

                // insert abi decode
                self.abi.decode(
                    binary,
                    function,
                    &mut args,
                    input,
                    input_len,
                    params,
                    contract.ns,
                );

                let params_ty = constructor_function
                    .get_type()
                    .get_param_types()
                    .last()
                    .unwrap()
                    .into_pointer_type();

                args.push(
                    binary
                        .builder
                        .build_pointer_cast(sol_params, params_ty, "")
                        .into(),
                );

                binary
                    .builder
                    .build_call(constructor_function, &args, "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            } else {
                // return 0 for success
                binary.context.i64_type().const_int(0, false).into()
            };

            binary.builder.build_return(Some(&ret));
        }

        binary.builder.position_at_end(constructor_block);

        let magic_value = binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(sol_params, 9, "contract")
                    .unwrap(),
                "magic",
            )
            .into_int_value();

        binary
            .builder
            .build_switch(magic_value, badmagic_block, &cases);
    }

    /// Free binary storage and zero out
    fn storage_free<'b>(
        &self,
        binary: &Binary<'b>,
        ty: &ast::Type,
        data: PointerValue<'b>,
        slot: IntValue<'b>,
        function: FunctionValue<'b>,
        zero: bool,
        ns: &ast::Namespace,
    ) {
        if !zero && !ty.is_dynamic(ns) {
            // nothing to do
            return;
        }

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            binary.builder.build_call(
                binary.module.get_function("account_data_free").unwrap(),
                &[data.into(), offset.into()],
                "",
            );

            // account_data_alloc will return 0 if the string is length 0
            let new_offset = binary.context.i32_type().const_zero();

            binary.builder.build_store(offset_ptr, new_offset);
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // delete the existing storage
            let mut elem_slot = slot;

            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            if elem_ty.is_dynamic(ns) || zero {
                let length = if let Some(length) = dim[0].as_ref() {
                    binary
                        .context
                        .i32_type()
                        .const_int(length.to_u64().unwrap(), false)
                } else {
                    elem_slot = binary
                        .builder
                        .build_load(offset_ptr, "offset")
                        .into_int_value();

                    self.storage_array_length(binary, function, slot, elem_ty, ns)
                };

                let elem_size = elem_ty.size_of(ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(binary, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(binary, "offset", slot.get_type(), elem_slot.into());

                let _ = builder.over(binary, binary.context.i32_type().const_zero(), length);

                let offset_val = offset_phi.into_int_value();

                let elem_ty = ty.array_deref();

                self.storage_free(
                    binary,
                    elem_ty.deref_any(),
                    data,
                    offset_val,
                    function,
                    zero,
                    ns,
                );

                let offset_val = binary.builder.build_int_add(
                    offset_val,
                    binary.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(binary, "offset", offset_val.into());

                // done
                builder.finish(binary);
            }

            // if the array was dynamic, free the array itself
            if dim[0].is_none() {
                let slot = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                binary.builder.build_call(
                    binary.module.get_function("account_data_free").unwrap(),
                    &[data.into(), slot.into()],
                    "",
                );

                // account_data_alloc will return 0 if the string is length 0
                let new_offset = binary.context.i32_type().const_zero();

                binary.builder.build_store(offset_ptr, new_offset);
            }
        } else if let ast::Type::Struct(struct_no) = ty {
            for (i, field) in ns.structs[*struct_no].fields.iter().enumerate() {
                let field_offset = ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                let offset = binary.builder.build_int_add(
                    slot,
                    binary.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                self.storage_free(binary, &field.ty, data, offset, function, zero, ns);
            }
        } else {
            let ty = binary.llvm_type(ty, ns);

            binary.builder.build_store(
                binary
                    .builder
                    .build_pointer_cast(member, ty.ptr_type(AddressSpace::Generic), ""),
                ty.into_int_type().const_zero(),
            );
        }
    }

    /// An entry in a sparse array or mapping
    fn sparse_entry<'b>(
        &self,
        binary: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> BasicTypeEnum<'b> {
        let key = if matches!(
            key_ty,
            ast::Type::String | ast::Type::DynamicBytes | ast::Type::Mapping(_, _)
        ) {
            binary.context.i32_type().into()
        } else {
            binary.llvm_type(key_ty, ns)
        };

        binary
            .context
            .struct_type(
                &[
                    key,                              // key
                    binary.context.i32_type().into(), // next field
                    if value_ty.is_mapping() {
                        binary.context.i32_type().into()
                    } else {
                        binary.llvm_type(value_ty, ns) // value
                    },
                ],
                false,
            )
            .into()
    }

    /// Generate sparse lookup
    fn sparse_lookup_function<'b>(
        &self,
        binary: &Binary<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> FunctionValue<'b> {
        let function_name = format!(
            "sparse_lookup_{}_{}",
            key_ty.to_llvm_string(ns),
            value_ty.to_llvm_string(ns)
        );

        if let Some(function) = binary.module.get_function(&function_name) {
            return function;
        }

        // The function takes an offset (of the mapping or sparse array), the key which
        // is the index, and it should return an offset.
        let function_ty = binary.function_type(
            &[ast::Type::Uint(32), key_ty.clone()],
            &[ast::Type::Uint(32)],
            ns,
        );

        let function =
            binary
                .module
                .add_function(&function_name, function_ty, Some(Linkage::Internal));

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        let offset = function.get_nth_param(0).unwrap().into_int_value();
        let key = function.get_nth_param(1).unwrap();

        let entry_ty = self.sparse_entry(binary, key_ty, value_ty, ns);
        let value_offset = unsafe {
            entry_ty
                .ptr_type(AddressSpace::Generic)
                .const_null()
                .const_gep(&[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(2, false),
                ])
                .const_to_int(binary.context.i32_type())
        };

        let data = self.contract_storage_data(binary);

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        // calculate the correct bucket. We have an prime number of
        let bucket = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            binary
                .builder
                .build_call(
                    binary.module.get_function("vector_hash").unwrap(),
                    &[key],
                    "hash",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else if key_ty.bits(ns) > 64 {
            binary
                .builder
                .build_int_truncate(key.into_int_value(), binary.context.i64_type(), "")
        } else {
            key.into_int_value()
        };

        let bucket = binary.builder.build_int_unsigned_rem(
            bucket,
            bucket
                .get_type()
                .const_int(crate::sema::SOLANA_BUCKET_SIZE, false),
            "",
        );

        let first_offset_ptr = unsafe {
            binary
                .builder
                .build_gep(offset_ptr, &[bucket], "bucket_list")
        };

        // we should now loop until offset is zero or we found it
        let loop_entry = binary.context.append_basic_block(function, "loop_entry");
        let end_of_bucket = binary.context.append_basic_block(function, "end_of_bucket");
        let examine_bucket = binary
            .context
            .append_basic_block(function, "examine_bucket");
        let found_entry = binary.context.append_basic_block(function, "found_entry");
        let next_entry = binary.context.append_basic_block(function, "next_entry");

        // let's enter the loop
        binary.builder.build_unconditional_branch(loop_entry);

        binary.builder.position_at_end(loop_entry);

        // we are walking the bucket list via the offset ptr
        let offset_ptr_phi = binary.builder.build_phi(
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        offset_ptr_phi.add_incoming(&[(&first_offset_ptr, entry)]);

        // load the offset and check for zero (end of bucket list)
        let offset = binary
            .builder
            .build_load(
                offset_ptr_phi.as_basic_value().into_pointer_value(),
                "offset",
            )
            .into_int_value();

        let is_offset_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            offset,
            offset.get_type().const_zero(),
            "offset_is_zero",
        );

        binary
            .builder
            .build_conditional_branch(is_offset_zero, end_of_bucket, examine_bucket);

        binary.builder.position_at_end(examine_bucket);

        // let's compare the key in this entry to the key we are looking for
        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };
        let entry_ptr = binary.builder.build_pointer_cast(
            member,
            entry_ty.ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let entry_key = binary
            .builder
            .build_load(
                unsafe {
                    binary.builder.build_gep(
                        entry_ptr,
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_zero(),
                        ],
                        "key_ptr",
                    )
                },
                "key",
            )
            .into_int_value();

        let matches = if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            // entry_key is an offset
            let entry_data = unsafe { binary.builder.build_gep(data, &[entry_key], "data") };
            let entry_length = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_len").unwrap(),
                    &[data.into(), entry_key.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            binary
                .builder
                .build_call(
                    binary.module.get_function("__memcmp").unwrap(),
                    &[
                        entry_data.into(),
                        entry_length.into(),
                        binary.vector_bytes(key).into(),
                        binary.vector_len(key).into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            binary.builder.build_int_compare(
                IntPredicate::EQ,
                key.into_int_value(),
                entry_key,
                "matches",
            )
        };

        binary
            .builder
            .build_conditional_branch(matches, found_entry, next_entry);

        binary.builder.position_at_end(found_entry);

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        binary.builder.build_store(
            ret_offset,
            binary
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_zero()));

        binary.builder.position_at_end(next_entry);

        let offset_ptr = binary
            .builder
            .build_struct_gep(entry_ptr, 1, "offset_ptr")
            .unwrap();

        offset_ptr_phi.add_incoming(&[(&offset_ptr, next_entry)]);

        binary.builder.build_unconditional_branch(loop_entry);

        let offset_ptr = offset_ptr_phi.as_basic_value().into_pointer_value();

        binary.builder.position_at_end(end_of_bucket);

        let entry_length = entry_ty
            .size_of()
            .unwrap()
            .const_cast(binary.context.i32_type(), false);

        let account = self.contract_storage_account(binary);

        let account_data_alloc = binary.module.get_function("account_data_alloc").unwrap();

        let arg1 = binary.builder.build_pointer_cast(
            account,
            account_data_alloc.get_type().get_param_types()[0].into_pointer_type(),
            "",
        );

        // account_data_alloc will return offset = 0 if the string is length 0
        let rc = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_alloc").unwrap(),
                &[arg1.into(), entry_length.into(), offset_ptr.into()],
                "rc",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_rc_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            binary.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = binary.context.append_basic_block(function, "rc_zero");

        binary
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        binary.builder.position_at_end(rc_not_zero);

        self.return_code(binary, rc);

        binary.builder.position_at_end(rc_zero);

        let offset = binary
            .builder
            .build_load(offset_ptr, "new_offset")
            .into_int_value();

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };

        // Clear memory. The length argument to __bzero8 is in lengths of 8 bytes. We round up to the nearest
        // 8 byte, since account_data_alloc also rounds up to the nearest 8 byte when allocating.
        let length = binary.builder.build_int_unsigned_div(
            binary.builder.build_int_add(
                entry_length,
                binary.context.i32_type().const_int(7, false),
                "",
            ),
            binary.context.i32_type().const_int(8, false),
            "length_div_8",
        );

        binary.builder.build_call(
            binary.module.get_function("__bzero8").unwrap(),
            &[member.into(), length.into()],
            "zeroed",
        );

        let entry_ptr = binary.builder.build_pointer_cast(
            member,
            entry_ty.ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        // set key
        if matches!(key_ty, ast::Type::String | ast::Type::DynamicBytes) {
            let new_string_length = binary.vector_len(key);
            let offset_ptr = binary
                .builder
                .build_struct_gep(entry_ptr, 0, "key_ptr")
                .unwrap();

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = binary.builder.build_int_compare(
                IntPredicate::EQ,
                rc,
                binary.context.i64_type().const_zero(),
                "is_rc_zero",
            );

            let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = binary.context.append_basic_block(function, "rc_zero");
            let memcpy = binary.context.append_basic_block(function, "memcpy");

            binary
                .builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

            binary.builder.position_at_end(rc_not_zero);

            self.return_code(
                binary,
                binary.context.i64_type().const_int(5u64 << 32, false),
            );

            binary.builder.position_at_end(rc_zero);

            let new_offset = binary.builder.build_load(offset_ptr, "new_offset");

            binary.builder.build_unconditional_branch(memcpy);

            binary.builder.position_at_end(memcpy);

            let offset_phi = binary
                .builder
                .build_phi(binary.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                binary.builder.build_gep(
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            binary.builder.build_call(
                binary.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    binary.vector_bytes(key).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else {
            let key_ptr = binary
                .builder
                .build_struct_gep(entry_ptr, 0, "key_ptr")
                .unwrap();

            binary.builder.build_store(key_ptr, key);
        };

        let ret_offset = function.get_nth_param(2).unwrap().into_pointer_value();

        binary.builder.build_store(
            ret_offset,
            binary
                .builder
                .build_int_add(offset, value_offset, "value_offset"),
        );

        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_zero()));

        function
    }

    /// Do a lookup/subscript in a sparse array or mapping; this will call a function
    fn sparse_lookup<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        key_ty: &ast::Type,
        value_ty: &ast::Type,
        slot: IntValue<'b>,
        index: BasicValueEnum<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let offset = binary.build_alloca(function, binary.context.i32_type(), "offset");

        let current_block = binary.builder.get_insert_block().unwrap();

        let lookup = self.sparse_lookup_function(binary, key_ty, value_ty, ns);

        binary.builder.position_at_end(current_block);

        let parameters = self.sol_parameters(binary);

        let rc = binary
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
        let is_rc_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            rc.get_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = binary.context.append_basic_block(function, "rc_zero");

        binary
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        binary.builder.position_at_end(rc_not_zero);

        self.return_code(binary, rc);

        binary.builder.position_at_end(rc_zero);

        binary.builder.build_load(offset, "offset").into_int_value()
    }
}

impl<'a> TargetRuntime<'a> for SolanaTarget {
    /// Solana does not use slot based-storage so override
    fn storage_delete(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) {
        // binary storage is in 2nd account
        let data = self.contract_storage_data(binary);

        self.storage_free(binary, ty, data, *slot, function, true, ns);
    }

    fn set_storage_extfunc(
        &self,
        _binary: &Binary,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        // unused
        unreachable!();
    }

    fn get_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        // unused
        unreachable!();
    }

    fn get_storage_bytes_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let get_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, get_block, bang_block);

        binary.builder.position_at_end(bang_block);

        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(get_block);

        let offset = binary.builder.build_int_add(offset, index, "offset");

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };

        binary.builder.build_load(member, "val").into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        binary: &Binary,
        function: FunctionValue,
        slot: IntValue,
        index: IntValue,
        val: IntValue,
    ) {
        let data = self.contract_storage_data(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range =
            binary
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let set_block = binary.context.append_basic_block(function, "in_range");
        let bang_block = binary.context.append_basic_block(function, "bang_block");

        binary
            .builder
            .build_conditional_branch(in_range, set_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        binary.builder.position_at_end(set_block);

        let offset = binary.builder.build_int_add(offset, index, "offset");

        let member = unsafe { binary.builder.build_gep(data, &[offset], "data") };

        binary.builder.build_store(member, val);
    }

    fn storage_subscript(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
        ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let account = self.contract_storage_account(binary);

        if let ast::Type::Mapping(key, value) = ty.deref_any() {
            self.sparse_lookup(binary, function, key, value, slot, index, ns)
        } else if ty.is_sparse_solana(ns) {
            // sparse array
            let elem_ty = ty.storage_array_elem().deref_into();

            let key = ast::Type::Uint(256);

            self.sparse_lookup(binary, function, &key, &elem_ty, slot, index, ns)
        } else {
            // 3rd member of account is data pointer
            let data = unsafe {
                binary.builder.build_gep(
                    account,
                    &[
                        binary.context.i32_type().const_zero(),
                        binary.context.i32_type().const_int(3, false),
                    ],
                    "data",
                )
            };

            let data = binary.builder.build_load(data, "data").into_pointer_value();

            let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let elem_ty = ty.storage_array_elem().deref_into();

            let elem_size = binary
                .context
                .i32_type()
                .const_int(elem_ty.size_of(ns).to_u64().unwrap(), false);

            binary.builder.build_int_add(
                offset,
                binary
                    .builder
                    .build_int_mul(index.into_int_value(), elem_size, ""),
                "",
            )
        }
    }

    fn storage_push(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        val: BasicValueEnum<'a>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let member_size = binary
            .context
            .i32_type()
            .const_int(ty.size_of(ns).to_u64().unwrap(), false);
        let new_length = binary
            .builder
            .build_int_add(length, member_size, "new_length");

        let rc = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_realloc").unwrap(),
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

        let is_rc_zero = binary.builder.build_int_compare(
            IntPredicate::EQ,
            rc,
            binary.context.i64_type().const_zero(),
            "is_rc_zero",
        );

        let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
        let rc_zero = binary.context.append_basic_block(function, "rc_zero");

        binary
            .builder
            .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

        binary.builder.position_at_end(rc_not_zero);

        self.return_code(
            binary,
            binary.context.i64_type().const_int(5u64 << 32, false),
        );

        binary.builder.position_at_end(rc_zero);

        let mut new_offset = binary.builder.build_int_add(
            binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value(),
            length,
            "",
        );

        self.storage_store(binary, ty, &mut new_offset, val, function, ns);

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
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        slot: IntValue<'a>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };
        let offset_ptr = binary.builder.build_pointer_cast(
            member,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "offset_ptr",
        );

        let offset = binary
            .builder
            .build_load(offset_ptr, "offset")
            .into_int_value();

        let length = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // do bounds check on index
        let in_range = binary.builder.build_int_compare(
            IntPredicate::NE,
            binary.context.i32_type().const_zero(),
            length,
            "index_in_range",
        );

        let bang_block = binary.context.append_basic_block(function, "bang_block");
        let retrieve_block = binary.context.append_basic_block(function, "in_range");

        binary
            .builder
            .build_conditional_branch(in_range, retrieve_block, bang_block);

        binary.builder.position_at_end(bang_block);
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );

        let member_size = binary
            .context
            .i32_type()
            .const_int(ty.size_of(ns).to_u64().unwrap(), false);

        binary.builder.position_at_end(retrieve_block);

        let new_length = binary
            .builder
            .build_int_sub(length, member_size, "new_length");

        let mut new_offset = binary.builder.build_int_add(offset, new_length, "");

        let val = self.storage_load(binary, ty, &mut new_offset, function, ns);

        // delete existing storage -- pointers need to be freed
        //self.storage_free(binary, ty, account, data, new_offset, function, false);

        // we can assume pointer will stay the same after realloc to smaller size
        binary.builder.build_call(
            binary.module.get_function("account_data_realloc").unwrap(),
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
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        elem_ty: &ast::Type,
        ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let data = self.contract_storage_data(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[slot], "data") };

        let offset = binary
            .builder
            .build_load(
                binary.builder.build_pointer_cast(
                    member,
                    binary.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                "offset",
            )
            .into_int_value();

        let member_size = binary
            .context
            .i32_type()
            .const_int(elem_ty.size_of(ns).to_u64().unwrap(), false);

        let length_bytes = binary
            .builder
            .build_call(
                binary.module.get_function("account_data_len").unwrap(),
                &[data.into(), offset.into()],
                "length",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        binary
            .builder
            .build_int_unsigned_div(length_bytes, member_size, "")
    }

    fn get_storage_int(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ty: IntType<'a>,
    ) -> IntValue<'a> {
        // unused
        unreachable!();
    }

    /// Recursively load a type from binary storage. This overrides the default method
    /// in the trait, which is for chains with 256 bit storage keys.
    fn storage_load(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        let data = self.contract_storage_data(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[*slot], "data") };

        match ty {
            ast::Type::String | ast::Type::DynamicBytes => {
                let offset = binary
                    .builder
                    .build_load(
                        binary.builder.build_pointer_cast(
                            member,
                            binary.context.i32_type().ptr_type(AddressSpace::Generic),
                            "",
                        ),
                        "offset",
                    )
                    .into_int_value();

                let string_length = binary
                    .builder
                    .build_call(
                        binary.module.get_function("account_data_len").unwrap(),
                        &[data.into(), offset.into()],
                        "free",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let string_data =
                    unsafe { binary.builder.build_gep(data, &[offset], "string_data") };

                binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            string_length.into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            string_data.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Type::Struct(struct_no) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);
                // LLVMSizeOf() produces an i64
                let size = binary.builder.build_int_truncate(
                    llvm_ty.size_of().unwrap(),
                    binary.context.i32_type(),
                    "size_of",
                );

                let new = binary
                    .builder
                    .build_call(
                        binary.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let dest = binary.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    "dest",
                );

                for (i, field) in ns.structs[*struct_no].fields.iter().enumerate() {
                    let field_offset = ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                    let mut offset = binary.builder.build_int_add(
                        *slot,
                        binary.context.i32_type().const_int(field_offset, false),
                        "field_offset",
                    );

                    let val = self.storage_load(binary, &field.ty, &mut offset, function, ns);

                    let elem = unsafe {
                        binary.builder.build_gep(
                            dest,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    binary.builder.build_store(elem, val);
                }

                dest.into()
            }
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                let dest;
                let length;
                let mut slot = *slot;

                if dim[0].is_some() {
                    // LLVMSizeOf() produces an i64 and malloc takes i32
                    let size = binary.builder.build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        binary.context.i32_type(),
                        "size_of",
                    );

                    let new = binary
                        .builder
                        .build_call(
                            binary.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    dest = binary.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );
                    length = binary
                        .context
                        .i32_type()
                        .const_int(dim[0].as_ref().unwrap().to_u64().unwrap(), false);
                } else {
                    let elem_size = binary.builder.build_int_truncate(
                        binary
                            .context
                            .i32_type()
                            .const_int(elem_ty.size_of(ns).to_u64().unwrap(), false),
                        binary.context.i32_type(),
                        "size_of",
                    );

                    length = self.storage_array_length(binary, function, slot, elem_ty, ns);

                    slot = binary
                        .builder
                        .build_load(
                            binary.builder.build_pointer_cast(
                                member,
                                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                                "",
                            ),
                            "offset",
                        )
                        .into_int_value();

                    dest = binary.vector_new(length, elem_size, None);
                };

                let elem_size = elem_ty.size_of(ns).to_u64().unwrap();

                // loop over the array
                let mut builder = LoopBuilder::new(binary, function);

                // we need a phi for the offset
                let offset_phi =
                    builder.add_loop_phi(binary, "offset", slot.get_type(), slot.into());

                let index = builder.over(binary, binary.context.i32_type().const_zero(), length);

                let elem = binary.array_subscript(ty.deref_any(), dest, index, ns);

                let elem_ty = ty.array_deref();

                let mut offset_val = offset_phi.into_int_value();

                let val = self.storage_load(
                    binary,
                    elem_ty.deref_memory(),
                    &mut offset_val,
                    function,
                    ns,
                );

                binary.builder.build_store(elem, val);

                offset_val = binary.builder.build_int_add(
                    offset_val,
                    binary.context.i32_type().const_int(elem_size, false),
                    "new_offset",
                );

                // set the offset for the next iteration of the loop
                builder.set_loop_phi_value(binary, "offset", offset_val.into());

                // done
                builder.finish(binary);

                dest.into()
            }
            _ => binary.builder.build_load(
                binary.builder.build_pointer_cast(
                    member,
                    binary.llvm_type(ty, ns).ptr_type(AddressSpace::Generic),
                    "",
                ),
                "",
            ),
        }
    }

    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        val: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
    ) {
        let data = self.contract_storage_data(binary);
        let account = self.contract_storage_account(binary);

        // the slot is simply the offset after the magic
        let member = unsafe { binary.builder.build_gep(data, &[*slot], "data") };

        if *ty == ast::Type::String || *ty == ast::Type::DynamicBytes {
            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = binary
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let existing_string_length = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_len").unwrap(),
                    &[data.into(), offset.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let new_string_length = binary.vector_len(val);

            let allocation_necessary = binary.builder.build_int_compare(
                IntPredicate::NE,
                existing_string_length,
                new_string_length,
                "allocation_necessary",
            );

            let entry = binary.builder.get_insert_block().unwrap();

            let realloc = binary.context.append_basic_block(function, "realloc");
            let memcpy = binary.context.append_basic_block(function, "memcpy");

            binary
                .builder
                .build_conditional_branch(allocation_necessary, realloc, memcpy);

            binary.builder.position_at_end(realloc);

            // do not realloc since we're copying everything
            binary.builder.build_call(
                binary.module.get_function("account_data_free").unwrap(),
                &[data.into(), offset.into()],
                "free",
            );

            // account_data_alloc will return offset = 0 if the string is length 0
            let rc = binary
                .builder
                .build_call(
                    binary.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into(), offset_ptr.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let is_rc_zero = binary.builder.build_int_compare(
                IntPredicate::EQ,
                rc,
                binary.context.i64_type().const_zero(),
                "is_rc_zero",
            );

            let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
            let rc_zero = binary.context.append_basic_block(function, "rc_zero");

            binary
                .builder
                .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

            binary.builder.position_at_end(rc_not_zero);

            self.return_code(
                binary,
                binary.context.i64_type().const_int(5u64 << 32, false),
            );

            binary.builder.position_at_end(rc_zero);

            let new_offset = binary.builder.build_load(offset_ptr, "new_offset");

            binary.builder.build_unconditional_branch(memcpy);

            binary.builder.position_at_end(memcpy);

            let offset_phi = binary
                .builder
                .build_phi(binary.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, rc_zero), (&offset, entry)]);

            let dest_string_data = unsafe {
                binary.builder.build_gep(
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            binary.builder.build_call(
                binary.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    binary.vector_bytes(val).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else if let ast::Type::Array(elem_ty, dim) = ty {
            // make sure any pointers are freed
            self.storage_free(binary, ty, data, *slot, function, false, ns);

            let offset_ptr = binary.builder.build_pointer_cast(
                member,
                binary.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let length = if let Some(length) = dim[0].as_ref() {
                binary
                    .context
                    .i32_type()
                    .const_int(length.to_u64().unwrap(), false)
            } else {
                binary.vector_len(val)
            };

            let mut elem_slot = *slot;

            if dim[0].is_none() {
                // reallocate to the right size
                let member_size = binary
                    .context
                    .i32_type()
                    .const_int(elem_ty.size_of(ns).to_u64().unwrap(), false);
                let new_length = binary
                    .builder
                    .build_int_mul(length, member_size, "new_length");
                let offset = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();

                let rc = binary
                    .builder
                    .build_call(
                        binary.module.get_function("account_data_realloc").unwrap(),
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

                let is_rc_zero = binary.builder.build_int_compare(
                    IntPredicate::EQ,
                    rc,
                    binary.context.i64_type().const_zero(),
                    "is_rc_zero",
                );

                let rc_not_zero = binary.context.append_basic_block(function, "rc_not_zero");
                let rc_zero = binary.context.append_basic_block(function, "rc_zero");

                binary
                    .builder
                    .build_conditional_branch(is_rc_zero, rc_zero, rc_not_zero);

                binary.builder.position_at_end(rc_not_zero);

                self.return_code(
                    binary,
                    binary.context.i64_type().const_int(5u64 << 32, false),
                );

                binary.builder.position_at_end(rc_zero);

                elem_slot = binary
                    .builder
                    .build_load(offset_ptr, "offset")
                    .into_int_value();
            }

            let elem_size = elem_ty.size_of(ns).to_u64().unwrap();

            // loop over the array
            let mut builder = LoopBuilder::new(binary, function);

            // we need a phi for the offset
            let offset_phi =
                builder.add_loop_phi(binary, "offset", slot.get_type(), elem_slot.into());

            let index = builder.over(binary, binary.context.i32_type().const_zero(), length);

            let elem = binary.array_subscript(ty, val.into_pointer_value(), index, ns);

            let mut offset_val = offset_phi.into_int_value();

            let elem_ty = ty.array_deref();

            self.storage_store(
                binary,
                elem_ty.deref_any(),
                &mut offset_val,
                binary.builder.build_load(elem, "array_elem"),
                function,
                ns,
            );

            offset_val = binary.builder.build_int_add(
                offset_val,
                binary.context.i32_type().const_int(elem_size, false),
                "new_offset",
            );

            // set the offset for the next iteration of the loop
            builder.set_loop_phi_value(binary, "offset", offset_val.into());

            // done
            builder.finish(binary);
        } else if let ast::Type::Struct(struct_no) = ty {
            for (i, field) in ns.structs[*struct_no].fields.iter().enumerate() {
                let field_offset = ns.structs[*struct_no].offsets[i].to_u64().unwrap();

                let mut offset = binary.builder.build_int_add(
                    *slot,
                    binary.context.i32_type().const_int(field_offset, false),
                    "field_offset",
                );

                let elem = unsafe {
                    binary.builder.build_gep(
                        val.into_pointer_value(),
                        &[
                            binary.context.i32_type().const_zero(),
                            binary.context.i32_type().const_int(i as u64, false),
                        ],
                        &field.name,
                    )
                };

                // free any existing dynamic storage
                self.storage_free(binary, &field.ty, data, offset, function, false, ns);

                self.storage_store(
                    binary,
                    &field.ty,
                    &mut offset,
                    binary.builder.build_load(elem, &field.name),
                    function,
                    ns,
                );
            }
        } else {
            binary.builder.build_store(
                binary.builder.build_pointer_cast(
                    member,
                    val.get_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                val,
            );
        }
    }

    fn keccak256_hash(
        &self,
        _binary: &Binary,
        _src: PointerValue,
        _length: IntValue,
        _dest: PointerValue,
        _ns: &ast::Namespace,
    ) {
        unreachable!();
    }

    fn return_empty_abi(&self, binary: &Binary) {
        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        // set return data
        binary.builder.build_call(
            binary.module.get_function("sol_set_return_data").unwrap(),
            &[
                data.into(),
                binary
                    .builder
                    .build_int_z_extend(length, binary.context.i64_type(), "length")
                    .into(),
            ],
            "",
        );

        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i64_type().const_int(0, false)));
    }

    fn assert_failure<'b>(&self, binary: &'b Binary, data: PointerValue, length: IntValue) {
        // the reason code should be null (and already printed)
        binary.builder.build_call(
            binary.module.get_function("sol_set_return_data").unwrap(),
            &[
                data.into(),
                binary
                    .builder
                    .build_int_z_extend(length, binary.context.i64_type(), "length")
                    .into(),
            ],
            "",
        );

        // return 1 for failure
        binary.builder.build_return(Some(
            &binary.context.i64_type().const_int(1u64 << 32, false),
        ));
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> PointerValue<'b> {
        ethabiencoder::encode_to_vector(binary, function, packed, args, tys, true, ns)
    }

    fn abi_encode(
        &self,
        binary: &Binary<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue<'a>,
        args: &[BasicValueEnum<'a>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        debug_assert_eq!(args.len(), tys.len());

        let mut tys = tys.to_vec();

        let packed = if let Some(selector) = selector {
            tys.insert(0, ast::Type::Uint(32));
            vec![selector.into()]
        } else {
            vec![]
        };

        let encoder = ethabiencoder::EncoderBuilder::new(
            binary, function, load, &packed, args, &tys, true, ns,
        );

        let length = encoder.encoded_length();

        let encoded_data = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        encoder.finish(binary, function, encoded_data, ns);

        (encoded_data, length)
    }

    fn abi_decode<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
        ns: &ast::Namespace,
    ) {
        self.abi
            .decode(binary, function, args, data, length, spec, ns);
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        let string_len64 =
            binary
                .builder
                .build_int_z_extend(string_len, binary.context.i64_type(), "");

        binary.builder.build_call(
            binary.module.get_function("sol_log_").unwrap(),
            &[string_ptr.into(), string_len64.into()],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        _gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
        space: Option<IntValue<'b>>,
        ns: &ast::Namespace,
    ) {
        // abi encode the arguments. The
        let mut tys = vec![
            ast::Type::Uint(64),
            ast::Type::Bytes(4),
            ast::Type::Bytes(1),
        ];

        if let Some(function_no) = constructor_no {
            for param in &ns.functions[function_no].params {
                tys.push(param.ty.clone());
            }
        };

        let value = if let Some(value) = value {
            value
        } else {
            binary.context.i64_type().const_zero()
        };

        let packed = [
            value.into(),
            binary
                .context
                .i32_type()
                .const_int(ns.contracts[contract_no].selector().to_be() as u64, false)
                .into(),
            binary.context.i8_type().const_zero().into(),
        ];

        let encoder = ethabiencoder::EncoderBuilder::new(
            binary, function, false, &packed, args, &tys, true, ns,
        );

        let length = encoder.encoded_length();
        let address_length = binary
            .context
            .i32_type()
            .const_int(ns.address_length as u64 * 2, false);

        let malloc_length = binary
            .builder
            .build_int_add(length, address_length, "malloc_length");

        // The format of the payload is:
        // 32 bytes recv (will be filled in by create_contract C function)
        // 32 bytes sender (will be filled in by create_contract C function)
        // 4 bytes contract selector/magic
        // remainder: eth abi encoded constructor arguments
        let payload = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let enc = unsafe { binary.builder.build_gep(payload, &[address_length], "enc") };

        encoder.finish(binary, function, enc, ns);

        let space = binary.builder.build_int_add(
            binary.context.i64_type().const_int(
                ns.contracts[contract_no]
                    .fixed_layout_size
                    .to_u64()
                    .unwrap(),
                false,
            ),
            space.unwrap_or_else(|| binary.context.i64_type().const_int(1024, false)),
            "space",
        );

        let sol_params = function.get_last_param().unwrap().into_pointer_value();

        let create_contract = binary.module.get_function("create_contract").unwrap();

        let arg4 = binary.builder.build_pointer_cast(
            sol_params,
            create_contract.get_type().get_param_types()[3].into_pointer_type(),
            "",
        );

        let ret = binary
            .builder
            .build_call(
                create_contract,
                &[
                    payload.into(),
                    malloc_length.into(),
                    space.into(),
                    arg4.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        binary.builder.build_call(
            binary.module.get_function("__beNtoleN").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        payload,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .context
                    .i32_type()
                    .const_int(ns.address_length as u64, false)
                    .into(),
            ],
            "",
        );

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i64_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // do not abort execution; return success or not in success variable
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");

            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            binary.builder.build_return(Some(&ret));

            binary.builder.position_at_end(success_block);
        }
    }

    /// Call external binary
    fn external_call<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        _ty: ast::CallTy,
        _ns: &ast::Namespace,
    ) {
        debug_assert!(address.is_none());

        let parameters = self.sol_parameters(binary);

        let external_call = binary.module.get_function("external_call").unwrap();

        let arg2 = binary.builder.build_pointer_cast(
            parameters,
            external_call.get_type().get_param_types()[2].into_pointer_type(),
            "",
        );

        let ret = binary
            .builder
            .build_call(
                external_call,
                &[payload.into(), payload_len.into(), arg2.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i64_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            // we're in a try statement. This means:
            // do not abort execution; return success or not in success variable
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");

            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            // should we log "call failed?"
            self.assert_failure(
                binary,
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                binary.context.i32_type().const_zero(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    /// Get return buffer for external call
    fn return_data<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
    ) -> PointerValue<'b> {
        let null_u8_ptr = binary
            .context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .const_zero();

        let length_as_64 = binary
            .builder
            .build_call(
                binary.module.get_function("sol_get_return_data").unwrap(),
                &[
                    null_u8_ptr.into(),
                    binary.context.i64_type().const_zero().into(),
                    null_u8_ptr.into(),
                ],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let length =
            binary
                .builder
                .build_int_truncate(length_as_64, binary.context.i32_type(), "length");

        let malloc_length = binary.builder.build_int_add(
            length,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .size_of()
                .unwrap()
                .const_cast(binary.context.i32_type(), false),
            "size",
        );

        let p = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let v = binary.builder.build_pointer_cast(
            p,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "string",
        );

        let data_len = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_zero(),
                ],
                "data_len",
            )
        };

        binary.builder.build_store(data_len, length);

        let data_size = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(1, false),
                ],
                "data_size",
            )
        };

        binary.builder.build_store(data_size, length);

        let data = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(2, false),
                ],
                "data",
            )
        };

        let program_id = binary.build_array_alloca(
            function,
            binary.context.i8_type(),
            binary.context.i32_type().const_int(32, false),
            "program_id",
        );

        binary.builder.build_call(
            binary.module.get_function("sol_get_return_data").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        data,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                length_as_64.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        program_id,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        v
    }

    fn return_code<'b>(&self, binary: &'b Binary, ret: IntValue<'b>) {
        binary.builder.build_return(Some(&ret));
    }

    /// Value received
    fn value_transferred<'b>(&self, binary: &Binary<'b>, _ns: &ast::Namespace) -> IntValue<'b> {
        let parameters = self.sol_parameters(binary);

        binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_struct_gep(parameters, 14, "value")
                    .unwrap(),
                "value",
            )
            .into_int_value()
    }

    /// Terminate execution, destroy binary and send remaining funds to addr
    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: IntValue<'b>, _ns: &ast::Namespace) {
        unimplemented!();
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        binary: &Binary<'b>,
        _contract: &ast::Contract,
        function: FunctionValue<'b>,
        _event_no: usize,
        data: &[BasicValueEnum<'b>],
        data_tys: &[ast::Type],
        topics: &[BasicValueEnum<'b>],
        topic_tys: &[ast::Type],
        ns: &ast::Namespace,
    ) {
        let fields = binary.build_array_alloca(
            function,
            binary.module.get_struct_type("SolLogDataField").unwrap(),
            binary.context.i32_type().const_int(2, false),
            "fields",
        );

        let (topic_ptr, topic_len) =
            self.abi_encode(binary, None, false, function, topics, topic_tys, ns);

        let field_data = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_zero(),
                ],
                "field_data",
            )
        };

        binary.builder.build_store(field_data, topic_ptr);

        let field_len = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(1, false),
                ],
                "field_len",
            )
        };

        binary.builder.build_store(
            field_len,
            binary
                .builder
                .build_int_z_extend(topic_len, binary.context.i64_type(), "topic_len64"),
        );

        let (data_ptr, data_len) =
            self.abi_encode(binary, None, false, function, data, data_tys, ns);

        let field_data = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_int(1, false),
                    binary.context.i32_type().const_zero(),
                ],
                "field_data",
            )
        };

        binary.builder.build_store(field_data, data_ptr);

        let field_len = unsafe {
            binary.builder.build_gep(
                fields,
                &[
                    binary.context.i32_type().const_int(1, false),
                    binary.context.i32_type().const_int(1, false),
                ],
                "field_len",
            )
        };

        binary.builder.build_store(
            field_len,
            binary
                .builder
                .build_int_z_extend(data_len, binary.context.i64_type(), "data_len64"),
        );

        binary.builder.build_call(
            binary.module.get_function("sol_log_data").unwrap(),
            &[
                fields.into(),
                binary.context.i64_type().const_int(2, false).into(),
            ],
            "",
        );
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        binary: &Binary<'b>,
        expr: &ast::Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        match expr {
            ast::Expression::Builtin(_, _, ast::Builtin::Timestamp, _) => {
                let parameters = self.sol_parameters(binary);

                let sol_clock = binary.module.get_function("sol_clock").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    parameters,
                    sol_clock.get_type().get_param_types()[0].into_pointer_type(),
                    "",
                );

                let clock = binary
                    .builder
                    .build_call(sol_clock, &[arg1.into()], "clock")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let timestamp = binary
                    .builder
                    .build_struct_gep(clock, 4, "unix_timestamp")
                    .unwrap();

                binary.builder.build_load(timestamp, "timestamp")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::BlockNumber | ast::Builtin::Slot, _) => {
                let parameters = self.sol_parameters(binary);

                let sol_clock = binary.module.get_function("sol_clock").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    parameters,
                    sol_clock.get_type().get_param_types()[0].into_pointer_type(),
                    "",
                );

                let clock = binary
                    .builder
                    .build_call(sol_clock, &[arg1.into()], "clock")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let slot = binary.builder.build_struct_gep(clock, 0, "slot").unwrap();

                binary.builder.build_load(slot, "timestamp")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Sender, _) => {
                let parameters = self.sol_parameters(binary);

                let sender = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(parameters, 10, "sender")
                            .unwrap(),
                        "sender",
                    )
                    .into_pointer_value();

                let value =
                    binary.build_alloca(function, binary.address_type(ns), "sender_address");

                binary.builder.build_call(
                    binary.module.get_function("__beNtoleN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                sender,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                value,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .context
                            .i32_type()
                            .const_int(ns.address_length as u64, false)
                            .into(),
                    ],
                    "",
                );

                binary.builder.build_load(value, "sender_address")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Value, _) => {
                self.value_transferred(binary, ns).into()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::GetAddress, _) => {
                let parameters = self.sol_parameters(binary);

                let account_id = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(parameters, 4, "account_id")
                            .unwrap(),
                        "account_id",
                    )
                    .into_pointer_value();

                let value = binary.build_alloca(function, binary.address_type(ns), "self_address");

                binary.builder.build_call(
                    binary.module.get_function("__beNtoleN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                account_id,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                value,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .context
                            .i32_type()
                            .const_int(ns.address_length as u64, false)
                            .into(),
                    ],
                    "",
                );

                binary.builder.build_load(value, "self_address")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Calldata, _) => {
                let sol_params = self.sol_parameters(binary);

                let input = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(sol_params, 5, "input")
                            .unwrap(),
                        "data",
                    )
                    .into_pointer_value();

                let input_len = binary
                    .builder
                    .build_load(
                        binary
                            .builder
                            .build_struct_gep(sol_params, 6, "input_len")
                            .unwrap(),
                        "data_len",
                    )
                    .into_int_value();

                let input_len = binary.builder.build_int_truncate(
                    input_len,
                    binary.context.i32_type(),
                    "input_len",
                );

                binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            input_len.into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            input.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::SignatureVerify, args) => {
                assert_eq!(args.len(), 3);

                let address = binary.build_alloca(function, binary.address_type(ns), "address");

                binary.builder.build_store(
                    address,
                    self.expression(binary, &args[0], vartab, function, ns)
                        .into_int_value(),
                );

                let message = self.expression(binary, &args[1], vartab, function, ns);
                let signature = self.expression(binary, &args[2], vartab, function, ns);
                let parameters = self.sol_parameters(binary);
                let signature_verify = binary.module.get_function("signature_verify").unwrap();

                let arg1 = binary.builder.build_pointer_cast(
                    message.into_pointer_value(),
                    signature_verify.get_type().get_param_types()[1].into_pointer_type(),
                    "",
                );

                let arg2 = binary.builder.build_pointer_cast(
                    signature.into_pointer_value(),
                    signature_verify.get_type().get_param_types()[2].into_pointer_type(),
                    "",
                );

                let ret = binary
                    .builder
                    .build_call(
                        signature_verify,
                        &[
                            binary
                                .builder
                                .build_pointer_cast(
                                    address,
                                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            arg1.into(),
                            arg2.into(),
                            parameters.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                binary
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        ret,
                        binary.context.i64_type().const_zero(),
                        "success",
                    )
                    .into()
            }
            _ => unimplemented!(),
        }
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let (fname, hashlen) = match hash {
            HashTy::Keccak256 => ("sol_keccak256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("sol_sha256", 32),
            _ => unreachable!(),
        };

        let res = binary.build_array_alloca(
            function,
            binary.context.i8_type(),
            binary.context.i32_type().const_int(hashlen, false),
            "res",
        );

        if hash == HashTy::Ripemd160 {
            binary.builder.build_call(
                binary.module.get_function(fname).unwrap(),
                &[input.into(), input_len.into(), res.into()],
                "hash",
            );
        } else {
            let u64_ty = binary.context.i64_type();

            let sol_keccak256 = binary.module.get_function(fname).unwrap();

            // The first argument is a SolBytes *, get the struct
            let sol_bytes = sol_keccak256.get_type().get_param_types()[0]
                .into_pointer_type()
                .get_element_type()
                .into_struct_type();

            let array = binary.build_alloca(function, sol_bytes, "sol_bytes");

            binary.builder.build_store(
                binary.builder.build_struct_gep(array, 0, "input").unwrap(),
                input,
            );

            binary.builder.build_store(
                binary
                    .builder
                    .build_struct_gep(array, 1, "input_len")
                    .unwrap(),
                binary
                    .builder
                    .build_int_z_extend(input_len, u64_ty, "input_len"),
            );

            binary.builder.build_call(
                sol_keccak256,
                &[
                    array.into(),
                    binary.context.i32_type().const_int(1, false).into(),
                    res.into(),
                ],
                "hash",
            );
        }

        // bytes32 needs to reverse bytes
        let temp = binary.build_alloca(
            function,
            binary.llvm_type(&ast::Type::Bytes(hashlen as u8), ns),
            "hash",
        );

        binary.builder.build_call(
            binary.module.get_function("__beNtoleN").unwrap(),
            &[
                res.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        temp,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        binary.builder.build_load(temp, "hash").into_int_value()
    }
}
