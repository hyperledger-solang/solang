use num_traits::ToPrimitive;
use resolver;

use inkwell::types::BasicType;
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
        load: bool,
        function: FunctionValue,
        ty: &resolver::Type,
        arg: BasicValueEnum<'a>,
        fixed: &mut PointerValue<'a>,
        offset: &mut IntValue<'a>,
        dynamic: &mut PointerValue<'a>,
    ) {
        match &ty {
            resolver::Type::Bool
            | resolver::Type::Address
            | resolver::Type::Contract(_)
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Bytes(_) => {
                self.encode_primitive(contract, load, ty, *fixed, arg);

                *fixed = unsafe {
                    contract.builder.build_gep(
                        *fixed,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            resolver::Type::Enum(n) => {
                self.encode_primitive(contract, load, &contract.ns.enums[*n].ty, *fixed, arg);
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
                        fixed,
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
                                offset,
                                dynamic,
                            );
                        },
                    );
                } else {
                    // write the current offset to fixed
                    self.encode_primitive(
                        contract,
                        false,
                        &resolver::Type::Uint(32),
                        *fixed,
                        (*offset).into(),
                    );

                    *fixed = unsafe {
                        contract.builder.build_gep(
                            *fixed,
                            &[contract.context.i32_type().const_int(32, false)],
                            "",
                        )
                    };

                    // Now, write the length to dynamic
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

                    // write the current offset to fixed
                    self.encode_primitive(
                        contract,
                        false,
                        &resolver::Type::Uint(32),
                        *dynamic,
                        len.into(),
                    );

                    *dynamic = unsafe {
                        contract.builder.build_gep(
                            *dynamic,
                            &[contract.context.i32_type().const_int(32, false)],
                            "",
                        )
                    };

                    *offset = contract.builder.build_int_add(
                        *offset,
                        contract.context.i32_type().const_int(32, false),
                        "",
                    );

                    // details about our array elements
                    let elem_ty = ty.array_deref();
                    let llvm_elem_ty = contract.llvm_var(&elem_ty);
                    let elem_size = llvm_elem_ty
                        .into_pointer_type()
                        .get_element_type()
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

                    let mut fixed = *dynamic;

                    let fixed_elems_length = contract.builder.build_int_add(
                        len,
                        contract
                            .context
                            .i32_type()
                            .const_int(self.encoded_fixed_length(&elem_ty, contract.ns), false),
                        "",
                    );

                    *offset = contract
                        .builder
                        .build_int_add(*offset, fixed_elems_length, "");

                    *dynamic = unsafe {
                        contract
                            .builder
                            .build_gep(*dynamic, &[fixed_elems_length], "")
                    };

                    contract.emit_static_loop_with_pointer(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        &mut fixed,
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
                                offset,
                                dynamic,
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

                    self.encode_ty(
                        contract,
                        true,
                        function,
                        &field.ty,
                        elem.into(),
                        fixed,
                        offset,
                        dynamic,
                    );
                }
            }
            resolver::Type::Undef => unreachable!(),
            resolver::Type::StorageRef(_) => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(ty) => {
                self.encode_ty(contract, load, function, ty, arg, fixed, offset, dynamic);
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                // write the current offset to fixed
                self.encode_primitive(
                    contract,
                    false,
                    &resolver::Type::Uint(32),
                    *fixed,
                    (*offset).into(),
                );

                *fixed = unsafe {
                    contract.builder.build_gep(
                        *fixed,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // Now, write the length to dynamic
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

                // write the current offset to fixed
                self.encode_primitive(
                    contract,
                    false,
                    &resolver::Type::Uint(32),
                    *dynamic,
                    len.into(),
                );

                *dynamic = unsafe {
                    contract.builder.build_gep(
                        *dynamic,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                *offset = contract.builder.build_int_add(
                    *offset,
                    contract.context.i32_type().const_int(32, false),
                    "",
                );

                // now copy the string data
                let string_start = unsafe {
                    contract.builder.build_gep(
                        arg.into_pointer_value(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(2, false),
                        ],
                        "string_start",
                    )
                };

                contract.builder.build_call(
                    contract.module.get_function("__memcpy").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                *dynamic,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "encoded_string",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                string_start,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "string_start",
                            )
                            .into(),
                        len.into(),
                    ],
                    "",
                );

                // round up the length to the next 32 bytes block
                let len = contract.builder.build_and(
                    contract.builder.build_int_add(
                        len,
                        contract.context.i32_type().const_int(31, false),
                        "",
                    ),
                    contract.context.i32_type().const_int(!31, false),
                    "",
                );

                *dynamic = unsafe { contract.builder.build_gep(*dynamic, &[len], "") };

                *offset = contract.builder.build_int_add(*offset, len, "");
            }
        };
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract,
        load: bool,
        ty: &resolver::Type,
        dest: PointerValue,
        arg: BasicValueEnum,
    ) {
        match ty {
            resolver::Type::Bool => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let value = contract.builder.build_select(
                    arg.into_int_value(),
                    contract.context.i8_type().const_int(1, false),
                    contract.context.i8_type().const_zero(),
                    "bool_val",
                );

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
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
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                if let resolver::Type::Int(_) = ty {
                    let negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        arg.into_int_value(),
                        contract.context.i8_type().const_zero(),
                        "neg",
                    );

                    let signval = contract
                        .builder
                        .build_select(
                            negative,
                            contract.context.i64_type().const_int(std::u64::MAX, true),
                            contract.context.i64_type().const_zero(),
                            "val",
                        )
                        .into_int_value();

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

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                contract.builder.build_store(dest, arg);
            }
            resolver::Type::Contract(_)
            | resolver::Type::Address
            | resolver::Type::Uint(_)
            | resolver::Type::Int(_)
                if load =>
            {
                let n = match ty {
                    resolver::Type::Contract(_) | resolver::Type::Address => {
                        contract.ns.address_length as u16 * 8
                    }
                    resolver::Type::Uint(b) => *b,
                    resolver::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                let arg8 = contract.builder.build_pointer_cast(
                    arg.into_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "arg8",
                );

                // first clear/set the upper bits
                if n < 256 {
                    if let resolver::Type::Int(_) = ty {
                        let signdest = unsafe {
                            contract.builder.build_gep(
                                arg8,
                                &[contract
                                    .context
                                    .i32_type()
                                    .const_int((n as u64 / 8) - 1, false)],
                                "signbyte",
                            )
                        };

                        let negative = contract.builder.build_int_compare(
                            IntPredicate::SLT,
                            contract
                                .builder
                                .build_load(signdest, "signbyte")
                                .into_int_value(),
                            contract.context.i8_type().const_zero(),
                            "neg",
                        );

                        let signval = contract
                            .builder
                            .build_select(
                                negative,
                                contract.context.i64_type().const_int(std::u64::MAX, true),
                                contract.context.i64_type().const_zero(),
                                "val",
                            )
                            .into_int_value();

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
                }

                contract.builder.build_call(
                    contract.module.get_function("__leNtobe32").unwrap(),
                    &[
                        arg8.into(),
                        dest8.into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(n as u64 / 8, false)
                            .into(),
                    ],
                    "",
                );
            }
            resolver::Type::Contract(_)
            | resolver::Type::Address
            | resolver::Type::Uint(_)
            | resolver::Type::Int(_)
                if !load =>
            {
                let n = match ty {
                    resolver::Type::Contract(_) | resolver::Type::Address => {
                        contract.ns.address_length as u16 * 8
                    }
                    resolver::Type::Uint(b) => *b,
                    resolver::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                // first clear/set the upper bits
                if n < 256 {
                    if let resolver::Type::Int(_) = ty {
                        let negative = contract.builder.build_int_compare(
                            IntPredicate::SLT,
                            arg.into_int_value(),
                            arg.get_type().into_int_type().const_zero(),
                            "neg",
                        );

                        let signval = contract
                            .builder
                            .build_select(
                                negative,
                                contract.context.i64_type().const_int(std::u64::MAX, true),
                                contract.context.i64_type().const_zero(),
                                "val",
                            )
                            .into_int_value();

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
                }

                let temp = contract
                    .builder
                    .build_alloca(arg.into_int_value().get_type(), &format!("uint{}", n));

                contract.builder.build_store(temp, arg.into_int_value());

                contract.builder.build_call(
                    contract.module.get_function("__leNtobe32").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                temp,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        dest8.into(),
                        contract
                            .context
                            .i32_type()
                            .const_int(n as u64 / 8, false)
                            .into(),
                    ],
                    "",
                );
            }
            resolver::Type::Bytes(1) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_store(dest8, arg);
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

                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                val,
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
                            .context
                            .i32_type()
                            .const_int(*n as u64, false)
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

    /// decode a single primitive which is always encoded in 32 bytes
    fn decode_primitive<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        ty: &resolver::Type,
        to: Option<PointerValue<'b>>,
        offset: &mut IntValue<'b>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
    ) -> BasicValueEnum<'b> {
        // TODO: investigate whether we can use build_int_nuw_add() and avoid 64 bit conversions
        let new_offset = contract.builder.build_int_add(
            *offset,
            contract.context.i64_type().const_int(32, false),
            "next_offset",
        );

        self.check_overrun(contract, function, new_offset, length);

        let data = unsafe { contract.builder.build_gep(data, &[*offset], "") };

        *offset = new_offset;

        let ty = if let resolver::Type::Enum(n) = ty {
            &contract.ns.enums[*n].ty
        } else {
            ty
        };

        match &ty {
            resolver::Type::Bool => {
                // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                // which is unneeded (hopefully)
                // cast to 64 bit pointer
                let bool_ptr = contract.builder.build_pointer_cast(
                    data,
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
                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        data,
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
                let int_type = contract
                    .context
                    .custom_width_int_type(contract.ns.address_length as u32 * 8);
                let type_size = int_type.size_of();

                let store =
                    to.unwrap_or_else(|| contract.builder.build_alloca(int_type, "address"));

                contract.builder.build_call(
                    contract.module.get_function("__be32toleN").unwrap(),
                    &[
                        data.into(),
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
                        data.into(),
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
                let val = contract.builder.build_load(data, "bytes1");

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
                        data.into(),
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
            _ => unreachable!(),
        }
    }

    /// recursively decode a single ty
    fn decode_ty<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        ty: &resolver::Type,
        to: Option<PointerValue<'b>>,
        offset: &mut IntValue<'b>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
    ) -> BasicValueEnum<'b> {
        match &ty {
            resolver::Type::Array(_, dim) => {
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

                if let Some(d) = &dim[0] {
                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        offset,
                        |index: IntValue<'b>, offset: &mut IntValue<'b>| {
                            let elem = unsafe {
                                contract.builder.build_gep(
                                    dest,
                                    &[contract.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            self.decode_ty(
                                contract,
                                function,
                                &ty,
                                Some(elem),
                                offset,
                                data,
                                length,
                            );
                        },
                    );
                } else {
                    let mut dataoffset = contract.builder.build_int_z_extend(
                        self.decode_primitive(
                            contract,
                            function,
                            &resolver::Type::Uint(32),
                            None,
                            offset,
                            data,
                            length,
                        )
                        .into_int_value(),
                        contract.context.i64_type(),
                        "data_offset",
                    );
                    let array_len = self
                        .decode_primitive(
                            contract,
                            function,
                            &resolver::Type::Uint(32),
                            None,
                            &mut dataoffset,
                            data,
                            length,
                        )
                        .into_int_value();

                    let elem_ty = contract.llvm_var(&ty.deref());
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
                            &[
                                contract
                                    .builder
                                    .build_int_truncate(
                                        array_len,
                                        contract.context.i32_type(),
                                        "array_len",
                                    )
                                    .into(),
                                elem_size.into(),
                                init.into(),
                            ],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        array_len,
                        &mut dataoffset,
                        |elem_no: IntValue<'b>, offset: &mut IntValue<'b>| {
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

                            self.decode_ty(
                                contract,
                                function,
                                &ty,
                                Some(elem),
                                offset,
                                data,
                                length,
                            );
                        },
                    );
                }

                if let Some(to) = to {
                    contract.builder.build_store(to, dest);
                }

                dest.into()
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

                let struct_pointer = contract.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    &contract.ns.structs[*n].name,
                );

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            struct_pointer,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.decode_ty(
                        contract,
                        function,
                        &field.ty,
                        Some(elem),
                        offset,
                        data,
                        length,
                    );
                }

                if let Some(to) = to {
                    contract.builder.build_store(to, struct_pointer);
                }

                struct_pointer.into()
            }
            resolver::Type::Ref(ty) => {
                self.decode_ty(contract, function, ty, to, offset, data, length)
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                // we read the offset and the length as 32 bits. Since we are in 32 bits wasm,
                // we cannot deal with more than 4GB of abi encoded data.
                let mut dataoffset = contract.builder.build_int_z_extend(
                    self.decode_primitive(
                        contract,
                        function,
                        &resolver::Type::Uint(32),
                        None,
                        offset,
                        data,
                        length,
                    )
                    .into_int_value(),
                    contract.context.i64_type(),
                    "data_offset",
                );

                let string_len = contract.builder.build_int_z_extend(
                    self.decode_primitive(
                        contract,
                        function,
                        &resolver::Type::Uint(32),
                        None,
                        &mut dataoffset,
                        data,
                        length,
                    )
                    .into_int_value(),
                    contract.context.i64_type(),
                    "string_len",
                );

                let string_end =
                    contract
                        .builder
                        .build_int_add(dataoffset, string_len, "stringend");

                self.check_overrun(contract, function, string_end, length);

                let string_start = unsafe {
                    contract
                        .builder
                        .build_gep(data, &[dataoffset], "string_start")
                };

                let v = contract
                    .builder
                    .build_call(
                        contract.module.get_function("vector_new").unwrap(),
                        &[
                            contract
                                .builder
                                .build_int_truncate(
                                    string_len,
                                    contract.context.i32_type(),
                                    "string_len",
                                )
                                .into(),
                            contract.context.i32_type().const_int(1, false).into(),
                            string_start.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let v = contract.builder.build_pointer_cast(
                    v.into_pointer_value(),
                    contract
                        .module
                        .get_type("struct.vector")
                        .unwrap()
                        .ptr_type(AddressSpace::Generic),
                    "string",
                );

                if let Some(to) = to {
                    contract.builder.build_store(to, v);
                }

                v.into()
            }
            _ => self.decode_primitive(contract, function, ty, to, offset, data, length),
        }
    }

    /// Check that data has not overrun end
    fn check_overrun(
        &self,
        contract: &Contract,
        function: FunctionValue,
        offset: IntValue,
        end: IntValue,
    ) {
        let in_bounds = contract
            .builder
            .build_int_compare(IntPredicate::ULE, offset, end, "");

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

    /// abi decode the encoded data into the BasicValueEnums
    pub fn decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        datalength: IntValue<'b>,
        spec: &[resolver::Parameter],
    ) {
        let data = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "data",
        );

        let mut offset = contract.context.i64_type().const_zero();

        let data_length = contract.builder.build_int_z_extend(
            datalength,
            contract.context.i64_type(),
            "data_length",
        );

        for arg in spec {
            args.push(self.decode_ty(
                contract,
                function,
                &arg.ty,
                None,
                &mut offset,
                data,
                data_length,
            ));
        }
    }
}
