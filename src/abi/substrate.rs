// SPDX-License-Identifier: Apache-2.0
use contract_metadata::{
    CodeHash, Compiler, Contract, ContractMetadata, Language, Source, SourceCompiler,
    SourceLanguage, SourceWasm,
};
use ink::metadata::{
    layout::{FieldLayout, Layout, LayoutKey, LeafLayout, RootLayout, StructLayout},
    ConstructorSpec, ContractSpec, EventParamSpec, EventSpec, InkProject, MessageParamSpec,
    MessageSpec, ReturnTypeSpec, TypeSpec,
};

use serde_json::Value;

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use scale_info::{
    form::PortableForm, Field, Path, PortableRegistryBuilder, Type, TypeDef, TypeDefArray,
    TypeDefComposite, TypeDefPrimitive, TypeDefSequence, TypeDefTuple, TypeDefVariant, Variant,
};
use semver::Version;
use solang_parser::pt;

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

/// Given an `ast::Type`, find and register the `scale_info::Type` definition in the registry
fn resolve_ast(ty: &ast::Type, ns: &ast::Namespace, registry: &mut PortableRegistryBuilder) -> u32 {
    match ty {
        //  should reflect address_length for different substrate runtime
        ast::Type::Address(_) | ast::Type::Contract(_) => {
            // substituted to [u8; address_length]
            let address_ty = resolve_ast(
                &ast::Type::Array(
                    Box::new(ast::Type::Uint(8)),
                    vec![ArrayLength::Fixed(BigInt::from(ns.address_length))],
                ),
                ns,
                registry,
            );
            // substituted to struct { AccountId }
            let field = Field::new(None, address_ty.into(), None, vec![]);
            let c = TypeDefComposite::new(vec![field]);
            let path = path!("ink_env", "types", "AccountId");
            let ty: Type<PortableForm> =
                Type::new(path, vec![], TypeDef::Composite(c), Default::default());
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
        // substituted to [u8; len]
        ast::Type::Bytes(n) => resolve_ast(
            &ast::Type::Array(
                Box::new(ast::Type::Uint(8)),
                vec![ArrayLength::Fixed(BigInt::from(*n as i8))],
            ),
            ns,
            registry,
        ),
        // substituted to Vec<u8>
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
            let variants = decl
                .values
                .iter()
                .enumerate()
                .map(|(idx, (k, _))| Variant {
                    name: k.clone(),
                    fields: Default::default(),
                    index: idx as u8,
                    docs: Default::default(),
                })
                .collect::<Vec<_>>();
            let variant = TypeDef::Variant(TypeDefVariant::new(variants));
            let path = path!(&decl.name);
            let ty = Type::new(path, vec![], variant, Default::default());
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
            let composite = TypeDef::Composite(TypeDefComposite::new(fields));
            let path = path!("ExternalFunction");
            let ty = Type::new(path, vec![], composite, Default::default());
            registry.register_type(ty)
        }
        ast::Type::UserType(no) => {
            let decl = &ns.user_types[*no];
            let resolved = resolve_ast(&decl.ty, ns, registry);
            match (decl.name.as_ref(), decl.loc) {
                // Builtin Hash type from ink primitives
                ("Hash", pt::Loc::Builtin) => {
                    let field = Field::new(None, resolved.into(), None, vec![]);
                    let composite = TypeDef::Composite(TypeDefComposite::new([field]));
                    let path = path!("ink_env", "types", "Hash");
                    registry.register_type(Type::new(path, vec![], composite, vec![]))
                }
                _ => resolved,
            }
        }
        ast::Type::Mapping(ast::Mapping { key, value, .. }) => {
            resolve_ast(key, ns, registry);
            resolve_ast(value, ns, registry)
        }
        _ => unreachable!(),
    }
}

/// Recursively build the storage layout after all types are registered
fn type_to_storage_layout(
    key: u32,
    root: LayoutKey,
    registry: &PortableRegistryBuilder,
) -> Layout<PortableForm> {
    let ty = registry.get(key).unwrap();
    match &ty.type_def {
        TypeDef::Composite(inner) => Layout::Struct(StructLayout::new(
            ty.path.ident().unwrap_or_default(),
            inner.fields.iter().map(|field| {
                FieldLayout::new(
                    field.name.clone().unwrap_or_default(),
                    type_to_storage_layout(field.ty.id, root, registry),
                )
            }),
        )),
        _ => Layout::Leaf(LeafLayout::new(root, key.into())),
    }
}

/// Generate `InkProject` from `ast::Type` and `ast::Namespace`
pub fn gen_project(contract_no: usize, ns: &ast::Namespace) -> InkProject {
    let mut registry = PortableRegistryBuilder::new();

    // This is only used by off-chain tooling. At the moment there is no such tooling available yet.
    // So it is not exactly clear yet what this should look like.
    // For now it just contains all root layouts (you get all storage keys in use).
    let fields: Vec<FieldLayout<PortableForm>> = ns.contracts[contract_no]
        .layout
        .iter()
        .filter_map(|layout| {
            let var = &ns.contracts[layout.contract_no].variables[layout.var_no];
            if let Some(slot) = layout.slot.to_u32() {
                let ty = resolve_ast(&layout.ty, ns, &mut registry);
                let layout_key = LayoutKey::new(slot);
                let root = RootLayout::new(
                    layout_key,
                    type_to_storage_layout(ty, layout_key, &registry),
                );
                Some(FieldLayout::new(var.name.clone(), root))
            } else {
                None
            }
        })
        .collect();
    let contract_name = ns.contracts[contract_no].name.clone();
    let storage = Layout::Struct(StructLayout::new(contract_name, fields));

    let constructor_spec = |f: &Function| -> ConstructorSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));
        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);

                let path = registry.get(ty).unwrap().path.clone();
                let spec = TypeSpec::new(ty.into(), path);

                MessageParamSpec::new(p.name_as_str().to_string())
                    .of_type(spec)
                    .done()
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();

        ConstructorSpec::from_label(if f.name.is_empty() { "new" } else { &f.name }.into())
            .selector(f.selector(ns, &contract_no).try_into().unwrap())
            .payable(payable)
            .args(args)
            .docs(vec![render(&f.tags).as_str()])
            .returns(ReturnTypeSpec::new(None))
            .done()
    };

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
        .map(constructor_spec)
        .collect::<Vec<ConstructorSpec<PortableForm>>>();

    let message_spec = |f: &Function| -> MessageSpec<PortableForm> {
        let payable = matches!(f.mutability, ast::Mutability::Payable(_));
        let mutates = matches!(
            f.mutability,
            ast::Mutability::Payable(_) | ast::Mutability::Nonpayable(_)
        );
        let ret_spec: Option<TypeSpec<PortableForm>> = match f.returns.len() {
            0 => None,
            1 => {
                let ty = resolve_ast(&f.returns[0].ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path.clone();
                Some(TypeSpec::new(ty.into(), path))
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

                let path = path!(&ns.contracts[contract_no].name, &f.name, "return_type");

                let ty = registry.register_type(Type::new(
                    path,
                    vec![],
                    TypeDef::Tuple(t),
                    Default::default(),
                ));
                let path = registry.get(ty).unwrap().path.clone();
                Some(TypeSpec::new(ty.into(), path))
            }
        };
        let ret_type = ReturnTypeSpec::new(ret_spec);
        let args = f
            .params
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path.clone();
                let spec = TypeSpec::new(ty.into(), path);

                MessageParamSpec::new(p.name_as_str().to_string())
                    .of_type(spec)
                    .done()
            })
            .collect::<Vec<MessageParamSpec<PortableForm>>>();
        let label = if f.mangled_name_contracts.contains(&contract_no) {
            &f.mangled_name
        } else {
            &f.name
        };
        MessageSpec::from_label(label.into())
            .selector(f.selector(ns, &contract_no).try_into().unwrap())
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
            // libraries are never in the public interface
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
        .map(message_spec)
        .collect::<Vec<MessageSpec<PortableForm>>>();

    let mut event_spec = |e: &EventDecl| -> EventSpec<PortableForm> {
        let args = e
            .fields
            .iter()
            .map(|p| {
                let ty = resolve_ast(&p.ty, ns, &mut registry);
                let path = registry.get(ty).unwrap().path.clone();
                let spec = TypeSpec::new(ty.into(), path);
                EventParamSpec::new(p.name_as_str().into())
                    .of_type(spec)
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
        .emits_events
        .iter()
        .map(|event_no| {
            let event = &ns.events[*event_no];
            event_spec(event)
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
    let source = Source::new(
        Some(source_wasm),
        CodeHash(code_hash),
        language,
        compiler,
        None,
    );

    let mut builder = Contract::builder();
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
    builder.version(Version::new(0, 0, 1));
    let contract = builder.build().unwrap();

    let project_json = serde_json::to_value(gen_project(contract_no, ns)).unwrap();
    let abi = serde_json::from_value(project_json).unwrap();

    serde_json::to_value(ContractMetadata::new(source, contract, None, abi)).unwrap()
}
