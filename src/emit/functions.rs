// SPDX-License-Identifier: Apache-2.0

use crate::{
    emit::{binary::Binary, cfg::emit_cfg, TargetRuntime},
    sema::ast::{Contract, Namespace, Type},
};
use inkwell::{module::Linkage, values::FunctionValue};

/// Emit all functions, constructors, fallback and receiver
pub(super) fn emit_functions<'a, T: TargetRuntime<'a>>(
    target: &mut T,
    bin: &mut Binary<'a>,
    contract: &Contract,
    ns: &Namespace,
) {
    let mut defines = Vec::new();

    for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
        if !cfg.is_placeholder() {
            let ftype = bin.function_type(
                &cfg.params
                    .iter()
                    .map(|p| p.ty.clone())
                    .collect::<Vec<Type>>(),
                &cfg.returns
                    .iter()
                    .map(|p| p.ty.clone())
                    .collect::<Vec<Type>>(),
                ns,
            );

            let func_decl = if let Some(func) = bin.module.get_function(&cfg.name) {
                // must not have a body yet
                assert_eq!(func.get_first_basic_block(), None);

                func
            } else {
                bin.module
                    .add_function(&cfg.name, ftype, Some(Linkage::Internal))
            };

            bin.functions.insert(cfg_no, func_decl);

            defines.push((func_decl, cfg));
        }
    }

    for (func_decl, cfg) in defines {
        emit_cfg(target, bin, contract, cfg, func_decl, ns);
    }
}

/// Emit the storage initializers
pub(super) fn emit_initializer<'a, T: TargetRuntime<'a>>(
    target: &mut T,
    bin: &mut Binary<'a>,
    contract: &Contract,
    ns: &Namespace,
) -> FunctionValue<'a> {
    let function_ty = bin.function_type(&[], &[], ns);

    let function = bin.module.add_function(
        &format!("sol::{}::storage_initializers", contract.name),
        function_ty,
        Some(Linkage::Internal),
    );

    let cfg = &contract.cfg[contract.initializer.unwrap()];

    emit_cfg(target, bin, contract, cfg, function, ns);

    function
}
