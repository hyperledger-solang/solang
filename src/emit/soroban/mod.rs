// SPDX-License-Identifier: Apache-2.0

pub(super) mod target;
use crate::codegen::cfg::ControlFlowGraph;
use crate::emit::cfg::emit_cfg;
use crate::{
    codegen::{cfg::ASTFunction, Options},
    emit::Binary,
    sema::ast,
};
use inkwell::{
    context::Context,
    module::{Linkage, Module},
};
use soroban_sdk::xdr::{
    DepthLimitedWrite, ScEnvMetaEntry, ScSpecEntry, ScSpecFunctionInputV0, ScSpecFunctionV0,
    ScSpecTypeDef, StringM, WriteXdr,
};

const SOROBAN_ENV_INTERFACE_VERSION: u64 = 85899345977;

pub struct SorobanTarget;

impl SorobanTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        opt: &'a Options,
        contract_no: usize,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.id.name,
            &filename,
            opt,
            std_lib,
            None,
        );

        Self::emit_functions_with_spec(contract, &mut binary, ns, context, contract_no);
        Self::emit_env_meta_entries(context, &mut binary);

        binary
    }

    // In Soroban, the public functions specifications is embeded in the contract binary.
    // for each function, emit both the function spec entry and the function body.
    fn emit_functions_with_spec<'a>(
        contract: &'a ast::Contract,
        binary: &mut Binary<'a>,
        ns: &'a ast::Namespace,
        context: &'a Context,
        contract_no: usize,
    ) {
        let mut defines = Vec::new();

        for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
            let ftype = binary.function_type(
                &cfg.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                &cfg.returns.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                ns,
            );

            // For each function, determine the name and the linkage
            // Soroban has no dispatcher, so all externally addressable functions are exported and should be named the same as the original function name in the source code.
            // If there are duplicate function names, then the function name in the source is mangled to include the signature.
            let default_constructor = ns.default_constructor(contract_no);
            let name = {
                if cfg.public {
                    let f = match &cfg.function_no {
                        ASTFunction::SolidityFunction(no) | ASTFunction::YulFunction(no) => {
                            &ns.functions[*no]
                        }
                        _ => &default_constructor,
                    };

                    if f.mangled_name_contracts.contains(&contract_no) {
                        &f.mangled_name
                    } else {
                        &f.id.name
                    }
                } else {
                    &cfg.name
                }
            };

            Self::emit_function_spec_entry(context, cfg, name.clone(), binary);

            let linkage = if cfg.public {
                Linkage::External
            } else {
                Linkage::Internal
            };

            let func_decl = if let Some(func) = binary.module.get_function(name) {
                // must not have a body yet
                assert_eq!(func.get_first_basic_block(), None);

                func
            } else {
                binary.module.add_function(name, ftype, Some(linkage))
            };

            binary.functions.insert(cfg_no, func_decl);

            defines.push((func_decl, cfg));
        }

        for (func_decl, cfg) in defines {
            emit_cfg(&mut SorobanTarget, binary, contract, cfg, func_decl, ns);
        }
    }

    fn emit_env_meta_entries<'a>(context: &'a Context, binary: &mut Binary<'a>) {
        let mut meta = DepthLimitedWrite::new(Vec::new(), 10);
        ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(SOROBAN_ENV_INTERFACE_VERSION)
            .write_xdr(&mut meta)
            .expect("writing env meta interface version to xdr");
        Self::add_custom_section(context, &binary.module, "contractenvmetav0", meta.inner);
    }

    fn emit_function_spec_entry<'a>(
        context: &'a Context,
        cfg: &'a ControlFlowGraph,
        name: String,
        binary: &mut Binary<'a>,
    ) {
        if cfg.public && !cfg.is_placeholder() {
            // TODO: Emit custom type spec entries.
            let mut spec = DepthLimitedWrite::new(Vec::new(), 10);
            ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
                name: name
                    .try_into()
                    .unwrap_or_else(|_| panic!("function name {:?} exceeds limit", cfg.name)),
                inputs: cfg
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| ScSpecFunctionInputV0 {
                        name: p
                            .id
                            .as_ref()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| i.to_string())
                            .try_into()
                            .expect("function input name exceeds limit"),
                        type_: ScSpecTypeDef::U32, // TODO: Map type.
                        doc: StringM::default(),   // TODO: Add doc.
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("function input count exceeds limit"),
                outputs: cfg
                    .returns
                    .iter()
                    .map(|_| ScSpecTypeDef::U32) // TODO: Map type.
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("function output count exceeds limit"),
                doc: StringM::default(), // TODO: Add doc.
            })
            .write_xdr(&mut spec)
            .unwrap_or_else(|_| panic!("writing spec to xdr for function {}", cfg.name));

            Self::add_custom_section(context, &binary.module, "contractspecv0", spec.inner);
        }
    }

    fn add_custom_section<'a>(
        context: &'a Context,
        module: &Module<'a>,
        name: &'a str,
        value: Vec<u8>,
    ) {
        let value_str = unsafe {
            // TODO: Figure out the right way to generate the LLVM metadata for
            // a slice of bytes.
            String::from_utf8_unchecked(value)
        };

        module
            .add_global_metadata(
                "wasm.custom_sections",
                &context.metadata_node(&[
                    context.metadata_string(name).into(),
                    context.metadata_string(&value_str).into(),
                ]),
            )
            .expect("adding spec as metadata");
    }
}
