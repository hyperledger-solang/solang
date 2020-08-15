use blake2_rfc;
use codegen::cfg::HashTy;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use num_traits::ToPrimitive;
use parser::pt;
use sema::ast;
use std::collections::HashMap;

use super::{Contract, TargetRuntime, Variable};

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
        let mut b = SubstrateTarget {
            unique_strings: HashMap::new(),
        };

        b.declare_externals(&c);

        c.emit_functions(&mut b);

        b.emit_deploy(&c);
        b.emit_call(&c);

        c.internalize(&[
            "deploy",
            "call",
            "ext_scratch_size",
            "ext_scratch_read",
            "ext_scratch_write",
            "ext_set_storage",
            "ext_get_storage",
            "ext_clear_storage",
            "ext_hash_keccak_256",
            "ext_hash_sha2_256",
            "ext_hash_blake2_128",
            "ext_hash_blake2_256",
            "ext_return",
            "ext_println",
            "ext_instantiate",
            "ext_call",
            "ext_value_transferred",
            "ext_minimum_balance",
            "ext_random",
            "ext_address",
            "ext_balance",
            "ext_block_number",
            "ext_now",
            "ext_gas_price",
            "ext_gas_left",
            "ext_caller",
            "ext_tombstone_deposit",
            "ext_terminate",
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

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "",
        );

        // copy arguments from scratch buffer
        let args_length = contract
            .builder
            .build_call(
                contract.module.get_function("ext_scratch_size").unwrap(),
                &[],
                "scratch_size",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        contract.builder.build_store(
            contract.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        let args = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[args_length],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        contract
            .builder
            .build_store(contract.calldata_data.as_pointer_value(), args);

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                args.into(),
                contract.context.i32_type().const_zero().into(),
                args_length,
            ],
            "",
        );

        let args = contract.builder.build_pointer_cast(
            args,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );

        // after copying stratch, first thing to do is abort value transfers if constructors not payable
        if abort_value_transfers {
            contract.abort_if_value_transfer(self, function);
        }

        (args, args_length.into_int_value())
    }

    fn declare_externals(&self, contract: &Contract) {
        let u8_ptr = contract
            .context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .into();
        let u32_val = contract.context.i32_type().into();
        let u64_val = contract.context.i64_type().into();

        // Access to scratch buffer
        contract.module.add_function(
            "ext_scratch_size",
            contract.context.i32_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_scratch_read",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                    contract.context.i32_type().into(), // offset
                    contract.context.i32_type().into(), // len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_hash_keccak_256",
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
            "ext_hash_sha2_256",
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
            "ext_hash_blake2_128",
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
            "ext_hash_blake2_256",
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
            "ext_scratch_write",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // dest_ptr
                    contract.context.i32_type().into(), // len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_random",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // subject_ptr
                    contract.context.i32_type().into(), // subject_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_set_storage",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // key_ptr
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // value_ptr
                    contract.context.i32_type().into(), // value_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_println",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // string_ptr
                    contract.context.i32_type().into(), // string_len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_clear_storage",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // key_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_get_storage",
            contract.context.i32_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // key_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_return",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // data ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_instantiate",
            contract.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // code hash ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_call",
            contract.context.i32_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                    u64_val, // gas
                    u8_ptr, u32_val, // value ptr and len
                    u8_ptr, u32_val, // input ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_value_transferred",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_address",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_balance",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_minimum_balance",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_block_number",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_now",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_tombstone_deposit",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_gas_price",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_gas_left",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_caller",
            contract.context.void_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "ext_terminate",
            contract.context.void_type().fn_type(
                &[
                    u8_ptr, u32_val, // address ptr and len
                ],
                false,
            ),
            Some(Linkage::External),
        );
    }

    fn emit_deploy(&mut self, contract: &Contract) {
        let initializer = contract.emit_initializer(self);

        // create deploy function
        let function = contract.module.add_function(
            "deploy",
            contract.context.i32_type().fn_type(&[], false),
            None,
        );

        // deploy always receives an endowment so no value check here
        let (deploy_args, deploy_args_length) =
            self.public_function_prelude(contract, function, false);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(
            pt::FunctionTy::Constructor,
            deploy_args,
            deploy_args_length,
            function,
            Some(fallback_block),
            self,
            |_| false,
        );

        // emit fallback code
        contract.builder.position_at_end(fallback_block);
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(2, false)));
    }

    fn emit_call(&self, contract: &Contract) {
        // create call function
        let function = contract.module.add_function(
            "call",
            contract.context.i32_type().fn_type(&[], false),
            None,
        );

        let (call_args, call_args_length) = self.public_function_prelude(
            contract,
            function,
            contract.function_abort_value_transfers,
        );

        contract.emit_function_dispatch(
            pt::FunctionTy::Function,
            call_args,
            call_args_length,
            function,
            None,
            self,
            |func| !contract.function_abort_value_transfers && !func.is_payable(),
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

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(3, false)));

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

                    v.into()
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
    pub fn encode_ty<'a>(
        &self,
        contract: &Contract<'a>,
        load: bool,
        packed: bool,
        function: FunctionValue,
        ty: &ast::Type,
        arg: BasicValueEnum<'a>,
        data: &mut PointerValue<'a>,
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
            _ => unreachable!(),
        };
    }

    /// Calculate the maximum space a type will need when encoded. This is used for
    /// allocating enough space to do abi encoding. The length for vectors is always
    /// assumed to be five, even when it can be encoded in less bytes. The overhead
    /// of calculating the exact size is not worth reducing the malloc by a few bytes.
    pub fn encoded_length<'a>(
        &self,
        arg: BasicValueEnum<'a>,
        load: bool,
        packed: bool,
        ty: &ast::Type,
        function: FunctionValue,
        contract: &Contract<'a>,
    ) -> IntValue<'a> {
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
            _ => unreachable!(),
        }
    }

    /// Create a unique salt each time this function is called.
    fn contract_unique_salt<'a>(
        &mut self,
        contract: &'a Contract,
        contract_no: usize,
    ) -> (PointerValue<'a>, IntValue<'a>) {
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

impl TargetRuntime for SubstrateTarget {
    fn clear_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) {
        contract.builder.build_call(
            contract.module.get_function("ext_clear_storage").unwrap(),
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

    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    ) {
        // TODO: check for non-zero
        contract.builder.build_call(
            contract.module.get_function("ext_set_storage").unwrap(),
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

    fn set_storage_string<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
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
            contract.module.get_function("ext_set_storage").unwrap(),
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
    fn get_storage_int<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let exists = contract
            .builder
            .build_call(
                contract.module.get_function("ext_get_storage").unwrap(),
                &[contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
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

        let dest = contract.builder.build_alloca(ty, "int");

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                ty.size_of()
                    .const_cast(contract.context.i32_type(), false)
                    .into(),
            ],
            "",
        );

        let loaded_int = contract.builder.build_load(dest, "int");

        contract.builder.build_unconditional_branch(done_storage);

        contract.builder.position_at_end(done_storage);

        let res = contract.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[(&loaded_int, retrieve_block), (&ty.const_zero(), entry)]);

        res.as_basic_value().into_int_value()
    }

    /// Read string from substrate storage
    fn get_storage_string<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        contract
            .builder
            .build_call(
                contract
                    .module
                    .get_function("substrate_get_string")
                    .unwrap(),
                &[contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Read string from substrate storage
    fn get_storage_bytes_subscript<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a> {
        contract
            .builder
            .build_call(
                contract
                    .module
                    .get_function("substrate_get_string_subscript")
                    .unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    index.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    fn set_storage_bytes_subscript<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        index: IntValue<'a>,
        val: IntValue<'a>,
    ) {
        contract.builder.build_call(
            contract
                .module
                .get_function("substrate_set_string_subscript")
                .unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                index.into(),
                val.into(),
            ],
            "",
        );
    }

    /// Push a byte onto a bytes string in storage
    fn storage_bytes_push<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        val: IntValue<'a>,
    ) {
        contract.builder.build_call(
            contract
                .module
                .get_function("substrate_bytes_push")
                .unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                val.into(),
            ],
            "",
        );
    }

    /// Pop a value from a bytes string
    fn storage_bytes_pop<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        contract
            .builder
            .build_call(
                contract.module.get_function("substrate_bytes_pop").unwrap(),
                &[contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    /// Calculate length of storage dynamic bytes
    fn storage_string_length<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        contract
            .builder
            .build_call(
                contract
                    .module
                    .get_function("substrate_string_length")
                    .unwrap(),
                &[contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    fn return_empty_abi(&self, contract: &Contract) {
        // This will clear the scratch buffer
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_write").unwrap(),
            &[
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

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_zero()));
    }

    fn return_u32<'b>(&self, contract: &'b Contract, ret: IntValue<'b>) {
        contract.builder.build_return(Some(&ret));
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
            contract.module.get_function("ext_hash_keccak_256").unwrap(),
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
            contract.module.get_function("ext_scratch_write").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_zero()));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_write").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));
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
        selector: Option<u32>,
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

        if selector.is_some() {
            length = contract.builder.build_int_add(
                length,
                contract
                    .context
                    .i32_type()
                    .const_int(std::mem::size_of::<u32>() as u64, false),
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
                    contract.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                contract
                    .context
                    .i32_type()
                    .const_int(selector as u64, false),
            );

            argsdata = unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract
                        .context
                        .i32_type()
                        .const_int(std::mem::size_of_val(&selector) as u64, false)],
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
            contract.module.get_function("ext_println").unwrap(),
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
        constructor_no: usize,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
    ) {
        let resolver_contract = &contract.ns.contracts[contract_no];

        let constructor = &resolver_contract
            .functions
            .iter()
            .filter(|f| f.is_constructor())
            .nth(constructor_no)
            .unwrap();

        let mut args = args.to_vec();
        let mut params = constructor.params.to_vec();

        // salt
        let salt_ty = ast::Type::Uint(256);

        if let Some(salt) = salt {
            args.push(salt.into());
        } else {
            let salt = contract
                .builder
                .build_alloca(contract.llvm_type(&salt_ty), "salt");

            let (ptr, len) = self.contract_unique_salt(contract, contract_no);

            contract.builder.build_call(
                contract.module.get_function("ext_random").unwrap(),
                &[ptr.into(), len.into()],
                "random",
            );

            contract.builder.build_call(
                contract.module.get_function("ext_scratch_read").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            salt,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    contract.context.i32_type().const_zero().into(),
                    contract.context.i32_type().const_int(32, false).into(),
                ],
                "random",
            );

            args.push(contract.builder.build_load(salt, "salt"));
        }

        params.push(ast::Parameter {
            loc: pt::Loc(0, 0, 0),
            ty: salt_ty,
            name: "salt".to_string(),
        });

        // input
        let (input, input_len) = self.abi_encode(
            contract,
            Some(constructor.selector()),
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
            contract.builder.build_call(
                contract.module.get_function("ext_minimum_balance").unwrap(),
                &[],
                "minimum_balance",
            );

            contract.builder.build_call(
                contract.module.get_function("ext_scratch_read").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    contract.context.i32_type().const_zero().into(),
                    contract
                        .context
                        .i32_type()
                        .const_int(contract.ns.value_length as u64, false)
                        .into(),
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

        let wasm = target_contract.wasm(true).expect("compile should succeeed");

        // code hash
        let codehash = contract.emit_global_string(
            &format!("contract_{}_codehash", resolver_contract.name),
            blake2_rfc::blake2b::blake2b(32, &[], &wasm).as_bytes(),
            true,
        );

        // ext_instantiate returns 0x0100 if the contract cannot be instantiated
        // due to insufficient funds, etc. If the return value is < 0x100, then
        // this is return value from the constructor (or deploy function) of
        // the contract
        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("ext_instantiate").unwrap(),
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

        // scratch buffer contains address
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                address.into(),
                contract.context.i32_type().const_zero().into(),
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.address_length as u64, false)
                    .into(),
            ],
            "",
        );

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

            // if ext_instantiate returned 0x100, we cannot return that here. This is because
            // only the lower 8 bits of our return value are taken.

            // ext_call can return 0x100 if the call cannot be made. We cannot return this value
            // from the smart contract, so replace it with 4.
            let call_not_made = contract.builder.build_int_compare(
                IntPredicate::EQ,
                ret,
                contract.context.i32_type().const_int(0x100, false),
                "success",
            );

            let ret = contract
                .builder
                .build_select(
                    call_not_made,
                    contract.context.i32_type().const_int(4, false),
                    ret,
                    "return_value",
                )
                .into_int_value();

            contract.builder.build_return(Some(&ret));

            contract.builder.position_at_end(success_block);
        }
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        _ty: ast::CallTy,
    ) -> IntValue<'b> {
        // balance is a u128
        let value_ptr = contract
            .builder
            .build_alloca(contract.value_type(), "balance");
        contract.builder.build_store(value_ptr, value);

        // do the actual call
        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("ext_call").unwrap(),
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
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        // ext_call can return 0x100 if the call cannot be made. We cannot return this value
        // from the smart contract, so replace it with 4.
        let call_not_made = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            contract.context.i32_type().const_int(0x100, false),
            "success",
        );

        contract
            .builder
            .build_select(
                call_not_made,
                contract.context.i32_type().const_int(4, false),
                ret,
                "return_value",
            )
            .into_int_value()
    }

    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let length = contract
            .builder
            .build_call(
                contract.module.get_function("ext_scratch_size").unwrap(),
                &[],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

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

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                length.into(),
            ],
            "",
        );

        v
    }

    /// Substrate value is usually 128 bits
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        let value = contract
            .builder
            .build_alloca(contract.value_type(), "value_transferred");

        contract.builder.build_call(
            contract
                .module
                .get_function("ext_value_transferred")
                .unwrap(),
            &[],
            "value_transferred",
        );

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        value,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.value_length as u64, false)
                    .into(),
            ],
            "value_transferred",
        );

        contract
            .builder
            .build_load(value, "value_transferred")
            .into_int_value()
    }

    /// Substrate value is usually 128 bits
    fn balance<'b>(&self, contract: &Contract<'b>, _addr: IntValue<'b>) -> IntValue<'b> {
        let value = contract
            .builder
            .build_alloca(contract.value_type(), "balance");

        contract.builder.build_call(
            contract.module.get_function("ext_balance").unwrap(),
            &[],
            "balance",
        );

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        value,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.value_length as u64, false)
                    .into(),
            ],
            "balance",
        );

        contract
            .builder
            .build_load(value, "balance")
            .into_int_value()
    }

    /// Substrate value is usually 128 bits
    fn get_address<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        let value = contract
            .builder
            .build_alloca(contract.address_type(), "self_address");

        contract.builder.build_call(
            contract.module.get_function("ext_address").unwrap(),
            &[],
            "self_address",
        );

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        value,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                contract
                    .context
                    .i32_type()
                    .const_int(contract.ns.address_length as u64, false)
                    .into(),
            ],
            "self_address",
        );

        contract
            .builder
            .build_load(value, "self_address")
            .into_int_value()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, contract: &Contract<'b>, addr: IntValue<'b>) {
        let address = contract
            .builder
            .build_alloca(contract.address_type(), "address");

        contract.builder.build_store(address, addr);

        contract.builder.build_call(
            contract.module.get_function("ext_terminate").unwrap(),
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
            HashTy::Keccak256 => ("ext_hash_keccak_256", 32),
            HashTy::Ripemd160 => ("ripemd160", 20),
            HashTy::Sha256 => ("ext_hash_sha2_256", 32),
            HashTy::Blake2_128 => ("ext_hash_blake2_128", 16),
            HashTy::Blake2_256 => ("ext_hash_blake2_256", 32),
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

    /// builtin expressions
    fn builtin<'b>(
        &self,
        contract: &Contract<'b>,
        expr: &ast::Expression,
        vartab: &[Variable<'b>],
        function: FunctionValue<'b>,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'b> {
        macro_rules! get_seal_value {
            ($name:literal, $func:literal, $width:expr) => {{
                let value = contract
                    .builder
                    .build_alloca(contract.context.custom_width_int_type($width), $name);

                contract.builder.build_call(
                    contract.module.get_function($func).unwrap(),
                    &[],
                    $name,
                );

                contract.builder.build_call(
                    contract.module.get_function("ext_scratch_read").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                value,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract.context.i32_type().const_zero().into(),
                        contract
                            .context
                            .i32_type()
                            .const_int($width as u64 / 8, false)
                            .into(),
                    ],
                    $name,
                );

                contract.builder.build_load(value, $name)
            }};
        };

        match expr {
            ast::Expression::Builtin(_, _, ast::Builtin::BlockNumber, _) => {
                let block_number =
                    get_seal_value!("block_number", "ext_block_number", 32).into_int_value();

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
                let milliseconds = get_seal_value!("timestamp", "ext_now", 64).into_int_value();

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
                get_seal_value!("gas_left", "ext_gas_left", 64)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Gasprice, _) => get_seal_value!(
                "gas_price",
                "ext_gas_price",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::Sender, _) => {
                get_seal_value!("caller", "ext_caller", 256)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Value, _) => get_seal_value!(
                "value",
                "ext_value_transferred",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::MinimumBalance, _) => get_seal_value!(
                "minimum_balance",
                "ext_minimum_balance",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::TombstoneDeposit, _) => get_seal_value!(
                "tombstone_deposit",
                "ext_tombstone_deposit",
                contract.ns.value_length as u32 * 8
            ),
            ast::Expression::Builtin(_, _, ast::Builtin::Random, args) => {
                let subject = contract
                    .expression(&args[0], vartab, function, runtime)
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

                contract.builder.build_call(
                    contract.module.get_function("ext_random").unwrap(),
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
                    ],
                    "random",
                );

                let hash = contract
                    .builder
                    .build_alloca(contract.context.custom_width_int_type(256), "hash");

                contract.builder.build_call(
                    contract.module.get_function("ext_scratch_read").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                hash,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract.context.i32_type().const_zero().into(),
                        contract.context.i32_type().const_int(32, false).into(),
                    ],
                    "random",
                );

                contract.builder.build_load(hash, "hash")
            }
            _ => unimplemented!(),
        }
    }
}
