use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use num_traits::ToPrimitive;
use std::collections::HashMap;

use super::{Contract, TargetRuntime, Variable};

// When using the seal api, we use our own scratch buffer.
const SCRATCH_SIZE: u32 = 32 * 1024;

pub struct SubstrateTarget {
    unique_strings: HashMap<usize, usize>,
}

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Contract<'a> {
        let mut c = Contract::new(context, contract, ns, filename, opt, None);

        let scratch_len = c.module.add_global(
            context.i32_type(),
            Some(AddressSpace::Generic),
            "scratch_len",
        );
        scratch_len.set_linkage(Linkage::Internal);
        scratch_len.set_initializer(&context.i32_type().get_undef());

        c.scratch_len = Some(scratch_len);

        let scratch = c.module.add_global(
            context.i8_type().array_type(SCRATCH_SIZE),
            Some(AddressSpace::Generic),
            "scratch",
        );
        scratch.set_linkage(Linkage::Internal);
        scratch.set_initializer(&context.i8_type().array_type(SCRATCH_SIZE).get_undef());
        c.scratch = Some(scratch);

        let mut b = SubstrateTarget {
            unique_strings: HashMap::new(),
        };

        b.declare_externals(&c);

        b.emit_functions(&mut c);

        b.emit_deploy(&mut c);
        b.emit_call(&c);

        c.internalize(&[
            "deploy",
            "call",
            "seal_input",
            "seal_set_storage",
            "seal_get_storage",
            "seal_clear_storage",
            "seal_hash_keccak_256",
            "seal_hash_sha2_256",
            "seal_hash_blake2_128",
            "seal_hash_blake2_256",
            "seal_return",
            "seal_println",
            "seal_instantiate",
            "seal_call",
            "seal_value_transferred",
            "seal_minimum_balance",
            "seal_random",
            "seal_address",
            "seal_balance",
            "seal_block_number",
            "seal_now",
            "seal_gas_price",
            "seal_gas_left",
            "seal_caller",
            "seal_tombstone_deposit",
            "seal_terminate",
            "seal_deposit_event",
        ]);

        c
    }

    fn public_function_prelude<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        abort_value_transfers: bool,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        // after copying stratch, first thing to do is abort value transfers if constructors not payable
        if abort_value_transfers {
            self.abort_if_value_transfer(contract, function);
        }

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "",
        );

        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        // copy arguments from input buffer
        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        contract.builder.build_call(
            contract.module.get_function("seal_input").unwrap(),
            &[scratch_buf.into(), scratch_len.into()],
            "",
        );

        let args = contract.builder.build_pointer_cast(
            scratch_buf,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );
        let args_length = contract.builder.build_load(scratch_len, "input_len");

        // store the length in case someone wants it via msg.data
        contract.builder.build_store(
            contract.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        (args, args_length.into_int_value())
    }

    fn declare_externals(&self, contract: &Contract) {
        let u8_ptr = contract
            .context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .into();
        let u32_val = contract.context.i32_type().into();
        let u32_ptr = contract
            .context
            .i32_type()
            .ptr_type(AddressSpace::Generic)
            .into();
        let u64_val = contract.context.i64_type().into();

        contract.module.add_function(
            "seal_input",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_hash_keccak_256",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    contract.context.i32_type().into(), // len
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_hash_sha2_256",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    contract.context.i32_type().into(), // len
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_hash_blake2_128",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    contract.context.i32_type().into(), // len
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_hash_blake2_256",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // src_ptr
                    contract.context.i32_type().into(), // len
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_random",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_val, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_set_storage",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr,  // key_ptr
                    u8_ptr,  // value_ptr
                    u32_val, // value_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_println",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr,  // string_ptr
                    u32_val, // string_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_clear_storage",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr, // key_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_get_storage",
            contract
                .context
                .i32_type()
                .fn_type(&[u8_ptr, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_return",
            contract.context.void_type().fn_type(
                &[
                    u32_val, u8_ptr, u32_val, // flags, data ptr, and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_instantiate",
            contract.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // code hash ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                    u8_ptr, u32_ptr, // address ptr and len
                    u8_ptr, u32_ptr, // output ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_call",
            contract.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                    u8_ptr, u32_ptr, // output ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_value_transferred",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_address",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_balance",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_minimum_balance",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_block_number",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_now",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_tombstone_deposit",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_weight_to_fee",
            contract
                .context
                .void_type()
                .fn_type(&[u64_val, u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_gas_left",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_caller",
            contract
                .context
                .void_type()
                .fn_type(&[u8_ptr, u32_ptr], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_terminate",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "seal_deposit_event",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // topic ptr and len
                    u8_ptr, u32_val, // data ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );
    }

    fn emit_deploy(&mut self, contract: &mut Contract) {
        let initializer = self.emit_initializer(contract);

        // create deploy function
        let function = contract.module.add_function(
            "deploy",
            contract.context.void_type().fn_type(&[], false),
            None,
        );

        // deploy always receives an endowment so no value check here
        let (deploy_args, deploy_args_length) =
            self.public_function_prelude(contract, function, false);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Constructor,
            deploy_args,
            deploy_args_length,
            function,
            Some(fallback_block),
            |_| false,
        );

        // emit fallback code
        contract.builder.position_at_end(fallback_block);

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

    fn emit_call(&mut self, contract: &Contract) {
        // create call function
        let function = contract.module.add_function(
            "call",
            contract.context.void_type().fn_type(&[], false),
            None,
        );

        let (call_args, call_args_length) = self.public_function_prelude(
            contract,
            function,
            contract.function_abort_value_transfers,
        );

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            call_args,
            call_args_length,
            function,
            None,
            |func| !contract.function_abort_value_transfers && func.nonpayable,
        );
    }

    /// ABI decode a single primitive
    fn decode_primitive<'b>(
        &self,
        contract: &Contract<'b>,
        ty: &ast::Type,
        src: PointerValue<'b>,
    ) -> (BasicValueEnum<'b>, u64) {
        match ty {
            ast::Type::Bool => {
                let val = contract.builder.build_int_compare(
                    IntPredicate::EQ,
                    contract
                        .builder
                        .build_load(src, "abi_bool")
                        .into_int_value(),
                    contract.context.i8_type().const_int(1, false),
                    "bool",
                );
                (val.into(), 1)
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_) => {
                let bits = match ty {
                    ast::Type::Uint(n) | ast::Type::Int(n) => *n as u32,
                    _ => contract.ns.address_length as u32 * 8,
                };

                let int_type = contract.context.custom_width_int_type(bits);

                let val = contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        src,
                        int_type.ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "",
                );

                let len = bits as u64 / 8;

                (val, len)
            }
            ast::Type::Bytes(len) => {
                let int_type = contract.context.custom_width_int_type(*len as u32 * 8);

                let buf = contract.builder.build_alloca(int_type, "buf");

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        src.into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                buf,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(*len as u64, false)
                            .into(),
                    ],
                    "",
                );

                (
                    contract.builder.build_load(buf, &format!("bytes{}", len)),
                    *len as u64,
                )
            }
            _ => unreachable!(),
        }
    }

    /// Check that data has not overrun end. We do not check if we have more data than provided;
    /// there could be a salt there for the constructor.
    fn check_overrun(
        &self,
        contract: &Contract,
        function: FunctionValue,
        data: PointerValue,
        end: PointerValue,
    ) {
        let in_bounds = contract.builder.build_int_compare(
            IntPredicate::ULE,
            contract
                .builder
                .build_ptr_to_int(data, contract.context.i32_type(), "args"),
            contract
                .builder
                .build_ptr_to_int(end, contract.context.i32_type(), "end"),
            "is_done",
        );

        let success_block = contract.context.append_basic_block(function, "success");
        let bail_block = contract.context.append_basic_block(function, "bail");
        contract
            .builder
            .build_conditional_branch(in_bounds, success_block, bail_block);

        contract.builder.position_at_end(bail_block);

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

    /// recursively encode a single ty
    fn decode_ty<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        ty: &ast::Type,
        data: &mut PointerValue<'b>,
        end: PointerValue<'b>,
    ) -> BasicValueEnum<'b> {
        match &ty {
            ast::Type::Bool
            | ast::Type::Address(_)
            | ast::Type::Contract(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_) => {
                let (arg, arglen) = self.decode_primitive(contract, ty, *data);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "abi_ptr",
                    )
                };

                self.check_overrun(contract, function, *data, end);

                arg
            }
            ast::Type::Enum(n) => {
                self.decode_ty(contract, function, &contract.ns.enums[*n].ty, data, end)
            }
            ast::Type::Struct(n) => {
                let llvm_ty = contract.llvm_type(ty.deref_any());

                let size = llvm_ty
                    .size_of()
                    .unwrap()
                    .const_cast(contract.context.i32_type(), false);

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

                    let val = self.decode_ty(contract, function, &field.ty, data, end);

                    contract.builder.build_store(elem, val);
                }

                dest.into()
            }
            ast::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let llvm_ty = contract.llvm_type(ty.deref_any());

                    let size = llvm_ty
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

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

                    contract.emit_static_loop_with_pointer(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        data,
                        |index: IntValue<'b>, data: &mut PointerValue<'b>| {
                            let elem = unsafe {
                                contract.builder.build_gep(
                                    dest,
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val = self.decode_ty(contract, function, &ty, data, end);
                            contract.builder.build_store(elem, val);
                        },
                    );

                    dest.into()
                } else {
                    let len = contract
                        .builder
                        .build_alloca(contract.context.i32_type(), "length");

                    *data = contract
                        .builder
                        .build_call(
                            contract.module.get_function("compact_decode_u32").unwrap(),
                            &[(*data).into(), len.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let len = contract
                        .builder
                        .build_load(len, "array.len")
                        .into_int_value();

                    // details about our array elements
                    let elem_ty = contract.llvm_var(&ty.array_elem());
                    let elem_size = elem_ty
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

                    let init = contract.builder.build_int_to_ptr(
                        contract.context.i32_type().const_all_ones(),
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    let v = contract
                        .builder
                        .build_call(
                            contract.module.get_function("vector_new").unwrap(),
                            &[len.into(), elem_size.into(), init.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    contract.emit_loop_cond_first_with_pointer(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        data,
                        |elem_no: IntValue<'b>, data: &mut PointerValue<'b>| {
                            let index = contract.builder.build_int_mul(elem_no, elem_size, "");

                            let element_start = unsafe {
                                contract.builder.build_gep(
                                    v,
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let elem = contract.builder.build_pointer_cast(
                                element_start,
                                elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            let ty = ty.array_deref();

                            let val = self.decode_ty(contract, function, &ty, data, end);
                            contract.builder.build_store(elem, val);
                        },
                    );

                    contract
                        .builder
                        .build_pointer_cast(
                            v,
                            contract
                                .module
                                .get_struct_type("struct.vector")
                                .unwrap()
                                .ptr_type(AddressSpace::Generic),
                            "string",
                        )
                        .into()
                }
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let from = contract.builder.build_alloca(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "from",
                );

                contract.builder.build_store(from, *data);

                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("scale_decode_string").unwrap(),
                        &[from.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                *data = contract
                    .builder
                    .build_load(from, "data")
                    .into_pointer_value();

                self.check_overrun(contract, function, *data, end);

                contract
                    .builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        contract
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "string",
                    )
                    .into()
            }
            ast::Type::Ref(ty) => self.decode_ty(contract, function, ty, data, end),
            ast::Type::ExternalFunction { .. } => {
                let address =
                    self.decode_ty(contract, function, &ast::Type::Address(false), data, end);
                let selector = self.decode_ty(contract, function, &ast::Type::Uint(32), data, end);

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

                contract.builder.build_store(selector_member, selector);

                ef.into()
            }
            _ => unreachable!(),
        }
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        load: bool,
        ty: &ast::Type,
        dest: PointerValue,
        arg: BasicValueEnum,
    ) -> u64 {
        match ty {
            ast::Type::Bool => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                contract.builder.build_store(
                    dest,
                    contract.builder.build_int_z_extend(
                        arg.into_int_value(),
                        contract.context.i8_type(),
                        "bool",
                    ),
                );

                1
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_) => {
                let len = match ty {
                    ast::Type::Uint(n) | ast::Type::Int(n) => *n as u64 / 8,
                    _ => contract.ns.address_length as u64,
                };

                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                contract.builder.build_store(
                    contract.builder.build_pointer_cast(
                        dest,
                        arg.into_int_value()
                            .get_type()
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    arg.into_int_value(),
                );

                len
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = contract
                        .builder
                        .build_alloca(arg.into_int_value().get_type(), &format!("bytes{}", n));

                    contract.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                val,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        dest.into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(*n as u64, false)
                            .into(),
                    ],
                    "",
                );

                *n as u64
            }
            _ => unimplemented!(),
        }
    }

    /// recursively encode argument. The encoded data is written to the data pointer,
    /// and the pointer is updated point after the encoded data.
    pub fn encode_ty<'x>(
        &self,
        contract: &Contract<'x>,
        load: bool,
        packed: bool,
        function: FunctionValue,
        ty: &ast::Type,
        arg: BasicValueEnum<'x>,
        data: &mut PointerValue<'x>,
    ) {
        match &ty {
            ast::Type::Bool
            | ast::Type::Address(_)
            | ast::Type::Contract(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_) => {
                let arglen = self.encode_primitive(contract, load, ty, *data, arg);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "",
                    )
                };
            }
            ast::Type::Enum(n) => {
                self.encode_primitive(contract, load, &contract.ns.enums[*n].ty, *data, arg);
            }
            ast::Type::Array(_, dim) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                if let Some(d) = &dim[0] {
                    contract.emit_static_loop_with_pointer(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        data,
                        |index, data| {
                            let elem = unsafe {
                                contract.builder.build_gep(
                                    arg.into_pointer_value(),
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let ty = ty.array_deref();

                            self.encode_ty(
                                contract,
                                true,
                                packed,
                                function,
                                &ty.deref_any(),
                                elem.into(),
                                data,
                            );
                        },
                    );
                } else {
                    let len = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_zero(),
                            ],
                            "array.len",
                        )
                    };

                    let len = contract
                        .builder
                        .build_load(len, "array.len")
                        .into_int_value();

                    if !packed {
                        *data = contract
                            .builder
                            .build_call(
                                contract.module.get_function("compact_encode_u32").unwrap(),
                                &[(*data).into(), len.into()],
                                "",
                            )
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }

                    // details about our array elements
                    let elem_ty = ty.array_deref();
                    let llvm_elem_ty = contract.llvm_var(&elem_ty);
                    let elem_size = llvm_elem_ty
                        .into_pointer_type()
                        .get_element_type()
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

                    contract.emit_static_loop_with_pointer(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        data,
                        |elem_no, data| {
                            let index = contract.builder.build_int_mul(elem_no, elem_size, "");

                            let element_start = unsafe {
                                contract.builder.build_gep(
                                    arg.into_pointer_value(),
                                    &[
                                        contract.context.i32_type().const_zero(),
                                        contract.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let elem = contract.builder.build_pointer_cast(
                                element_start,
                                llvm_elem_ty.into_pointer_type(),
                                "entry",
                            );

                            let ty = ty.array_deref();

                            self.encode_ty(
                                contract,
                                true,
                                packed,
                                function,
                                &ty.deref_any(),
                                elem.into(),
                                data,
                            );
                        },
                    );
                }
            }
            ast::Type::Struct(n) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.encode_ty(
                        contract,
                        true,
                        packed,
                        function,
                        &field.ty,
                        elem.into(),
                        data,
                    );
                }
            }
            ast::Type::Ref(ty) => {
                self.encode_ty(contract, load, packed, function, ty, arg, data);
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                if !packed {
                    let function = contract.module.get_function("scale_encode_string").unwrap();

                    *data = contract
                        .builder
                        .build_call(
                            function,
                            &[
                                (*data).into(),
                                // when we call LinkModules2() some types like vector get renamed to vector.1
                                contract
                                    .builder
                                    .build_pointer_cast(
                                        arg.into_pointer_value(),
                                        function.get_type().get_param_types()[1]
                                            .into_pointer_type(),
                                        "vector",
                                    )
                                    .into(),
                            ],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else {
                    let len = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_zero(),
                            ],
                            "string.len",
                        )
                    };

                    let p = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(2, false),
                            ],
                            "string.data",
                        )
                    };

                    let len = contract
                        .builder
                        .build_load(len, "array.len")
                        .into_int_value();

                    contract.builder.build_call(
                        contract.module.get_function("__memcpy").unwrap(),
                        &[
                            (*data).into(),
                            contract
                                .builder
                                .build_pointer_cast(
                                    p,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            len.into(),
                        ],
                        "",
                    );

                    *data = unsafe { contract.builder.build_gep(*data, &[len], "") };
                }
            }
            ast::Type::ExternalFunction { .. } => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let address_member = unsafe {
                    contract.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "address",
                    )
                };

                let address = contract.builder.build_load(address_member, "address");

                self.encode_ty(
                    contract,
                    false,
                    false,
                    function,
                    &ast::Type::Address(false),
                    address,
                    data,
                );

                let selector_member = unsafe {
                    contract.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(1, false),
                        ],
                        "selector",
                    )
                };

                let selector = contract.builder.build_load(selector_member, "selector");

                self.encode_ty(
                    contract,
                    false,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    selector,
                    data,
                );
            }
            _ => unreachable!(),
        };
    }

    /// Calculate the maximum space a type will need when encoded. This is used for
    /// allocating enough space to do abi encoding. The length for vectors is always
    /// assumed to be five, even when it can be encoded in less bytes. The overhead
    /// of calculating the exact size is not worth reducing the malloc by a few bytes.
    pub fn encoded_length<'x>(
        &self,
        arg: BasicValueEnum<'x>,
        load: bool,
        packed: bool,
        ty: &ast::Type,
        function: FunctionValue,
        contract: &Contract<'x>,
    ) -> IntValue<'x> {
        match ty {
            ast::Type::Bool => contract.context.i32_type().const_int(1, false),
            ast::Type::Uint(n) | ast::Type::Int(n) => {
                contract.context.i32_type().const_int(*n as u64 / 8, false)
            }
            ast::Type::Bytes(n) => contract.context.i32_type().const_int(*n as u64, false),
            ast::Type::Address(_) | ast::Type::Contract(_) => contract
                .context
                .i32_type()
                .const_int(contract.ns.address_length as u64, false),
            ast::Type::Enum(n) => self.encoded_length(
                arg,
                load,
                packed,
                &contract.ns.enums[*n].ty,
                function,
                contract,
            ),
            ast::Type::Struct(n) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut sum = contract.context.i32_type().const_zero();

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    sum = contract.builder.build_int_add(
                        sum,
                        self.encoded_length(
                            elem.into(),
                            true,
                            packed,
                            &field.ty,
                            function,
                            contract,
                        ),
                        "",
                    );
                }

                sum
            }
            ast::Type::Array(_, dims) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut dynamic_array = false;
                let mut encoded_length = contract.context.i32_type().const_zero();

                let array_length = match dims.last().unwrap() {
                    None => {
                        let len = unsafe {
                            contract.builder.build_gep(
                                arg.into_pointer_value(),
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "array.len",
                            )
                        };

                        dynamic_array = true;

                        // dynamic length array needs length if not packed
                        if !packed {
                            encoded_length = contract.context.i32_type().const_int(5, false);
                        }

                        contract
                            .builder
                            .build_load(len, "array.len")
                            .into_int_value()
                    }
                    Some(d) => contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                let elem_ty = ty.array_deref();
                let llvm_elem_ty = contract.llvm_var(&elem_ty);

                if elem_ty.is_dynamic(contract.ns) {
                    contract.emit_static_loop_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        array_length,
                        &mut encoded_length,
                        |index, sum| {
                            let elem = if dynamic_array {
                                let index = contract.builder.build_int_mul(
                                    index,
                                    llvm_elem_ty
                                        .into_pointer_type()
                                        .get_element_type()
                                        .size_of()
                                        .unwrap()
                                        .const_cast(contract.context.i32_type(), false),
                                    "",
                                );

                                let p = unsafe {
                                    contract.builder.build_gep(
                                        arg.into_pointer_value(),
                                        &[
                                            contract.context.i32_type().const_zero(),
                                            contract.context.i32_type().const_int(2, false),
                                            index,
                                        ],
                                        "index_access",
                                    )
                                };
                                contract.builder.build_pointer_cast(
                                    p,
                                    llvm_elem_ty.into_pointer_type(),
                                    "elem",
                                )
                            } else {
                                unsafe {
                                    contract.builder.build_gep(
                                        arg.into_pointer_value(),
                                        &[contract.context.i32_type().const_zero(), index],
                                        "index_access",
                                    )
                                }
                            };

                            *sum = contract.builder.build_int_add(
                                self.encoded_length(
                                    elem.into(),
                                    true,
                                    packed,
                                    &elem_ty,
                                    function,
                                    contract,
                                ),
                                *sum,
                                "",
                            );
                        },
                    );

                    encoded_length
                } else {
                    let elem = if dynamic_array {
                        let p = unsafe {
                            contract.builder.build_gep(
                                arg.into_pointer_value(),
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(2, false),
                                ],
                                "index_access",
                            )
                        };

                        contract.builder.build_pointer_cast(
                            p,
                            llvm_elem_ty.into_pointer_type(),
                            "elem",
                        )
                    } else {
                        unsafe {
                            contract.builder.build_gep(
                                arg.into_pointer_value(),
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_zero(),
                                ],
                                "index_access",
                            )
                        }
                    };

                    contract.builder.build_int_add(
                        encoded_length,
                        contract.builder.build_int_mul(
                            self.encoded_length(
                                elem.into(),
                                true,
                                packed,
                                &elem_ty,
                                function,
                                contract,
                            ),
                            array_length,
                            "",
                        ),
                        "",
                    )
                }
            }
            ast::Type::Ref(r) => self.encoded_length(arg, load, packed, r, function, contract),
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // A string or bytes type has to be encoded by: one compact integer for
                // the length, followed by the bytes themselves. Here we assume that the
                // length requires 5 bytes.
                let len = unsafe {
                    contract.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "string.len",
                    )
                };

                let len = contract
                    .builder
                    .build_load(len, "string.len")
                    .into_int_value();

                if packed {
                    len
                } else {
                    contract.builder.build_int_add(
                        len,
                        contract.context.i32_type().const_int(5, false),
                        "",
                    )
                }
            }
            ast::Type::ExternalFunction { .. } => {
                // address + 4 bytes selector
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.address_length as u64 + 4, false)
            }
            _ => unreachable!(),
        }
    }

    /// Create a unique salt each time this function is called.
    fn contract_unique_salt<'x>(
        &mut self,
        contract: &'x Contract,
        contract_no: usize,
    ) -> (PointerValue<'x>, IntValue<'x>) {
        let counter = *self.unique_strings.get(&contract_no).unwrap_or(&0);

        let contract_name = &contract.ns.contracts[contract_no].name;

        let unique = format!("{}-{}", contract_name, counter);

        let salt = contract.emit_global_string(
            &format!("salt_{}_{}", contract_name, counter),
            blake2_rfc::blake2b::blake2b(32, &[], unique.as_bytes()).as_bytes(),
            true,
        );

        self.unique_strings.insert(contract_no, counter + 1);

        (salt, contract.context.i32_type().const_int(32, false))
    }
}

impl<'a> TargetRuntime<'a> for SubstrateTarget {
    fn clear_storage(&self, contract: &Contract, _function: FunctionValue, slot: PointerValue) {
        contract.builder.build_call(
            contract.module.get_function("seal_clear_storage").unwrap(),
            &[contract
                .builder
                .build_pointer_cast(
                    slot,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "",
        );
    }

    fn set_storage(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        // TODO: check for non-zero
        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest.get_type()
                    .get_element_type()
                    .into_int_type()
                    .size_of()
                    .const_cast(contract.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn set_storage_extfunc(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest.get_type()
                    .get_element_type()
                    .size_of()
                    .unwrap()
                    .const_cast(contract.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn get_storage_extfunc(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        let ty = contract.llvm_type(&ast::Type::ExternalFunction {
            params: Vec::new(),
            mutability: None,
            returns: Vec::new(),
        });

        let len = ty
            .into_pointer_type()
            .get_element_type()
            .size_of()
            .unwrap()
            .const_cast(contract.context.i32_type(), false);

        let ef = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[len.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();
        contract.builder.build_store(scratch_len, len);

        let _exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    ef.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        // TODO: decide behaviour if not exist

        contract
            .builder
            .build_pointer_cast(ef, ty.into_pointer_type(), "function_type")
    }

    fn set_storage_string(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        let len = unsafe {
            contract.builder.build_gep(
                dest,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_zero(),
                ],
                "ptr.string.len",
            )
        };

        let len = contract.builder.build_load(len, "string.len");

        let data = unsafe {
            contract.builder.build_gep(
                dest,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(2, false),
                ],
                "ptr.string.data",
            )
        };

        // TODO: check for non-zero
        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                len,
            ],
            "",
        );
    }

    /// Read from substrate storage
    fn get_storage_int(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();
        let ty_len = ty.size_of().const_cast(contract.context.i32_type(), false);
        contract.builder.build_store(scratch_len, ty_len);

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let entry = contract.builder.get_insert_block().unwrap();
        let retrieve_block = contract.context.append_basic_block(function, "in_storage");
        let done_storage = contract
            .context
            .append_basic_block(function, "done_storage");

        contract
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        contract.builder.position_at_end(retrieve_block);

        let dest = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            ty.ptr_type(AddressSpace::Generic),
            "scratch_ty_buf",
        );

        let loaded_int = contract.builder.build_load(dest, "int");

        contract.builder.build_unconditional_branch(done_storage);

        contract.builder.position_at_end(done_storage);

        let res = contract.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[(&loaded_int, retrieve_block), (&ty.const_zero(), entry)]);

        res.as_basic_value().into_int_value()
    }

    /// Read string from substrate storage
    fn get_storage_string(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = contract.builder.build_select(
            exists,
            contract.builder.build_load(scratch_len, "string_len"),
            contract.context.i32_type().const_zero().into(),
            "string_length",
        );

        contract
            .builder
            .build_call(
                contract.module.get_function("vector_new").unwrap(),
                &[
                    length,
                    contract.context.i32_type().const_int(1, false).into(),
                    scratch_buf.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Read string from substrate storage
    fn get_storage_bytes_subscript(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = contract
            .builder
            .build_select(
                exists,
                contract.builder.build_load(scratch_len, "string_len"),
                contract.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range =
            contract
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let retrieve_block = contract.context.append_basic_block(function, "in_range");
        let bang_block = contract.context.append_basic_block(function, "bang_block");

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

        contract.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            contract.builder.build_gep(
                contract.scratch.unwrap().as_pointer_value(),
                &[contract.context.i32_type().const_zero(), index],
                "data_offset",
            )
        };

        contract
            .builder
            .build_load(offset, "value")
            .into_int_value()
    }

    fn set_storage_bytes_subscript(
        &self,
        contract: &Contract,
        function: FunctionValue,
        slot: PointerValue,
        index: IntValue,
        val: IntValue,
    ) {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = contract
            .builder
            .build_select(
                exists,
                contract.builder.build_load(scratch_len, "string_len"),
                contract.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range =
            contract
                .builder
                .build_int_compare(IntPredicate::ULT, index, length, "index_in_range");

        let retrieve_block = contract.context.append_basic_block(function, "in_range");
        let bang_block = contract.context.append_basic_block(function, "bang_block");

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

        contract.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            contract.builder.build_gep(
                contract.scratch.unwrap().as_pointer_value(),
                &[contract.context.i32_type().const_zero(), index],
                "data_offset",
            )
        };

        // set the result
        contract.builder.build_store(offset, val);

        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                length.into(),
            ],
            "",
        );
    }

    /// Push a byte onto a bytes string in storage
    fn storage_bytes_push(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
        val: IntValue,
    ) {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        // Since we are going to add one byte, we set the buffer length to one less. This will
        // trap for us if it does not fit, so we don't have to code this ourselves
        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64 - 1, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = contract
            .builder
            .build_select(
                exists,
                contract.builder.build_load(scratch_len, "string_len"),
                contract.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // set the result
        let offset = unsafe {
            contract.builder.build_gep(
                contract.scratch.unwrap().as_pointer_value(),
                &[contract.context.i32_type().const_zero(), length],
                "data_offset",
            )
        };

        contract.builder.build_store(offset, val);

        // Set the new length
        let length = contract.builder.build_int_add(
            length,
            contract.context.i32_type().const_int(1, false),
            "new_length",
        );

        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                length.into(),
            ],
            "",
        );
    }

    /// Pop a value from a bytes string
    fn storage_bytes_pop(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        let length = contract
            .builder
            .build_select(
                exists,
                contract.builder.build_load(scratch_len, "string_len"),
                contract.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value();

        // do bounds check on index
        let in_range = contract.builder.build_int_compare(
            IntPredicate::EQ,
            contract.context.i32_type().const_zero(),
            length,
            "index_in_range",
        );

        let retrieve_block = contract.context.append_basic_block(function, "in_range");
        let bang_block = contract.context.append_basic_block(function, "bang_block");

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

        contract.builder.position_at_end(retrieve_block);

        let offset = unsafe {
            contract.builder.build_gep(
                contract.scratch.unwrap().as_pointer_value(),
                &[contract.context.i32_type().const_zero(), length],
                "data_offset",
            )
        };

        let val = contract.builder.build_load(offset, "popped_value");

        // Set the new length
        let new_length = contract.builder.build_int_sub(
            length,
            contract.context.i32_type().const_int(1, false),
            "new_length",
        );

        contract.builder.build_call(
            contract.module.get_function("seal_set_storage").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                scratch_buf.into(),
                new_length.into(),
            ],
            "",
        );

        val.into_int_value()
    }

    /// Calculate length of storage dynamic bytes
    fn storage_string_length(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("seal_get_storage").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists",
        );

        contract
            .builder
            .build_select(
                exists,
                contract.builder.build_load(scratch_len, "string_len"),
                contract.context.i32_type().const_zero().into(),
                "string_length",
            )
            .into_int_value()
    }

    fn return_empty_abi(&self, contract: &Contract) {
        contract.builder.build_call(
            contract.module.get_function("seal_return").unwrap(),
            &[
                contract.context.i32_type().const_zero().into(),
                contract
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_zero()
                    .into(),
                contract.context.i32_type().const_zero().into(),
            ],
            "",
        );

        contract.builder.build_unreachable();
    }

    fn return_u32<'b>(&self, contract: &'b Contract, _ret: IntValue<'b>) {
        // we can't return specific errors
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

    /// Call the  keccak256 host function
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        contract.builder.build_call(
            contract
                .module
                .get_function("seal_hash_keccak_256")
                .unwrap(),
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

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("seal_return").unwrap(),
            &[
                contract.context.i32_type().const_zero().into(),
                data.into(),
                length.into(),
            ],
            "",
        );

        contract.builder.build_unreachable();
    }

    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("seal_return").unwrap(),
            &[
                contract.context.i32_type().const_int(1, false).into(),
                data.into(),
                length.into(),
            ],
            "",
        );

        contract.builder.build_unreachable();
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        datalength: IntValue<'b>,
        spec: &[ast::Parameter],
    ) {
        let mut argsdata = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        let argsend = unsafe {
            contract
                .builder
                .build_gep(argsdata, &[datalength], "argsend")
        };

        for param in spec {
            args.push(self.decode_ty(contract, function, &param.ty, &mut argsdata, argsend));
        }
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<IntValue<'b>>,
        function: FunctionValue,
        packed: bool,
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
    ) -> PointerValue<'b> {
        // first calculate how much memory we need to allocate
        let mut length = contract.context.i32_type().const_zero();

        // note that encoded_length return the exact value for packed encoding
        for (i, ty) in tys.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                self.encoded_length(args[i], false, packed, &ty, function, contract),
                "",
            );
        }

        if selector.is_some() {
            length = contract.builder.build_int_add(
                length,
                contract
                    .context
                    .i32_type()
                    .size_of()
                    .const_cast(contract.context.i32_type(), false),
                "",
            );
        }

        let malloc_length = contract.builder.build_int_add(
            length,
            contract
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .size_of()
                .unwrap()
                .const_cast(contract.context.i32_type(), false),
            "size",
        );

        let p = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let v = contract.builder.build_pointer_cast(
            p,
            contract
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "string",
        );

        // if it's packed, we have the correct length already
        if packed {
            let data_len = unsafe {
                contract.builder.build_gep(
                    v,
                    &[
                        contract.context.i32_type().const_zero(),
                        contract.context.i32_type().const_zero(),
                    ],
                    "data_len",
                )
            };

            contract.builder.build_store(data_len, length);
        }

        let data_size = unsafe {
            contract.builder.build_gep(
                v,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(1, false),
                ],
                "data_size",
            )
        };

        contract.builder.build_store(data_size, length);

        let data = unsafe {
            contract.builder.build_gep(
                v,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(2, false),
                ],
                "data",
            )
        };

        // now encode each of the arguments
        let data = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        let mut argsdata = data;

        if let Some(selector) = selector {
            // we need to byte-swap our bytes4 type

            let temp = contract
                .builder
                .build_alloca(selector.get_type(), "selector");

            contract.builder.build_store(temp, selector);

            // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
            contract.builder.build_call(
                contract.module.get_function("__leNtobeN").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            temp,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    data.into(),
                    contract.context.i32_type().const_int(4, false).into(),
                ],
                "",
            );

            argsdata = unsafe {
                contract.builder.build_gep(
                    argsdata,
                    &[contract
                        .context
                        .i32_type()
                        .size_of()
                        .const_cast(contract.context.i32_type(), false)],
                    "",
                )
            };
        }

        for (i, ty) in tys.iter().enumerate() {
            self.encode_ty(
                contract,
                false,
                packed,
                function,
                &ty,
                args[i],
                &mut argsdata,
            );
        }

        if !packed {
            let length = contract.builder.build_int_sub(
                contract
                    .builder
                    .build_ptr_to_int(argsdata, contract.context.i32_type(), "end"),
                contract
                    .builder
                    .build_ptr_to_int(data, contract.context.i32_type(), "begin"),
                "datalength",
            );

            let data_len = unsafe {
                contract.builder.build_gep(
                    v,
                    &[
                        contract.context.i32_type().const_zero(),
                        contract.context.i32_type().const_zero(),
                    ],
                    "data_len",
                )
            };

            contract.builder.build_store(data_len, length);
        }

        v
    }

    ///  ABI encode the return values for the function
    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<IntValue<'b>>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[ast::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        // first calculate how much memory we need to allocate
        let mut length = contract.context.i32_type().const_zero();

        // note that encoded_length overestimates how data we need
        for (i, field) in spec.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                self.encoded_length(args[i], load, false, &field.ty, function, contract),
                "",
            );
        }

        if let Some(selector) = selector {
            length = contract.builder.build_int_add(
                length,
                selector
                    .get_type()
                    .size_of()
                    .const_cast(contract.context.i32_type(), false),
                "",
            );
        }

        let data = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // now encode each of the arguments
        let mut argsdata = data;

        if let Some(selector) = selector {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    data,
                    selector.get_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                selector,
            );

            argsdata = unsafe {
                contract.builder.build_gep(
                    data,
                    &[selector
                        .get_type()
                        .size_of()
                        .const_cast(contract.context.i32_type(), false)],
                    "",
                )
            };
        }

        for (i, arg) in spec.iter().enumerate() {
            self.encode_ty(
                contract,
                load,
                false,
                function,
                &arg.ty,
                args[i],
                &mut argsdata,
            );
        }

        // we cannot use the length returned by encoded_length; calculate actual length
        let length = contract.builder.build_int_sub(
            contract
                .builder
                .build_ptr_to_int(argsdata, contract.context.i32_type(), "end"),
            contract
                .builder
                .build_ptr_to_int(data, contract.context.i32_type(), "begin"),
            "datalength",
        );

        (data, length)
    }

    fn print(&self, contract: &Contract, string_ptr: PointerValue, string_len: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("seal_println").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }

    fn create_contract<'b>(
        &mut self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
    ) {
        let resolver_contract = &contract.ns.contracts[contract_no];

        let constructor = match constructor_no {
            Some(function_no) => &resolver_contract.functions[function_no],
            None => &resolver_contract.default_constructor.as_ref().unwrap().0,
        };

        let mut args = args.to_vec();
        let mut params = constructor.params.to_vec();
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        // salt
        let salt_ty = ast::Type::Uint(256);

        if let Some(salt) = salt {
            args.push(salt.into());
        } else {
            let (ptr, len) = self.contract_unique_salt(contract, contract_no);

            contract.builder.build_store(
                scratch_len,
                contract.context.i32_type().const_int(32, false),
            );

            contract.builder.build_call(
                contract.module.get_function("seal_random").unwrap(),
                &[
                    ptr.into(),
                    len.into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "random",
            );

            args.push(
                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract
                            .context
                            .custom_width_int_type(256)
                            .ptr_type(AddressSpace::Generic),
                        "salt_buf",
                    ),
                    "salt",
                ),
            );
        }

        params.push(ast::Parameter {
            loc: pt::Loc(0, 0, 0),
            ty: salt_ty,
            ty_loc: pt::Loc(0, 0, 0),
            name: "salt".to_string(),
            name_loc: None,
            indexed: false,
        });

        // input
        let (input, input_len) = self.abi_encode(
            contract,
            Some(
                contract
                    .context
                    .i32_type()
                    .const_int(constructor.selector() as u64, false),
            ),
            false,
            function,
            &args,
            &params,
        );

        let value_ptr = contract
            .builder
            .build_alloca(contract.value_type(), "balance");

        // balance is a u128, make sure it's enough to cover existential_deposit
        if let Some(value) = value {
            contract.builder.build_store(value_ptr, value);
        } else {
            let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

            contract.builder.build_store(
                scratch_len,
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.value_length as u64, false),
            );

            contract.builder.build_call(
                contract
                    .module
                    .get_function("seal_minimum_balance")
                    .unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    scratch_len.into(),
                ],
                "minimum_balance",
            );
        }

        // wasm
        let target_contract = Contract::build(
            contract.context,
            &resolver_contract,
            contract.ns,
            "",
            contract.opt,
        );

        let wasm = target_contract.code(true).expect("compile should succeeed");

        // code hash
        let codehash = contract.emit_global_string(
            &format!("contract_{}_codehash", resolver_contract.name),
            blake2_rfc::blake2b::blake2b(32, &[], &wasm).as_bytes(),
            true,
        );

        let address_len_ptr = contract
            .builder
            .build_alloca(contract.context.i32_type(), "address_len_ptr");

        contract.builder.build_store(
            address_len_ptr,
            contract
                .context
                .i32_type()
                .const_int(contract.ns.address_length as u64, false),
        );

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        // seal_instantiate returns 0x0100 if the contract cannot be instantiated
        // due to insufficient funds, etc. If the return value is < 0x100, then
        // this is return value from the constructor (or deploy function) of
        // the contract
        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("seal_instantiate").unwrap(),
                &[
                    codehash.into(),
                    contract.context.i32_type().const_int(32, false).into(),
                    gas.into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    contract.context.i32_type().const_int(16, false).into(),
                    input.into(),
                    input_len.into(),
                    address.into(),
                    address_len_ptr.into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            contract.context.i32_type().const_zero(),
            "success",
        );

        let success_block = contract.context.append_basic_block(function, "success");
        let bail_block = contract.context.append_basic_block(function, "bail");
        contract
            .builder
            .build_conditional_branch(is_success, success_block, bail_block);

        contract.builder.position_at_end(success_block);

        if let Some(success) = success {
            // we're in the try path. This means:
            // return success or not in success variable
            // do not abort execution
            //
            *success = is_success.into();

            let done_block = contract.context.append_basic_block(function, "done");
            contract.builder.build_unconditional_branch(done_block);
            contract.builder.position_at_end(bail_block);
            contract.builder.build_unconditional_branch(done_block);
            contract.builder.position_at_end(done_block);
        } else {
            contract.builder.position_at_end(bail_block);

            self.assert_failure(
                contract,
                scratch_buf,
                contract
                    .builder
                    .build_load(scratch_len, "string_len")
                    .into_int_value(),
            );

            contract.builder.position_at_end(success_block);
        }
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        _ty: ast::CallTy,
    ) {
        // balance is a u128
        let value_ptr = contract
            .builder
            .build_alloca(contract.value_type(), "balance");
        contract.builder.build_store(value_ptr, value);

        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        // do the actual call
        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("seal_call").unwrap(),
                &[
                    address.into(),
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.address_length as u64, false)
                        .into(),
                    gas.into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.value_length as u64, false)
                        .into(),
                    payload.into(),
                    payload_len.into(),
                    scratch_buf.into(),
                    scratch_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            contract.context.i32_type().const_zero(),
            "success",
        );

        let success_block = contract.context.append_basic_block(function, "success");
        let bail_block = contract.context.append_basic_block(function, "bail");
        contract
            .builder
            .build_conditional_branch(is_success, success_block, bail_block);

        contract.builder.position_at_end(success_block);

        if let Some(success) = success {
            // we're in the try path. This means:
            // return success or not in success variable
            // do not abort execution
            //
            *success = is_success.into();

            let done_block = contract.context.append_basic_block(function, "done");
            contract.builder.build_unconditional_branch(done_block);
            contract.builder.position_at_end(bail_block);
            contract.builder.build_unconditional_branch(done_block);
            contract.builder.position_at_end(done_block);
        } else {
            contract.builder.position_at_end(bail_block);

            self.assert_failure(
                contract,
                scratch_buf,
                contract
                    .builder
                    .build_load(scratch_len, "string_len")
                    .into_int_value(),
            );

            contract.builder.position_at_end(success_block);
        }
    }

    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        let length = contract.builder.build_load(scratch_len, "string_len");

        contract
            .builder
            .build_call(
                contract.module.get_function("vector_new").unwrap(),
                &[
                    length,
                    contract.context.i32_type().const_int(1, false).into(),
                    scratch_buf.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Substrate value is usually 128 bits
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        let scratch_buf = contract.builder.build_pointer_cast(
            contract.scratch.unwrap().as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "scratch_buf",
        );
        let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

        contract.builder.build_store(
            scratch_len,
            contract
                .context
                .i32_type()
                .const_int(contract.ns.value_length as u64, false),
        );

        contract.builder.build_call(
            contract
                .module
                .get_function("seal_value_transferred")
                .unwrap(),
            &[scratch_buf.into(), scratch_len.into()],
            "value_transferred",
        );

        contract
            .builder
            .build_load(
                contract.builder.build_pointer_cast(
                    scratch_buf,
                    contract.value_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                "value_transferred",
            )
            .into_int_value()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, contract: &Contract<'b>, addr: IntValue<'b>) {
        let address = contract
            .builder
            .build_alloca(contract.address_type(), "address");

        contract.builder.build_store(address, addr);

        contract.builder.build_call(
            contract.module.get_function("seal_terminate").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        address,
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
            "terminated",
        );
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
            HashTy::Keccak256 => ("seal_hash_keccak_256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("seal_hash_sha2_256", 32),
            HashTy::Blake2_128 => ("seal_hash_blake2_128", 16),
            HashTy::Blake2_256 => ("seal_hash_blake2_256", 32),
        };

        let res = contract.builder.build_array_alloca(
            contract.context.i8_type(),
            contract.context.i32_type().const_int(hashlen, false),
            "res",
        );

        contract.builder.build_call(
            contract.module.get_function(fname).unwrap(),
            &[input.into(), input_len.into(), res.into()],
            "hash",
        );

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

    /// Substrate events should be prefixed with the index of the event in the metadata
    fn event_id<'b>(&self, contract: &Contract<'b>, event_no: usize) -> Option<IntValue<'b>> {
        let event_id = contract
            .contract
            .sends_events
            .iter()
            .position(|e| *e == event_no)
            .unwrap();

        Some(contract.context.i8_type().const_int(event_id as u64, false))
    }

    /// Send event
    fn send_event<'b>(
        &self,
        contract: &Contract<'b>,
        event_no: usize,
        data_ptr: PointerValue<'b>,
        data_len: IntValue<'b>,
        topics: Vec<(PointerValue<'b>, IntValue<'b>)>,
    ) {
        let event = &contract.ns.events[event_no];

        let topic_count = topics.len() + if event.anonymous { 0 } else { 1 };
        let topic_size = contract.context.i32_type().const_int(
            if topic_count > 0 {
                32 * topic_count as u64 + 1
            } else {
                0
            },
            false,
        );

        let topic_buf = if topic_count > 0 {
            // the topic buffer is a vector of hashes.
            let topic_buf = contract.builder.build_array_alloca(
                contract.context.i8_type(),
                topic_size,
                "topic",
            );

            // a vector with scale encoding first has the length. Since we will never have more than
            // 64 topics (we're limited to 4 at the moment), we can assume this is a single byte
            contract.builder.build_store(
                topic_buf,
                contract
                    .context
                    .i8_type()
                    .const_int(topic_count as u64 * 4, false),
            );

            let mut dest = unsafe {
                contract.builder.build_gep(
                    topic_buf,
                    &[contract.context.i32_type().const_int(1, false)],
                    "dest",
                )
            };

            if !event.anonymous {
                let hash = contract.emit_global_string(
                    &format!("event_{}_signature", event),
                    blake2_rfc::blake2b::blake2b(32, &[], event.signature.as_bytes()).as_bytes(),
                    true,
                );

                contract.builder.build_call(
                    contract.module.get_function("__memcpy8").unwrap(),
                    &[
                        dest.into(),
                        hash.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                dest = unsafe {
                    contract.builder.build_gep(
                        dest,
                        &[contract.context.i32_type().const_int(32, false)],
                        "dest",
                    )
                };
            }

            for (ptr, len) in topics {
                contract.builder.build_call(
                    contract
                        .module
                        .get_function("seal_hash_blake2_256")
                        .unwrap(),
                    &[ptr.into(), len.into(), dest.into()],
                    "hash",
                );

                dest = unsafe {
                    contract.builder.build_gep(
                        dest,
                        &[contract.context.i32_type().const_int(32, false)],
                        "dest",
                    )
                };
            }

            topic_buf
        } else {
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null()
        };

        contract.builder.build_call(
            contract.module.get_function("seal_deposit_event").unwrap(),
            &[
                topic_buf.into(),
                topic_size.into(),
                data_ptr.into(),
                data_len.into(),
            ],
            "",
        );
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        contract: &Contract<'b>,
        expr: &ast::Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        macro_rules! get_seal_value {
            ($name:literal, $func:literal, $width:expr) => {{
                let scratch_buf = contract.builder.build_pointer_cast(
                    contract.scratch.unwrap().as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                contract.builder.build_store(
                    scratch_len,
                    contract
                        .context
                        .i32_type()
                        .const_int($width as u64 / 8, false),
                );

                contract.builder.build_call(
                    contract.module.get_function($func).unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    $name,
                );

                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract
                            .context
                            .custom_width_int_type($width)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    $name,
                )
            }};
        };

        match expr {
            ast::Expression::Builtin(_, _, ast::Builtin::Calldata, _) => {
                // allocate vector for input
                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[
                            contract.builder.build_load(
                                contract.calldata_len.as_pointer_value(),
                                "calldata_len",
                            ),
                            contract.context.i32_type().const_int(1, false).into(),
                            contract
                                .builder
                                .build_int_to_ptr(
                                    contract.context.i32_type().const_all_ones(),
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "no_initializer",
                                )
                                .into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

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

                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                // copy arguments from input buffer
                contract.builder.build_store(
                    scratch_len,
                    contract
                        .context
                        .i32_type()
                        .const_int(SCRATCH_SIZE as u64, false),
                );

                // retrieve the data
                contract.builder.build_call(
                    contract.module.get_function("seal_input").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "data",
                            )
                            .into(),
                        scratch_len.into(),
                    ],
                    "",
                );

                v
            }
            ast::Expression::Builtin(_, _, ast::Builtin::BlockNumber, _) => {
                let block_number =
                    get_seal_value!("block_number", "seal_block_number", 32).into_int_value();

                // Cast to 64 bit
                contract
                    .builder
                    .build_int_z_extend_or_bit_cast(
                        block_number,
                        contract.context.i64_type(),
                        "block_number",
                    )
                    .into()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Timestamp, _) => {
                let milliseconds = get_seal_value!("timestamp", "seal_now", 64).into_int_value();

                // Solidity expects the timestamp in seconds, not milliseconds
                contract
                    .builder
                    .build_int_unsigned_div(
                        milliseconds,
                        contract.context.i64_type().const_int(1000, false),
                        "seconds",
                    )
                    .into()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Gasleft, _) => {
                get_seal_value!("gas_left", "seal_gas_left", 64)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Gasprice, expr) => {
                // gasprice is available as "tx.gasprice" which will give you the price for one unit
                // of gas, or "tx.gasprice(uint64)" which will give you the price of N gas units
                let gas = if expr.is_empty() {
                    contract.context.i64_type().const_int(1, false)
                } else {
                    self.expression(contract, &expr[0], vartab, function)
                        .into_int_value()
                };

                let scratch_buf = contract.builder.build_pointer_cast(
                    contract.scratch.unwrap().as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                contract.builder.build_store(
                    scratch_len,
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.value_length as u64, false),
                );

                contract.builder.build_call(
                    contract.module.get_function("seal_weight_to_fee").unwrap(),
                    &[gas.into(), scratch_buf.into(), scratch_len.into()],
                    "gas_price",
                );

                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract
                            .context
                            .custom_width_int_type(contract.ns.value_length as u32 * 8)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "price",
                )
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Sender, _) => {
                get_seal_value!("caller", "seal_caller", 256)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Value, _) => {
                self.value_transferred(contract).into()
            }
            ast::Expression::Builtin(_, _, ast::Builtin::MinimumBalance, _) => get_seal_value!(
                "minimum_balance",
                "seal_minimum_balance",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::TombstoneDeposit, _) => get_seal_value!(
                "tombstone_deposit",
                "seal_tombstone_deposit",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::Random, args) => {
                let subject = self
                    .expression(contract, &args[0], vartab, function)
                    .into_pointer_value();

                let subject_data = unsafe {
                    contract.builder.build_gep(
                        subject,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(2, false),
                        ],
                        "subject_data",
                    )
                };

                let subject_len = unsafe {
                    contract.builder.build_gep(
                        subject,
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_zero(),
                        ],
                        "subject_len",
                    )
                };

                let scratch_buf = contract.builder.build_pointer_cast(
                    contract.scratch.unwrap().as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                contract.builder.build_store(
                    scratch_len,
                    contract.context.i32_type().const_int(32, false),
                );

                contract.builder.build_call(
                    contract.module.get_function("seal_random").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                subject_data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "subject_data",
                            )
                            .into(),
                        contract.builder.build_load(subject_len, "subject_len"),
                        scratch_buf.into(),
                        scratch_len.into(),
                    ],
                    "random",
                );

                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract
                            .context
                            .custom_width_int_type(256)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "hash",
                )
            }
            ast::Expression::Builtin(_, _, ast::Builtin::GetAddress, _) => {
                let scratch_buf = contract.builder.build_pointer_cast(
                    contract.scratch.unwrap().as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                contract.builder.build_store(
                    scratch_len,
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.address_length as u64, false),
                );

                contract.builder.build_call(
                    contract.module.get_function("seal_address").unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    "address",
                );

                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract.address_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "self_address",
                )
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Balance, _) => {
                let scratch_buf = contract.builder.build_pointer_cast(
                    contract.scratch.unwrap().as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "scratch_buf",
                );
                let scratch_len = contract.scratch_len.unwrap().as_pointer_value();

                contract.builder.build_store(
                    scratch_len,
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.value_length as u64, false),
                );

                contract.builder.build_call(
                    contract.module.get_function("seal_balance").unwrap(),
                    &[scratch_buf.into(), scratch_len.into()],
                    "balance",
                );

                contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        scratch_buf,
                        contract.value_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "balance",
                )
            }
            _ => unimplemented!(),
        }
    }
}
