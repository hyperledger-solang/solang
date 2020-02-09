use num_traits::ToPrimitive;
use parser::ast;
use resolver;

use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;

use super::Contract;

pub struct EthAbiEncoder {}

impl EthAbiEncoder {
    /// recursively encode argument. The encoded data is written to the data pointer,
    /// and the pointer is updated point after the encoded data.
    pub fn encode_ty<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
        ty: &resolver::Type,
        arg: BasicValueEnum,
        data: &mut PointerValue<'a>,
    ) {
        match &ty {
            resolver::Type::Primitive(e) => {
                self.encode_primitive(contract, e, *data, arg);

                *data = unsafe {
                    contract.builder.build_gep(
                        *data,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            resolver::Type::Enum(n) => {
                self.encode_primitive(contract, &contract.ns.enums[*n].ty, *data, arg);
            }
            resolver::Type::FixedArray(_, dim) => {
                contract.emit_static_loop(
                    function,
                    0,
                    dim[0].to_u64().unwrap(),
                    data,
                    |index, data| {
                        let elem = unsafe {
                            contract
                                .builder
                                .build_gep(
                                    arg.into_pointer_value(),
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                                .into()
                        };

                        self.encode_ty(contract, function, &ty.deref(), elem, data);
                    },
                );
            }
            resolver::Type::Struct(n) => {
                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract
                            .builder
                            .build_gep(
                                arg.into_pointer_value(),
                                &[
                                    contract.context.i32_type().const_zero(),
                                    contract.context.i32_type().const_int(i as u64, false),
                                ],
                                &field.name,
                            )
                            .into()
                    };

                    self.encode_ty(contract, function, &field.ty, elem, data);
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Ref(ty) => {
                self.encode_ty(contract, function, ty, arg, data);
            }
        };
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        ty: &ast::PrimitiveType,
        dest: PointerValue,
        val: BasicValueEnum,
    ) {
        match ty {
            ast::PrimitiveType::Bool => {
                // first clear
                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__bzero8").unwrap(),
                    &[
                        dest8.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                let val = if val.is_pointer_value() {
                    contract.builder.build_load(val.into_pointer_value(), "")
                } else {
                    val
                };
                let value = contract.builder.build_select(
                    val.into_int_value(),
                    contract.context.i8_type().const_int(1, false),
                    contract.context.i8_type().const_zero(),
                    "bool_val",
                );

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                contract.builder.build_store(dest, value);
            }
            ast::PrimitiveType::Int(8) | ast::PrimitiveType::Uint(8) => {
                let signval = if let ast::PrimitiveType::Int(8) = ty {
                    let negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        val.into_int_value(),
                        contract.context.i8_type().const_zero(),
                        "neg",
                    );

                    contract
                        .builder
                        .build_select(
                            negative,
                            contract.context.i64_type().const_zero(),
                            contract.context.i64_type().const_int(std::u64::MAX, true),
                            "val",
                        )
                        .into_int_value()
                } else {
                    contract.context.i64_type().const_zero()
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__memset8").unwrap(),
                    &[
                        dest8.into(),
                        signval.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                let val = if val.is_pointer_value() {
                    contract.builder.build_load(val.into_pointer_value(), "")
                } else {
                    val
                };
                contract.builder.build_store(dest, val);
            }
            ast::PrimitiveType::Address
            | ast::PrimitiveType::Uint(_)
            | ast::PrimitiveType::Int(_) => {
                let n = match ty {
                    ast::PrimitiveType::Address => 160,
                    ast::PrimitiveType::Uint(b) => *b,
                    ast::PrimitiveType::Int(b) => *b,
                    _ => unreachable!(),
                };

                // first clear/set the upper bits
                if n < 256 {
                    let signval = if let ast::PrimitiveType::Int(8) = ty {
                        let negative = contract.builder.build_int_compare(
                            IntPredicate::SLT,
                            val.into_int_value(),
                            contract.context.i8_type().const_zero(),
                            "neg",
                        );

                        contract
                            .builder
                            .build_select(
                                negative,
                                contract.context.i64_type().const_zero(),
                                contract.context.i64_type().const_int(std::u64::MAX, true),
                                "val",
                            )
                            .into_int_value()
                    } else {
                        contract.context.i64_type().const_zero()
                    };

                    let dest8 = contract.builder.build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid",
                    );

                    contract.builder.build_call(
                        contract.module.get_function("__memset8").unwrap(),
                        &[
                            dest8.into(),
                            signval.into(),
                            contract.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = contract.context.custom_width_int_type(n as u32);
                let type_size = int_type.size_of();

                let store = if val.is_pointer_value() {
                    val.into_pointer_value()
                } else {
                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_store(store, val);

                    store
                };

                contract.builder.build_call(
                    contract.module.get_function("__leNtobe32").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                dest,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "dest",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "")
                            .into(),
                    ],
                    "",
                );
            }
            ast::PrimitiveType::Bytes(1) => {
                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__bzero8").unwrap(),
                    &[
                        dest8.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                let val = if val.is_pointer_value() {
                    contract.builder.build_load(val.into_pointer_value(), "")
                } else {
                    val
                };
                contract.builder.build_store(dest8, val);
            }
            ast::PrimitiveType::Bytes(b) => {
                // first clear/set the upper bits
                if *b < 32 {
                    let dest8 = contract.builder.build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid",
                    );

                    contract.builder.build_call(
                        contract.module.get_function("__bzero8").unwrap(),
                        &[
                            dest8.into(),
                            contract.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = contract.context.custom_width_int_type(*b as u32 * 8);
                let type_size = int_type.size_of();

                let store = if val.is_pointer_value() {
                    val.into_pointer_value()
                } else {
                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_store(store, val);

                    store
                };

                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                dest,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "dest",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "")
                            .into(),
                    ],
                    "",
                );
            }
            _ => unimplemented!(),
        }
    }

    /// Return the encoded length of the given type
    pub fn encoded_length(&self, ty: &resolver::Type, contract: &resolver::Contract) -> u64 {
        match ty {
            resolver::Type::Primitive(_) => 32,
            resolver::Type::Enum(_) => 32,
            resolver::Type::Struct(n) => contract.structs[*n]
                .fields
                .iter()
                .map(|f| self.encoded_length(&f.ty, contract))
                .sum(),
            resolver::Type::FixedArray(ty, dims) => {
                self.encoded_length(ty, contract)
                    * dims.iter().map(|d| d.to_u64().unwrap()).product::<u64>()
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::Ref(r) => self.encoded_length(r, contract),
            resolver::Type::StorageRef(r) => self.encoded_length(r, contract),
        }
    }

    /// recursively encode a single ty
    fn decode_ty<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        ty: &resolver::Type,
        to: Option<PointerValue<'b>>,
        data: &mut PointerValue<'b>,
    ) -> BasicValueEnum<'b> {
        let pty = match &ty {
            resolver::Type::Primitive(e) => e,
            resolver::Type::Enum(n) => &contract.ns.enums[*n].ty,
            resolver::Type::FixedArray(_, dim) => {
                let to = to.unwrap_or_else(|| {
                    contract
                        .builder
                        .build_alloca(ty.llvm_type(contract.ns, contract.context), "")
                });

                contract.emit_static_loop(
                    function,
                    0,
                    dim[0].to_u64().unwrap(),
                    data,
                    |index: IntValue<'b>, data: &mut PointerValue<'b>| {
                        let elem = unsafe {
                            contract.builder.build_gep(
                                to,
                                &[contract.context.i32_type().const_zero(), index],
                                "index_access",
                            )
                        };

                        self.decode_ty(contract, function, &ty.deref(), Some(elem), data);
                    },
                );

                return to.into();
            }
            resolver::Type::Struct(n) => {
                let to = to.unwrap_or_else(|| {
                    contract
                        .builder
                        .build_alloca(ty.llvm_type(contract.ns, contract.context), "")
                });

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

                    self.decode_ty(contract, function, &field.ty, Some(elem), data);
                }

                return to.into();
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(ty) => {
                return self.decode_ty(contract, function, ty, to, data);
            }
            resolver::Type::Ref(ty) => {
                return self.decode_ty(contract, function, ty, to, data);
            }
        };

        let val = match pty {
            ast::PrimitiveType::Bool => {
                // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                // which is unneeded (hopefully)
                // cast to 64 bit pointer
                let bool_ptr = contract.builder.build_pointer_cast(
                    *data,
                    contract.context.i64_type().ptr_type(AddressSpace::Generic),
                    "",
                );

                let bool_ptr = unsafe {
                    contract.builder.build_gep(
                        bool_ptr,
                        &[contract.context.i32_type().const_int(3, false)],
                        "bool_ptr",
                    )
                };

                let val = contract.builder.build_int_compare(
                    IntPredicate::NE,
                    contract
                        .builder
                        .build_load(bool_ptr, "abi_bool")
                        .into_int_value(),
                    contract.context.i64_type().const_zero(),
                    "bool",
                );
                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                val.into()
            }
            ast::PrimitiveType::Uint(8) | ast::PrimitiveType::Int(8) => {
                let int8_ptr = contract.builder.build_pointer_cast(
                    *data,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                );

                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        int8_ptr,
                        &[contract.context.i32_type().const_int(31, false)],
                        "bool_ptr",
                    )
                };

                let val = contract.builder.build_load(int8_ptr, "abi_int8");

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }

                val
            }
            ast::PrimitiveType::Address => {
                let int_type = contract.context.custom_width_int_type(160);
                let type_size = int_type.size_of();

                let store =
                    to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "address"));

                contract.builder.build_call(
                    contract.module.get_function("__be32toleN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                *data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "size")
                            .into(),
                    ],
                    "",
                );

                store.into()
            }
            ast::PrimitiveType::Uint(n) | ast::PrimitiveType::Int(n) => {
                let int_type = contract.context.custom_width_int_type(*n as u32);
                let type_size = int_type.size_of();

                let store = to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "stack"));

                contract.builder.build_call(
                    contract.module.get_function("__be32toleN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                *data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "size")
                            .into(),
                    ],
                    "",
                );

                if *n <= 64 && to.is_none() {
                    contract
                        .builder
                        .build_load(store, &format!("abi_int{}", *n))
                } else {
                    store.into()
                }
            }
            ast::PrimitiveType::Bytes(1) => {
                let val = contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        *data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "bytes1",
                );

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                val
            }
            ast::PrimitiveType::Bytes(b) => {
                let int_type = contract.context.custom_width_int_type(*b as u32 * 8);
                let type_size = int_type.size_of();

                let store = to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "stack"));

                contract.builder.build_call(
                    contract.module.get_function("__beNtoleN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                *data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "size")
                            .into(),
                    ],
                    "",
                );

                if *b <= 8 && to.is_none() {
                    contract.builder.build_load(store, &format!("bytes{}", *b))
                } else {
                    store.into()
                }
            }
            _ => panic!(),
        };

        *data = unsafe {
            contract.builder.build_gep(
                *data,
                &[contract.context.i32_type().const_int(8, false)],
                "data_next",
            )
        };

        val
    }

    /// abi decode the encoded data into the BasicValueEnums
    pub fn decode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        let expected_length = spec
            .params
            .iter()
            .map(|arg| self.encoded_length(&arg.ty, contract.ns))
            .sum();
        let mut data = data;
        let decode_block = contract.context.append_basic_block(function, "abi_decode");
        let wrong_length_block = contract
            .context
            .append_basic_block(function, "wrong_abi_length");

        let is_ok = contract.builder.build_int_compare(
            IntPredicate::EQ,
            length,
            contract
                .context
                .i32_type()
                .const_int(expected_length, false),
            "correct_length",
        );

        contract
            .builder
            .build_conditional_branch(is_ok, &decode_block, &wrong_length_block);

        // FIXME: generate a call to revert/abort with some human readable error or error code
        contract.builder.position_at_end(&wrong_length_block);
        contract.builder.build_unreachable();

        contract.builder.position_at_end(&decode_block);

        for arg in &spec.params {
            args.push(self.decode_ty(contract, function, &arg.ty, None, &mut data));
        }
    }
}
