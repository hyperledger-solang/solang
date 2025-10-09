// SPDX-License-Identifier: Apache-2.0

use crate::codegen::dispatch::polkadot::DispatchType;
use crate::codegen::Options;
use crate::emit::functions::emit_functions;
use crate::emit::Binary;
use crate::emit_context;
use crate::sema::ast::{Contract, Namespace};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;

mod storage;
mod target;

pub struct StylusTarget;

impl StylusTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a Contract,
        ns: &'a Namespace,
        opt: &'a Options,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut bin = Binary::new(
            context,
            ns,
            &contract.id.name,
            filename.as_str(),
            opt,
            std_lib,
            None,
        );

        let args = bin.module.add_global(
            bin.context.ptr_type(AddressSpace::default()),
            Some(AddressSpace::default()),
            "args",
        );
        args.set_linkage(Linkage::Internal);
        args.set_initializer(&bin.context.ptr_type(AddressSpace::default()).get_undef());

        bin.args = Some(args);

        let args_len = bin.module.add_global(
            bin.context.i32_type(),
            Some(AddressSpace::default()),
            "args_len",
        );
        args_len.set_linkage(Linkage::Internal);
        args_len.set_initializer(&bin.context.i32_type().get_undef());

        bin.args_len = Some(args_len);

        let return_code = bin.module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "return_code",
        );
        return_code.set_linkage(Linkage::Internal);
        // smoelius: Stylus uses 0 for success and 1 for failure:
        // https://github.com/OffchainLabs/stylus-sdk-rs/blob/8940922c2454b4edb8a560b7c24caa522d352364/stylus-proc/src/macros/entrypoint.rs#L176-L179
        return_code.set_initializer(&context.i32_type().const_zero());

        bin.return_code = Some(return_code);

        let return_data_len = bin.module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "return_data_len",
        );
        return_data_len.set_linkage(Linkage::Internal);
        return_data_len.set_initializer(&context.i32_type().get_undef());

        bin.return_data_len = Some(return_data_len);

        let mut target = StylusTarget;

        target.declare_externals(&bin);

        emit_functions(&mut target, &mut bin, contract);

        target.emit_dispatch(&mut bin);

        bin.internalize(&[
            "account_balance",
            "account_code",
            "account_code_size",
            "account_codehash",
            "block_basefee",
            "block_coinbase",
            "block_gas_limit",
            "block_number",
            "block_timestamp",
            "chainid",
            "call_contract",
            "contract_address",
            "create1",
            "create2",
            "delegate_call_contract",
            "emit_log",
            "evm_gas_left",
            "math_div",
            "math_mod",
            "math_pow",
            "math_add_mod",
            "math_mul_mod",
            "log_txt",
            "msg_reentrant",
            "msg_sender",
            "msg_value",
            "native_keccak256",
            "pay_for_memory_grow",
            "read_args",
            "read_return_data",
            "return_data_size",
            "static_call_contract",
            "storage_flush_cache",
            "storage_cache_bytes32",
            "storage_load_bytes32",
            "transient_store_bytes32",
            "transient_load_bytes32",
            "tx_gas_price",
            "tx_origin",
            "write_result",
        ]);

        bin
    }

    fn public_function_prelude<'a>(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        emit_context!(bin);

        let entry = bin.context.append_basic_block(function, "entry");

        bin.builder.position_at_end(entry);

        // init our heap
        bin.builder
            .build_call(
                bin.module.get_function("__init_heap").unwrap(),
                &[],
                "__init_heap",
            )
            .unwrap();

        let args_len = function.get_nth_param(0).unwrap();

        let args = call!("__malloc", &[args_len.into()], "__malloc")
            .try_as_basic_value()
            .left()
            .unwrap();

        call!("read_args", &[args.into()], "read_args");

        (args.into_pointer_value(), args_len.into_int_value())
    }

    fn declare_externals(&self, bin: &Binary) {
        let ctx = bin.context;
        let u8_ptr = ctx.ptr_type(AddressSpace::default()).into();
        let u16_val = ctx.i16_type().into();
        let u32_ptr = ctx.ptr_type(AddressSpace::default()).into();
        let u32_val = ctx.i32_type().into();
        let u64_val = ctx.i64_type().into();

        macro_rules! external {
            ($name:literal, $fn_type:ident $(,)? $( $args:expr ),*) => {
                bin.module.add_function(
                    $name,
                    ctx.$fn_type().fn_type(&[$($args),*], false),
                    Some(Linkage::External),
                );
            };
        }

        external!("account_balance", void_type, u8_ptr, u8_ptr);
        external!("account_code", i32_type, u8_ptr, u32_val, u32_val, u8_ptr);
        external!("account_code_size", i32_type, u8_ptr);
        external!("account_codehash", void_type, u8_ptr, u8_ptr);
        external!("block_basefee", void_type, u8_ptr);
        external!("block_coinbase", void_type, u8_ptr);
        external!("block_gas_limit", i64_type);
        external!("block_number", i64_type);
        external!("block_timestamp", i64_type);
        external!(
            "call_contract",
            i8_type,
            u8_ptr,
            u8_ptr,
            u32_val,
            u8_ptr,
            u64_val,
            u32_ptr
        );
        external!("chainid", i64_type);
        external!("contract_address", void_type, u8_ptr);
        external!("create1", void_type, u8_ptr, u32_val, u8_ptr, u8_ptr, u32_ptr);
        external!("create2", void_type, u8_ptr, u32_val, u8_ptr, u8_ptr, u8_ptr, u32_ptr);
        external!(
            "delegate_call_contract",
            i8_type,
            u8_ptr,
            u8_ptr,
            u32_val,
            u64_val,
            u8_ptr
        );
        external!("emit_log", void_type, u8_ptr, u32_val, u32_val);
        external!("evm_gas_left", i64_type);
        external!("log_txt", void_type, u8_ptr, u32_val);
        external!("msg_reentrant", i32_type);
        external!("math_add_mod", void_type, u8_ptr, u8_ptr, u8_ptr);
        external!("math_div", void_type, u8_ptr, u8_ptr);
        external!("math_mod", void_type, u8_ptr, u8_ptr);
        external!("math_mul_mod", void_type, u8_ptr, u8_ptr, u8_ptr);
        external!("math_pow", void_type, u8_ptr, u8_ptr);
        external!("msg_sender", void_type, u8_ptr);
        external!("msg_value", void_type, u8_ptr);
        external!("native_keccak256", void_type, u8_ptr, u32_val, u8_ptr);
        external!("pay_for_memory_grow", void_type, u16_val);
        external!("read_args", void_type, u8_ptr);
        external!("read_return_data", i32_type, u8_ptr, u32_val, u32_val);
        external!("return_data_size", i32_type);
        external!(
            "static_call_contract",
            i8_type,
            u8_ptr,
            u8_ptr,
            u32_val,
            u64_val,
            u8_ptr
        );
        external!("storage_cache_bytes32", void_type, u8_ptr, u8_ptr);
        external!("storage_flush_cache", void_type, u32_val);
        external!("storage_load_bytes32", void_type, u8_ptr, u8_ptr);
        external!("transient_store_bytes32", void_type, u8_ptr, u8_ptr);
        external!("transient_load_bytes32", void_type, u8_ptr, u8_ptr);
        external!("tx_gas_price", void_type, u8_ptr);
        external!("tx_origin", void_type, u8_ptr);
        external!("write_result", void_type, u8_ptr, u32_val);
    }

    fn emit_dispatch(&mut self, bin: &mut Binary) {
        let ty = bin
            .context
            .i32_type()
            .fn_type(&[bin.context.i32_type().into()], false);
        let func = bin.module.add_function("user_entrypoint", ty, None);
        let (args, args_len) = self.public_function_prelude(bin, func);
        self.assign_args_globals(bin, args, args_len);
        // smoelius: FIXME: zero
        let zero = bin.context.custom_width_int_type(256).const_zero();
        let args = &[
            BasicMetadataValueEnum::PointerValue(args),
            BasicMetadataValueEnum::IntValue(args_len),
            BasicMetadataValueEnum::IntValue(zero),
            BasicMetadataValueEnum::PointerValue(bin.selector.as_pointer_value()),
        ];
        let dispatch_cfg_name = &DispatchType::Call.to_string();
        let cfg = bin.module.get_function(dispatch_cfg_name).unwrap();
        bin.builder
            .build_call(cfg, args, dispatch_cfg_name)
            .unwrap();

        let return_code = bin
            .builder
            .build_load(
                bin.context.i32_type(),
                bin.return_code.unwrap().as_pointer_value(),
                "return_code",
            )
            .unwrap();
        let return_code: &dyn BasicValue = &return_code;
        bin.builder.build_return(Some(return_code)).unwrap();
    }

    fn assign_args_globals<'a>(
        &self,
        bin: &Binary<'a>,
        args: PointerValue<'a>,
        args_len: IntValue<'a>,
    ) {
        bin.builder
            .build_store(bin.args.unwrap().as_pointer_value(), args)
            .unwrap();
        bin.builder
            .build_store(bin.args_len.unwrap().as_pointer_value(), args_len)
            .unwrap();
    }
}
