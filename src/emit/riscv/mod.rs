// SPDX-License-Identifier: Apache-2.0

//! An r55 contract is two ELF images. The deploy image runs on `CREATE`: it
//! runs the storage initializers and the constructor, then returns the
//! runtime image, which becomes the account's code and runs on every `CALL`.
//!
//! ```text
//! contract.bin = 0xFF ++ deploy ELF
//!                          └── .rodata: runtime_code = 0xFF ++ runtime ELF
//! ```

use crate::codegen::targets::riscv::dispatch::DispatchType;
use crate::codegen::{Options, STORAGE_INITIALIZER};
use crate::emit::binary::Binary;
use crate::emit::functions::emit_functions;
use crate::emit::riscv::target::RiscvTargetRuntime;
use crate::emit::Generate;
use crate::sema::ast::{Contract, Namespace};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue};
use inkwell::AddressSpace;

pub mod codegen;
mod target;

/// r55 runs code starting with this byte on its RISC-V emulator.
pub(crate) const R55_CODE_MARKER: u8 = 0xff;

pub struct RiscvTarget;

impl RiscvTarget {
    /// Build the deploy image, keeping the runtime image in
    /// [`Binary::runtime`].
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a Contract,
        ns: &'a Namespace,
        opt: &'a Options,
    ) -> Binary<'a> {
        let runtime = Self::new_binary(context, std_lib, contract, ns, opt, None);
        Self::emit_runtime_entry(&runtime);

        let runtime_elf = runtime
            .code(Generate::Linked)
            .unwrap_or_else(|err| panic!("cannot build the runtime image: {err}"));

        let deploy = Self::new_binary(context, std_lib, contract, ns, opt, Some(Box::new(runtime)));
        Self::emit_deploy_entry(&deploy, &runtime_elf);

        deploy
    }

    fn new_binary<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a Contract,
        ns: &'a Namespace,
        opt: &'a Options,
        runtime: Option<Box<Binary<'a>>>,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut bin = Binary::new(
            context,
            ns,
            &contract.id.name,
            filename.as_str(),
            opt,
            std_lib,
            runtime,
        );

        target::declare_syscalls(&mut bin);

        // The optimizer drops what an image's entry point cannot reach.
        emit_functions(&mut RiscvTargetRuntime, &mut bin, contract);

        bin
    }

    fn emit_runtime_entry(bin: &Binary) {
        let func = Self::begin_entry(bin);

        let dispatch = bin
            .module
            .get_function(&DispatchType::Call.to_string())
            .expect("call dispatcher is emitted for every contract");
        Self::call_with_input(bin, func, dispatch);

        bin.builder.build_unreachable().unwrap();
        bin.internalize(&["solang_dispatch"]);
    }

    fn emit_deploy_entry(bin: &Binary, runtime_elf: &[u8]) {
        let mut runtime_code = vec![R55_CODE_MARKER];
        runtime_code.extend_from_slice(runtime_elf);

        let initializer = bin.context.const_string(&runtime_code, false);
        let global = bin
            .module
            .add_global(initializer.get_type(), None, "runtime_code");
        global.set_initializer(&initializer);
        global.set_constant(true);
        global.set_linkage(Linkage::Internal);

        let func = Self::begin_entry(bin);

        let storage_initializer = bin
            .functions
            .values()
            .find(|f| f.get_name().to_bytes() == STORAGE_INITIALIZER.as_bytes())
            .expect("storage initializer is always present");
        bin.builder
            .build_call(*storage_initializer, &[], "")
            .unwrap();

        let dispatch = bin
            .module
            .get_function(&DispatchType::Deploy.to_string())
            .expect("deploy dispatcher is emitted for every contract");
        Self::call_with_input(bin, func, dispatch);

        let return_fn = bin.module.get_function("__sys_return").unwrap();
        let args: Vec<BasicMetadataValueEnum> = vec![
            global.as_pointer_value().into(),
            bin.context
                .i64_type()
                .const_int(runtime_code.len() as u64, false)
                .into(),
        ];
        bin.builder.build_call(return_fn, &args, "").unwrap();
        bin.builder.build_unreachable().unwrap();

        bin.internalize(&["solang_dispatch"]);
    }

    /// `solang_dispatch(ptr, i32)` is called by `_start` in stdlib/riscv.c.
    fn begin_entry<'a>(bin: &Binary<'a>) -> FunctionValue<'a> {
        let context = bin.context;
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let i32_ty = context.i32_type();

        let func = bin.module.add_function(
            "solang_dispatch",
            context
                .void_type()
                .fn_type(&[ptr_ty.into(), i32_ty.into()], false),
            None,
        );

        let entry = context.append_basic_block(func, "entry");
        bin.builder.position_at_end(entry);

        let init_heap = bin.module.get_function("__init_heap").unwrap();
        bin.builder.build_call(init_heap, &[], "").unwrap();

        func
    }

    fn call_with_input<'a>(
        bin: &Binary<'a>,
        entry: FunctionValue<'a>,
        dispatch: FunctionValue<'a>,
    ) {
        bin.builder
            .build_call(
                dispatch,
                &[
                    entry.get_nth_param(0).unwrap().into(),
                    entry.get_nth_param(1).unwrap().into(),
                ],
                "",
            )
            .unwrap();
    }
}
