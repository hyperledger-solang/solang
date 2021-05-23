use crate::sema::ast;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use num_traits::ToPrimitive;

use super::loop_builder::LoopBuilder;
use super::{Binary, ReturnCode};

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
        binary: &Binary<'a>,
        function: FunctionValue,
        load_args: bool,
        packed: &'b [BasicValueEnum<'a>],
        args: &'b [BasicValueEnum<'a>],
        tys: &'b [ast::Type],
        bswap: bool,
        ns: &ast::Namespace,
    ) -> Self {
        debug_assert_eq!(packed.len() + args.len(), tys.len());

        let args_tys = &tys[packed.len()..];

        let offset = binary.context.i32_type().const_int(
            args_tys
                .iter()
                .map(|ty| EncoderBuilder::encoded_fixed_length(ty, ns))
                .sum(),
            false,
        );

        let mut length = offset;

        // calculate the packed length
        for (i, arg) in packed.iter().enumerate() {
            length = binary.builder.build_int_add(
                length,
                EncoderBuilder::encoded_packed_length(
                    *arg, load_args, &tys[i], function, binary, ns,
                ),
                "",
            );
        }

        // now add the dynamic lengths
        for (i, arg) in args.iter().enumerate() {
            length = binary.builder.build_int_add(
                length,
                EncoderBuilder::encoded_dynamic_length(
                    *arg,
                    load_args,
                    &args_tys[i],
                    function,
                    binary,
                    ns,
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
        binary: &Binary<'c>,
        ns: &ast::Namespace,
    ) -> IntValue<'c> {
        match ty {
            ast::Type::Struct(n) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_sum = binary.context.i32_type().const_zero();

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    let len = EncoderBuilder::encoded_packed_length(
                        elem.into(),
                        true,
                        &field.ty,
                        function,
                        binary,
                        ns,
                    );

                    normal_sum = binary.builder.build_int_add(normal_sum, len, "");
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_sum = binary.context.i32_type().const_zero();

                for field in &ns.structs[*n].fields {
                    null_sum = binary.builder.build_int_add(
                        null_sum,
                        EncoderBuilder::encoded_packed_length(
                            binary.default_value(&field.ty, ns),
                            false,
                            &field.ty,
                            function,
                            binary,
                            ns,
                        ),
                        "",
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let sum = binary.builder.build_phi(binary.context.i32_type(), "sum");

                sum.add_incoming(&[(&normal_sum, normal_struct), (&null_sum, null_struct)]);

                sum.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) if elem_ty.is_dynamic(ns) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let sum = binary.context.i32_type().const_zero();

                let len = match dims.last().unwrap() {
                    None => binary.vector_len(arg),
                    Some(d) => binary
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let null_array = binary.context.append_basic_block(function, "null_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut normal_length = sum;

                binary.builder.position_at_end(normal_array);

                // the element of the array are dynamic; we need to iterate over the array to find the encoded length
                binary.emit_loop_cond_first_with_int(
                    function,
                    binary.context.i32_type().const_zero(),
                    len,
                    &mut normal_length,
                    |index, sum| {
                        let elem = binary.array_subscript(ty, arg.into_pointer_value(), index, ns);

                        *sum = binary.builder.build_int_add(
                            EncoderBuilder::encoded_packed_length(
                                elem.into(),
                                true,
                                &elem_ty,
                                function,
                                binary,
                                ns,
                            ),
                            *sum,
                            "",
                        );
                    },
                );

                binary.builder.build_unconditional_branch(done_array);

                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                let elem = binary.default_value(&elem_ty.deref_any(), ns);

                let null_length = binary.builder.build_int_add(
                    binary.builder.build_int_mul(
                        EncoderBuilder::encoded_packed_length(
                            elem, false, elem_ty, function, binary, ns,
                        ),
                        len,
                        "",
                    ),
                    sum,
                    "",
                );

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let encoded_length = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "encoded_length");

                encoded_length
                    .add_incoming(&[(&normal_length, normal_array), (&null_length, null_array)]);

                encoded_length.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = match dims.last().unwrap() {
                    None => binary.vector_len(arg),
                    Some(d) => binary
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                // plus fixed size elements
                binary.builder.build_int_mul(
                    len,
                    EncoderBuilder::encoded_packed_length(
                        arg, false, &elem_ty, function, binary, ns,
                    ),
                    "",
                )
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                binary.vector_len(arg)
            }
            ast::Type::Uint(n) | ast::Type::Int(n) => {
                binary.context.i32_type().const_int((*n as u64) / 8, false)
            }
            ast::Type::Bytes(n) => binary.context.i32_type().const_int(*n as u64, false),
            ast::Type::Enum(_) | ast::Type::Bool => binary.context.i32_type().const_int(1, false),
            ast::Type::Contract(_) | ast::Type::Address(_) => binary
                .context
                .i32_type()
                .const_int(ns.address_length as u64, false),
            ast::Type::Ref(ty) => {
                EncoderBuilder::encoded_packed_length(arg, false, ty, function, binary, ns)
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
        binary: &Binary<'c>,
        ns: &ast::Namespace,
    ) -> IntValue<'c> {
        match ty {
            ast::Type::Struct(n) if ty.is_dynamic(ns) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_sum = binary.context.i32_type().const_zero();

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    // a struct with dynamic fields gets stored in the dynamic part
                    normal_sum = binary.builder.build_int_add(
                        normal_sum,
                        binary
                            .context
                            .i32_type()
                            .const_int(EncoderBuilder::encoded_fixed_length(&field.ty, ns), false),
                        "",
                    );

                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    let len = EncoderBuilder::encoded_dynamic_length(
                        elem.into(),
                        true,
                        &field.ty,
                        function,
                        binary,
                        ns,
                    );

                    normal_sum = binary.builder.build_int_add(normal_sum, len, "");
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_sum = binary.context.i32_type().const_zero();

                for field in &ns.structs[*n].fields {
                    // a struct with dynamic fields gets stored in the dynamic part
                    null_sum = binary.builder.build_int_add(
                        null_sum,
                        binary
                            .context
                            .i32_type()
                            .const_int(EncoderBuilder::encoded_fixed_length(&field.ty, ns), false),
                        "",
                    );

                    null_sum = binary.builder.build_int_add(
                        null_sum,
                        EncoderBuilder::encoded_dynamic_length(
                            binary.default_value(&field.ty, ns),
                            false,
                            &field.ty,
                            function,
                            binary,
                            ns,
                        ),
                        "",
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let sum = binary.builder.build_phi(binary.context.i32_type(), "sum");

                sum.add_incoming(&[(&normal_sum, normal_struct), (&null_sum, null_struct)]);

                sum.as_basic_value().into_int_value()
            }
            ast::Type::Array(elem_ty, dims) if ty.is_dynamic(ns) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let mut sum = binary.context.i32_type().const_zero();

                let len = match dims.last().unwrap() {
                    None => {
                        let array_len = binary.vector_len(arg);

                        // A dynamic array will store its own length
                        sum = binary.builder.build_int_add(
                            sum,
                            binary.context.i32_type().const_int(32, false),
                            "",
                        );

                        array_len
                    }
                    Some(d) => binary
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false),
                };

                // plus fixed size elements
                sum = binary.builder.build_int_add(
                    sum,
                    binary.builder.build_int_mul(
                        len,
                        binary
                            .context
                            .i32_type()
                            .const_int(EncoderBuilder::encoded_fixed_length(&elem_ty, ns), false),
                        "",
                    ),
                    "",
                );

                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let null_array = binary.context.append_basic_block(function, "null_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut normal_length = sum;

                binary.builder.position_at_end(normal_array);

                // the element of the array are dynamic; we need to iterate over the array to find the encoded length
                if elem_ty.is_dynamic(ns) {
                    binary.emit_loop_cond_first_with_int(
                        function,
                        binary.context.i32_type().const_zero(),
                        len,
                        &mut normal_length,
                        |index, sum| {
                            let elem =
                                binary.array_subscript(ty, arg.into_pointer_value(), index, ns);

                            *sum = binary.builder.build_int_add(
                                EncoderBuilder::encoded_dynamic_length(
                                    elem.into(),
                                    true,
                                    &elem_ty,
                                    function,
                                    binary,
                                    ns,
                                ),
                                *sum,
                                "",
                            );
                        },
                    );
                }

                binary.builder.build_unconditional_branch(done_array);

                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                let elem = binary.default_value(&elem_ty.deref_any(), ns);

                let null_length = binary.builder.build_int_add(
                    binary.builder.build_int_mul(
                        EncoderBuilder::encoded_dynamic_length(
                            elem, false, elem_ty, function, binary, ns,
                        ),
                        len,
                        "",
                    ),
                    sum,
                    "",
                );

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let encoded_length = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "encoded_length");

                encoded_length
                    .add_incoming(&[(&normal_length, normal_array), (&null_length, null_array)]);

                encoded_length.as_basic_value().into_int_value()
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // The dynamic part is the length (=32 bytes) and the string
                // data itself. Length 0 occupies no space, length 1-32 occupies
                // 32 bytes, etc
                binary.builder.build_and(
                    binary.builder.build_int_add(
                        binary.vector_len(arg),
                        binary.context.i32_type().const_int(32 + 31, false),
                        "",
                    ),
                    binary.context.i32_type().const_int(!31, false),
                    "",
                )
            }
            _ => binary.context.i32_type().const_zero(),
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
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        output: PointerValue<'a>,
        ns: &ast::Namespace,
    ) {
        let mut output = output;
        let mut ty_iter = self.tys.iter();

        for arg in self.packed.iter() {
            let ty = ty_iter.next().unwrap();

            self.encode_packed_ty(binary, self.load_args, function, ty, *arg, &mut output, ns);
        }

        // We use a little trick here. The length might or might not include the selector.
        // The length will be a multiple of 32 plus the selector (4). So by dividing by 8,
        // we lose the selector.
        binary.builder.build_call(
            binary.module.get_function("__bzero8").unwrap(),
            &[
                output.into(),
                binary
                    .builder
                    .build_int_unsigned_div(
                        self.length,
                        binary.context.i32_type().const_int(8, false),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        let mut output = output;
        let mut offset = self.offset;
        let mut dynamic = unsafe { binary.builder.build_gep(output, &[self.offset], "") };

        for arg in self.args.iter() {
            let ty = ty_iter.next().unwrap();

            self.encode_ty(
                binary,
                ns,
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
        binary: &Binary<'a>,
        ns: &ast::Namespace,
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
                self.encode_primitive(binary, load, function, ty, *fixed, arg, ns);

                *fixed = unsafe {
                    binary.builder.build_gep(
                        *fixed,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            ast::Type::Enum(n) => {
                self.encode_primitive(binary, load, function, &ns.enums[*n].ty, *fixed, arg, ns);

                *fixed = unsafe {
                    binary.builder.build_gep(
                        *fixed,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };
            }
            ast::Type::Array(elem_ty, dim) if ty.is_dynamic(ns) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // if the array is of dynamic length, or has dynamic array elements, then it is written to
                // the dynamic section.

                // write the current offset to fixed
                self.encode_primitive(
                    binary,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    *fixed,
                    (*offset).into(),
                    ns,
                );

                *fixed = unsafe {
                    binary.builder.build_gep(
                        *fixed,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                let array_length = if let Some(d) = &dim[0] {
                    // fixed length
                    binary
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false)
                } else {
                    // Now, write the length to dynamic
                    let len = binary.vector_len(arg);

                    // write the current offset to fixed
                    self.encode_primitive(
                        binary,
                        false,
                        function,
                        &ast::Type::Uint(32),
                        *dynamic,
                        len.into(),
                        ns,
                    );

                    *dynamic = unsafe {
                        binary.builder.build_gep(
                            *dynamic,
                            &[binary.context.i32_type().const_int(32, false)],
                            "",
                        )
                    };

                    *offset = binary.builder.build_int_add(
                        *offset,
                        binary.context.i32_type().const_int(32, false),
                        "",
                    );

                    len
                };

                let array_data_offset = binary.builder.build_int_mul(
                    binary
                        .context
                        .i32_type()
                        .const_int(EncoderBuilder::encoded_fixed_length(&elem_ty, ns), false),
                    array_length,
                    "array_data_offset",
                );

                let normal_fixed = *dynamic;
                let null_fixed = *dynamic;

                *dynamic = unsafe { binary.builder.build_gep(*dynamic, &[array_data_offset], "") };

                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let null_array = binary.context.append_basic_block(function, "null_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(binary, function);

                let mut normal_fixed = builder
                    .add_loop_phi(
                        binary,
                        "fixed",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        normal_fixed.into(),
                    )
                    .into_pointer_value();

                let mut normal_array_data_offset = builder
                    .add_loop_phi(
                        binary,
                        "offset",
                        binary.context.i32_type(),
                        array_data_offset.into(),
                    )
                    .into_int_value();

                let mut normal_dynamic = builder
                    .add_loop_phi(
                        binary,
                        "dynamic",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let index =
                    builder.over(binary, binary.context.i32_type().const_zero(), array_length);

                // loop body
                let elem = binary.array_subscript(ty, arg.into_pointer_value(), index, ns);

                self.encode_ty(
                    binary,
                    ns,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_fixed,
                    &mut normal_array_data_offset,
                    &mut normal_dynamic,
                );

                builder.set_loop_phi_value(binary, "fixed", normal_fixed.into());
                builder.set_loop_phi_value(binary, "offset", normal_array_data_offset.into());
                builder.set_loop_phi_value(binary, "dynamic", normal_dynamic.into());

                builder.finish(binary);

                let normal_dynamic = builder.get_loop_phi("dynamic");
                let normal_array_data_offset = builder.get_loop_phi("offset");

                binary.builder.build_unconditional_branch(done_array);

                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                let mut builder = LoopBuilder::new(binary, function);

                let mut null_fixed = builder
                    .add_loop_phi(
                        binary,
                        "fixed",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        null_fixed.into(),
                    )
                    .into_pointer_value();

                let mut null_array_data_offset = builder
                    .add_loop_phi(
                        binary,
                        "offset",
                        binary.context.i32_type(),
                        array_data_offset.into(),
                    )
                    .into_int_value();

                let mut null_dynamic = builder
                    .add_loop_phi(
                        binary,
                        "dynamic",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let _ = builder.over(binary, binary.context.i32_type().const_zero(), array_length);

                // loop body
                let elem = binary.default_value(&elem_ty.deref_any(), ns);

                self.encode_ty(
                    binary,
                    ns,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_fixed,
                    &mut null_array_data_offset,
                    &mut null_dynamic,
                );

                builder.set_loop_phi_value(binary, "fixed", null_fixed.into());
                builder.set_loop_phi_value(binary, "offset", null_array_data_offset.into());
                builder.set_loop_phi_value(binary, "dynamic", null_dynamic.into());

                builder.finish(binary);

                let null_dynamic = builder.get_loop_phi("dynamic");
                let null_array_data_offset = builder.get_loop_phi("offset");

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let dynamic_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi
                    .add_incoming(&[(&normal_dynamic, normal_array), (&null_dynamic, null_array)]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let array_array_offset_phi = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "array_data_offset");

                array_array_offset_phi.add_incoming(&[
                    (&normal_array_data_offset, normal_array),
                    (&null_array_data_offset, null_array),
                ]);

                *offset = binary.builder.build_int_add(
                    array_array_offset_phi.as_basic_value().into_int_value(),
                    *offset,
                    "new_offset",
                );
            }
            ast::Type::Array(elem_ty, dim) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dim = dim[0].as_ref().unwrap().to_u64().unwrap();

                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let null_array = binary.context.append_basic_block(function, "null_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(binary, function);

                let mut normal_fixed = builder
                    .add_loop_phi(
                        binary,
                        "fixed",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*fixed).into(),
                    )
                    .into_pointer_value();

                let mut normal_offset = builder
                    .add_loop_phi(
                        binary,
                        "offset",
                        binary.context.i32_type(),
                        (*offset).into(),
                    )
                    .into_int_value();

                let mut normal_dynamic = builder
                    .add_loop_phi(
                        binary,
                        "dynamic",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                let index = builder.over(
                    binary,
                    binary.context.i64_type().const_zero(),
                    binary.context.i64_type().const_int(dim, false),
                );

                // loop body
                let elem = unsafe {
                    binary.builder.build_gep(
                        arg.into_pointer_value(),
                        &[binary.context.i32_type().const_zero(), index],
                        "index_access",
                    )
                };

                self.encode_ty(
                    binary,
                    ns,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_fixed,
                    &mut normal_offset,
                    &mut normal_dynamic,
                );

                builder.set_loop_phi_value(binary, "fixed", normal_fixed.into());
                builder.set_loop_phi_value(binary, "offset", normal_offset.into());
                builder.set_loop_phi_value(binary, "dynamic", normal_dynamic.into());

                builder.finish(binary);

                let normal_fixed = builder.get_loop_phi("fixed");
                let normal_offset = builder.get_loop_phi("offset");
                let normal_dynamic = builder.get_loop_phi("dynamic");

                binary.builder.build_unconditional_branch(done_array);

                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                // Create a loop for generating an array of empty values
                // FIXME: all fixed-length types are encoded as zeros, and the memory has
                // already been zero'ed out, so this is pointless. Just step over it.
                let elem = binary.default_value(&elem_ty.deref_any(), ns);

                let mut builder = LoopBuilder::new(binary, function);

                let mut null_fixed = builder
                    .add_loop_phi(
                        binary,
                        "fixed",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*fixed).into(),
                    )
                    .into_pointer_value();

                let mut null_offset = builder
                    .add_loop_phi(
                        binary,
                        "offset",
                        binary.context.i32_type(),
                        (*offset).into(),
                    )
                    .into_int_value();

                let mut null_dynamic = builder
                    .add_loop_phi(
                        binary,
                        "dynamic",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*dynamic).into(),
                    )
                    .into_pointer_value();

                builder.over(
                    binary,
                    binary.context.i64_type().const_zero(),
                    binary.context.i64_type().const_int(dim, false),
                );

                // loop body
                self.encode_ty(
                    binary,
                    ns,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_fixed,
                    &mut null_offset,
                    &mut null_dynamic,
                );

                builder.set_loop_phi_value(binary, "fixed", null_fixed.into());
                builder.set_loop_phi_value(binary, "offset", null_offset.into());
                builder.set_loop_phi_value(binary, "dynamic", null_dynamic.into());

                builder.finish(binary);

                let null_fixed = builder.get_loop_phi("fixed");
                let null_offset = builder.get_loop_phi("offset");
                let null_dynamic = builder.get_loop_phi("dynamic");

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let fixed_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "fixed",
                );

                fixed_phi.add_incoming(&[(&normal_fixed, normal_array), (&null_fixed, null_array)]);

                *fixed = fixed_phi.as_basic_value().into_pointer_value();

                let offset_phi = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_array), (&null_offset, null_array)]);

                *offset = offset_phi.as_basic_value().into_int_value();

                let dynamic_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi
                    .add_incoming(&[(&normal_dynamic, normal_array), (&null_dynamic, null_array)]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Struct(n) if ty.is_dynamic(ns) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // write the current offset to fixed
                self.encode_primitive(
                    binary,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    *fixed,
                    (*offset).into(),
                    ns,
                );

                *fixed = unsafe {
                    binary.builder.build_gep(
                        *fixed,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                let mut normal_fields_dynamic = *dynamic;
                let mut null_fields_dynamic = *dynamic;

                // add size of fixed fields to dynamic
                let fixed_field_length = ns.structs[*n]
                    .fields
                    .iter()
                    .map(|f| EncoderBuilder::encoded_fixed_length(&f.ty, ns))
                    .sum();

                *dynamic = unsafe {
                    binary.builder.build_gep(
                        *dynamic,
                        &[binary
                            .context
                            .i32_type()
                            .const_int(fixed_field_length, false)],
                        "",
                    )
                };

                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                let mut normal_dynamic = *dynamic;
                let mut null_dynamic = *dynamic;
                let normal_offset = *offset;
                let null_offset = *offset;

                binary.builder.position_at_end(normal_struct);

                let mut temp_offset = binary
                    .context
                    .i32_type()
                    .const_int(fixed_field_length, false);

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg.into_pointer_value(),
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.encode_ty(
                        binary,
                        ns,
                        true,
                        function,
                        &field.ty,
                        elem.into(),
                        &mut normal_fields_dynamic,
                        &mut temp_offset,
                        &mut normal_dynamic,
                    );
                }

                let normal_offset = binary.builder.build_int_add(normal_offset, temp_offset, "");

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut temp_offset = binary
                    .context
                    .i32_type()
                    .const_int(fixed_field_length, false);

                for field in &ns.structs[*n].fields {
                    let elem = binary.default_value(&field.ty, ns);

                    self.encode_ty(
                        binary,
                        ns,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_fields_dynamic,
                        &mut temp_offset,
                        &mut null_dynamic,
                    );
                }

                let null_offset = binary.builder.build_int_add(null_offset, temp_offset, "");

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let dynamic_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi.add_incoming(&[
                    (&normal_dynamic, normal_struct),
                    (&null_dynamic, null_struct),
                ]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let offset_phi = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_struct), (&null_offset, null_struct)]);

                *offset = offset_phi.as_basic_value().into_int_value();
            }
            ast::Type::Struct(n) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(arg.into_pointer_value(), "")
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary.builder.build_is_null(arg, "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_fixed = *fixed;
                let mut normal_offset = *offset;
                let mut normal_dynamic = *dynamic;

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.encode_ty(
                        binary,
                        ns,
                        true,
                        function,
                        &field.ty,
                        elem.into(),
                        &mut normal_fixed,
                        &mut normal_offset,
                        &mut normal_dynamic,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_fixed = *fixed;
                let mut null_offset = *offset;
                let mut null_dynamic = *dynamic;

                // FIXME: abi encoding fixed length fields with default values. This should always be 0
                for field in &ns.structs[*n].fields {
                    let elem = binary.default_value(&field.ty, ns);

                    self.encode_ty(
                        binary,
                        ns,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_fixed,
                        &mut null_offset,
                        &mut null_dynamic,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let fixed_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "fixed",
                );

                fixed_phi
                    .add_incoming(&[(&normal_fixed, normal_struct), (&null_fixed, null_struct)]);

                *fixed = fixed_phi.as_basic_value().into_pointer_value();

                let dynamic_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dynamic",
                );

                dynamic_phi.add_incoming(&[
                    (&normal_dynamic, normal_struct),
                    (&null_dynamic, null_struct),
                ]);

                *dynamic = dynamic_phi.as_basic_value().into_pointer_value();

                let offset_phi = binary
                    .builder
                    .build_phi(binary.context.i32_type(), "offset");

                offset_phi
                    .add_incoming(&[(&normal_offset, normal_struct), (&null_offset, null_struct)]);

                *offset = offset_phi.as_basic_value().into_int_value();
            }
            ast::Type::Ref(ty) => {
                self.encode_ty(binary, ns, load, function, ty, arg, fixed, offset, dynamic);
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                // write the current offset to fixed
                self.encode_primitive(
                    binary,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    *fixed,
                    (*offset).into(),
                    ns,
                );

                *fixed = unsafe {
                    binary.builder.build_gep(
                        *fixed,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = binary.vector_len(arg);

                // write the length to dynamic
                self.encode_primitive(
                    binary,
                    false,
                    function,
                    &ast::Type::Uint(32),
                    *dynamic,
                    len.into(),
                    ns,
                );

                *dynamic = unsafe {
                    binary.builder.build_gep(
                        *dynamic,
                        &[binary.context.i32_type().const_int(32, false)],
                        "",
                    )
                };

                *offset = binary.builder.build_int_add(
                    *offset,
                    binary.context.i32_type().const_int(32, false),
                    "",
                );

                // now copy the string data
                let string_start = binary.vector_bytes(arg);

                binary.builder.build_call(
                    binary.module.get_function("__memcpy").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                *dynamic,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "encoded_string",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                string_start,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "string_start",
                            )
                            .into(),
                        len.into(),
                    ],
                    "",
                );

                // round up the length to the next 32 bytes block
                let len = binary.builder.build_and(
                    binary.builder.build_int_add(
                        len,
                        binary.context.i32_type().const_int(31, false),
                        "",
                    ),
                    binary.context.i32_type().const_int(!31, false),
                    "",
                );

                *dynamic = unsafe { binary.builder.build_gep(*dynamic, &[len], "") };

                *offset = binary.builder.build_int_add(*offset, len, "");
            }
            _ => unreachable!(),
        };
    }

    /// Recursively encode a value in arg. The load argument specifies if the arg is a pointer
    /// to the value, or the value itself. The fixed pointer points to the fixed, non-dynamic part
    /// of the encoded data. The offset is current offset for dynamic fields.
    fn encode_packed_ty(
        &self,
        binary: &Binary<'a>,
        load: bool,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        arg: BasicValueEnum<'a>,
        output: &mut PointerValue<'a>,
        ns: &ast::Namespace,
    ) {
        match &ty {
            ast::Type::Bool => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let value = binary.builder.build_select(
                    arg.into_int_value(),
                    binary.context.i8_type().const_int(1, false),
                    binary.context.i8_type().const_zero(),
                    "bool_val",
                );

                binary.builder.build_store(*output, value);

                *output = unsafe {
                    binary.builder.build_gep(
                        *output,
                        &[binary.context.i32_type().const_int(1, false)],
                        "",
                    )
                };
            }
            ast::Type::Bytes(1) | ast::Type::Int(8) | ast::Type::Uint(8) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                binary.builder.build_store(*output, arg.into_int_value());

                *output = unsafe {
                    binary.builder.build_gep(
                        *output,
                        &[binary.context.i32_type().const_int(1, false)],
                        "",
                    )
                };
            }
            ast::Type::Uint(n) | ast::Type::Int(n)
                if self.bswap && (*n == 16 || *n == 32 || *n == 64) =>
            {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                // now convert to be
                let bswap = binary.llvm_bswap(*n as u32);

                let val = binary
                    .builder
                    .build_call(bswap, &[arg], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                binary.builder.build_store(
                    binary.builder.build_pointer_cast(
                        *output,
                        val.get_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    val,
                );

                *output = unsafe {
                    binary.builder.build_gep(
                        *output,
                        &[binary.context.i32_type().const_int(*n as u64 / 8, false)],
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
                    ast::Type::Contract(_) | ast::Type::Address(_) => ns.address_length as u16 * 8,
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let arg8 = binary.builder.build_pointer_cast(
                    arg.into_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "arg8",
                );

                let len = binary.context.i32_type().const_int(n as u64 / 8, false);

                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[arg8.into(), (*output).into(), len.into()],
                    "",
                );

                *output = unsafe { binary.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Contract(_)
            | ast::Type::Address(_)
            | ast::Type::Uint(_)
            | ast::Type::Int(_)
                if !load =>
            {
                let n = match ty {
                    ast::Type::Contract(_) | ast::Type::Address(_) => ns.address_length as u16 * 8,
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let temp = binary.build_alloca(
                    function,
                    arg.into_int_value().get_type(),
                    &format!("uint{}", n),
                );

                binary.builder.build_store(temp, arg.into_int_value());

                let len = binary.context.i32_type().const_int(n as u64 / 8, false);

                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                temp,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        (*output).into(),
                        len.into(),
                    ],
                    "",
                );

                *output = unsafe { binary.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = binary.build_alloca(
                        function,
                        arg.into_int_value().get_type(),
                        &format!("bytes{}", n),
                    );

                    binary.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                let len = binary.context.i32_type().const_int(*n as u64, false);

                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                val,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        (*output).into(),
                        len.into(),
                    ],
                    "",
                );

                *output = unsafe { binary.builder.build_gep(*output, &[len], "") };
            }
            ast::Type::Array(elem_ty, dim) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let array_length = if let Some(d) = &dim[0] {
                    // fixed length
                    binary
                        .context
                        .i32_type()
                        .const_int(d.to_u64().unwrap(), false)
                } else {
                    // Now, write the length to dynamic
                    binary.vector_len(arg)
                };

                let normal_array = binary.context.append_basic_block(function, "normal_array");
                let null_array = binary.context.append_basic_block(function, "null_array");
                let done_array = binary.context.append_basic_block(function, "done_array");

                let is_null = binary
                    .builder
                    .build_is_null(arg.into_pointer_value(), "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_array, normal_array);

                binary.builder.position_at_end(normal_array);

                let mut builder = LoopBuilder::new(binary, function);

                let mut normal_output = builder
                    .add_loop_phi(
                        binary,
                        "output",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*output).into(),
                    )
                    .into_pointer_value();

                let index =
                    builder.over(binary, binary.context.i32_type().const_zero(), array_length);

                // loop body
                let elem = binary.array_subscript(ty, arg.into_pointer_value(), index, ns);

                self.encode_packed_ty(
                    binary,
                    true,
                    function,
                    &elem_ty.deref_any(),
                    elem.into(),
                    &mut normal_output,
                    ns,
                );

                builder.set_loop_phi_value(binary, "output", normal_output.into());

                builder.finish(binary);

                binary.builder.build_unconditional_branch(done_array);

                let normal_output = builder.get_loop_phi("output");
                let normal_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_array);

                let mut builder = LoopBuilder::new(binary, function);

                let mut null_output = builder
                    .add_loop_phi(
                        binary,
                        "output",
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        (*output).into(),
                    )
                    .into_pointer_value();

                let _ = builder.over(binary, binary.context.i32_type().const_zero(), array_length);

                // loop body
                let elem = binary.default_value(&elem_ty.deref_any(), ns);

                self.encode_packed_ty(
                    binary,
                    false,
                    function,
                    &elem_ty.deref_any(),
                    elem,
                    &mut null_output,
                    ns,
                );

                builder.set_loop_phi_value(binary, "output", null_output.into());

                builder.finish(binary);

                let null_output = builder.get_loop_phi("output");

                binary.builder.build_unconditional_branch(done_array);

                let null_array = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_array);

                let output_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "output",
                );

                output_phi
                    .add_incoming(&[(&normal_output, normal_array), (&null_output, null_array)]);

                *output = output_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Struct(n) => {
                let arg = if load {
                    binary
                        .builder
                        .build_load(arg.into_pointer_value(), "")
                        .into_pointer_value()
                } else {
                    arg.into_pointer_value()
                };

                let null_struct = binary.context.append_basic_block(function, "null_struct");
                let normal_struct = binary.context.append_basic_block(function, "normal_struct");
                let done_struct = binary.context.append_basic_block(function, "done_struct");

                let is_null = binary.builder.build_is_null(arg, "is_null");

                binary
                    .builder
                    .build_conditional_branch(is_null, null_struct, normal_struct);

                binary.builder.position_at_end(normal_struct);

                let mut normal_output = *output;

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            arg,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.encode_packed_ty(
                        binary,
                        true,
                        function,
                        &field.ty,
                        elem.into(),
                        &mut normal_output,
                        ns,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let normal_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(null_struct);

                let mut null_output = *output;

                // FIXME: abi encoding fixed length fields with default values. This should always be 0
                for field in &ns.structs[*n].fields {
                    let elem = binary.default_value(&field.ty, ns);

                    self.encode_packed_ty(
                        binary,
                        false,
                        function,
                        &field.ty,
                        elem,
                        &mut null_output,
                        ns,
                    );
                }

                binary.builder.build_unconditional_branch(done_struct);

                let null_struct = binary.builder.get_insert_block().unwrap();

                binary.builder.position_at_end(done_struct);

                let output_phi = binary.builder.build_phi(
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "output",
                );

                output_phi
                    .add_incoming(&[(&normal_output, normal_struct), (&null_output, null_struct)]);

                *output = output_phi.as_basic_value().into_pointer_value();
            }
            ast::Type::Ref(ty) => {
                self.encode_packed_ty(binary, load, function, ty, arg, output, ns);
            }
            ast::Type::String | ast::Type::DynamicBytes => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let len = binary.vector_len(arg);

                // now copy the string data
                let string_start = binary.vector_bytes(arg);

                binary.builder.build_call(
                    binary.module.get_function("__memcpy").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                *output,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "encoded_string",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                string_start,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "string_start",
                            )
                            .into(),
                        len.into(),
                    ],
                    "",
                );

                *output = unsafe { binary.builder.build_gep(*output, &[len], "") };
            }
            _ => unreachable!(),
        };
    }

    /// ABI encode a single primitive
    fn encode_primitive(
        &self,
        binary: &Binary<'a>,
        load: bool,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        dest: PointerValue,
        arg: BasicValueEnum<'a>,
        ns: &ast::Namespace,
    ) {
        match ty {
            ast::Type::Bool => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let value = binary.builder.build_select(
                    arg.into_int_value(),
                    binary.context.i8_type().const_int(1, false),
                    binary.context.i8_type().const_zero(),
                    "bool_val",
                );

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                let dest = unsafe {
                    binary.builder.build_gep(
                        dest8,
                        &[binary.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                binary.builder.build_store(dest, value);
            }
            ast::Type::Int(8) | ast::Type::Uint(8) => {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                if let ast::Type::Int(_) = ty {
                    let negative = binary.builder.build_int_compare(
                        IntPredicate::SLT,
                        arg.into_int_value(),
                        binary.context.i8_type().const_zero(),
                        "neg",
                    );

                    let signval = binary
                        .builder
                        .build_select(
                            negative,
                            binary.context.i64_type().const_int(std::u64::MAX, true),
                            binary.context.i64_type().const_zero(),
                            "val",
                        )
                        .into_int_value();

                    binary.builder.build_call(
                        binary.module.get_function("__memset8").unwrap(),
                        &[
                            dest8.into(),
                            signval.into(),
                            binary.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                let dest = unsafe {
                    binary.builder.build_gep(
                        dest8,
                        &[binary.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                binary.builder.build_store(dest, arg);
            }
            ast::Type::Uint(n) | ast::Type::Int(n)
                if self.bswap && (*n == 16 || *n == 32 || *n == 64) =>
            {
                let arg = if load {
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                if let ast::Type::Int(_) = ty {
                    let negative = binary.builder.build_int_compare(
                        IntPredicate::SLT,
                        arg.into_int_value(),
                        arg.into_int_value().get_type().const_zero(),
                        "neg",
                    );

                    let signval = binary
                        .builder
                        .build_select(
                            negative,
                            binary.context.i64_type().const_int(std::u64::MAX, true),
                            binary.context.i64_type().const_zero(),
                            "val",
                        )
                        .into_int_value();

                    binary.builder.build_call(
                        binary.module.get_function("__memset8").unwrap(),
                        &[
                            dest8.into(),
                            signval.into(),
                            binary.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                // now convert to be
                let bswap = binary.llvm_bswap(*n as u32);

                let val = binary
                    .builder
                    .build_call(bswap, &[arg], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // our value is big endian, 32 bytes. So, find the offset within the 32 bytes
                // where our value starts
                let int8_ptr = unsafe {
                    binary.builder.build_gep(
                        dest8,
                        &[binary
                            .context
                            .i32_type()
                            .const_int(32 - (*n as u64 / 8), false)],
                        "uint_ptr",
                    )
                };

                let int_type = binary.context.custom_width_int_type(*n as u32);

                binary.builder.build_store(
                    binary.builder.build_pointer_cast(
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
                    ast::Type::Contract(_) | ast::Type::Address(_) => ns.address_length as u16 * 8,
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                let arg8 = binary.builder.build_pointer_cast(
                    arg.into_pointer_value(),
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "arg8",
                );

                // first clear/set the upper bits
                if n < 256 {
                    if let ast::Type::Int(_) = ty {
                        let signdest = unsafe {
                            binary.builder.build_gep(
                                arg8,
                                &[binary
                                    .context
                                    .i32_type()
                                    .const_int((n as u64 / 8) - 1, false)],
                                "signbyte",
                            )
                        };

                        let negative = binary.builder.build_int_compare(
                            IntPredicate::SLT,
                            binary
                                .builder
                                .build_load(signdest, "signbyte")
                                .into_int_value(),
                            binary.context.i8_type().const_zero(),
                            "neg",
                        );

                        let signval = binary
                            .builder
                            .build_select(
                                negative,
                                binary.context.i64_type().const_int(std::u64::MAX, true),
                                binary.context.i64_type().const_zero(),
                                "val",
                            )
                            .into_int_value();

                        binary.builder.build_call(
                            binary.module.get_function("__memset8").unwrap(),
                            &[
                                dest8.into(),
                                signval.into(),
                                binary.context.i32_type().const_int(4, false).into(),
                            ],
                            "",
                        );
                    }
                }

                binary.builder.build_call(
                    binary.module.get_function("__leNtobe32").unwrap(),
                    &[
                        arg8.into(),
                        dest8.into(),
                        binary
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
                    ast::Type::Contract(_) | ast::Type::Address(_) => ns.address_length as u16 * 8,
                    ast::Type::Uint(b) => *b,
                    ast::Type::Int(b) => *b,
                    _ => unreachable!(),
                };

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "dest8",
                );

                // first clear/set the upper bits
                if n < 256 {
                    if let ast::Type::Int(_) = ty {
                        let negative = binary.builder.build_int_compare(
                            IntPredicate::SLT,
                            arg.into_int_value(),
                            arg.get_type().into_int_type().const_zero(),
                            "neg",
                        );

                        let signval = binary
                            .builder
                            .build_select(
                                negative,
                                binary.context.i64_type().const_int(std::u64::MAX, true),
                                binary.context.i64_type().const_zero(),
                                "val",
                            )
                            .into_int_value();

                        binary.builder.build_call(
                            binary.module.get_function("__memset8").unwrap(),
                            &[
                                dest8.into(),
                                signval.into(),
                                binary.context.i32_type().const_int(4, false).into(),
                            ],
                            "",
                        );
                    }
                }

                let temp = binary.build_alloca(
                    function,
                    arg.into_int_value().get_type(),
                    &format!("uint{}", n),
                );

                binary.builder.build_store(temp, arg.into_int_value());

                binary.builder.build_call(
                    binary.module.get_function("__leNtobe32").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                temp,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        dest8.into(),
                        binary
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
                    binary.builder.build_load(arg.into_pointer_value(), "")
                } else {
                    arg
                };

                let dest8 = binary.builder.build_pointer_cast(
                    dest,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                binary.builder.build_store(dest8, arg);
            }
            ast::Type::Bytes(n) => {
                let val = if load {
                    arg.into_pointer_value()
                } else {
                    let temp = binary.build_alloca(
                        function,
                        arg.into_int_value().get_type(),
                        &format!("bytes{}", n),
                    );

                    binary.builder.build_store(temp, arg.into_int_value());

                    temp
                };

                binary.builder.build_call(
                    binary.module.get_function("__leNtobeN").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                val,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                dest,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "dest",
                            )
                            .into(),
                        binary.context.i32_type().const_int(*n as u64, false).into(),
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
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &ast::Type,
        to: Option<PointerValue<'a>>,
        offset: &mut IntValue<'a>,
        data: PointerValue<'a>,
        length: IntValue,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        // TODO: investigate whether we can use build_int_nuw_add() and avoid 64 bit conversions
        let new_offset = binary.builder.build_int_add(
            *offset,
            binary.context.i64_type().const_int(32, false),
            "next_offset",
        );

        self.check_overrun(binary, function, new_offset, length);

        let data = unsafe { binary.builder.build_gep(data, &[*offset], "") };

        *offset = new_offset;

        let ty = if let ast::Type::Enum(n) = ty {
            &ns.enums[*n].ty
        } else {
            ty
        };

        match &ty {
            ast::Type::Bool => {
                // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                // which is unneeded (hopefully)
                // cast to 64 bit pointer
                let bool_ptr = binary.builder.build_pointer_cast(
                    data,
                    binary.context.i64_type().ptr_type(AddressSpace::Generic),
                    "",
                );

                let bool_ptr = unsafe {
                    binary.builder.build_gep(
                        bool_ptr,
                        &[binary.context.i32_type().const_int(3, false)],
                        "bool_ptr",
                    )
                };

                let val = binary.builder.build_int_compare(
                    IntPredicate::NE,
                    binary
                        .builder
                        .build_load(bool_ptr, "abi_bool")
                        .into_int_value(),
                    binary.context.i64_type().const_zero(),
                    "bool",
                );
                if let Some(p) = to {
                    binary.builder.build_store(p, val);
                }
                val.into()
            }
            ast::Type::Uint(8) | ast::Type::Int(8) => {
                let int8_ptr = unsafe {
                    binary.builder.build_gep(
                        data,
                        &[binary.context.i32_type().const_int(31, false)],
                        "bool_ptr",
                    )
                };

                let val = binary.builder.build_load(int8_ptr, "abi_int8");

                if let Some(p) = to {
                    binary.builder.build_store(p, val);
                }

                val
            }
            ast::Type::Address(_) | ast::Type::Contract(_) => {
                let int_type = binary
                    .context
                    .custom_width_int_type(ns.address_length as u32 * 8);
                let type_size = int_type.size_of();

                let store =
                    to.unwrap_or_else(|| binary.build_alloca(function, int_type, "address"));

                binary.builder.build_call(
                    binary.module.get_function("__be32toleN").unwrap(),
                    &[
                        data.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                store,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .builder
                            .build_int_truncate(type_size, binary.context.i32_type(), "size")
                            .into(),
                    ],
                    "",
                );

                if to.is_none() {
                    binary.builder.build_load(store, "address")
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
                    binary.builder.build_gep(
                        data,
                        &[binary
                            .context
                            .i32_type()
                            .const_int(32 - (bits as u64 / 8), false)],
                        "uint8_ptr",
                    )
                };

                let val = binary.builder.build_load(
                    binary.builder.build_pointer_cast(
                        int8_ptr,
                        binary
                            .context
                            .custom_width_int_type(bits as u32)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    &format!("be{}", *n),
                );

                // now convert to le
                let bswap = binary.llvm_bswap(bits as u32);

                let mut val = binary
                    .builder
                    .build_call(bswap, &[val], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                if bits > *n {
                    val = binary.builder.build_int_truncate(
                        val,
                        binary.context.custom_width_int_type(*n as u32),
                        "",
                    );
                }

                if let Some(p) = to {
                    binary.builder.build_store(p, val);
                }

                val.into()
            }
            ast::Type::Uint(n) | ast::Type::Int(n) => {
                let int_type = binary.context.custom_width_int_type(*n as u32);
                let type_size = int_type.size_of();

                let store = to.unwrap_or_else(|| binary.build_alloca(function, int_type, "stack"));

                binary.builder.build_call(
                    binary.module.get_function("__be32toleN").unwrap(),
                    &[
                        data.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                store,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .builder
                            .build_int_truncate(type_size, binary.context.i32_type(), "size")
                            .into(),
                    ],
                    "",
                );

                if to.is_none() {
                    binary.builder.build_load(store, &format!("abi_int{}", n))
                } else {
                    store.into()
                }
            }
            ast::Type::Bytes(1) => {
                let val = binary.builder.build_load(data, "bytes1");

                if let Some(p) = to {
                    binary.builder.build_store(p, val);
                }
                val
            }
            ast::Type::Bytes(b) => {
                let int_type = binary.context.custom_width_int_type(*b as u32 * 8);

                let store = to.unwrap_or_else(|| binary.build_alloca(function, int_type, "stack"));

                binary.builder.build_call(
                    binary.module.get_function("__beNtoleN").unwrap(),
                    &[
                        data.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                store,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary.context.i32_type().const_int(*b as u64, false).into(),
                    ],
                    "",
                );

                if to.is_none() {
                    binary.builder.build_load(store, &format!("bytes{}", *b))
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
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        ty: &ast::Type,
        to: Option<PointerValue<'b>>,
        offset: &mut IntValue<'b>,
        base_offset: IntValue<'b>,
        data: PointerValue<'b>,
        length: IntValue,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        match &ty {
            ast::Type::Array(elem_ty, dim) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                let size = llvm_ty
                    .size_of()
                    .unwrap()
                    .const_cast(binary.context.i32_type(), false);

                let dest;

                if let Some(d) = &dim[0] {
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

                    // if the struct has dynamic fields, read offset from dynamic section and
                    // read fields from there
                    let mut dataoffset = if ty.is_dynamic(ns) {
                        let dataoffset = binary.builder.build_int_z_extend(
                            self.decode_primitive(
                                binary,
                                function,
                                &ast::Type::Uint(32),
                                None,
                                offset,
                                data,
                                length,
                                ns,
                            )
                            .into_int_value(),
                            binary.context.i64_type(),
                            "rel_struct_offset",
                        );

                        binary
                            .builder
                            .build_int_add(dataoffset, base_offset, "abs_struct_offset")
                    } else {
                        *offset
                    };

                    // In dynamic struct sections, the offsets are relative to the start of the section.
                    // Ethereum ABI encoding is just insane.
                    let base_offset = if ty.is_dynamic(ns) {
                        dataoffset
                    } else {
                        base_offset
                    };

                    binary.emit_loop_cond_first_with_int(
                        function,
                        binary.context.i64_type().const_zero(),
                        binary
                            .context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        &mut dataoffset,
                        |index: IntValue<'b>, offset: &mut IntValue<'b>| {
                            let elem = unsafe {
                                binary.builder.build_gep(
                                    dest,
                                    &[binary.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            self.decode_ty(
                                binary,
                                function,
                                &elem_ty,
                                Some(elem),
                                offset,
                                base_offset,
                                data,
                                length,
                                ns,
                            );
                        },
                    );

                    // if the struct is not dynamic, we have read the fields from fixed section so update
                    if !ty.is_dynamic(ns) {
                        *offset = dataoffset;
                    }
                } else {
                    let mut dataoffset = binary.builder.build_int_add(
                        binary.builder.build_int_z_extend(
                            self.decode_primitive(
                                binary,
                                function,
                                &ast::Type::Uint(32),
                                None,
                                offset,
                                data,
                                length,
                                ns,
                            )
                            .into_int_value(),
                            binary.context.i64_type(),
                            "data_offset",
                        ),
                        base_offset,
                        "array_data_offset",
                    );

                    let array_len = self
                        .decode_primitive(
                            binary,
                            function,
                            &ast::Type::Uint(32),
                            None,
                            &mut dataoffset,
                            data,
                            length,
                            ns,
                        )
                        .into_int_value();

                    // in dynamic arrays, offsets are counted from after the array length
                    let base_offset = dataoffset;

                    let llvm_elem_ty = binary.llvm_var(&elem_ty.deref_any(), ns);
                    let elem_size = llvm_elem_ty
                        .size_of()
                        .unwrap()
                        .const_cast(binary.context.i32_type(), false);

                    let init = binary.builder.build_int_to_ptr(
                        binary.context.i32_type().const_all_ones(),
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    dest = binary
                        .builder
                        .build_call(
                            binary.module.get_function("vector_new").unwrap(),
                            &[
                                binary
                                    .builder
                                    .build_int_truncate(
                                        array_len,
                                        binary.context.i32_type(),
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

                    binary.emit_loop_cond_first_with_int(
                        function,
                        binary.context.i32_type().const_zero(),
                        array_len,
                        &mut dataoffset,
                        |elem_no: IntValue<'b>, offset: &mut IntValue<'b>| {
                            let index = binary.builder.build_int_mul(elem_no, elem_size, "");

                            let element_start = unsafe {
                                binary.builder.build_gep(
                                    dest,
                                    &[
                                        binary.context.i32_type().const_zero(),
                                        binary.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let elem = binary.builder.build_pointer_cast(
                                element_start,
                                llvm_elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            self.decode_ty(
                                binary,
                                function,
                                &elem_ty,
                                Some(elem),
                                offset,
                                base_offset,
                                data,
                                length,
                                ns,
                            );
                        },
                    );
                }

                if let Some(to) = to {
                    binary.builder.build_store(to, dest);
                }

                dest.into()
            }
            ast::Type::Struct(n) => {
                let llvm_ty = binary.llvm_type(ty.deref_any(), ns);

                let size = llvm_ty
                    .size_of()
                    .unwrap()
                    .const_cast(binary.context.i32_type(), false);

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

                let struct_pointer = binary.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    &ns.structs[*n].name,
                );

                // if the struct has dynamic fields, read offset from dynamic section and
                // read fields from there
                let mut dataoffset = if ty.is_dynamic(ns) {
                    let dataoffset = binary.builder.build_int_z_extend(
                        self.decode_primitive(
                            binary,
                            function,
                            &ast::Type::Uint(32),
                            None,
                            offset,
                            data,
                            length,
                            ns,
                        )
                        .into_int_value(),
                        binary.context.i64_type(),
                        "rel_struct_offset",
                    );

                    binary
                        .builder
                        .build_int_add(dataoffset, base_offset, "abs_struct_offset")
                } else {
                    *offset
                };

                // In dynamic struct sections, the offsets are relative to the start of the section.
                // Ethereum ABI encoding is just insane.
                let base_offset = if ty.is_dynamic(ns) {
                    dataoffset
                } else {
                    base_offset
                };

                for (i, field) in ns.structs[*n].fields.iter().enumerate() {
                    let elem = unsafe {
                        binary.builder.build_gep(
                            struct_pointer,
                            &[
                                binary.context.i32_type().const_zero(),
                                binary.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.decode_ty(
                        binary,
                        function,
                        &field.ty,
                        Some(elem),
                        &mut dataoffset,
                        base_offset,
                        data,
                        length,
                        ns,
                    );
                }

                // if the struct is not dynamic, we have read the fields from fixed section so update
                if !ty.is_dynamic(ns) {
                    *offset = dataoffset;
                }

                if let Some(to) = to {
                    binary.builder.build_store(to, struct_pointer);
                }

                struct_pointer.into()
            }
            ast::Type::Ref(ty) => self.decode_ty(
                binary,
                function,
                ty,
                to,
                offset,
                base_offset,
                data,
                length,
                ns,
            ),
            ast::Type::String | ast::Type::DynamicBytes => {
                // we read the offset and the length as 32 bits. Since we are in 32 bits wasm,
                // we cannot deal with more than 4GB of abi encoded data.
                let mut dataoffset = binary.builder.build_int_z_extend(
                    self.decode_primitive(
                        binary,
                        function,
                        &ast::Type::Uint(32),
                        None,
                        offset,
                        data,
                        length,
                        ns,
                    )
                    .into_int_value(),
                    binary.context.i64_type(),
                    "data_offset",
                );

                dataoffset = binary
                    .builder
                    .build_int_add(dataoffset, base_offset, "data_offset");

                let string_len = binary.builder.build_int_z_extend(
                    self.decode_primitive(
                        binary,
                        function,
                        &ast::Type::Uint(32),
                        None,
                        &mut dataoffset,
                        data,
                        length,
                        ns,
                    )
                    .into_int_value(),
                    binary.context.i64_type(),
                    "string_len",
                );

                // Special case string_len == 0 => null pointer?
                let string_end = binary
                    .builder
                    .build_int_add(dataoffset, string_len, "stringend");

                self.check_overrun(binary, function, string_end, length);

                let string_start = unsafe {
                    binary
                        .builder
                        .build_gep(data, &[dataoffset], "string_start")
                };

                let v = binary
                    .builder
                    .build_call(
                        binary.module.get_function("vector_new").unwrap(),
                        &[
                            binary
                                .builder
                                .build_int_truncate(
                                    string_len,
                                    binary.context.i32_type(),
                                    "string_len",
                                )
                                .into(),
                            binary.context.i32_type().const_int(1, false).into(),
                            string_start.into(),
                        ],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let v = binary.builder.build_pointer_cast(
                    v.into_pointer_value(),
                    binary
                        .module
                        .get_struct_type("struct.vector")
                        .unwrap()
                        .ptr_type(AddressSpace::Generic),
                    "string",
                );

                if let Some(to) = to {
                    binary.builder.build_store(to, v);
                }

                v.into()
            }
            _ => self.decode_primitive(binary, function, ty, to, offset, data, length, ns),
        }
    }

    /// Check that data has not overrun end
    fn check_overrun(
        &self,
        binary: &Binary,
        function: FunctionValue,
        offset: IntValue,
        end: IntValue,
    ) {
        let in_bounds = binary
            .builder
            .build_int_compare(IntPredicate::ULE, offset, end, "");

        let success_block = binary.context.append_basic_block(function, "success");
        let bail_block = binary.context.append_basic_block(function, "bail");
        binary
            .builder
            .build_conditional_branch(in_bounds, success_block, bail_block);

        binary.builder.position_at_end(bail_block);

        binary
            .builder
            .build_return(Some(&binary.return_values[&ReturnCode::AbiEncodingInvalid]));

        binary.builder.position_at_end(success_block);
    }

    /// abi decode the encoded data into the BasicValueEnums
    pub fn decode<'a>(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        args: &mut Vec<BasicValueEnum<'a>>,
        data: PointerValue<'a>,
        datalength: IntValue<'a>,
        spec: &[ast::Parameter],
        ns: &ast::Namespace,
    ) {
        let data = binary.builder.build_pointer_cast(
            data,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "data",
        );

        let mut offset = binary.context.i64_type().const_zero();

        let data_length = if datalength.get_type().get_bit_width() != 64 {
            binary
                .builder
                .build_int_z_extend(datalength, binary.context.i64_type(), "data_length")
        } else {
            datalength
        };

        for arg in spec {
            args.push(self.decode_ty(
                binary,
                function,
                &arg.ty,
                None,
                &mut offset,
                binary.context.i64_type().const_zero(),
                data,
                data_length,
                ns,
            ));
        }
    }
}

/// ABI encode into a vector for abi.encode* style builtin functions
pub fn encode_to_vector<'b>(
    binary: &Binary<'b>,
    function: FunctionValue<'b>,
    packed: &[BasicValueEnum<'b>],
    args: &[BasicValueEnum<'b>],
    tys: &[ast::Type],
    bswap: bool,
    ns: &ast::Namespace,
) -> PointerValue<'b> {
    let encoder = EncoderBuilder::new(binary, function, false, packed, args, tys, bswap, ns);

    let length = encoder.encoded_length();

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

    let data = binary.builder.build_pointer_cast(
        data,
        binary.context.i8_type().ptr_type(AddressSpace::Generic),
        "",
    );

    encoder.finish(binary, function, data, ns);

    v
}
