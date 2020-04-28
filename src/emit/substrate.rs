use resolver;

use blake2_rfc;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use num_traits::ToPrimitive;

use super::{Contract, TargetRuntime};

pub struct SubstrateTarget {}

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        ns: &'a resolver::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Contract<'a> {
        let mut c = Contract::new(context, contract, ns, filename, opt, None);
        let b = SubstrateTarget {};

        b.declare_externals(&c);

        c.emit_functions(&b);

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
            "ext_return",
            "ext_print",
            "ext_instantiate",
            "ext_call",
        ]);

        c
    }

    fn public_function_prelude<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
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
            "ext_print",
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
    }

    fn emit_deploy(&self, contract: &Contract) {
        let initializer = contract.emit_initializer(self);

        // create deploy function
        let function = contract.module.add_function(
            "deploy",
            contract.context.i32_type().fn_type(&[], false),
            None,
        );

        let (deploy_args, deploy_args_length) = self.public_function_prelude(contract, function);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(
            &contract.contract.constructors,
            &contract.constructors,
            deploy_args,
            deploy_args_length,
            function,
            fallback_block,
            self,
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

        let (call_args, call_args_length) = self.public_function_prelude(contract, function);

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(
            &contract.contract.functions,
            &contract.functions,
            call_args,
            call_args_length,
            function,
            fallback_block,
            self,
        );

        // emit fallback code
        contract.builder.position_at_end(fallback_block);

        if let Some(fallback) = contract.contract.fallback_function() {
            contract
                .builder
                .build_call(contract.functions[fallback], &[], "");

            contract
                .builder
                .build_return(Some(&contract.context.i32_type().const_zero()));
        } else {
            contract
                .builder
                .build_return(Some(&contract.context.i32_type().const_int(2, false)));
        }
    }

    /// ABI decode a single primitive
    fn decode_primitive<'b>(
        &self,
        contract: &Contract<'b>,
        ty: &resolver::Type,
        src: PointerValue<'b>,
    ) -> (BasicValueEnum<'b>, u64) {
        match ty {
            resolver::Type::Bool => {
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
            resolver::Type::Contract(_)
            | resolver::Type::Address
            | resolver::Type::Uint(_)
            | resolver::Type::Int(_) => {
                let bits = match ty {
                    resolver::Type::Uint(n) | resolver::Type::Int(n) => *n as u32,
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
            resolver::Type::Bytes(len) => {
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

    /// Check that data has not overrun end. If last is true, then data must be equal to end
    fn check_overrun(
        &self,
        contract: &Contract,
        function: FunctionValue,
        data: PointerValue,
        end: PointerValue,
        last: bool,
    ) {
        let in_bounds = contract.builder.build_int_compare(
            if last {
                IntPredicate::EQ
            } else {
                IntPredicate::ULE
            },
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
        ty: &resolver::Type,
        data: &mut PointerValue<'b>,
        end: PointerValue<'b>,
    ) -> BasicValueEnum<'b> {
        match &ty {
            resolver::Type::Bool
            | resolver::Type::Address
            | resolver::Type::Contract(_)
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                let (arg, arglen) = self.decode_primitive(contract, ty, *data);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "abi_ptr",
                    )
                };

                self.check_overrun(contract, function, *data, end, false);

                arg
            }
            resolver::Type::Enum(n) => {
                self.decode_ty(contract, function, &contract.ns.enums[*n].ty, data, end)
            }
            resolver::Type::Struct(n) => {
                let llvm_ty = contract.llvm_type(ty.deref());

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
            resolver::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let llvm_ty = contract.llvm_type(ty.deref());

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
            resolver::Type::String | resolver::Type::DynamicBytes => {
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

                self.check_overrun(contract, function, *data, end, false);

                v
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(ty) => self.decode_ty(contract, function, ty, data, end),
        }
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        load: bool,
        ty: &resolver::Type,
        dest: PointerValue,
        arg: BasicValueEnum,
    ) -> u64 {
        match ty {
            resolver::Type::Bool => {
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
            resolver::Type::Contract(_)
            | resolver::Type::Address
            | resolver::Type::Uint(_)
            | resolver::Type::Int(_) => {
                let len = match ty {
                    resolver::Type::Uint(n) | resolver::Type::Int(n) => *n as u64 / 8,
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
            resolver::Type::Bytes(n) => {
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
        function: FunctionValue,
        ty: &resolver::Type,
        arg: BasicValueEnum<'a>,
        data: &mut PointerValue<'a>,
    ) {
        match &ty {
            resolver::Type::Bool
            | resolver::Type::Address
            | resolver::Type::Contract(_)
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                let arglen = self.encode_primitive(contract, load, ty, *data, arg);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "",
                    )
                };
            }
            resolver::Type::Enum(n) => {
                self.encode_primitive(contract, load, &contract.ns.enums[*n].ty, *data, arg);
            }
            resolver::Type::Array(_, dim) => {
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
                                function,
                                &ty.deref(),
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
                                function,
                                &ty.deref(),
                                elem.into(),
                                data,
                            );
                        },
                    );
                }
            }
            resolver::Type::Struct(n) => {
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

                    self.encode_ty(contract, true, function, &field.ty, elem.into(), data);
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(ty) => {
                self.encode_ty(contract, load, function, ty, arg, data);
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

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
                                    function.get_type().get_param_types()[1].into_pointer_type(),
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
            }
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
        ty: &resolver::Type,
        function: FunctionValue,
        contract: &Contract<'a>,
    ) -> IntValue<'a> {
        match ty {
            resolver::Type::Bool => contract.context.i32_type().const_int(1, false),
            resolver::Type::Uint(n) | resolver::Type::Int(n) => {
                contract.context.i32_type().const_int(*n as u64 / 8, false)
            }
            resolver::Type::Bytes(n) => contract.context.i32_type().const_int(*n as u64, false),
            resolver::Type::Address | resolver::Type::Contract(_) => contract
                .context
                .i32_type()
                .const_int(contract.ns.address_length as u64, false),
            resolver::Type::Enum(n) => {
                self.encoded_length(arg, load, &contract.ns.enums[*n].ty, function, contract)
            }
            resolver::Type::Struct(n) => {
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
                        self.encoded_length(elem.into(), true, &field.ty, function, contract),
                        "",
                    );
                }

                sum
            }
            resolver::Type::Array(_, dims) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut dynamic_array = false;

                let len = match dims.last().unwrap() {
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
                    let mut sum = contract.context.i32_type().const_zero();

                    contract.emit_static_loop_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        &mut sum,
                        |index, sum| {
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

                            let elem = if dynamic_array {
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
                                    &elem_ty,
                                    function,
                                    contract,
                                ),
                                *sum,
                                "",
                            );
                        },
                    );

                    sum
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

                    contract.builder.build_int_mul(
                        self.encoded_length(elem.into(), true, &elem_ty, function, contract),
                        len,
                        "",
                    )
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(r) => self.encoded_length(arg, load, r, function, contract),
            resolver::Type::String | resolver::Type::DynamicBytes => {
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

                contract.builder.build_int_add(
                    contract
                        .builder
                        .build_load(len, "string.len")
                        .into_int_value(),
                    contract.context.i32_type().const_int(5, false),
                    "",
                )
            }
        }
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
        spec: &[resolver::Parameter],
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
            let v = self.decode_ty(contract, function, &param.ty, &mut argsdata, argsend);

            args.push(if param.ty.stack_based() && !param.ty.is_reference_type() {
                let s = contract.builder.build_alloca(v.get_type(), &param.name);

                contract.builder.build_store(s, v);

                s.into()
            } else {
                v
            });
        }

        // Actually we always end with two checks: with last = false, and then another
        // with last = true. We rely on llvm to optimize the former away
        self.check_overrun(contract, function, argsdata, argsend, true);
    }

    ///  ABI encode the return values for the function
    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<u32>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[resolver::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        // first calculate how much memory we need to allocate
        let mut length = contract.context.i32_type().const_zero();

        // note that encoded_length overestimates how data we need
        for (i, field) in spec.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                self.encoded_length(args[i], load, &field.ty, function, contract),
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
            self.encode_ty(contract, load, function, &arg.ty, args[i], &mut argsdata);
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
            contract.module.get_function("ext_print").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }

    fn create_contract<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        contract_no: usize,
        constructor_no: usize,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
    ) {
        let resolver_contract = &contract.ns.contracts[contract_no];
        let constructor = &resolver_contract.constructors[constructor_no];

        // input
        let (input, input_len) = self.abi_encode(
            contract,
            Some(constructor.selector()),
            false,
            function,
            args,
            &constructor.params,
        );

        // balance is a u64
        let balance = contract.emit_global_string("balance", &[0u8; 4], true);

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

        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("ext_instantiate").unwrap(),
                &[
                    codehash.into(),
                    contract.context.i32_type().const_int(32, false).into(),
                    contract.context.i64_type().const_zero().into(),
                    balance.into(),
                    contract.context.i32_type().const_int(8, false).into(),
                    input.into(),
                    input_len.into(),
                ],
                "",
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

        contract.builder.build_return(Some(&ret));
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
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
    ) -> IntValue<'b> {
        // balance is a u64
        let balance = contract.emit_global_string("balance", &[0u8; 8], true);

        // call create
        contract
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
                    contract.context.i64_type().const_zero().into(),
                    balance.into(),
                    contract.context.i32_type().const_int(8, false).into(),
                    payload.into(),
                    payload_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    fn return_data<'b>(&self, contract: &Contract<'b>) -> (PointerValue<'b>, IntValue<'b>) {
        let length = contract
            .builder
            .build_call(
                contract.module.get_function("ext_scratch_size").unwrap(),
                &[],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let return_data = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[length],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                return_data.into(),
                contract.context.i32_type().const_zero().into(),
                length,
            ],
            "",
        );

        (return_data, length.into_int_value())
    }
}
