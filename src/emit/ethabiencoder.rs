use crate::sema::ast;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use num_traits::ToPrimitive;

use super::loop_builder::LoopBuilder;
use super::{Contract, ReturnCode};

/// Generate an in-place abi encoder. This is done in several stages:
/// 1) EncoderBuilder::new() generates the code which calculates the required encoded length at runtime
/// 2) EncoderBuilder::encoded_length() returns the required length
/// 3) EncoderBuilder::finish() generates the code which encodes the data to the pointer provided. The
///    called should ensure there is enough space.
pub struct EncoderBuilder<'a, 'b> {
    length: IntValue<'a>,
    offset: IntValue<'a>,
    load_args: bool,
    packed: &'b [BasicValueEnum<'a>],
    args: &'b [BasicValueEnum<'a>],
    tys: &'b [ast::Type],
    bswap: bool,
}

impl<'a, 'b> EncoderBuilder<'a, 'b> {
    /// Create a new encoder. This will generate the code which calculates the length of encoded data
    pub fn new(
        contract: &Contract<'a>,
        function: FunctionValue,
        load_args: bool,
        packed: &'b [BasicValueEnum<'a>],
        args: &'b [BasicValueEnum<'a>],
        tys: &'b [ast::Type],
        bswap: bool,
    ) -> Self {
        debug_assert_eq!(packed.len() + args.len(), tys.len());

        let args_tys = &tys[packed.len()..];

        let offset = contract.context.i32_type().const_int(
            args_tys
                .iter()
                .map(|ty| EncoderBuilder::encoded_fixed_length(ty, contract.ns))
                .sum(),
            false,
        );

        let mut length = offset;

        // calculate the packed length
        for (i, arg) in packed.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                EncoderBuilder::encoded_packed_length(*arg, load_args, &tys[i], function, contract),
                "",
            );
        }

        // now add the dynamic lengths
        for (i, arg) in args.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                EncoderBuilder::encoded_dynamic_length(
                    *arg,
                    load_args,
                    &args_tys[i],
                    function,
                    contract,
                ),
                "",
            );
        }

        EncoderBuilder {
            length,
            offset,
            load_args,
            packed,
            args,
            tys,
            bswap,
        }
    }

    /// Return the total length
    pub fn encoded_length(&self) -> IntValue<'a> {
        self.length
    }

    /// Return the amount of fixed and dynamic storage required to store a type
    fn encoded_packed_length<'c>(
        arg: BasicValueEnum<'c>,
        load: bool,
        ty: &ast::Type,
        function: FunctionValue,
        contract: &Contract<'c>,
    ) -> IntValue<'c> {
        match ty {
            ast::Type::Struct(n) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let normal_struct = contract
                    .context
                    .append_basic_block(function, "normal_struct");
                let null_struct = contract.context.append_basic_block(function, "null_struct");
                let done_struct = contract.context.append_basic_block(function, "done_struct");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                contract.builder.position_at_end(normal_struct);

                let mut normal_sum = contract.context.i32_type().const_zero();

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

                    let len = EncoderBuilder::encoded_packed_length(
                        elem.into(),
                        true,
                        &field.ty,
                        function,
                        contract,
                    );

                    normal_sum = contract.builder.build_int_add(normal_sum, len, "");
                }

                contract.builder.build_unconditional_branch(done_struct);

                let normal_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_struct);

                let mut null_sum = contract.context.i32_type().const_zero();

                for field in &contract.ns.structs[*n].fields {
                    null_sum = contract.builder.build_int_add(
                        null_sum,
                        EncoderBuilder::encoded_packed_length(
                            contract.default_value(&field.ty),
                            false,
                            &field.ty,
                            function,
                            contract,
                        ),
                        "",
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let null_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_struct);

                let sum = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "sum");

                sum.add_incoming(&[(&normal_sum, normal_struct), (&null_sum, null_struct)]);

                sum.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) if elem_ty.is_dynamic(contract.ns) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let sum = contract.context.i32_type().const_zero();

                let len = match dims.last().unwrap() {
                    None => contract.vector_len(arg),
                    Some(d) => contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                let normal_array = contract
                    .context
                    .append_basic_block(function, "normal_array");
                let null_array = contract.context.append_basic_block(function, "null_array");
                let done_array = contract.context.append_basic_block(function, "done_array");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                contract.builder.position_at_end(normal_array);

                let mut normal_length = sum;

                contract.builder.position_at_end(normal_array);

                // the element of the array are dynamic; we need to iterate over the array to find the encoded length
                contract.emit_loop_cond_first_with_int(
                    function,
                    contract.context.i32_type().const_zero(),
                    len,
                    &mut normal_length,
                    |index, sum| {
                        let elem = contract.array_subscript(ty, arg.into_pointer_value(), index);

                        *sum = contract.builder.build_int_add(
                            EncoderBuilder::encoded_packed_length(
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

                contract.builder.build_unconditional_branch(done_array);

                let normal_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_array);

                let elem = contract.default_value(&elem_ty.deref_any());

                let null_length = contract.builder.build_int_add(
                    contract.builder.build_int_mul(
                        EncoderBuilder::encoded_packed_length(
                            elem, false, elem_ty, function, contract,
                        ),
                        len,
                        "",
                    ),
                    sum,
                    "",
                );

                contract.builder.build_unconditional_branch(done_array);

                let null_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_array);

                let encoded_length = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "encoded_length");

                encoded_length
                    .add_incoming(&[(&normal_length, normal_array), (&null_length, null_array)]);

                encoded_length.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = match dims.last().unwrap() {
                    None => contract.vector_len(arg),
                    Some(d) => contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                // plus fixed size elements
                contract.builder.build_int_mul(
                    len,
                    EncoderBuilder::encoded_packed_length(arg, false, &elem_ty, function, contract),
                    "",
                )
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                contract.vector_len(arg)
            }
            ast::Type::Uint(n) | ast::Type::Int(n) => contract
                .context
                .i32_type()
                .const_int((*n as u64) / 8, false),
            ast::Type::Bytes(n) => contract.context.i32_type().const_int(*n as u64, false),
            ast::Type::Enum(_) | ast::Type::Bool => contract.context.i32_type().const_int(1, false),
            ast::Type::Contract(_) | ast::Type::Address(_) => contract
                .context
                .i32_type()
                .const_int(contract.ns.address_length as u64, false),
            ast::Type::Ref(ty) => {
                EncoderBuilder::encoded_packed_length(arg, false, ty, function, contract)
            }
            _ => unreachable!(),
        }
    }

    /// Return the amount of fixed and dynamic storage required to store a type
    fn encoded_dynamic_length<'c>(
        arg: BasicValueEnum<'c>,
        load: bool,
        ty: &ast::Type,
        function: FunctionValue,
        contract: &Contract<'c>,
    ) -> IntValue<'c> {
        match ty {
            ast::Type::Struct(n) if ty.is_dynamic(contract.ns) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let normal_struct = contract
                    .context
                    .append_basic_block(function, "normal_struct");
                let null_struct = contract.context.append_basic_block(function, "null_struct");
                let done_struct = contract.context.append_basic_block(function, "done_struct");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                contract.builder.position_at_end(normal_struct);

                let mut normal_sum = contract.context.i32_type().const_zero();

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    // a struct with dynamic fields gets stored in the dynamic part
                    normal_sum = contract.builder.build_int_add(
                        normal_sum,
                        contract.context.i32_type().const_int(
                            EncoderBuilder::encoded_fixed_length(&field.ty, contract.ns),
                            false,
                        ),
                        "",
                    );

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

                    let len = EncoderBuilder::encoded_dynamic_length(
                        elem.into(),
                        true,
                        &field.ty,
                        function,
                        contract,
                    );

                    normal_sum = contract.builder.build_int_add(normal_sum, len, "");
                }

                contract.builder.build_unconditional_branch(done_struct);

                let normal_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_struct);

                let mut null_sum = contract.context.i32_type().const_zero();

                for field in &contract.ns.structs[*n].fields {
                    // a struct with dynamic fields gets stored in the dynamic part
                    null_sum = contract.builder.build_int_add(
                        null_sum,
                        contract.context.i32_type().const_int(
                            EncoderBuilder::encoded_fixed_length(&field.ty, contract.ns),
                            false,
                        ),
                        "",
                    );

                    null_sum = contract.builder.build_int_add(
                        null_sum,
                        EncoderBuilder::encoded_dynamic_length(
                            contract.default_value(&field.ty),
                            false,
                            &field.ty,
                            function,
                            contract,
                        ),
                        "",
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let null_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_struct);

                let sum = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "sum");

                sum.add_incoming(&[(&normal_sum, normal_struct), (&null_sum, null_struct)]);

                sum.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) if ty.is_dynamic(contract.ns) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut sum = contract.context.i32_type().const_zero();

                let len = match dims.last().unwrap() {
                    None => {
                        let array_len = contract.vector_len(arg);

                        // A dynamic array will store its own length
                        sum = contract.builder.build_int_add(
                            sum,
                            contract.context.i32_type().const_int(32, false),
                            "",
                        );

                        array_len
                    }
                    Some(d) => contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                // plus fixed size elements
                sum = contract.builder.build_int_add(
                    sum,
                    contract.builder.build_int_mul(
                        len,
                        contract.context.i32_type().const_int(
                            EncoderBuilder::encoded_fixed_length(&elem_ty, contract.ns),
                            false,
                        ),
                        "",
                    ),
                    "",
                );

                let normal_array = contract
                    .context
                    .append_basic_block(function, "normal_array");
                let null_array = contract.context.append_basic_block(function, "null_array");
                let done_array = contract.context.append_basic_block(function, "done_array");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                contract.builder.position_at_end(normal_array);

                let mut normal_length = sum;

                contract.builder.position_at_end(normal_array);

                // the element of the array are dynamic; we need to iterate over the array to find the encoded length
                if elem_ty.is_dynamic(contract.ns) {
                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i32_type().const_zero(),
                        len,
                        &mut normal_length,
                        |index, sum| {
                            let elem =
                                contract.array_subscript(ty, arg.into_pointer_value(), index);

                            *sum = contract.builder.build_int_add(
                                EncoderBuilder::encoded_dynamic_length(
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

                contract.builder.build_unconditional_branch(done_array);

                let normal_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_array);

                let elem = contract.default_value(&elem_ty.deref_any());

                let null_length = contract.builder.build_int_add(
                    contract.builder.build_int_mul(
                        EncoderBuilder::encoded_dynamic_length(
                            elem, false, elem_ty, function, contract,
                        ),
                        len,
                        "",
                    ),
                    sum,
                    "",
                );

                contract.builder.build_unconditional_branch(done_array);

                let null_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_array);

                let encoded_length = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "encoded_length");

                encoded_length
                    .add_incoming(&[(&normal_length, normal_array), (&null_length, null_array)]);

                encoded_length.as_basic_value().into_int_value()
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // The dynamic part is the length (=32 bytes) and the string
                // data itself. Length 0 occupies no space, length 1-32 occupies
                // 32 bytes, etc
                contract.builder.build_and(
                    contract.builder.build_int_add(
                        contract.vector_len(arg),
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
    fn encoded_fixed_length(ty: &ast::Type, ns: &ast::Namespace) -> u64 {
        match ty {
            ast::Type::Bool
            | ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_)
            | ast::Type::ExternalFunction { .. } => 32,
            // String and Dynamic bytes use 32 bytes for the offset into dynamic encoded
            ast::Type::String
            | ast::Type::DynamicBytes
            | ast::Type::Struct(_)
            | ast::Type::Array(_, _)
                if ty.is_dynamic(ns) =>
            {
                32
            }
            ast::Type::Enum(_) => 32,
            ast::Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| EncoderBuilder::encoded_fixed_length(&f.ty, ns))
                .sum(),
            ast::Type::Array(ty, dims) => {
                // The array must be fixed, dynamic arrays are handled above
                let product: u64 = dims
                    .iter()
                    .map(|d| d.as_ref().unwrap().to_u64().unwrap())
                    .product();

                product * EncoderBuilder::encoded_fixed_length(&ty, ns)
            }
            ast::Type::Ref(r) => EncoderBuilder::encoded_fixed_length(r, ns),
            ast::Type::StorageRef(r) => EncoderBuilder::encoded_fixed_length(r, ns),
            _ => unreachable!(),
        }
    }

    /// Make it so
    pub fn finish(
        self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        output: PointerValue<'a>,
    ) {
        let mut output = output;
        let mut ty_iter = self.tys.iter();

        for arg in self.packed.iter() {
            let ty = ty_iter.next().unwrap();

            self.encode_packed_ty(contract, self.load_args, function, ty, *arg, &mut output);
        }

        // We use a little trick here. The length might or might not include the selector.
        // The length will be a multiple of 32 plus the selector (4). So by dividing by 8,
        // we lose the selector.
        contract.builder.build_call(
            contract.module.get_function("__bzero8").unwrap(),
            &[
                output.into(),
                contract
                    .builder
                    .build_int_unsigned_div(
                        self.length,
                        contract.context.i32_type().const_int(8, false),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        let mut output = output;
        let mut offset = self.offset;
        let mut dynamic = unsafe { contract.builder.build_gep(output, &[self.offset], "") };

        for arg in self.args.iter() {
            let ty = ty_iter.next().unwrap();

            self.encode_ty(
                contract,
                self.load_args,
                function,
                ty,
                *arg,
                &mut output,
                &mut offset,
                &mut dynamic,
            );
        }
    }

    /// Recursively encode a value in arg. The load argument specifies if the arg is a pointer
    /// to the value, or the value itself. The fixed pointer points to the fixed, non-dynamic part
    /// of the encoded data. The offset is current offset for dynamic fields.
    fn encode_ty(
        &self,
        contract: &Contract<'a>,
        load: bool,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        arg: BasicValueEnum<'a>,
        fixed: &mut PointerValue<'a>,
        offset: &mut IntValue<'a>,
        dynamic: &mut PointerValue<'a>,
    ) {
        match &ty {
            ast::Type::Bool
            | ast::Type::Address(_)
            | ast::Type::Contract(_)
            | ast::Type::Int(_)
            | ast::Type::Uint(_)
            | ast::Type::Bytes(_) => {
                self.encode_primitive(contract, load, function, ty, *fixed, arg);

                *fixed = unsafe {
                    contract.builder.build_gep(
                        *fixed,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            ast::Type::Enum(n) => {
                self.encode_primitive(
                    contract,
                    load,
                    function,
                    &contract.ns.enums[*n].ty,
                    *fixed,
                    arg,
                );

                *fixed = unsafe {
                    contract.builder.build_gep(
                        *fixed,
                        &[contract.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            ast::Type::Array(elem_ty, dim) if ty.is_dynamic(contract.ns) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // if the array is of dynamic length, or has dynamic array elements, then it is written to
                // the dynamic section.

                // write the current offset to fixed
                self.encode_primitive(
                    contract,
                    false,
                    function,
                    &ast::Type::Uint(32),
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

                let array_length = if let Some(d) = &dim[0] {
                    // fixed length
                    contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false)
                } else {
                    // Now, write the length to dynamic
                    let len = contract.vector_len(arg);

                    // write the current offset to fixed
                    self.encode_primitive(
                        contract,
                        false,
                        function,
                        &ast::Type::Uint(32),
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

                    len
                };

                let array_data_offset = contract.builder.build_int_mul(
                    contract.context.i32_type().const_int(
                        EncoderBuilder::encoded_fixed_length(&elem_ty, contract.ns),
                        false,
                    ),
                    array_length,
                    "array_data_offset",
                );

                let normal_fixed = *dynamic;
                let null_fixed = *dynamic;

                *dynamic = unsafe {
                    contract
                        .builder
                        .build_gep(*dynamic, &[array_data_offset], "")
                };

                let normal_array = contract
                    .context
                    .append_basic_block(function, "normal_array");
                let null_array = contract.context.append_basic_block(function, "null_array");
                let done_array = contract.context.append_basic_block(function, "done_array");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                contract.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(contract, function);

                let mut normal_fixed = builder
                    .add_loop_phi(
                        contract,
                        "fixed",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        normal_fixed.into(),
                    )
                    .into_pointer_value();

                let mut normal_array_data_offset = builder
                    .add_loop_phi(
                        contract,
                        "offset",
                        contract.context.i32_type(),
                        array_data_offset.into(),
                    )
                    .into_int_value();

                let mut normal_dynamic = builder
                    .add_loop_phi(
                        contract,
                        "dynamic",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let index = builder.over(
                    contract,
                    contract.context.i32_type().const_zero(),
                    array_length,
                );

                // loop body
                let elem = contract.array_subscript(ty, arg.into_pointer_value(), index);

                self.encode_ty(
                    contract,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_fixed,
                    &mut normal_array_data_offset,
                    &mut normal_dynamic,
                );

                builder.set_loop_phi_value(contract, "fixed", normal_fixed.into());
                builder.set_loop_phi_value(contract, "offset", normal_array_data_offset.into());
                builder.set_loop_phi_value(contract, "dynamic", normal_dynamic.into());

                builder.finish(contract);

                let normal_dynamic = builder.get_loop_phi("dynamic");
                let normal_array_data_offset = builder.get_loop_phi("offset");

                contract.builder.build_unconditional_branch(done_array);

                let normal_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_array);

                let mut builder = LoopBuilder::new(contract, function);

                let mut null_fixed = builder
                    .add_loop_phi(
                        contract,
                        "fixed",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        null_fixed.into(),
                    )
                    .into_pointer_value();

                let mut null_array_data_offset = builder
                    .add_loop_phi(
                        contract,
                        "offset",
                        contract.context.i32_type(),
                        array_data_offset.into(),
                    )
                    .into_int_value();

                let mut null_dynamic = builder
                    .add_loop_phi(
                        contract,
                        "dynamic",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let _ = builder.over(
                    contract,
                    contract.context.i32_type().const_zero(),
                    array_length,
                );

                // loop body
                let elem = contract.default_value(&elem_ty.deref_any());

                self.encode_ty(
                    contract,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_fixed,
                    &mut null_array_data_offset,
                    &mut null_dynamic,
                );

                builder.set_loop_phi_value(contract, "fixed", null_fixed.into());
                builder.set_loop_phi_value(contract, "offset", null_array_data_offset.into());
                builder.set_loop_phi_value(contract, "dynamic", null_dynamic.into());

                builder.finish(contract);

                let null_dynamic = builder.get_loop_phi("dynamic");
                let null_array_data_offset = builder.get_loop_phi("offset");

                contract.builder.build_unconditional_branch(done_array);

                let null_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_array);

                let dynamic_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi
                    .add_incoming(&[(&normal_dynamic, normal_array), (&null_dynamic, null_array)]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let array_array_offset_phi = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "array_data_offset");

                array_array_offset_phi.add_incoming(&[
                    (&normal_array_data_offset, normal_array),
                    (&null_array_data_offset, null_array),
                ]);

                *offset = contract.builder.build_int_add(
                    array_array_offset_phi.as_basic_value().into_int_value(),
                    *offset,
                    "new_offset",
                );
            }
            ast::Type::Array(elem_ty, dim) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dim = dim[0].as_ref().unwrap().to_u64().unwrap();

                let normal_array = contract
                    .context
                    .append_basic_block(function, "normal_array");
                let null_array = contract.context.append_basic_block(function, "null_array");
                let done_array = contract.context.append_basic_block(function, "done_array");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                contract.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(contract, function);

                let mut normal_fixed = builder
                    .add_loop_phi(
                        contract,
                        "fixed",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*fixed).into(),
                    )
                    .into_pointer_value();

                let mut normal_offset = builder
                    .add_loop_phi(
                        contract,
                        "offset",
                        contract.context.i32_type(),
                        (*offset).into(),
                    )
                    .into_int_value();

                let mut normal_dynamic = builder
                    .add_loop_phi(
                        contract,
                        "dynamic",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let index = builder.over(
                    contract,
                    contract.context.i64_type().const_zero(),
                    contract.context.i64_type().const_int(dim, false),
                );

                // loop body
                let elem = unsafe {
                    contract.builder.build_gep(
                        arg.into_pointer_value(),
                        &[contract.context.i32_type().const_zero(), index],
                        "index_access",
                    )
                };

                self.encode_ty(
                    contract,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_fixed,
                    &mut normal_offset,
                    &mut normal_dynamic,
                );

                builder.set_loop_phi_value(contract, "fixed", normal_fixed.into());
                builder.set_loop_phi_value(contract, "offset", normal_offset.into());
                builder.set_loop_phi_value(contract, "dynamic", normal_dynamic.into());

                builder.finish(contract);

                let normal_fixed = builder.get_loop_phi("fixed");
                let normal_offset = builder.get_loop_phi("offset");
                let normal_dynamic = builder.get_loop_phi("dynamic");

                contract.builder.build_unconditional_branch(done_array);

                let normal_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_array);

                // Create a loop for generating an array of empty values
                // FIXME: all fixed-length types are encoded as zeros, and the memory has
                // already been zero'ed out, so this is pointless. Just step over it.
                let elem = contract.default_value(&elem_ty.deref_any());

                let mut builder = LoopBuilder::new(contract, function);

                let mut null_fixed = builder
                    .add_loop_phi(
                        contract,
                        "fixed",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*fixed).into(),
                    )
                    .into_pointer_value();

                let mut null_offset = builder
                    .add_loop_phi(
                        contract,
                        "offset",
                        contract.context.i32_type(),
                        (*offset).into(),
                    )
                    .into_int_value();

                let mut null_dynamic = builder
                    .add_loop_phi(
                        contract,
                        "dynamic",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                builder.over(
                    contract,
                    contract.context.i64_type().const_zero(),
                    contract.context.i64_type().const_int(dim, false),
                );

                // loop body
                self.encode_ty(
                    contract,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_fixed,
                    &mut null_offset,
                    &mut null_dynamic,
                );

                builder.set_loop_phi_value(contract, "fixed", null_fixed.into());
                builder.set_loop_phi_value(contract, "offset", null_offset.into());
                builder.set_loop_phi_value(contract, "dynamic", null_dynamic.into());

                builder.finish(contract);

                let null_fixed = builder.get_loop_phi("fixed");
                let null_offset = builder.get_loop_phi("offset");
                let null_dynamic = builder.get_loop_phi("dynamic");

                contract.builder.build_unconditional_branch(done_array);

                let null_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_array);

                let fixed_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "fixed",
                );

                fixed_phi.add_incoming(&[(&normal_fixed, normal_array), (&null_fixed, null_array)]);

                *fixed = fixed_phi.as_basic_value().into_pointer_value();

                let offset_phi = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_array), (&null_offset, null_array)]);

                *offset = offset_phi.as_basic_value().into_int_value();

                let dynamic_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi
                    .add_incoming(&[(&normal_dynamic, normal_array), (&null_dynamic, null_array)]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Struct(n) if ty.is_dynamic(contract.ns) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // write the current offset to fixed
                self.encode_primitive(
                    contract,
                    false,
                    function,
                    &ast::Type::Uint(32),
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

                let mut normal_fields_dynamic = *dynamic;
                let mut null_fields_dynamic = *dynamic;

                // add size of fixed fields to dynamic
                let fixed_field_length = contract.ns.structs[*n]
                    .fields
                    .iter()
                    .map(|f| EncoderBuilder::encoded_fixed_length(&f.ty, contract.ns))
                    .sum();

                *dynamic = unsafe {
                    contract.builder.build_gep(
                        *dynamic,
                        &[contract
                            .context
                            .i32_type()
                            .const_int(fixed_field_length, false)],
                        "",
                    )
                };

                let null_struct = contract.context.append_basic_block(function, "null_struct");
                let normal_struct = contract
                    .context
                    .append_basic_block(function, "normal_struct");
                let done_struct = contract.context.append_basic_block(function, "done_struct");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                let mut normal_dynamic = *dynamic;
                let mut null_dynamic = *dynamic;
                let normal_offset = *offset;
                let null_offset = *offset;

                contract.builder.position_at_end(normal_struct);

                let mut temp_offset = contract
                    .context
                    .i32_type()
                    .const_int(fixed_field_length, false);

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
                        &mut normal_fields_dynamic,
                        &mut temp_offset,
                        &mut normal_dynamic,
                    );
                }

                let normal_offset = contract
                    .builder
                    .build_int_add(normal_offset, temp_offset, "");

                contract.builder.build_unconditional_branch(done_struct);

                let normal_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_struct);

                let mut temp_offset = contract
                    .context
                    .i32_type()
                    .const_int(fixed_field_length, false);

                for field in &contract.ns.structs[*n].fields {
                    let elem = contract.default_value(&field.ty);

                    self.encode_ty(
                        contract,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_fields_dynamic,
                        &mut temp_offset,
                        &mut null_dynamic,
                    );
                }

                let null_offset = contract.builder.build_int_add(null_offset, temp_offset, "");

                contract.builder.build_unconditional_branch(done_struct);

                let null_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_struct);

                let dynamic_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi.add_incoming(&[
                    (&normal_dynamic, normal_struct),
                    (&null_dynamic, null_struct),
                ]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let offset_phi = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_struct), (&null_offset, null_struct)]);

                *offset = offset_phi.as_basic_value().into_int_value();
            }
            ast::Type::Struct(n) => {
                let arg = if load {
                    contract
                        .builder
                        .build_load(arg.into_pointer_value(), "")
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_struct = contract.context.append_basic_block(function, "null_struct");
                let normal_struct = contract
                    .context
                    .append_basic_block(function, "normal_struct");
                let done_struct = contract.context.append_basic_block(function, "done_struct");

                let is_null = contract.builder.build_is_null(arg, "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                contract.builder.position_at_end(normal_struct);

                let mut normal_fixed = *fixed;
                let mut normal_offset = *offset;
                let mut normal_dynamic = *dynamic;

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            arg,
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
                        &mut normal_fixed,
                        &mut normal_offset,
                        &mut normal_dynamic,
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let normal_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_struct);

                let mut null_fixed = *fixed;
                let mut null_offset = *offset;
                let mut null_dynamic = *dynamic;

                // FIXME: abi encoding fixed length fields with default values. This should always be 0
                for field in &contract.ns.structs[*n].fields {
                    let elem = contract.default_value(&field.ty);

                    self.encode_ty(
                        contract,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_fixed,
                        &mut null_offset,
                        &mut null_dynamic,
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let null_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_struct);

                let fixed_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "fixed",
                );

                fixed_phi
                    .add_incoming(&[(&normal_fixed, normal_struct), (&null_fixed, null_struct)]);

                *fixed = fixed_phi.as_basic_value().into_pointer_value();

                let dynamic_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi.add_incoming(&[
                    (&normal_dynamic, normal_struct),
                    (&null_dynamic, null_struct),
                ]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let offset_phi = contract
                    .builder
                    .build_phi(contract.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_struct), (&null_offset, null_struct)]);

                *offset = offset_phi.as_basic_value().into_int_value();
            }
            ast::Type::Ref(ty) => {
                self.encode_ty(contract, load, function, ty, arg, fixed, offset, dynamic);
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                // write the current offset to fixed
                self.encode_primitive(
                    contract,
                    false,
                    function,
                    &ast::Type::Uint(32),
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

                let len = contract.vector_len(arg);

                // write the length to dynamic
                self.encode_primitive(
                    contract,
                    false,
                    function,
                    &ast::Type::Uint(32),
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
                let string_start = contract.vector_bytes(arg);

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
            _ => unreachable!(),
        };
    }

    /// Recursively encode a value in arg. The load argument specifies if the arg is a pointer
    /// to the value, or the value itself. The fixed pointer points to the fixed, non-dynamic part
    /// of the encoded data. The offset is current offset for dynamic fields.
    fn encode_packed_ty(
        &self,
        contract: &Contract<'a>,
        load: bool,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        arg: BasicValueEnum<'a>,
        output: &mut PointerValue<'a>,
    ) {
        match &ty {
            ast::Type::Bool => {
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

                contract.builder.build_store(*output, value);

                *output = unsafe {
                    contract.builder.build_gep(
                        *output,
                        &[contract.context.i32_type().const_int(1, false)],
                        "",
                    )
                };
            }
            ast::Type::Bytes(1) | ast::Type::Int(8) | ast::Type::Uint(8) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                contract.builder.build_store(*output, arg.into_int_value());

                *output = unsafe {
                    contract.builder.build_gep(
                        *output,
                        &[contract.context.i32_type().const_int(1, false)],
                        "",
                    )
                };
            }
            ast::Type::Uint(n) | ast::Type::Int(n)
                if self.bswap && (*n == 16 || *n == 32 || *n == 64) =>
            {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // now convert to be
                let bswap = contract.llvm_bswap(*n as u32);

                let val = contract
                    .builder
                    .build_call(bswap, &[arg], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                contract.builder.build_store(
                    contract.builder.build_pointer_cast(
                        *output,
                        val.get_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    val,
                );

                *output = unsafe {
                    contract.builder.build_gep(
                        *output,
                        &[contract.context.i32_type().const_int(*n as u64 / 8, false)],
                        "",
                    )
                };
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_)
                if load =>
            {
                let n = match ty {
                    ast::Type::Contract(_) | ast::Type::Address(_) => {
                        contract.ns.address_length as u16 * 8
                    }
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let arg8 = contract.builder.build_pointer_cast(
                    arg.into_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "arg8",
                );

                let len = contract.context.i32_type().const_int(n as u64 / 8, false);

                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[arg8.into(), (*output).into(), len.into()],
                    "",
                );

                *output = unsafe { contract.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_)
                if !load =>
            {
                let n = match ty {
                    ast::Type::Contract(_) | ast::Type::Address(_) => {
                        contract.ns.address_length as u16 * 8
                    }
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let temp = contract.build_alloca(
                    function,
                    arg.into_int_value().get_type(),
                    &format!("uint{}", n),
                );

                contract.builder.build_store(temp, arg.into_int_value());

                let len = contract.context.i32_type().const_int(n as u64 / 8, false);

                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                temp,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        (*output).into(),
                        len.into(),
                    ],
                    "",
                );

                *output = unsafe { contract.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = contract.build_alloca(
                        function,
                        arg.into_int_value().get_type(),
                        &format!("bytes{}", n),
                    );

                    contract.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                let len = contract.context.i32_type().const_int(*n as u64, false);

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
                        (*output).into(),
                        len.into(),
                    ],
                    "",
                );

                *output = unsafe { contract.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Array(elem_ty, dim) => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let array_length = if let Some(d) = &dim[0] {
                    // fixed length
                    contract
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false)
                } else {
                    // Now, write the length to dynamic
                    contract.vector_len(arg)
                };

                let normal_array = contract
                    .context
                    .append_basic_block(function, "normal_array");
                let null_array = contract.context.append_basic_block(function, "null_array");
                let done_array = contract.context.append_basic_block(function, "done_array");

                let is_null = contract
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                contract.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(contract, function);

                let mut normal_output = builder
                    .add_loop_phi(
                        contract,
                        "output",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*output).into(),
                    )
                    .into_pointer_value();

                let index = builder.over(
                    contract,
                    contract.context.i32_type().const_zero(),
                    array_length,
                );

                // loop body
                let elem = contract.array_subscript(ty, arg.into_pointer_value(), index);

                self.encode_packed_ty(
                    contract,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_output,
                );

                builder.set_loop_phi_value(contract, "output", normal_output.into());

                builder.finish(contract);

                contract.builder.build_unconditional_branch(done_array);

                let normal_output = builder.get_loop_phi("output");
                let normal_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_array);

                let mut builder = LoopBuilder::new(contract, function);

                let mut null_output = builder
                    .add_loop_phi(
                        contract,
                        "output",
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*output).into(),
                    )
                    .into_pointer_value();

                let _ = builder.over(
                    contract,
                    contract.context.i32_type().const_zero(),
                    array_length,
                );

                // loop body
                let elem = contract.default_value(&elem_ty.deref_any());

                self.encode_packed_ty(
                    contract,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_output,
                );

                builder.set_loop_phi_value(contract, "output", null_output.into());

                builder.finish(contract);

                let null_output = builder.get_loop_phi("output");

                contract.builder.build_unconditional_branch(done_array);

                let null_array = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_array);

                let output_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "output",
                );

                output_phi
                    .add_incoming(&[(&normal_output, normal_array), (&null_output, null_array)]);

                *output = output_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Struct(n) => {
                let arg = if load {
                    contract
                        .builder
                        .build_load(arg.into_pointer_value(), "")
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_struct = contract.context.append_basic_block(function, "null_struct");
                let normal_struct = contract
                    .context
                    .append_basic_block(function, "normal_struct");
                let done_struct = contract.context.append_basic_block(function, "done_struct");

                let is_null = contract.builder.build_is_null(arg, "is_null");

                contract
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                contract.builder.position_at_end(normal_struct);

                let mut normal_output = *output;

                for (i, field) in contract.ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        contract.builder.build_gep(
                            arg,
                            &[
                                contract.context.i32_type().const_zero(),
                                contract.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.encode_packed_ty(
                        contract,
                        true,
                        function,
                        &field.ty,
                        elem.into(),
                        &mut normal_output,
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let normal_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(null_struct);

                let mut null_output = *output;

                // FIXME: abi encoding fixed length fields with default values. This should always be 0
                for field in &contract.ns.structs[*n].fields {
                    let elem = contract.default_value(&field.ty);

                    self.encode_packed_ty(
                        contract,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_output,
                    );
                }

                contract.builder.build_unconditional_branch(done_struct);

                let null_struct = contract.builder.get_insert_block().unwrap();

                contract.builder.position_at_end(done_struct);

                let output_phi = contract.builder.build_phi(
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "output",
                );

                output_phi
                    .add_incoming(&[(&normal_output, normal_struct), (&null_output, null_struct)]);

                *output = output_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Ref(ty) => {
                self.encode_packed_ty(contract, load, function, ty, arg, output);
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = contract.vector_len(arg);

                // now copy the string data
                let string_start = contract.vector_bytes(arg);

                contract.builder.build_call(
                    contract.module.get_function("__memcpy").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                *output,
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

                *output = unsafe { contract.builder.build_gep(*output, &[len], "") };
            }
            _ => unreachable!(),
        };
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        contract: &Contract<'a>,
        load: bool,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        dest: PointerValue,
        arg: BasicValueEnum<'a>,
    ) {
        match ty {
            ast::Type::Bool => {
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
            ast::Type::Int(8) | ast::Type::Uint(8) => {
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

                if let ast::Type::Int(_) = ty {
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
            ast::Type::Uint(n) | ast::Type::Int(n)
                if self.bswap && (*n == 16 || *n == 32 || *n == 64) =>
            {
                let arg = if load {
                    contract.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                if let ast::Type::Int(_) = ty {
                    let negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        arg.into_int_value(),
                        arg.into_int_value().get_type().const_zero(),
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

                // now convert to be
                let bswap = contract.llvm_bswap(*n as u32);

                let val = contract
                    .builder
                    .build_call(bswap, &[arg], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // our value is big endian, 32 bytes. So, find the offset within the 32 bytes
                // where our value starts
                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract
                            .context
                            .i32_type()
                            .const_int(32 - (*n as u64 / 8), false)],
                        "uint_ptr",
                    )
                };

                let int_type = contract.context.custom_width_int_type(*n as u32);

                contract.builder.build_store(
                    contract.builder.build_pointer_cast(
                        int8_ptr,
                        int_type.ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    val,
                );
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_)
                if load =>
            {
                let n = match ty {
                    ast::Type::Contract(_) | ast::Type::Address(_) => {
                        contract.ns.address_length as u16 * 8
                    }
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
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
                    if let ast::Type::Int(_) = ty {
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
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_)
                if !load =>
            {
                let n = match ty {
                    ast::Type::Contract(_) | ast::Type::Address(_) => {
                        contract.ns.address_length as u16 * 8
                    }
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                // first clear/set the upper bits
                if n < 256 {
                    if let ast::Type::Int(_) = ty {
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

                let temp = contract.build_alloca(
                    function,
                    arg.into_int_value().get_type(),
                    &format!("uint{}", n),
                );

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
            ast::Type::Bytes(1) => {
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
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = contract.build_alloca(
                        function,
                        arg.into_int_value().get_type(),
                        &format!("bytes{}", n),
                    );

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
}

pub struct EthAbiDecoder {
    pub bswap: bool,
}

impl EthAbiDecoder {
    /// decode a single primitive which is always encoded in 32 bytes
    fn decode_primitive<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        to: Option<PointerValue<'a>>,
        offset: &mut IntValue<'a>,
        data: PointerValue<'a>,
        length: IntValue,
    ) -> BasicValueEnum<'a> {
        // TODO: investigate whether we can use build_int_nuw_add() and avoid 64 bit conversions
        let new_offset = contract.builder.build_int_add(
            *offset,
            contract.context.i64_type().const_int(32, false),
            "next_offset",
        );

        self.check_overrun(contract, function, new_offset, length);

        let data = unsafe { contract.builder.build_gep(data, &[*offset], "") };

        *offset = new_offset;

        let ty = if let ast::Type::Enum(n) = ty {
            &contract.ns.enums[*n].ty
        } else {
            ty
        };

        match &ty {
            ast::Type::Bool => {
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
            ast::Type::Uint(8) | ast::Type::Int(8) => {
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
            ast::Type::Address(_) | ast::Type::Contract(_) => {
                let int_type = contract
                    .context
                    .custom_width_int_type(contract.ns.address_length as u32 * 8);
                let type_size = int_type.size_of();

                let store =
                    to.unwrap_or_else(|| contract.build_alloca(function, int_type, "address"));

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

                if to.is_none() {
                    contract.builder.build_load(store, "address")
                } else {
                    store.into()
                }
            }
            ast::Type::Uint(n) | ast::Type::Int(n) if self.bswap && *n <= 64 => {
                let bits = if n.is_power_of_two() {
                    *n
                } else {
                    n.next_power_of_two()
                };

                // our value is big endian, 32 bytes. So, find the offset within the 32 bytes
                // where our value starts
                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        data,
                        &[contract
                            .context
                            .i32_type()
                            .const_int(32 - (bits as u64 / 8), false)],
                        "uint8_ptr",
                    )
                };

                let val = contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        int8_ptr,
                        contract
                            .context
                            .custom_width_int_type(bits as u32)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    &format!("be{}", *n),
                );

                // now convert to le
                let bswap = contract.llvm_bswap(bits as u32);

                let mut val = contract
                    .builder
                    .build_call(bswap, &[val], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                if bits > *n {
                    val = contract.builder.build_int_truncate(
                        val,
                        contract.context.custom_width_int_type(*n as u32),
                        "",
                    );
                }

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }

                val.into()
            }
            ast::Type::Uint(n) | ast::Type::Int(n) => {
                let int_type = contract.context.custom_width_int_type(*n as u32);
                let type_size = int_type.size_of();

                let store =
                    to.unwrap_or_else(|| contract.build_alloca(function, int_type, "stack"));

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

                if to.is_none() {
                    contract.builder.build_load(store, &format!("abi_int{}", n))
                } else {
                    store.into()
                }
            }
            ast::Type::Bytes(1) => {
                let val = contract.builder.build_load(data, "bytes1");

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                val
            }
            ast::Type::Bytes(b) => {
                let int_type = contract.context.custom_width_int_type(*b as u32 * 8);

                let store =
                    to.unwrap_or_else(|| contract.build_alloca(function, int_type, "stack"));

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
                            .context
                            .i32_type()
                            .const_int(*b as u64, false)
                            .into(),
                    ],
                    "",
                );

                if to.is_none() {
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
        function: FunctionValue<'b>,
        ty: &ast::Type,
        to: Option<PointerValue<'b>>,
        offset: &mut IntValue<'b>,
        base_offset: IntValue<'b>,
        data: PointerValue<'b>,
        length: IntValue,
    ) -> BasicValueEnum<'b> {
        match &ty {
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = contract.llvm_type(ty.deref_any());

                let size = llvm_ty
                    .size_of()
                    .unwrap()
                    .const_cast(contract.context.i32_type(), false);

                let dest;

                if let Some(d) = &dim[0] {
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

                    // if the struct has dynamic fields, read offset from dynamic section and
                    // read fields from there
                    let mut dataoffset = if ty.is_dynamic(contract.ns) {
                        let dataoffset = contract.builder.build_int_z_extend(
                            self.decode_primitive(
                                contract,
                                function,
                                &ast::Type::Uint(32),
                                None,
                                offset,
                                data,
                                length,
                            )
                            .into_int_value(),
                            contract.context.i64_type(),
                            "rel_struct_offset",
                        );

                        contract
                            .builder
                            .build_int_add(dataoffset, base_offset, "abs_struct_offset")
                    } else {
                        *offset
                    };

                    // In dynamic struct sections, the offsets are relative to the start of the section.
                    // Ethereum ABI encoding is just insane.
                    let base_offset = if ty.is_dynamic(contract.ns) {
                        dataoffset
                    } else {
                        base_offset
                    };

                    contract.emit_loop_cond_first_with_int(
                        function,
                        contract.context.i64_type().const_zero(),
                        contract
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        &mut dataoffset,
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
                                &elem_ty,
                                Some(elem),
                                offset,
                                base_offset,
                                data,
                                length,
                            );
                        },
                    );

                    // if the struct is not dynamic, we have read the fields from fixed section so update
                    if !ty.is_dynamic(contract.ns) {
                        *offset = dataoffset;
                    }
                } else {
                    let mut dataoffset = contract.builder.build_int_add(
                        contract.builder.build_int_z_extend(
                            self.decode_primitive(
                                contract,
                                function,
                                &ast::Type::Uint(32),
                                None,
                                offset,
                                data,
                                length,
                            )
                            .into_int_value(),
                            contract.context.i64_type(),
                            "data_offset",
                        ),
                        base_offset,
                        "array_data_offset",
                    );

                    let array_len = self
                        .decode_primitive(
                            contract,
                            function,
                            &ast::Type::Uint(32),
                            None,
                            &mut dataoffset,
                            data,
                            length,
                        )
                        .into_int_value();

                    // in dynamic arrays, offsets are counted from after the array length
                    let base_offset = dataoffset;

                    let llvm_elem_ty = contract.llvm_var(&elem_ty.deref_any());
                    let elem_size = llvm_elem_ty
                        .size_of()
                        .unwrap()
                        .const_cast(contract.context.i32_type(), false);

                    let init = contract.builder.build_int_to_ptr(
                        contract.context.i32_type().const_all_ones(),
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    dest = contract
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
                                    dest,
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
                                llvm_elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            self.decode_ty(
                                contract,
                                function,
                                &elem_ty,
                                Some(elem),
                                offset,
                                base_offset,
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

                let struct_pointer = contract.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    &contract.ns.structs[*n].name,
                );

                // if the struct has dynamic fields, read offset from dynamic section and
                // read fields from there
                let mut dataoffset = if ty.is_dynamic(contract.ns) {
                    let dataoffset = contract.builder.build_int_z_extend(
                        self.decode_primitive(
                            contract,
                            function,
                            &ast::Type::Uint(32),
                            None,
                            offset,
                            data,
                            length,
                        )
                        .into_int_value(),
                        contract.context.i64_type(),
                        "rel_struct_offset",
                    );

                    contract
                        .builder
                        .build_int_add(dataoffset, base_offset, "abs_struct_offset")
                } else {
                    *offset
                };

                // In dynamic struct sections, the offsets are relative to the start of the section.
                // Ethereum ABI encoding is just insane.
                let base_offset = if ty.is_dynamic(contract.ns) {
                    dataoffset
                } else {
                    base_offset
                };

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
                        &mut dataoffset,
                        base_offset,
                        data,
                        length,
                    );
                }

                // if the struct is not dynamic, we have read the fields from fixed section so update
                if !ty.is_dynamic(contract.ns) {
                    *offset = dataoffset;
                }

                if let Some(to) = to {
                    contract.builder.build_store(to, struct_pointer);
                }

                struct_pointer.into()
            }
            ast::Type::Ref(ty) => self.decode_ty(
                contract,
                function,
                ty,
                to,
                offset,
                base_offset,
                data,
                length,
            ),
            ast::Type::String | ast::Type::DynamicBytes => {
                // we read the offset and the length as 32 bits. Since we are in 32 bits wasm,
                // we cannot deal with more than 4GB of abi encoded data.
                let mut dataoffset = contract.builder.build_int_z_extend(
                    self.decode_primitive(
                        contract,
                        function,
                        &ast::Type::Uint(32),
                        None,
                        offset,
                        data,
                        length,
                    )
                    .into_int_value(),
                    contract.context.i64_type(),
                    "data_offset",
                );

                dataoffset = contract
                    .builder
                    .build_int_add(dataoffset, base_offset, "data_offset");

                let string_len = contract.builder.build_int_z_extend(
                    self.decode_primitive(
                        contract,
                        function,
                        &ast::Type::Uint(32),
                        None,
                        &mut dataoffset,
                        data,
                        length,
                    )
                    .into_int_value(),
                    contract.context.i64_type(),
                    "string_len",
                );

                // Special case string_len == 0 => null pointer?
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
                        .get_struct_type("struct.vector")
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

        contract.builder.build_return(Some(
            &contract.return_values[&ReturnCode::AbiEncodingInvalid],
        ));

        contract.builder.position_at_end(success_block);
    }

    /// abi decode the encoded data into the BasicValueEnums
    pub fn decode<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue<'a>,
        args: &mut Vec<BasicValueEnum<'a>>,
        data: PointerValue<'a>,
        datalength: IntValue<'a>,
        spec: &[ast::Parameter],
    ) {
        let data = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "data",
        );

        let mut offset = contract.context.i64_type().const_zero();

        let data_length = if datalength.get_type().get_bit_width() != 64 {
            contract.builder.build_int_z_extend(
                datalength,
                contract.context.i64_type(),
                "data_length",
            )
        } else {
            datalength
        };

        for arg in spec {
            args.push(self.decode_ty(
                contract,
                function,
                &arg.ty,
                None,
                &mut offset,
                contract.context.i64_type().const_zero(),
                data,
                data_length,
            ));
        }
    }
}

/// ABI encode into a vector for abi.encode* style builtin functions
pub fn encode_to_vector<'b>(
    contract: &Contract<'b>,
    function: FunctionValue<'b>,
    packed: &[BasicValueEnum<'b>],
    args: &[BasicValueEnum<'b>],
    tys: &[ast::Type],
    bswap: bool,
) -> PointerValue<'b> {
    let encoder = EncoderBuilder::new(contract, function, false, packed, args, tys, bswap);

    let length = encoder.encoded_length();

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

    let data = contract.builder.build_pointer_cast(
        data,
        contract.context.i8_type().ptr_type(AddressSpace::Generic),
        "",
    );

    encoder.finish(contract, function, data);

    v
}
