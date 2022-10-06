// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;

use contract_metadata::{
    CodeHash, Compiler, Contract, ContractMetadata, Language, Source, SourceCompiler,
    SourceLanguage, SourceWasm,
};
use ink_metadata::{
    layout::{FieldLayout, Layout, LayoutKey, LeafLayout, StructLayout},
    ConstructorSpec, ContractSpec, EventParamSpec, EventSpec, InkProject, MessageParamSpec,
    MessageSpec, ReturnTypeSpec, TypeSpec,
};

use itertools::Itertools;
use serde_json::{Map, Value};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use scale_info::{
    form::PortableForm, registry::PortableType, Field, Path, PortableRegistry, Type, TypeDef,
    TypeDefArray, TypeDefComposite, TypeDefPrimitive, TypeDefSequence, TypeDefTuple,
    TypeDefVariant, Variant,
};
use semver::Version;
use solang_parser::pt;

use crate::sema::{
    ast::{self, ArrayLength, EventDecl, Function},
    tags::render,
};

fn path_from_str(s: impl Into<String>) -> Path<PortableForm> {
    serde_json::from_value::<Path<PortableForm>>(serde_json::json!(vec![s.into()])).unwrap()
}

fn primitive_to_ty(ty: &ast::Type, registry: &mut PortableRegistry) -> Type<PortableForm> {
    match ty {
        ast::Type::Int(_) | ast::Type::Uint(_) => int_to_ty(ty, registry),
        ast::Type::Bool => Type::new(
            path_from_str("bool"),
            vec![],
            TypeDef::Primitive(TypeDefPrimitive::Bool),
            Default::default(),
        ),
        ast::Type::String => Type::new(
            path_from_str("string"),
            vec![],
            TypeDef::Primitive(TypeDefPrimitive::Str),
            Default::default(),
        ),
        _ => unreachable!("non primitive types"),
    }
}

fn int_to_ty(ty: &ast::Type, registry: &mut PortableRegistry) -> Type<PortableForm> {
    let (signed, scalety) = match ty {
        ast::Type::Uint(n) => ('u', n.next_power_of_two()),
        ast::Type::Int(n) => ('i', n.next_power_of_two()),
        _ => unreachable!(),
    };

    let def = match (signed, scalety) {
        ('u', n) => match n {
            8 => TypeDefPrimitive::U8,
            16 => TypeDefPrimitive::U16,
            32 => TypeDefPrimitive::U32,
            64 => TypeDefPrimitive::U64,
            128 => TypeDefPrimitive::U128,
            256 => TypeDefPrimitive::U256,
            _ => unreachable!(),
        },
        ('i', n) => match n {
            8 => TypeDefPrimitive::I8,
            16 => TypeDefPrimitive::I16,
            32 => TypeDefPrimitive::I32,
            64 => TypeDefPrimitive::I64,
            128 => TypeDefPrimitive::I128,
            256 => TypeDefPrimitive::I256,

            _ => unreachable!(),
        },
        _ => {
            unreachable!()
        }
    };

    let path = serde_json::from_value::<Path<PortableForm>>(serde_json::json!(vec![format!(
        "{signed}{scalety}"
    )]))
    .unwrap();

    let ty = Type::new(path, vec![], TypeDef::Primitive(def), Default::default());

    get_or_register_ty(&ty, registry);

    ty
}

type Cache = HashMap<ast::Type, PortableType>;

/// given an `ast::Type`, find and register the `scale_info::Type` definition in the `PortableRegistry`
fn resolve_ast(
    ty: &ast::Type,
    ns: &ast::Namespace,
    registry: &mut PortableRegistry,
    cache: &mut Cache,
) -> PortableType {
    // early return if already cached
    if let Some(ty) = cache.get(ty) {
        return ty.clone();
    }

    match ty {
        //  should reflect address_length for different substrate runtime
        ast::Type::Address(_) | ast::Type::Contract(_) => {
            // substituted to [u8 ;address_length]

            let address_ty = resolve_ast(
                &ast::Type::Array(
                    Box::new(ast::Type::Uint(8)),
                    vec![ArrayLength::Fixed(BigInt::from(ns.address_length))],
                ),
                ns,
                registry,
                cache,
            );

            // substituded to struct { AccountId }
            let field = Field::new(None, address_ty.id().into(), None, vec![]);

            let c = TypeDefComposite::new(vec![field]);

            let path = serde_json::from_value::<Path<PortableForm>>(serde_json::json!([
                "ink_env",
                "types",
                "AccountId"
            ]))
            .unwrap();

            let ty: Type<PortableForm> =
                Type::new(path, vec![], TypeDef::Composite(c), Default::default());

            get_or_register_ty(&ty, registry)
        }

        // primitive types
        ast::Type::Bool | ast::Type::Int(_) | ast::Type::Uint(_) | ast::Type::String => {
            let ty = primitive_to_ty(ty, registry);
            get_or_register_ty(&ty, registry)
        }

        // resolve from the deepest element to outside
        // [[A; a: usize]; b: usize] -> Array(A_id, vec![a, b])
        ast::Type::Array(ty, dims) => {
            let mut ty = resolve_ast(ty, ns, registry, cache);

            for d in dims {
                if let ast::ArrayLength::Fixed(d) = d {
                    let def = TypeDefArray::new(d.to_u32().unwrap(), ty.id().into());

                    // resolve current depth
                    ty = get_or_register_ty(
                        &Type::new(
                            Default::default(),
                            vec![],
                            TypeDef::Array(def),
                            Default::default(),
                        ),
                        registry,
                    );
                } else {
                    let def = TypeDefSequence::new(ty.id().into());

                    // resolve current depth
                    ty = get_or_register_ty(
                        &Type::new(
                            Default::default(),
                            vec![],
                            TypeDef::Sequence(def),
                            Default::default(),
                        ),
                        registry,
                    );
                }
            }

            ty
        }
        // substituded to [u8; len]
        ast::Type::Bytes(n) => resolve_ast(
            &ast::Type::Array(
                Box::new(ast::Type::Uint(8)),
                vec![ArrayLength::Fixed(BigInt::from(*n as i8))],
            ),
            ns,
            registry,
            cache,
        ),
        // substituded to Vec<u8>
        ast::Type::DynamicBytes => resolve_ast(
            &ast::Type::Array(Box::new(ast::Type::Uint(8)), vec![ArrayLength::Dynamic]),
            ns,
            registry,
            cache,
        ),

        ast::Type::Struct(s) => {
            let def = s.definition(ns);

            let fields = def
                .fields
                .iter()
                .map(|f| {
                    let f_ty = resolve_ast(&f.ty, ns, registry, cache);

                    Field::new(
                        Some(f.name_as_str().to_string()),
                        f_ty.id().into(),
                        None,
                        vec![],
                    )
                })
                .collect::<Vec<Field<PortableForm>>>();

            let c = TypeDefComposite::<PortableForm> { fields };

            let path: Path<PortableForm> =
                serde_json::from_value(serde_json::json!([def.name])).unwrap();

            let ty = Type::new(path, vec![], TypeDef::Composite(c), Default::default());

            get_or_register_ty(&ty, registry)
        }
        ast::Type::Enum(n) => {
            let decl = &ns.enums[*n];

            let mut variants = decl.values.iter().collect_vec();

            // sort by discriminant
            variants.sort_by(|a, b| a.1 .1.cmp(&b.1 .1));

            let variants = variants
                .into_iter()
                .map(|(k, v)| Variant {
                    name: k.clone(),
                    fields: Default::default(),
                    index: v.1 as u8,
                    docs: Default::default(),
                })
                .collect::<Vec<_>>();

            let v = TypeDefVariant::new(variants);

            let path: Path<PortableForm> =
                serde_json::from_value(serde_json::json!([decl.name])).unwrap();

            let ty = Type::new(path, vec![], TypeDef::Variant(v), Default::default());

            get_or_register_ty(&ty, registry)
        }
        ast::Type::Ref(ty) => resolve_ast(ty, ns, registry, cache),
        ast::Type::StorageRef(_, ty) => resolve_ast(ty, ns, registry, cache),
        ast::Type::InternalFunction { .. } => resolve_ast(&ast::Type::Uint(8), ns, registry, cache),
        ast::Type::ExternalFunction { .. } => {
            let fields = [ast::Type::Address(false), ast::Type::Uint(32)]
                .into_iter()
                .map(|ty| {
                    let ty = resolve_ast(&ty, ns, registry, cache);

                    Field::new(
                        Default::default(),
                        ty.id().into(),
                        Default::default(),
                        Default::default(),
                    )
                })
                .collect::<Vec<_>>();

            let c = TypeDefComposite { fields };

            let path: Path<PortableForm> =
                serde_json::from_value(serde_json::json!(["ExternalFunction"])).unwrap();

            let ty = Type::new(path, vec![], TypeDef::Composite(c), Default::default());

            get_or_register_ty(&ty, registry)
        }
        ast::Type::UserType(no) => resolve_ast(&ns.user_types[*no].ty, ns, registry, cache),

        _ => unreachable!(),
    }
}

/// register new type if not already specified, type_id starts from 0
fn get_or_register_ty(ty: &Type<PortableForm>, registry: &mut PortableRegistry) -> PortableType {
    if let Some(t) = registry.types().iter().find(|e| e.ty() == ty) {
        t.clone()
    } else {
        let id = registry.types().len() as u32;
        let pty = PortableType::new_custom(id, ty.clone());
        let mut types: Vec<PortableType> = Vec::from(registry.types());
        types.push(pty.clone());
        *registry = PortableRegistry::new_from_types(types);

        pty
    }
}

/// generate `InkProject` from `ast::Type` and `ast::Namespace`
pub fn gen_project(contract_no: usize, ns: &ast::Namespace) -> InkProject {
    // manually building the PortableRegistry
    let mut registry = PortableRegistry::new_from_types(vec![]);

    // type cache to avoid resolving already resolved `ast::Type`
    let mut cache = Cache::new();

    let fields: Vec<FieldLayout<PortableForm>> = ns.contracts[contract_no]
        .layout
        .iter()
        .filter_map(|layout| {
            let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

            // TODO: consult ink storage layout, maybe mapping can be resolved?

            //mappings and large types cannot be represented
            if !var.ty.contains_mapping(ns) && var.ty.fits_in_memory(ns) {
                let layout_key = LayoutKey::new(layout.slot.to_u32().unwrap());

                let ty = resolve_ast(&layout.ty, ns, &mut registry, &mut cache);

                let leaf = LeafLayout::new_from_ty(layout_key, ty.id().into());

                let f = FieldLayout::new_custom(var.name.clone(), Layout::Leaf(leaf));

                Some(f)
            } else {
                None
            }
        })
        .collect();

    // FIXME: what shoudl be the name of the storage?
    let layout = Layout::Struct(StructLayout::new(String::new(), fields));

    let mut f_to_constructor = |f: &Function| -> ConstructorSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));
        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry, &mut cache);

                let spec = TypeSpec::new_from_ty(ty.id().into(), ty.ty().path().clone());

                MessageParamSpec::new_custom(p.name_as_str().to_string(), spec)
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();

        ConstructorSpec::from_label("new")
            .selector(f.selector().try_into().unwrap())
            .payable(payable)
            .args(args)
            .docs(vec![render(&f.tags).as_str()])
            .done()
    };

    // TODO: `cargo-transcode` can match constructor with different name, currently we all named them as "new", we might need to adopt this too?
    let constructors = ns.contracts[contract_no]
        .functions
        .iter()
        .filter_map(|i| {
            // include functions of type constructor
            let f = &ns.functions[*i];
            if f.is_constructor() {
                Some(f)
            } else {
                None
            }
        })
        .chain(
            // include default constructor if exists
            ns.contracts[contract_no]
                .default_constructor
                .as_ref()
                .map(|(e, _)| e),
        )
        .map(|f| f_to_constructor(f))
        .collect::<Vec<ConstructorSpec<PortableForm>>>();

    let mut f_to_message = |f: &Function| -> MessageSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));

        let mutates = matches!(
            f.mutability,
            ast::Mutability::Payable(_) | ast::Mutability::Nonpayable(_)
        );

        let ret_spec: Option<TypeSpec<PortableForm>> = match f.returns.len() {
            0 => None,
            1 => {
                let ty = resolve_ast(&f.returns[0].ty, ns, &mut registry, &mut cache);

                let spec = TypeSpec::new_from_ty(ty.id().into(), ty.ty().path().clone());

                Some(spec)
            }

            _ => {
                let fields = f
                    .returns
                    .iter()
                    .map(|r_p| {
                        let ty = resolve_ast(&r_p.ty, ns, &mut registry, &mut cache);

                        ty.id().into()
                    })
                    .collect::<Vec<_>>();

                let t = TypeDefTuple::new_custom(fields);

                let path = [
                    &ns.contracts[contract_no].name,
                    &f.name,
                    &"return_type".into(),
                ];

                let display_name: Path<PortableForm> =
                    serde_json::from_value(serde_json::json!(path)).unwrap();

                let ty = Type::new(display_name, vec![], TypeDef::Tuple(t), Default::default());

                let ty = get_or_register_ty(&ty, &mut registry);

                Some(TypeSpec::new_from_ty(
                    ty.id().into(),
                    ty.ty().path().clone(),
                ))
            }
        };

        let ret_type = ReturnTypeSpec::<PortableForm> { opt_type: ret_spec };

        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry, &mut cache);

                let spec = TypeSpec::new_from_ty(ty.id().into(), ty.ty().path().clone());

                MessageParamSpec::new_custom(p.name_as_str().to_string(), spec)
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();

        MessageSpec::from_label(&f.name)
            .selector(f.selector().try_into().unwrap())
            .mutates(mutates)
            .payable(payable)
            .args(args)
            .returns(ret_type)
            .docs(vec![render(&f.tags).as_str()])
            .done()
    };

    let messages = ns.contracts[contract_no]
        .all_functions
        .keys()
        .filter_map(|function_no| {
            let func = &ns.functions[*function_no];

            // escape if it's a library
            if let Some(base_contract_no) = func.contract_no {
                if ns.contracts[base_contract_no].is_library() {
                    return None;
                }
            }

            Some(func)
        })
        .filter(|f| match f.visibility {
            pt::Visibility::Public(_) | pt::Visibility::External(_) => matches!(
                f.ty,
                pt::FunctionTy::Function | pt::FunctionTy::Fallback | pt::FunctionTy::Receive
            ),
            _ => false,
        })
        .map(|f| f_to_message(f))
        .collect::<Vec<MessageSpec<PortableForm>>>();

    let mut e_to_evt = |e: &EventDecl| -> EventSpec<PortableForm> {
        let args = e
            .fields
            .iter()
            .map(|p| {
                let label = p.name_as_str().to_string();

                let ty = resolve_ast(&p.ty, ns, &mut registry, &mut cache);

                let spec = TypeSpec::new_from_ty(ty.id().into(), ty.ty().path().clone());

                EventParamSpec::new_custom(label, spec)
                    .indexed(p.indexed)
                    .docs(vec![])
                    .done()
            })
            .collect::<Vec<_>>();

        EventSpec::new(&e.name)
            .args(args)
            .docs(vec![render(&e.tags).as_str()])
            .done()
    };

    let events = ns.contracts[contract_no]
        .sends_events
        .iter()
        .map(|event_no| {
            let event = &ns.events[*event_no];

            e_to_evt(event)
        })
        .collect::<Vec<EventSpec<PortableForm>>>();

    let spec = ContractSpec::new()
        .constructors(constructors)
        .messages(messages)
        .events(events)
        .docs(vec![render(&ns.contracts[contract_no].tags).as_str()])
        .done();

    InkProject::new_portable(layout, spec, registry)
}

fn tags(contract_no: usize, tagname: &str, ns: &ast::Namespace) -> Vec<String> {
    ns.contracts[contract_no]
        .tags
        .iter()
        .filter_map(|tag| {
            if tag.tag == tagname {
                Some(tag.value.to_owned())
            } else {
                None
            }
        })
        .collect()
}

/// Generate the metadata for Substrate 3.0
pub fn metadata(contract_no: usize, code: &[u8], ns: &ast::Namespace) -> Value {
    let hash = blake2_rfc::blake2b::blake2b(32, &[], code);
    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let language = SourceLanguage::new(Language::Solidity, version.clone());
    let compiler = SourceCompiler::new(Compiler::Solang, version);
    let code_hash: [u8; 32] = hash.as_bytes().try_into().unwrap();
    let source_wasm = SourceWasm::new(code.to_vec());

    let source = Source::new(Some(source_wasm), CodeHash(code_hash), language, compiler);
    let mut builder = Contract::builder();

    // Add our name and tags
    builder.name(&ns.contracts[contract_no].name);

    let mut description = tags(contract_no, "title", ns);

    description.extend(tags(contract_no, "notice", ns));

    if !description.is_empty() {
        builder.description(description.join("\n"));
    };

    let authors = tags(contract_no, "author", ns);

    if !authors.is_empty() {
        builder.authors(authors);
    } else {
        builder.authors(vec!["unknown"]);
    }

    // FIXME: contract-metadata wants us to provide a version number, but there is no version in the solidity source
    // code. Since we must provide a valid semver version, we just provide a bogus value.Abi
    builder.version(Version::new(0, 0, 1));

    let contract = builder.build().unwrap();

    let project = gen_project(contract_no, ns);

    let abi_json: Map<String, Value> =
        serde_json::from_value(serde_json::to_value(project).unwrap()).unwrap();

    let metadata = ContractMetadata::new(source, contract, None, abi_json);

    // serialize to json
    serde_json::to_value(&metadata).unwrap()
}

pub fn load(s: &str) -> InkProject {
    let bundle = serde_json::from_str::<ContractMetadata>(s).unwrap();

<<<<<<< HEAD
    serde_json::from_value::<InkProject>(serde_json::to_value(bundle.abi).unwrap()).unwrap()
=======
    let fields = ns.contracts[contract_no]
        .layout
        .iter()
        .filter_map(|layout| {
            let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

            // mappings and large types cannot be represented
            if !var.ty.contains_mapping(ns) && var.ty.fits_in_memory(ns) {
                Some(StorageLayout {
                    name: var.name.to_string(),
                    layout: LayoutField {
                        cell: LayoutFieldCell {
                            key: format!("0x{:064X}", layout.slot),
                            ty: ty_to_abi(&var.ty, ns, &mut abi).ty,
                        },
                    },
                })
            } else {
                None
            }
        })
        .collect();

    abi.storage.structs.fields = fields;

    let mut constructors = ns.contracts[contract_no]
        .functions
        .iter()
        .filter_map(|function_no| {
            let f = &ns.functions[*function_no];
            if f.is_constructor() {
                Some(Constructor {
                    name: String::from("new"),
                    selector: render_selector(f),
                    args: f
                        .params
                        .iter()
                        .map(|p| parameter_to_abi(p, ns, &mut abi))
                        .collect(),
                    docs: vec![render(&f.tags)],
                })
            } else {
                None
            }
        })
        .collect::<Vec<Constructor>>();

    if let Some((f, _)) = &ns.contracts[contract_no].default_constructor {
        constructors.push(Constructor {
            name: String::from("new"),
            selector: render_selector(f),
            args: f
                .params
                .iter()
                .map(|p| parameter_to_abi(p, ns, &mut abi))
                .collect(),
            docs: vec![render(&f.tags)],
        });
    }

    let conflicting_names = non_unique_function_names(contract_no, ns);
    let messages = ns.contracts[contract_no]
        .all_functions
        .keys()
        .filter_map(|function_no| {
            let func = &ns.functions[*function_no];

            if let Some(base_contract_no) = func.contract_no {
                if ns.contracts[base_contract_no].is_library() {
                    return None;
                }
            }

            Some(func)
        })
        .filter(|f| match f.visibility {
            pt::Visibility::Public(_) | pt::Visibility::External(_) => {
                f.ty == pt::FunctionTy::Function
            }
            _ => false,
        })
        .map(|f| {
            let payable = matches!(f.mutability, ast::Mutability::Payable(_));

            Message {
                name: conflicting_names
                    .contains(&f.name)
                    .then(|| &f.mangled_name)
                    .unwrap_or(&f.name)
                    .into(),
                mutates: matches!(
                    f.mutability,
                    ast::Mutability::Payable(_) | ast::Mutability::Nonpayable(_)
                ),
                payable,
                return_type: match f.returns.len() {
                    0 => None,
                    1 => Some(ty_to_abi(&f.returns[0].ty, ns, &mut abi)),
                    _ => {
                        let fields = f
                            .returns
                            .iter()
                            .map(|f| StructField {
                                name: f.id.as_ref().map(|id| id.name.to_owned()),
                                ty: ty_to_abi(&f.ty, ns, &mut abi).ty,
                            })
                            .collect();

                        Some(ParamType {
                            ty: abi.struct_type(Vec::new(), fields),
                            display_name: vec![],
                        })
                    }
                },
                selector: render_selector(f),
                args: f
                    .params
                    .iter()
                    .map(|p| parameter_to_abi(p, ns, &mut abi))
                    .collect(),
                docs: vec![render(&f.tags)],
            }
        })
        .collect();

    let events = ns.contracts[contract_no]
        .sends_events
        .iter()
        .map(|event_no| {
            let event = &ns.events[*event_no];

            let name = event.name.to_owned();
            let args = event
                .fields
                .iter()
                .map(|p| ParamIndexed {
                    param: parameter_to_abi(p, ns, &mut abi),
                    indexed: p.indexed,
                })
                .collect();
            let docs = vec![render(&event.tags)];

            Event { docs, name, args }
        })
        .collect();

    abi.spec = Spec {
        constructors,
        messages,
        events,
    };

    abi
}

fn ty_to_abi(ty: &ast::Type, ns: &ast::Namespace, registry: &mut Abi) -> ParamType {
    match ty {
        ast::Type::Enum(n) => ParamType {
            ty: registry.builtin_enum_type(&ns.enums[*n]),
            display_name: vec![ns.enums[*n].name.to_owned()],
        },
        ast::Type::Bytes(n) => {
            let elem = registry.builtin_type("u8");
            ParamType {
                ty: registry.builtin_array_type(elem, *n as usize),
                display_name: vec![],
            }
        }
        ast::Type::Mapping(..) => unreachable!(),
        ast::Type::Array(ty, dims) => {
            let mut param_ty = ty_to_abi(ty, ns, registry);

            for d in dims {
                if let ast::ArrayLength::Fixed(d) = d {
                    param_ty = ParamType {
                        ty: registry.builtin_array_type(param_ty.ty, d.to_usize().unwrap()),
                        display_name: vec![],
                    }
                } else {
                    param_ty = ParamType {
                        ty: registry.builtin_slice_type(param_ty.ty),
                        display_name: vec![],
                    }
                }
            }

            param_ty
        }
        ast::Type::StorageRef(_, ty) => ty_to_abi(ty, ns, registry),
        ast::Type::Ref(ty) => ty_to_abi(ty, ns, registry),
        ast::Type::UserType(no) => ty_to_abi(&ns.user_types[*no].ty, ns, registry),
        ast::Type::Bool | ast::Type::Uint(_) | ast::Type::Int(_) => {
            let scalety = match ty {
                ast::Type::Bool => "bool".into(),
                // Substrate doesn't like primitive types which aren't a power of 2
                // The abi encoder/decoder fixes this automatically
                ast::Type::Uint(n) => format!("u{}", n.next_power_of_two()),
                ast::Type::Int(n) => format!("i{}", n.next_power_of_two()),
                _ => unreachable!(),
            };

            ParamType {
                ty: registry.builtin_type(&scalety),
                display_name: vec![scalety.to_string()],
            }
        }
        ast::Type::Address(_) | ast::Type::Contract(_) => {
            let elem = registry.builtin_type("u8");
            let ty = registry.builtin_array_type(elem, 32);

            ParamType {
                ty: registry.struct_type(
                    vec!["AccountId".to_owned()],
                    vec![StructField { name: None, ty }],
                ),
                display_name: vec!["AccountId".to_owned()],
            }
        }
        ast::Type::Struct(struct_type) => {
            let mut display_name = vec![struct_type.definition(ns).name.to_owned()];

            if let Some(contract_name) = &struct_type.definition(ns).contract {
                display_name.insert(0, contract_name.to_owned());
            }

            let def = struct_type.definition(ns);
            let fields = def
                .fields
                .iter()
                .map(|f| StructField {
                    name: Some(f.name_as_str().to_owned()),
                    ty: ty_to_abi(&f.ty, ns, registry).ty,
                })
                .collect();

            ParamType {
                ty: registry.struct_type(display_name.clone(), fields),
                display_name,
            }
        }
        ast::Type::DynamicBytes => {
            let elem = registry.builtin_type("u8");

            ParamType {
                ty: registry.builtin_slice_type(elem),
                display_name: vec![String::from("Vec")],
            }
        }
        ast::Type::String => ParamType {
            ty: registry.builtin_type("str"),
            display_name: vec![String::from("String")],
        },
        ast::Type::InternalFunction { .. } => ParamType {
            ty: registry.builtin_type("u32"),
            display_name: vec![String::from("FunctionSelector")],
        },
        ast::Type::ExternalFunction { .. } => {
            let fields = vec![
                StructField {
                    name: None,
                    ty: ty_to_abi(&ast::Type::Address(false), ns, registry).ty,
                },
                StructField {
                    name: None,
                    ty: ty_to_abi(&ast::Type::Uint(32), ns, registry).ty,
                },
            ];

            let display_name = vec![String::from("ExternalFunction")];

            ParamType {
                ty: registry.struct_type(display_name.clone(), fields),
                display_name,
            }
        }
        _ => unreachable!(),
    }
}

fn parameter_to_abi(param: &ast::Parameter, ns: &ast::Namespace, registry: &mut Abi) -> Param {
    Param {
        name: param.name_as_str().to_owned(),
        ty: ty_to_abi(&param.ty, ns, registry),
    }
}

/// Given an u32 selector, generate a byte string like: 0xF81E7E1A
fn render_selector(f: &ast::Function) -> String {
    format!("0x{}", hex::encode(f.selector()))
}

/// Given a selector like "0xF81E7E1A", parse the bytes. This function
/// does not validate the input.
fn parse_selector(selector: &str) -> Vec<u8> {
    hex::decode(&selector[2..]).unwrap()
>>>>>>> main
}
