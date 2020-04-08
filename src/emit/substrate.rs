use resolver;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use num_traits::ToPrimitive;

use super::{Contract, TargetRuntime};

pub struct SubstrateTarget {}

const ADDRESS_LENGTH: u64 = 20;

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        ns: &'a resolver::Namespace,
        filename: &'a str,
    ) -> Contract<'a> {
        let mut c = Contract::new(context, contract, ns, filename, None);
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
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // data_ptr
                    contract.context.i32_type().into(), // data_len
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
        to: Option<PointerValue<'b>>,
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
                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                (val.into(), 1)
            }
            resolver::Type::Uint(n) | resolver::Type::Int(n) => {
                let int_type = contract.context.custom_width_int_type(*n as u32);

                let store = to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "stack"));

                let val = contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        src,
                        int_type.ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "",
                );

                let len = *n as u64 / 8;

                if *n <= 64 && to.is_none() {
                    (val, len)
                } else {
                    contract.builder.build_store(store, val);

                    (store.into(), len)
                }
            }
            resolver::Type::Bytes(len) => {
                let int_type = contract.context.custom_width_int_type(*len as u32 * 8);

                let store = to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "stack"));

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        src.into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
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

                if *len <= 8 && to.is_none() {
                    (
                        contract.builder.build_load(store, &format!("bytes{}", len)),
                        *len as u64,
                    )
                } else {
                    (store.into(), *len as u64)
                }
            }
            resolver::Type::Address => {
                let int_type = contract.context.custom_width_int_type(160);

                let store =
                    to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "address"));

                // byte order needs to be reversed
                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        src.into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(ADDRESS_LENGTH, false)
                            .into(),
                    ],
                    "",
                );

                (store.into(), ADDRESS_LENGTH)
            }
            _ => unimplemented!(),
        }
    }

    /// recursively encode a single ty
    fn decode_ty<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        ty: &resolver::Type,
        to: Option<PointerValue<'b>>,
        data: &mut PointerValue<'b>,
    ) -> BasicValueEnum<'b> {
        match &ty {
            resolver::Type::Bool
            | resolver::Type::Address
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                let (arg, arglen) = self.decode_primitive(contract, ty, to, *data);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "abi_ptr",
                    )
                };
                arg
            }
            resolver::Type::Enum(n) => {
                self.decode_ty(contract, function, &contract.ns.enums[*n].ty, to, data)
            }
            resolver::Type::Struct(n) => {
                let to =
                    to.unwrap_or_else(|| contract.builder.build_alloca(contract.llvm_type(ty), ""));

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            to,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    if field.ty.is_reference_type() {
                        let val = contract
                            .builder
                            .build_alloca(contract.llvm_type(&field.ty), "");

                        self.decode_ty(contract, function, &field.ty, Some(val), data);

                        contract.builder.build_store(elem, val);
                    } else {
                        self.decode_ty(contract, function, &field.ty, Some(elem), data);
                    }
                }

                to.into()
            }
            resolver::Type::Array(_, dim) => {
                let to =
                    to.unwrap_or_else(|| contract.builder.build_alloca(contract.llvm_type(ty), ""));

                if let Some(d) = &dim[0] {
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
                                    to,
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let ty = ty.array_deref();

                            if ty.is_reference_type() {
                                let val = contract
                                    .builder
                                    .build_alloca(contract.llvm_type(&ty.deref()), "");
                                self.decode_ty(contract, function, &ty, Some(val), data);
                                contract.builder.build_store(elem, val);
                            } else {
                                self.decode_ty(contract, function, &ty, Some(elem), data);
                            }
                        },
                    );

                    to.into()
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

                            if ty.is_reference_type() {
                                let val = contract
                                    .builder
                                    .build_alloca(contract.llvm_type(&ty.deref()), "");
                                self.decode_ty(contract, function, &ty, Some(val), data);
                                contract.builder.build_store(elem, val);
                            } else {
                                self.decode_ty(contract, function, &ty, Some(elem), data);
                            }
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
                    .unwrap()
                    .into_pointer_value();

                *data = contract
                    .builder
                    .build_load(from, "data")
                    .into_pointer_value();

                v.into()
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(ty) => self.decode_ty(contract, function, ty, to, data),
        }
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        ty: &resolver::Type,
        dest: PointerValue,
        val: BasicValueEnum,
    ) -> u64 {
        match ty {
            resolver::Type::Bool => {
                let val = if val.is_pointer_value() {
                    contract.builder.build_load(val.into_pointer_value(), "")
                } else {
                    val
                };

                contract.builder.build_store(
                    dest,
                    contract.builder.build_int_z_extend(
                        val.into_int_value(),
                        contract.context.i8_type(),
                        "bool",
                    ),
                );
                1
            }
            resolver::Type::Uint(n) | resolver::Type::Int(n) => {
                let val = if val.is_pointer_value() {
                    contract.builder.build_load(val.into_pointer_value(), "")
                } else {
                    val
                };

                contract.builder.build_store(
                    contract.builder.build_pointer_cast(
                        dest,
                        val.into_int_value()
                            .get_type()
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    val.into_int_value(),
                );

                *n as u64 / 8
            }
            resolver::Type::Bytes(n) => {
                let val = if val.is_pointer_value() {
                    val.into_pointer_value()
                } else {
                    let temp = contract
                        .builder
                        .build_alloca(val.into_int_value().get_type(), &format!("bytes{}", n));

                    contract.builder.build_store(temp, val.into_int_value());

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
            resolver::Type::Address => {
                // byte order needs to be reversed
                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                val.into_pointer_value(),
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        dest.into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(ADDRESS_LENGTH, false)
                            .into(),
                    ],
                    "",
                );

                ADDRESS_LENGTH
            }
            _ => unimplemented!(),
        }
    }

    /// recursively encode argument. The encoded data is written to the data pointer,
    /// and the pointer is updated point after the encoded data.
    pub fn encode_ty<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        ty: &resolver::Type,
        arg: BasicValueEnum<'a>,
        data: &mut PointerValue<'a>,
    ) {
        let arg = if ty.is_reference_type() {
            contract.builder.build_load(arg.into_pointer_value(), "")
        } else {
            arg
        };

        match &ty {
            resolver::Type::Bool
            | resolver::Type::Address
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                let arglen = self.encode_primitive(contract, ty, *data, arg);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(arglen, false)],
                        "",
                    )
                };
            }
            resolver::Type::Enum(n) => {
                self.encode_primitive(contract, &contract.ns.enums[*n].ty, *data, arg);
            }
            resolver::Type::Array(_, dim) => {
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

                            self.encode_ty(contract, function, &ty.deref(), elem.into(), data);
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
                    let elem_ty = contract.llvm_type(&ty.array_elem());
                    let elem_size = contract.builder.build_int_truncate(
                        elem_ty.size_of().unwrap(),
                        contract.context.i32_type(),
                        "size_of",
                    );

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
                                elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            let ty = ty.array_deref();

                            self.encode_ty(contract, function, &ty.deref(), elem.into(), data);
                        },
                    );
                }
            }
            resolver::Type::Struct(n) => {
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

                    self.encode_ty(contract, function, &field.ty, elem.into(), data);
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(ty) => {
                self.encode_ty(contract, function, ty, arg, data);
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                *data = contract
                    .builder
                    .build_call(
                        contract.module.get_function("scale_encode_string").unwrap(),
                        &[(*data).into(), arg],
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
            resolver::Type::Address => contract.context.i32_type().const_int(ADDRESS_LENGTH, false),
            resolver::Type::Enum(n) => {
                self.encoded_length(arg, &contract.ns.enums[*n].ty, function, contract)
            }
            resolver::Type::Struct(n) => {
                let mut sum = contract.context.i32_type().const_zero();

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let mut elem = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    if field.ty.is_reference_type() {
                        elem = contract.builder.build_load(elem, "").into_pointer_value()
                    }

                    sum = contract.builder.build_int_add(
                        sum,
                        self.encoded_length(elem.into(), &field.ty, function, contract),
                        "",
                    );
                }

                sum
            }
            resolver::Type::Array(_, dims) => {
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

                let elem_ty = ty.array_elem();
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

                            let mut elem = unsafe {
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

                            if ty.is_reference_type() {
                                elem = contract.builder.build_load(elem, "").into_pointer_value()
                            }

                            *sum = contract.builder.build_int_add(
                                self.encoded_length(elem.into(), &elem_ty, function, contract),
                                *sum,
                                "",
                            );
                        },
                    );

                    sum
                } else {
                    // arg
                    let elem_ty = ty.array_deref();

                    let elem = unsafe {
                        contract.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_zero(),
                            ],
                            "index_access",
                        )
                    };

                    let arg = if elem_ty.is_reference_type() {
                        contract.builder.build_load(elem, "")
                    } else {
                        elem.into()
                    };

                    contract.builder.build_int_mul(
                        self.encoded_length(arg, &elem_ty, function, contract),
                        len,
                        "",
                    )
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(r) => self.encoded_length(arg, r, function, contract),
            resolver::Type::String | resolver::Type::DynamicBytes => {
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
        _datalength: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        let mut argsdata = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        for param in &spec.params {
            args.push(self.decode_ty(contract, function, &param.ty, None, &mut argsdata));
        }
    }

    /// Error encode
    fn error_encode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        arg: BasicValueEnum<'b>,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        // first calculate how much memory we need to allocate
        let length = contract.builder.build_int_add(
            contract.context.i32_type().const_int(4, false),
            self.encoded_length(arg, &resolver::Type::String, function, contract),
            "length",
        );

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

        contract.builder.build_store(
            contract.builder.build_pointer_cast(
                data,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "",
            ),
            contract.context.i32_type().const_int(0x08c3_79a0, false),
        );

        let mut argsdata = contract.builder.build_pointer_cast(
            unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract.context.i32_type().const_int(4, false)],
                    "",
                )
            },
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "",
        );

        argsdata = contract
            .builder
            .build_call(
                contract.module.get_function("scale_encode_string").unwrap(),
                &[argsdata.into(), arg],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

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

    ///  ABI encode the return values for the function
    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        // first calculate how much memory we need to allocate
        let mut length = contract.context.i32_type().const_zero();

        for (i, field) in spec.returns.iter().enumerate() {
            let val = if field.ty.is_reference_type() {
                contract
                    .builder
                    .build_load(args[i].into_pointer_value(), "")
            } else {
                args[i]
            };

            length = contract.builder.build_int_add(
                length,
                self.encoded_length(val, &field.ty, function, contract),
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

        for (i, arg) in spec.returns.iter().enumerate() {
            self.encode_ty(contract, function, &arg.ty, args[i], &mut argsdata);
        }

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
}
