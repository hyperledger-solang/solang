// src/emit/riscv/mod.rs
use crate::codegen::Options;
use crate::emit::binary::Binary;
use crate::emit::functions::emit_functions;
use crate::emit::riscv::target::RiscvTargetRuntime;
use crate::sema::ast::{Contract, Namespace};
use inkwell::context::Context;
use inkwell::module::Module;

mod target;

pub struct RiscvTarget;

impl RiscvTarget {
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

        // declare external syscall functions.
        target::declare_syscalls(&mut bin);

        // emit all contract functions and dispatch.
        emit_functions(&mut RiscvTargetRuntime, &mut bin, contract);

        // keep only _start externally visible.
        bin.internalize(&["_start"]);

        bin
    }
}
