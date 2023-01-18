pub(super) mod target;

use crate::emit::functions::emit_functions;
use crate::{codegen::Options, emit::Binary, sema::ast};
use inkwell::context::Context;
use inkwell::module::Module;

pub struct SorobanTarget;

impl SorobanTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: &'a Options,
    ) -> Binary<'a> {
        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename,
            opt,
            std_lib,
            None,
        );
        emit_functions(&mut SorobanTarget, &mut binary, contract, ns);
        binary
    }
}
