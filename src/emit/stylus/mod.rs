// SPDX-License-Identifier: Apache-2.0

use crate::codegen::dispatch::polkadot::DispatchType;
use crate::codegen::Options;
use crate::emit::functions::emit_functions;
use crate::emit::Binary;
use crate::emit_context;
use crate::sema::ast::{Contract, Namespace};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;

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

        let mut target = StylusTarget;

        target.declare_externals(&bin);

        emit_functions(&mut target, &mut bin, contract);

        target.emit_dispatch(&mut bin);

        bin.internalize(&[
            "log_txt",
            "msg_reentrant",
            "msg_value",
            "pay_for_memory_grow",
            "read_args",
            "storage_flush_cache",
            "storage_cache_bytes32",
            "storage_load_bytes32",
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
        let i32_val = ctx.i32_type().into();
        let u8_ptr = ctx.i8_type().ptr_type(AddressSpace::default()).into();

        macro_rules! external {
            ($name:literal, $fn_type:ident $(,)? $( $args:expr ),*) => {
                bin.module.add_function(
                    $name,
                    ctx.$fn_type().fn_type(&[$($args),*], false),
                    Some(Linkage::External),
                );
            };
        }

        external!("log_txt", void_type, u8_ptr, i32_val);
        external!("msg_reentrant", i32_type);
        external!("msg_value", void_type, i32_val);
        external!("pay_for_memory_grow", void_type, i32_val);
        external!("read_args", void_type, u8_ptr);
        external!("storage_cache_bytes32", void_type, u8_ptr, u8_ptr);
        external!("storage_flush_cache", void_type, i32_val);
        external!("storage_load_bytes32", void_type, u8_ptr, u8_ptr);
        external!("write_result", void_type, u8_ptr, i32_val);
    }

    fn emit_dispatch(&mut self, bin: &mut Binary) {
        let ty = bin
            .context
            .i32_type()
            .fn_type(&[bin.context.i32_type().into()], false);
        let func = bin.module.add_function("user_entrypoint", ty, None);
        let (args, args_len) = self.public_function_prelude(bin, func);
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

        bin.builder.build_unreachable().unwrap();
    }
}
