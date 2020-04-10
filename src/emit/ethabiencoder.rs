use num_traits::ToPrimitive;
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
        contract: &Contract<'a>,
        function: FunctionValue,
        ty: &resolver::Type,
        arg: BasicValueEnum,
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
            | resolver::Type::Contract(_)
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                self.encode_primitive(contract, ty, *data, arg);

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
                    // FIXME
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
            resolver::Type::String | resolver::Type::DynamicBytes => unimplemented!(),
        };
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        ty: &resolver::Type,
        dest: PointerValue,
        val: BasicValueEnum,
    ) {
        match ty {
            resolver::Type::Bool => {
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
            resolver::Type::Int(8) | resolver::Type::Uint(8) => {
                let signval = if let resolver::Type::Int(8) = ty {
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
            resolver::Type::Address | resolver::Type::Uint(_) | resolver::Type::Int(_) => {
                let n = match ty {
                    resolver::Type::Address => 160,
                    resolver::Type::Uint(b) => *b,
                    resolver::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                // first clear/set the upper bits
                if n < 256 {
                    let signval = if let resolver::Type::Int(8) = ty {
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
            resolver::Type::Bytes(1) => {
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
            resolver::Type::Bytes(b) => {
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

    /// Return the amount of fixed and dynamic storage required to store a type
    pub fn encoded_dynamic_length<'a>(
        &self,
        arg: BasicValueEnum<'a>,
        load: bool,
        ty: &resolver::Type,
        function: FunctionValue,
        contract: &Contract<'a>,
    ) -> IntValue<'a> {
        match ty {
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

                    let len = self.encoded_dynamic_length(
                        elem.into(),
                        true,
                        &field.ty,
                        function,
                        contract,
                    );

                    sum = contract.builder.build_int_add(sum, len, "");
                }

                sum
            }
            resolver::Type::Array(_, dims) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut sum = contract.context.i32_type().const_zero();
                let elem_ty = ty.array_deref();

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

                        let array_len = contract
                            .builder
                            .build_load(len, "array.len")
                            .into_int_value();

                        // A dynamic array will store its own length
                        sum = contract.builder.build_int_add(
                            sum,
                            contract.context.i32_type().const_int(32, false),
                            "",
                        );

                        // plus elements in dynamic storage
                        sum = contract.builder.build_int_add(
                            sum,
                            contract.builder.build_int_mul(
                                array_len,
                                contract.context.i32_type().const_int(
                                    self.encoded_fixed_length(&elem_ty, contract.ns),
                                    false,
                                ),
                                "",
                            ),
                            "",
                        );

                        array_len
                    }
                    Some(d) => contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                let llvm_elem_ty = contract.llvm_var(&elem_ty);

                if elem_ty.is_dynamic(contract.ns) {
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

                            let elem = unsafe {
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

                            let elem = contract.builder.build_pointer_cast(
                                elem,
                                llvm_elem_ty.into_pointer_type(),
                                "elem",
                            );

                            *sum = contract.builder.build_int_add(
                                self.encoded_dynamic_length(
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
                }

                sum
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

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

                // The dynamic part is the length (=32 bytes) and the string
                // data itself. Length 0 occupies no space, length 1-32 occupies
                // 32 bytes, etc
                contract.builder.build_and(
                    contract.builder.build_int_add(
                        contract
                            .builder
                            .build_load(len, "string.len")
                            .into_int_value(),
                        contract.context.i32_type().const_int(32 + 31, false),
                        "",
                    ),
                    contract.context.i32_type().const_int(!31, false),
                    "",
                )
            }
            _ => contract.context.i32_type().const_zero(),
        }
    }

    /// Return the encoded length of the given type, fixed part only
    pub fn encoded_fixed_length(&self, ty: &resolver::Type, ns: &resolver::Namespace) -> u64 {
        match ty {
            resolver::Type::Bool
            | resolver::Type::Contract(_)
            | resolver::Type::Address
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => 32,
            // String and Dynamic bytes use 32 bytes for the offset into dynamic encoded
            resolver::Type::String | resolver::Type::DynamicBytes => 32,
            resolver::Type::Enum(_) => 32,
            resolver::Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| self.encoded_fixed_length(&f.ty, ns))
                .sum(),
            resolver::Type::Array(ty, dims) => {
                let mut product = 1;

                for dim in dims {
                    match dim {
                        Some(d) => product *= d.to_u64().unwrap(),
                        None => {
                            return product * 32;
                        }
                    }
                }

                product * self.encoded_fixed_length(&ty, ns)
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(r) => self.encoded_fixed_length(r, ns),
            resolver::Type::StorageRef(r) => self.encoded_fixed_length(r, ns),
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
        let val = match &ty {
            resolver::Type::Bool => {
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
            resolver::Type::Uint(8) | resolver::Type::Int(8) => {
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
            resolver::Type::Address | resolver::Type::Contract(_) => {
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
            resolver::Type::Uint(n) | resolver::Type::Int(n) => {
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
                    contract.builder.build_load(store, &format!("abi_int{}", n))
                } else {
                    store.into()
                }
            }
            resolver::Type::Bytes(1) => {
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
            resolver::Type::Bytes(b) => {
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
            resolver::Type::Enum(n) => {
                return self.decode_ty(contract, function, &contract.ns.enums[*n].ty, to, data);
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
                } else {
                    // FIXME
                }

                return to.into();
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

                return to.into();
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::StorageRef(ty) => {
                return self.decode_ty(contract, function, ty, to, data);
            }
            resolver::Type::Ref(ty) => {
                return self.decode_ty(contract, function, ty, to, data);
            }
            resolver::Type::String | resolver::Type::DynamicBytes => unimplemented!(),
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
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        let expected_length = spec
            .params
            .iter()
            .map(|arg| self.encoded_fixed_length(&arg.ty, contract.ns))
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
            .build_conditional_branch(is_ok, decode_block, wrong_length_block);

        // FIXME: generate a call to revert/abort with some human readable error or error code
        contract.builder.position_at_end(wrong_length_block);
        contract.builder.build_unreachable();

        contract.builder.position_at_end(decode_block);

        for arg in &spec.params {
            args.push(self.decode_ty(contract, function, &arg.ty, None, &mut data));
        }
    }
}
