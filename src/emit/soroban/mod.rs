// SPDX-License-Identifier: Apache-2.0

pub(super) mod target;
use crate::codegen::cfg::ControlFlowGraph;
use crate::emit::cfg::emit_cfg;
use crate::{
    codegen::{cfg::ASTFunction, Options, STORAGE_INITIALIZER},
    emit::Binary,
    sema::ast,
};
use inkwell::{
    context::Context,
    module::{Linkage, Module},
};
use soroban_sdk::xdr::{
    DepthLimitedWrite, ScEnvMetaEntry, ScSpecEntry, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeDef, StringM, ToXdr, WriteXdr
};
use std::sync;
use std::ffi::CString;

const SOROBAN_ENV_INTERFACE_VERSION: u64 = 85899345920;

pub struct SorobanTarget ;

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
        Self::declare_externals(&mut binary);
        Self::emit_functions_with_spec(contract, &mut binary, ns, context, contract_no);

        Self::emit_initializer(&mut binary, ns);

        Self::emit_env_meta_entries(context, &mut binary, opt);

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
            println!("emit_functions_with_spec: cfg_name: {:?}", cfg.name);
            let ftype = binary.function_type(
                &cfg.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                &cfg.returns.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                ns,
            );

            // For each function, determine the name and the linkage
            // Soroban has no dispatcher, so all externally addressable functions are exported and should be named the same as the original function name in the source code.
            // If there are duplicate function names, then the function name in the source is mangled to include the signature.
            let default_constructor = ns.default_constructor(contract_no);
            let mut name = {
                if cfg.public {
                    let f = match &cfg.function_no {
                        ASTFunction::SolidityFunction(no) | ASTFunction::YulFunction(no) => {
                            &ns.functions[*no]
                        }
                        _ => &default_constructor,
                    };

                    if f.mangled_name_contracts.contains(&contract_no) {
                        f.mangled_name.clone()
                    } else {
                        f.id.name.clone()
                    }
                } else {
                    cfg.name.clone()
                }
            };

            /*if cfg.name == "storage_initializer" {
                name = "storage_initializer".to_owned();

                let returns = cfg.returns.clone();
                println!(
                    "emit_functions_with_spec: storage_initializer: returns: {:?}",
                    returns
                );
            }*/

            Self::emit_function_spec_entry(context, cfg.clone(), name.clone(), binary);

            let linkage = if cfg.public {
                Linkage::External
            } else {
                Linkage::Internal
            };

            let func_decl = if let Some(func) = binary.module.get_function(&name) {
                // must not have a body yet
                assert_eq!(func.get_first_basic_block(), None);

                func
            } else {
                binary.module.add_function(&name, ftype, Some(linkage))
            };

            binary.functions.insert(cfg_no, func_decl);

            defines.push((func_decl, cfg));
        }

        
         let init_type = context.i64_type().fn_type(&[], false);
        binary
            .module
            .add_function("storage_initializer", init_type, None);

        for (func_decl, cfg) in defines {
            emit_cfg(&mut SorobanTarget, binary, contract, cfg, func_decl, ns);
        }
    }

    fn emit_env_meta_entries<'a>(context: &'a Context, binary: &mut Binary<'a>, opt: &'a Options) {
        let mut meta = DepthLimitedWrite::new(Vec::new(), 10);
        let soroban_env_interface_version =
            opt.soroban_version.unwrap_or(SOROBAN_ENV_INTERFACE_VERSION);
        ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(soroban_env_interface_version)
            .write_xdr(&mut meta)
            .expect("writing env meta interface version to xdr");
        Self::add_custom_section(context, &binary.module, "contractenvmetav0", meta.inner);
    }

    fn emit_function_spec_entry<'a>(
        context: &'a Context,
        cfg: ControlFlowGraph,
        name: String,
        binary: &mut Binary<'a>,
    ) {
        if cfg.public && !cfg.is_placeholder() {
            // TODO: Emit custom type spec entries
            //let outputs = ScSpecTypeDef::Vec(());

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
                    .map(|return_type| {
                        let ty = return_type.ty.clone();
                        match ty {
                            ast::Type::Uint(32) => ScSpecTypeDef::U32,
                            ast::Type::Uint(64) => ScSpecTypeDef::U64,
                            ast::Type::Int(_) => ScSpecTypeDef::I32,
                            ast::Type::Bool => ScSpecTypeDef::Bool,
                            ast::Type::Address(_) => ScSpecTypeDef::Address,
                            ast::Type::Bytes(_) => ScSpecTypeDef::Bytes,
                            ast::Type::String => ScSpecTypeDef::String,
                            ast::Type::Void => ScSpecTypeDef::Void,
                            _ => panic!("unsupported return type {:?}", ty),
                        }
                    }) // TODO: Map type.
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

    fn add_import_section<'a>(
        context: &'a Context,
        module: &Module<'a>,
        name: &'a str,
        value: Vec<u8>,
    ) {
    }

    fn declare_externals(binary: &mut Binary) {
        let ty = binary.context.i64_type();
        let function_ty_1 = binary
            .context
            .i64_type()
            .fn_type(&[ty.into(), ty.into(), ty.into()], false);
        let function_ty = binary
            .context
            .i64_type()
            .fn_type(&[ty.into(), ty.into()], false);

        binary
            .module
            .add_function("l._", function_ty_1, Some(Linkage::External));
        binary
            .module
            .add_function("l.1", function_ty, Some(Linkage::External));
    }

    fn emit_initializer(binary: &mut Binary, _ns: &ast::Namespace) {
        let mut cfg = ControlFlowGraph::new("init".to_string(), ASTFunction::None);

        cfg.public = true;
        let void_param = ast::Parameter::new_default(ast::Type::Void);
        cfg.returns = sync::Arc::new(vec![void_param]);

        // /println!("emit_initializer: cfg: {:?}", cfg);
        Self::emit_function_spec_entry(binary.context, cfg, "init".to_string(), binary);

        let function_name = CString::new(STORAGE_INITIALIZER).unwrap();
        let mut storage_initializers = binary
            .functions
            .values()
            .filter(|f: &&inkwell::values::FunctionValue| f.get_name() == function_name.as_c_str());
        let storage_initializer = *storage_initializers
            .next()
            .expect("storage initializer is always present");
        assert!(storage_initializers.next().is_none());

        println!(
            "emit_initializer: storage_initializer: {:?}",
            storage_initializer
        );

        let void_type = binary.context.i64_type().fn_type(&[], false);
        let init = binary
            .module
            .add_function("init", void_type, Some(Linkage::External));
        let entry = binary.context.append_basic_block(init, "entry");

        binary.builder.position_at_end(entry);
        binary
            .builder
            .build_call(storage_initializer, &[], "storage_initializer").unwrap();

        let soroban_void = soroban_sdk::Val::VOID;
            

        // return zero
        let zero_val = binary.context.i64_type().const_int(2, false);
        binary.builder.build_return(Some(&zero_val)).unwrap();
    }

    /*
    fn public_function_prelude<'a>(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        storage_initializer: Option<FunctionValue>,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "")
            .unwrap();

        // Call the storage initializers on deploy
        if let Some(initializer) = storage_initializer {
            binary.builder.build_call(initializer, &[], "").unwrap();
        }

        let scratch_buf = binary.scratch.unwrap().as_pointer_value();
        let scratch_len = binary.scratch_len.unwrap().as_pointer_value();

        // copy arguments from input buffer
        binary
            .builder
            .build_store(
                scratch_len,
                binary
                    .context
                    .i32_type()
                    .const_int(SCRATCH_SIZE as u64, false),
            )
            .unwrap();

        binary
            .builder
            .build_call(
                binary.module.get_function("input").unwrap(),
                &[scratch_buf.into(), scratch_len.into()],
                "",
            )
            .unwrap();

        let args_length = binary
            .builder
            .build_load(binary.context.i32_type(), scratch_len, "input_len")
            .unwrap();

        // store the length in case someone wants it via msg.data
        binary
            .builder
            .build_store(
                binary.calldata_len.as_pointer_value(),
                args_length.into_int_value(),
            )
            .unwrap();

        (scratch_buf, args_length.into_int_value())
    }


    fn emit_dispatch(
        &mut self,
        storage_initializer: Option<FunctionValue>,
        bin: &mut Binary,
        ns: &Namespace,
    ) {
        let ty = bin.context.void_type().fn_type(&[], false);
        let export_name = if storage_initializer.is_some() {
            "deploy"
        } else {
            "call"
        };
        let func = bin.module.add_function(export_name, ty, None);
        let (input, input_length) = self.public_function_prelude(bin, func, storage_initializer);
        let args = vec![
            BasicMetadataValueEnum::PointerValue(input),
            BasicMetadataValueEnum::IntValue(input_length),
            BasicMetadataValueEnum::IntValue(self.value_transferred(bin, ns)),
            BasicMetadataValueEnum::PointerValue(bin.selector.as_pointer_value()),
        ];
        let dispatch_cfg_name = &storage_initializer
            .map(|_| DispatchType::Deploy)
            .unwrap_or(DispatchType::Call)
            .to_string();
        let cfg = bin.module.get_function(dispatch_cfg_name).unwrap();
        bin.builder
            .build_call(cfg, &args, dispatch_cfg_name)
            .unwrap();

        bin.builder.build_unreachable().unwrap();
    }
    */






}


