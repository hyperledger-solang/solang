// SPDX-License-Identifier: Apache-2.0

use crate::codegen::Options;
use crate::sema::ast::{Contract, Namespace};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;

use crate::emit::functions::{emit_functions, emit_initializer};
use crate::emit::{Binary, TargetRuntime};

mod storage;
pub(super) mod target;

// When using the seal api, we use our own scratch buffer.
const SCRATCH_SIZE: u32 = 32 * 1024;

#[macro_export]
macro_rules! emit_context {
    ($binary:expr) => {
        #[allow(unused_macros)]
        macro_rules! byte_ptr {
            () => {
                $binary.context.i8_type().ptr_type(AddressSpace::default())
            };
        }

        #[allow(unused_macros)]
        macro_rules! i32_const {
            ($val:expr) => {
                $binary.context.i32_type().const_int($val, false)
            };
        }

        #[allow(unused_macros)]
        macro_rules! i32_zero {
            () => {
                $binary.context.i32_type().const_zero()
            };
        }

        #[allow(unused_macros)]
        macro_rules! call {
            ($name:expr, $args:expr) => {
                $binary
                    .builder
                    .build_call($binary.module.get_function($name).unwrap(), $args, "")
            };
            ($name:expr, $args:expr, $call_name:literal) => {
                $binary.builder.build_call(
                    $binary.module.get_function($name).unwrap(),
                    $args,
                    $call_name,
                )
            };
        }

        #[allow(unused_macros)]
        macro_rules! seal_get_storage {
            ($key_ptr:expr, $key_len:expr, $value_ptr:expr, $value_len:expr) => {
                call!(
                    "seal_get_storage",
                    &[$key_ptr, $key_len, $value_ptr, $value_len]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
            };
        }

        #[allow(unused_macros)]
        macro_rules! seal_set_storage {
            ($key_ptr:expr, $key_len:expr, $value_ptr:expr, $value_len:expr) => {
                call!(
                    "seal_set_storage",
                    &[$key_ptr, $key_len, $value_ptr, $value_len]
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
            };
        }

        #[allow(unused_macros)]
        macro_rules! scratch_buf {
            () => {
                (
                    $binary.scratch.unwrap().as_pointer_value(),
                    $binary.scratch_len.unwrap().as_pointer_value(),
                )
            };
        }
    };
}

pub struct SubstrateTarget;

impl SubstrateTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a Contract,
        ns: &'a Namespace,
        opt: &'a Options,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename.as_str(),
            opt,
            std_lib,
            None,
        );

        binary.set_early_value_aborts(contract, ns);

        let scratch_len = binary.module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "scratch_len",
        );
        scratch_len.set_linkage(Linkage::Internal);
        scratch_len.set_initializer(&context.i32_type().get_undef());

        binary.scratch_len = Some(scratch_len);

        let scratch = binary.module.add_global(
            context.i8_type().array_type(SCRATCH_SIZE),
            Some(AddressSpace::default()),
            "scratch",
        );
        scratch.set_linkage(Linkage::Internal);
        scratch.set_initializer(&context.i8_type().array_type(SCRATCH_SIZE).get_undef());
        binary.scratch = Some(scratch);

        let mut target = SubstrateTarget;

        target.declare_externals(&binary);

        emit_functions(&mut target, &mut binary, contract, ns);

        let storage_initializer = emit_initializer(&mut target, &mut binary, contract, ns);
        target.emit_dispatch(Some(storage_initializer), &mut binary, ns);
        target.emit_dispatch(None, &mut binary, ns);

        binary.internalize(&[
            "deploy",
            "call",
            "call_chain_extension",
            "seal_input",
            "seal_set_storage",
            "seal_get_storage",
            "seal_clear_storage",
            "seal_hash_keccak_256",
            "seal_hash_sha2_256",
            "seal_hash_blake2_128",
            "seal_hash_blake2_256",
            "seal_return",
            "seal_debug_message",
            "seal_instantiate",
            "seal_call",
            "seal_value_transferred",
            "seal_minimum_balance",
            "seal_weight_to_fee",
            "instantiation_nonce",
            "seal_address",
            "seal_balance",
            "seal_block_number",
            "seal_now",
            "seal_gas_price",
            "seal_gas_left",
            "seal_caller",
            "seal_terminate",
            "seal_deposit_event",
            "seal_transfer",
        ]);

        binary
    }

    fn public_function_prelude<'a>(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "");

        let scratch_buf = binary.scratch.unwrap().as_pointer_value();
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        // copy arguments from input buffer
        binary.builder.build_store(
            scratch_len,
            binary
                .context
                .i32_type()
                .const_int(SCRATCH_SIZE as u64, false),
        );

        binary.builder.build_call(
            binary.module.get_function("seal_input").unwrap(),
            &[scratch_buf.into(), scratch_len.into()],
            "",
        );

        let args_length =
            binary
                .builder
                .build_load(binary.context.i32_type(), scratch_len, "input_len");

        // store the length in case someone wants it via msg.data
        binary.builder.build_store(
            binary.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        (scratch_buf, args_length.into_int_value())
    }

    fn declare_externals(&self, binary: &Binary) {
        let ctx = binary.context;
        let u8_ptr = ctx.i8_type().ptr_type(AddressSpace::default()).into();
        let u32_val = ctx.i32_type().into();
        let u32_ptr = ctx.i32_type().ptr_type(AddressSpace::default()).into();
        let u64_val = ctx.i64_type().into();

        macro_rules! external {
            ($name:literal, $fn_type:ident, $( $args:expr ),*) => {
                binary.module.add_function(
                    $name,
                    ctx.$fn_type().fn_type(&[$($args),*], false),
                    Some(Linkage::External),
                );
            };
        }

        external!(
            "call_chain_extension",
            i32_type,
            u32_val,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr
        );
        external!("seal_input", void_type, u8_ptr, u32_ptr);
        external!("seal_hash_keccak_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_sha2_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_blake2_128", void_type, u8_ptr, u32_val, u8_ptr);
        external!("seal_hash_blake2_256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("instantiation_nonce", i64_type,);
        external!(
            "seal_set_storage",
            i32_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_val
        );
        external!("seal_debug_message", i32_type, u8_ptr, u32_val);
        external!("seal_clear_storage", i32_type, u8_ptr, u32_val);
        external!(
            "seal_get_storage",
            i32_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr
        );
        external!("seal_return", void_type, u32_val, u8_ptr, u32_val);
        external!(
            "seal_instantiate",
            i32_type,
            u8_ptr,
            u64_val,
            u8_ptr,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr,
            u8_ptr,
            u32_ptr,
            u8_ptr,
            u32_val
        );
        external!(
            "seal_call",
            i32_type,
            u32_val,
            u8_ptr,
            u64_val,
            u8_ptr,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_ptr
        );
        external!("seal_transfer", i32_type, u8_ptr, u32_val, u8_ptr, u32_val);
        external!("seal_value_transferred", void_type, u8_ptr, u32_ptr);
        external!("seal_address", void_type, u8_ptr, u32_ptr);
        external!("seal_balance", void_type, u8_ptr, u32_ptr);
        external!("seal_minimum_balance", void_type, u8_ptr, u32_ptr);
        external!("seal_block_number", void_type, u8_ptr, u32_ptr);
        external!("seal_now", void_type, u8_ptr, u32_ptr);
        external!("seal_weight_to_fee", void_type, u64_val, u8_ptr, u32_ptr);
        external!("seal_gas_left", void_type, u8_ptr, u32_ptr);
        external!("seal_caller", void_type, u8_ptr, u32_ptr);
        external!("seal_terminate", void_type, u8_ptr);
        external!(
            "seal_deposit_event",
            void_type,
            u8_ptr,
            u32_val,
            u8_ptr,
            u32_val
        );
    }

    /// Emits the "deploy" function if `init` is `Some`, otherwise emits the "call" function.
    fn emit_dispatch(&mut self, init: Option<FunctionValue>, bin: &mut Binary, ns: &Namespace) {
        let ty = bin.context.void_type().fn_type(&[], false);
        let name = if init.is_some() { "deploy" } else { "call" };
        let func = bin.module.add_function(name, ty, None);
        let (input, input_length) = self.public_function_prelude(bin, func);
        if let Some(initializer) = init {
            bin.builder.build_call(initializer, &[], "");
        }
        let func = bin.module.get_function("substrate_dispatch").unwrap();
        let args = vec![
            BasicMetadataValueEnum::PointerValue(input),
            BasicMetadataValueEnum::IntValue(input_length),
            BasicMetadataValueEnum::IntValue(self.value_transferred(bin, ns)),
            BasicMetadataValueEnum::PointerValue(bin.selector.as_pointer_value()),
        ];
        bin.builder.build_call(func, &args, "substrate_dispatch");
        bin.builder.build_unreachable();
    }
}

/// Print the return code of API calls to the debug buffer.
fn log_return_code(binary: &Binary, api: &'static str, code: IntValue) {
    if !binary.options.log_api_return_codes {
        return;
    }

    emit_context!(binary);

    let fmt = format!("call: {api}=");
    let msg = fmt.as_bytes();
    let delimiter = b",\n";
    let delimiter_length = delimiter.len();
    let length = i32_const!(msg.len() as u64 + 16 + delimiter_length as u64);
    let out_buf =
        binary
            .builder
            .build_array_alloca(binary.context.i8_type(), length, "seal_ret_code_buf");
    let mut out_buf_offset = out_buf;

    let msg_string = binary.emit_global_string(&fmt, msg, true);
    let msg_len = binary.context.i32_type().const_int(msg.len() as u64, false);
    call!(
        "__memcpy",
        &[out_buf_offset.into(), msg_string.into(), msg_len.into()]
    );
    out_buf_offset = unsafe {
        binary
            .builder
            .build_gep(binary.context.i8_type(), out_buf_offset, &[msg_len], "")
    };

    let code = binary
        .builder
        .build_int_z_extend(code, binary.context.i64_type(), "val_64bits");
    out_buf_offset = call!("uint2dec", &[out_buf_offset.into(), code.into()])
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let delimiter_string = binary.emit_global_string("delimiter", delimiter, true);
    let lim_len = binary
        .context
        .i32_type()
        .const_int(delimiter_length as u64, false);
    call!(
        "__memcpy",
        &[
            out_buf_offset.into(),
            delimiter_string.into(),
            lim_len.into()
        ]
    );
    out_buf_offset = unsafe {
        binary
            .builder
            .build_gep(binary.context.i8_type(), out_buf_offset, &[lim_len], "")
    };

    let msg_len = binary.builder.build_int_sub(
        binary
            .builder
            .build_ptr_to_int(out_buf_offset, binary.context.i32_type(), "out_buf_idx"),
        binary
            .builder
            .build_ptr_to_int(out_buf, binary.context.i32_type(), "out_buf_ptr"),
        "msg_len",
    );
    call!("seal_debug_message", &[out_buf.into(), msg_len.into()]);
}
