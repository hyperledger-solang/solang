pub(super) mod target;

use crate::emit::functions::emit_functions;
use crate::{codegen::Options, emit::Binary, sema::ast};
use inkwell::context::Context;
use inkwell::module::Module;

use stellar_xdr::{
    ScEnvMetaEntry, ScSpecEntry, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeDef, WriteXdr,
};

// TODO: Handle error cases that are currently caught with .expect. Figure out
// how to return errors from build.

const SOROBAN_ENV_INTERFACE_VERSION: u64 = 27;
const SOROBAN_SYMBOL_MAX_LENGTH: usize = 10;

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

        emit_functions(&mut SorobanTarget, &mut binary, contract, ns, |cfg| {
            &cfg.original_name
        });

        Self::emit_env_meta_entries(context, &mut binary);
        Self::emit_spec_entries(context, contract, &mut binary);
        Self::internalize(contract, &mut binary);

        binary
    }

    fn internalize<'a>(contract: &ast::Contract, binary: &mut Binary<'a>) {
        let exports = contract
            .cfg
            .iter()
            .filter(|cfg| !cfg.is_placeholder())
            .filter(|cfg| cfg.public)
            .filter(|cfg| cfg.original_name.len() > 0)
            .map(|cfg| cfg.name.as_str())
            .collect::<Vec<_>>();
        binary.internalize(&exports);
    }

    fn emit_env_meta_entries<'a>(context: &'a Context, binary: &mut Binary<'a>) {
        let mut meta = vec![];
        ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(SOROBAN_ENV_INTERFACE_VERSION)
            .write_xdr(&mut meta)
            .expect("writing env meta interface version to xdr");
        add_custom_section(context, &binary.module, "contractenvmetav0", meta);
    }

    fn emit_spec_entries<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        binary: &mut Binary<'a>,
    ) {
        // TODO: Emit custom type spec entries.
        let mut spec = vec![];
        for cfg in &contract.cfg {
            if cfg.is_placeholder() {
                continue;
            }
            if !cfg.public {
                continue;
            }
            ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
                name: function_name(&cfg.name)
                    .try_into()
                    .expect(format!("function name {:?} exceeds limit", cfg.name).as_str()),
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
            })
            .write_xdr(&mut spec)
            .expect(format!("writing spec to xdr for function {}", cfg.name).as_str());
        }
        add_custom_section(context, &binary.module, "contractspecv0", spec);
    }
}

fn function_name<'a>(s: &'a str) -> &'a str {
    // Function names in the control flow graph are fully qualified and include
    // other information like the types of parameters. There's also special
    // cases where they contain information about whether they are a constructor
    // or a user function.

    // TODO: Find a better way to extract the original function name without the
    // additional. We might need to change the name value from a String to a
    // type that stores the information in separate fields. There also might be
    // a better way to do it than this which we're overlooking.

    let mut iter = s.splitn(4, "::");
    _ = iter.next().unwrap();
    _ = iter.next().unwrap();
    let kind = iter.next().unwrap();
    let name = iter.next().unwrap();
    match kind {
        "constructor" => "init",
        "function" => {
            let name = name.splitn(2, "__").next().unwrap();
            if name.len() <= SOROBAN_SYMBOL_MAX_LENGTH {
                name
            } else {
                &name[..SOROBAN_SYMBOL_MAX_LENGTH]
            }
        }
        _ => panic!("unsupported function kind {:?}", s),
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
        // a slice of bytes. As far as I can tell the inkwell interface only
        // provides a way to provide it as a &str, although internally it
        // immediately converts it to a CStr, and LLVM allows non-unicode
        // characters. We're currently misusing String's unchecked
        // conversion function to intentionally get a String that holds
        // non-utf8 data.
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
