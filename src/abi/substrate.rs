// SPDX-License-Identifier: Apache-2.0
use contract_metadata::{
    CodeHash, Compiler, Contract, ContractMetadata, Language, Source, SourceCompiler,
    SourceLanguage, SourceWasm,
};
use ink_metadata::{
    layout::{FieldLayout, Layout, LayoutKey, LeafLayout, RootLayout, StructLayout},
    ConstructorSpec, ContractSpec, EventParamSpec, EventSpec, InkProject, MessageParamSpec,
    MessageSpec, ReturnTypeSpec, TypeSpec,
};

use ink_primitives::KeyComposer;
use ink_storage::traits::{AutoKey, StorageKey};
use itertools::Itertools;
use serde_json::{Map, Value};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use scale_info::{
    form::PortableForm, Field, Path, PortableRegistryBuilder, Type, TypeDef, TypeDefArray,
    TypeDefComposite, TypeDefPrimitive, TypeDefSequence, TypeDefTuple, TypeDefVariant, Variant,
};
use semver::Version;
use solang_parser::pt;

use super::non_unique_function_names;
use crate::sema::{
    ast::{self, ArrayLength, EventDecl, Function},
    tags::render,
};

macro_rules! path {
    ($( $segments:expr ),*) => {
        Path::from_segments_unchecked([$($segments),*].iter().map(ToString::to_string))
    }
}

fn primitive_to_ty(ty: &ast::Type, registry: &mut PortableRegistryBuilder) -> u32 {
    match ty {
        ast::Type::Int(_) | ast::Type::Uint(_) => int_to_ty(ty, registry),
        ast::Type::Bool => registry.register_type(Type::new(
            path!("bool"),
            vec![],
            TypeDef::Primitive(TypeDefPrimitive::Bool),
            Default::default(),
        )),
        ast::Type::String => registry.register_type(Type::new(
            path!("string"),
            vec![],
            TypeDef::Primitive(TypeDefPrimitive::Str),
            Default::default(),
        )),
        _ => unreachable!("non primitive types"),
    }
}

fn int_to_ty(ty: &ast::Type, registry: &mut PortableRegistryBuilder) -> u32 {
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

    let path = path!(format!("{signed}{scalety}"));

    let ty = Type::new(path, vec![], TypeDef::Primitive(def), Default::default());

    registry.register_type(ty)
}

/// given an `ast::Type`, find and register the `scale_info::Type` definition in the `PortableRegistry`
fn resolve_ast(ty: &ast::Type, ns: &ast::Namespace, registry: &mut PortableRegistryBuilder) -> u32 {
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
            );

            // substituded to struct { AccountId }
            let field = Field::new(None, address_ty.into(), None, vec![]);

            let c = TypeDefComposite::new(vec![field]);

            let path = path!("ink_env", "types", "AccountId");

            let ty: Type<PortableForm> =
                Type::new(path, vec![], TypeDef::Composite(c), Default::default());

            //get_or_register_ty(&ty, registry)
            registry.register_type(ty)
        }

        // primitive types
        ast::Type::Bool | ast::Type::Int(_) | ast::Type::Uint(_) | ast::Type::String => {
            primitive_to_ty(ty, registry)
        }

        // resolve from the deepest element to outside
        // [[A; a: usize]; b: usize] -> Array(A_id, vec![a, b])
        ast::Type::Array(ty, dims) => {
            let mut ty = resolve_ast(ty, ns, registry);

            for d in dims {
                if let ast::ArrayLength::Fixed(d) = d {
                    let def = TypeDefArray::new(d.to_u32().unwrap(), ty.into());

                    // resolve current depth
                    ty = registry.register_type(Type::new(
                        Default::default(),
                        vec![],
                        TypeDef::Array(def),
                        Default::default(),
                    ));
                } else {
                    let def = TypeDefSequence::new(ty.into());

                    // resolve current depth
                    ty = registry.register_type(Type::new(
                        Default::default(),
                        vec![],
                        TypeDef::Sequence(def),
                        Default::default(),
                    ));
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
        ),
        // substituded to Vec<u8>
        ast::Type::DynamicBytes => resolve_ast(
            &ast::Type::Array(Box::new(ast::Type::Uint(8)), vec![ArrayLength::Dynamic]),
            ns,
            registry,
        ),
        ast::Type::Struct(s) => {
            let def = s.definition(ns);

            let fields = def
                .fields
                .iter()
                .map(|f| {
                    let f_ty = resolve_ast(&f.ty, ns, registry);

                    Field::new(Some(f.name_as_str().to_string()), f_ty.into(), None, vec![])
                })
                .collect::<Vec<Field<PortableForm>>>();

            let c = TypeDefComposite::new(fields);
            let path = path!(&def.name);
            let ty = Type::new(path, vec![], TypeDef::Composite(c), Default::default());
            registry.register_type(ty)
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

            let path = path!(&decl.name);

            let ty = Type::new(path, vec![], TypeDef::Variant(v), Default::default());
            registry.register_type(ty)
        }
        ast::Type::Ref(ty) => resolve_ast(ty, ns, registry),
        ast::Type::StorageRef(_, ty) => resolve_ast(ty, ns, registry),
        ast::Type::InternalFunction { .. } => resolve_ast(&ast::Type::Uint(8), ns, registry),
        ast::Type::ExternalFunction { .. } => {
            let fields = [ast::Type::Address(false), ast::Type::Uint(32)]
                .into_iter()
                .map(|ty| {
                    let ty = resolve_ast(&ty, ns, registry);

                    Field::new(
                        Default::default(),
                        ty.into(),
                        Default::default(),
                        Default::default(),
                    )
                })
                .collect::<Vec<_>>();

            let c = TypeDefComposite::new(fields);

            let path = path!("ExternalFunction");

            let ty = Type::new(path, vec![], TypeDef::Composite(c), Default::default());
            registry.register_type(ty)
        }
        ast::Type::UserType(no) => resolve_ast(&ns.user_types[*no].ty, ns, registry),

        _ => unreachable!(),
    }
}

/// Recoursively build the storage layout after all types are registered
fn type_to_storage_layout(key: u32, registry: &PortableRegistryBuilder) -> Layout<PortableForm> {
    let ty = registry.get(key).unwrap();
    match ty.type_def() {
        TypeDef::Composite(inner) => Layout::Struct(StructLayout::new(
            ty.path().ident().unwrap_or_default(),
            inner.fields().iter().map(|field| {
                FieldLayout::new_custom(
                    field.name().map(ToString::to_string).unwrap_or_default(),
                    type_to_storage_layout(field.ty().id(), registry),
                )
            }),
        )),
        _ => Layout::Leaf(LeafLayout::new_from_ty(
            <AutoKey as StorageKey>::KEY.into(),
            key.into(),
        )),
    }
}

/// generate `InkProject` from `ast::Type` and `ast::Namespace`
pub fn gen_project(contract_no: usize, ns: &ast::Namespace) -> InkProject {
    let mut registry = PortableRegistryBuilder::new();

    let fields: Vec<FieldLayout<PortableForm>> = ns.contracts[contract_no]
        .layout
        .iter()
        .filter_map(|layout| {
            let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

            // TODO impl mappings should be easy now
            // TODO move the memory fit check to sema maybe??
            if !var.ty.contains_mapping(ns) && var.ty.fits_in_memory(ns) {
                //let layout_key = LayoutKey::new(layout.slot.to_u32().unwrap());

                let ty = resolve_ast(&layout.ty, ns, &mut registry);

                //let leaf = LeafLayout::new_from_ty(layout_key, ty.into());

                //let f = FieldLayout::new_custom(var.name.clone(), Layout::Leaf(leaf));
                let f = FieldLayout::new_custom(
                    var.name.clone(),
                    type_to_storage_layout(ty, &registry),
                );

                Some(f)
            } else {
                None
            }
        })
        .collect();

    let contract_name = ns.contracts[contract_no].name.clone();
    let storage = Layout::Root(RootLayout::new(
        LayoutKey::new(0u32),
        Layout::Struct(StructLayout::new(contract_name, fields)),
    ));

    let f_to_constructor = |f: &Function| -> ConstructorSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));
        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);

                let path = registry.get(ty).unwrap().path().clone();
                let spec = TypeSpec::new_from_ty(ty.into(), path);

                MessageParamSpec::new_custom(p.name_as_str().to_string(), spec)
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();

        ConstructorSpec::from_label("new".to_string())
            .selector(f.selector().try_into().unwrap())
            .payable(payable)
            .args(args)
            .docs(vec![render(&f.tags)])
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
        .map(f_to_constructor)
        .collect::<Vec<ConstructorSpec<PortableForm>>>();

    let conflicting_names = non_unique_function_names(contract_no, ns);
    let f_to_message = |f: &Function| -> MessageSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));

        let mutates = matches!(
            f.mutability,
            ast::Mutability::Payable(_) | ast::Mutability::Nonpayable(_)
        );

        let ret_spec: Option<TypeSpec<PortableForm>> = match f.returns.len() {
            0 => None,
            1 => {
                let ty = resolve_ast(&f.returns[0].ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path().clone();
                Some(TypeSpec::new_from_ty(ty.into(), path))
            }
            _ => {
                let fields = f
                    .returns
                    .iter()
                    .map(|r_p| {
                        let ty = resolve_ast(&r_p.ty, ns, &mut registry);

                        ty.into()
                    })
                    .collect::<Vec<_>>();

                let t = TypeDefTuple::new_portable(fields);

                let path = path!(
                    &ns.contracts[contract_no].name,
                    &f.name,
                    &"return_type".into()
                );

                let ty = registry.register_type(Type::new(
                    path,
                    vec![],
                    TypeDef::Tuple(t),
                    Default::default(),
                ));
                let path = registry.get(ty).unwrap().path().clone();
                Some(TypeSpec::new_from_ty(ty.into(), path))
            }
        };

        let ret_type = ReturnTypeSpec::<PortableForm> { opt_type: ret_spec };

        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path().clone();
                let spec = TypeSpec::new_from_ty(ty.into(), path);

                MessageParamSpec::new_custom(p.name_as_str().to_string(), spec)
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();

        let label = conflicting_names
            .contains(&f.name)
            .then(|| &f.mangled_name)
            .unwrap_or(&f.name)
            .into();
        MessageSpec::from_label(label)
            .selector(f.selector().try_into().unwrap())
            .mutates(mutates)
            .payable(payable)
            .args(args)
            .returns(ret_type)
            .docs(vec![render(&f.tags)])
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
        .map(f_to_message)
        .collect::<Vec<MessageSpec<PortableForm>>>();

    let mut e_to_evt = |e: &EventDecl| -> EventSpec<PortableForm> {
        let args = e
            .fields
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path().clone();
                let spec = TypeSpec::new_from_ty(ty.into(), path);
                EventParamSpec::new_custom(p.name_as_str().into(), spec)
                    .indexed(p.indexed)
                    .docs(vec![])
                    .done()
            })
            .collect::<Vec<_>>();

        EventSpec::new(e.name.clone())
            .args(args)
            .docs(vec![render(&e.tags)])
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
        .docs(vec![render(&ns.contracts[contract_no].tags)])
        .done();

    InkProject::new_portable(storage, spec, registry.finish())
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

/// Generate the metadata for Substrate 4.0
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

    serde_json::from_value::<InkProject>(serde_json::to_value(bundle.abi).unwrap()).unwrap()
}
