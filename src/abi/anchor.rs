// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{
    ArrayLength, Contract, Function, Mutability, Namespace, StructDecl, StructType, Tag, Type,
};
use anchor_syn::idl::{
    Idl, IdlAccount, IdlAccountItem, IdlEnumVariant, IdlEvent, IdlEventField, IdlField,
    IdlInstruction, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use num_traits::ToPrimitive;
use semver::Version;
use std::collections::HashSet;

use convert_case::{Boundary, Case, Casing};
use sha2::{Digest, Sha256};

/// Generate discriminator based on the name of the function. This is the 8 byte
/// value anchor uses to dispatch function calls on. This should match
/// anchor's behaviour - we need to match the discriminator exactly
pub fn discriminator(namespace: &'static str, name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    // must match snake-case npm library, see
    // https://github.com/coral-xyz/anchor/blob/master/ts/packages/anchor/src/coder/borsh/instruction.ts#L389
    let normalized = name
        .from_case(Case::Camel)
        .without_boundaries(&[Boundary::LowerDigit])
        .to_case(Case::Snake);
    hasher.update(format!("{}:{}", namespace, normalized));
    hasher.finalize()[..8].to_vec()
}

/// Generate an Anchor IDL for a Solidity contract.
pub fn generate_anchor_idl(contract_no: usize, ns: &Namespace) -> Idl {
    let contract = &ns.contracts[contract_no];
    let docs = idl_docs(&contract.tags);
    let mut type_manager = TypeManager::new(ns);

    let instructions = idl_instructions(contract_no, contract, &mut type_manager, ns);

    let events = idl_events(contract, &mut type_manager, ns);

    Idl {
        version: Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap()
            .to_string(),
        name: ns.contracts[contract_no].name.clone(),
        docs,
        constants: vec![],
        instructions,
        state: None,
        accounts: vec![],
        types: type_manager.generate_custom_idl_types(),
        events,
        errors: None,
        metadata: None,
    }
}

/// Generate IDL events for a contract.
fn idl_events(
    contract: &Contract,
    type_manager: &mut TypeManager,
    ns: &Namespace,
) -> Option<Vec<IdlEvent>> {
    if contract.emits_events.is_empty() {
        None
    } else {
        let mut events: Vec<IdlEvent> = Vec::with_capacity(contract.emits_events.len());
        for event_no in &contract.emits_events {
            let def = &ns.events[*event_no];
            let mut fields: Vec<IdlEventField> = Vec::with_capacity(def.fields.len());
            for (item_no, item) in def.fields.iter().enumerate() {
                let name = if item.id.is_none() {
                    format!("field_{}", item_no)
                } else {
                    item.name_as_str().to_string()
                };

                fields.push(IdlEventField {
                    name,
                    ty: type_manager.convert(&item.ty),
                    index: item.indexed,
                });
            }

            events.push(IdlEvent {
                name: def.name.clone(),
                fields,
            });
        }

        Some(events)
    }
}

/// Generate the IDL instructions for a contract.
fn idl_instructions(
    contract_no: usize,
    contract: &Contract,
    type_manager: &mut TypeManager,
    ns: &Namespace,
) -> Vec<IdlInstruction> {
    let mut instructions: Vec<IdlInstruction> = Vec::new();

    if !contract.have_constructor(ns) {
        instructions.push(IdlInstruction {
            name: "new".to_string(),
            docs: None,
            accounts: vec![IdlAccountItem::IdlAccount(IdlAccount {
                name: "data_account".to_string(),
                is_mut: true,
                is_signer: false,
                is_optional: Some(false),
                docs: None,
                pda: None,
                relations: vec![],
            })],
            args: vec![],
            returns: None,
        })
    }

    for func_no in contract.all_functions.keys() {
        if !ns.functions[*func_no].is_public() {
            continue;
        }

        let func = &ns.functions[*func_no];
        let tags = idl_docs(&func.tags);

        let accounts = match &func.mutability {
            Mutability::Pure(_) => {
                vec![]
            }
            Mutability::View(_) => {
                vec![IdlAccountItem::IdlAccount(IdlAccount {
                    name: "data_account".to_string(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: Some(false),
                    docs: None,
                    pda: None,
                    relations: vec![],
                })]
            }
            _ => {
                vec![IdlAccountItem::IdlAccount(IdlAccount {
                    name: "data_account".to_string(),
                    is_mut: true,
                    is_signer: false,
                    is_optional: Some(false),
                    docs: None,
                    pda: None,
                    relations: vec![],
                })]
            }
        };

        let mut args: Vec<IdlField> = Vec::with_capacity(func.params.len());
        for (item_no, item) in func.params.iter().enumerate() {
            let name = if item.id.is_none() || item.id.as_ref().unwrap().name.is_empty() {
                format!("arg_{}", item_no)
            } else {
                item.id.as_ref().unwrap().name.clone()
            };
            args.push(IdlField {
                name,
                docs: None,
                ty: type_manager.convert(&item.ty),
            });
        }

        let name = if func.is_constructor() {
            "new".to_string()
        } else if func.mangled_name_contracts.contains(&contract_no) {
            func.mangled_name.clone()
        } else {
            func.name.clone()
        };

        let returns = if func.returns.is_empty() {
            None
        } else if func.returns.len() == 1 {
            Some(type_manager.convert(&func.returns[0].ty))
        } else {
            Some(type_manager.build_struct_for_return(func, &name))
        };

        instructions.push(IdlInstruction {
            name,
            docs: tags,
            accounts,
            args,
            returns,
        });
    }

    instructions
}

/// This struct accounts all the user defined types used in the contract that need to be present
/// in the IDL 'types' field.
struct TypeManager<'a> {
    namespace: &'a Namespace,
    added_types: HashSet<Type>,
    returns_structs: Vec<IdlTypeDefinition>,
    types: Vec<IdlTypeDefinition>,
}

impl TypeManager<'_> {
    fn new(ns: &Namespace) -> TypeManager {
        TypeManager {
            namespace: ns,
            added_types: HashSet::new(),
            types: Vec::new(),
            returns_structs: Vec::new(),
        }
    }

    /// Functions with multiple returns must return a struct in Anchor, so we build a return
    /// struct containing all the returned types.
    fn build_struct_for_return(&mut self, func: &Function, effective_name: &String) -> IdlType {
        let mut fields: Vec<IdlField> = Vec::with_capacity(func.returns.len());
        for (item_no, item) in func.returns.iter().enumerate() {
            let name = if let Some(id) = &item.id {
                id.name.clone()
            } else {
                format!("return_{}", item_no)
            };

            fields.push(IdlField {
                name,
                docs: None,
                ty: self.convert(&item.ty),
            });
        }

        let name = format!("{}_returns", effective_name);
        self.returns_structs.push(IdlTypeDefinition {
            name: name.clone(),
            docs: Some(vec![format!(
                "Data structure to hold the multiple returns of function {}",
                func.name
            )]),
            ty: IdlTypeDefinitionTy::Struct { fields },
        });

        IdlType::Defined(name)
    }

    /// Add a struct definition to the TypeManager
    fn add_struct_definition(&mut self, def: &StructDecl, ty: &Type) {
        if self.added_types.contains(ty) {
            return;
        }
        self.added_types.insert(ty.clone());

        let docs = idl_docs(&def.tags);

        let mut fields: Vec<IdlField> = Vec::with_capacity(def.fields.len());
        for item in &def.fields {
            fields.push(IdlField {
                name: item.name_as_str().to_string(),
                docs: None,
                ty: self.convert(&item.ty),
            });
        }

        self.types.push(IdlTypeDefinition {
            name: def.name.clone(),
            docs,
            ty: IdlTypeDefinitionTy::Struct { fields },
        });
    }

    /// This function ensures there are no name collisions on the structs created for the functions
    /// with multiple returns, before returning all the custom types needed for the IDL file.
    fn generate_custom_idl_types(self) -> Vec<IdlTypeDefinition> {
        let mut custom_types = self.types;
        let mut used_names: HashSet<String> = custom_types
            .iter()
            .map(|e| e.name.clone())
            .collect::<HashSet<String>>();

        for item in self.returns_structs {
            let mut value = 0;
            let mut name = item.name.clone();
            while used_names.contains(&name) {
                value += 1;
                name = format!("{}_{}", item.name, value);
            }
            used_names.insert(name.clone());
            custom_types.push(IdlTypeDefinition {
                name,
                docs: item.docs,
                ty: item.ty,
            });
        }

        custom_types
    }

    /// Add an enum definition to the TypeManager
    fn add_enum_definition(&mut self, enum_no: usize, ty: &Type) {
        if self.added_types.contains(ty) {
            return;
        }
        self.added_types.insert(ty.clone());
        let def = &self.namespace.enums[enum_no];

        let docs = idl_docs(&def.tags);

        let variants = def
            .values
            .iter()
            .map(|(name, _)| IdlEnumVariant {
                name: name.clone(),
                fields: None,
            })
            .collect::<Vec<IdlEnumVariant>>();

        self.types.push(IdlTypeDefinition {
            name: def.name.clone(),
            docs,
            ty: IdlTypeDefinitionTy::Enum { variants },
        });
    }

    /// Convert for AST Type to IDL Type
    fn convert(&mut self, ast_type: &Type) -> IdlType {
        match ast_type {
            Type::Bool => IdlType::Bool,
            Type::Int(n) => match *n {
                0..=8 => IdlType::I8,
                9..=16 => IdlType::I16,
                17..=32 => IdlType::I32,
                33..=64 => IdlType::I64,
                65..=128 => IdlType::I128,
                129..=256 => IdlType::I256,
                _ => unreachable!("Integers wider than 256 bits are not supported"),
            },
            Type::Uint(n) => match *n {
                0..=8 => IdlType::U8,
                9..=16 => IdlType::U16,
                17..=32 => IdlType::U32,
                33..=64 => IdlType::U64,
                65..=128 => IdlType::U128,
                129..=256 => IdlType::U256,
                _ => unreachable!("Unsigned integers wider than 256 bits are not supported"),
            },
            Type::DynamicBytes => IdlType::Bytes,
            Type::String => IdlType::String,
            Type::Address(_) | Type::Contract(_) => IdlType::PublicKey,
            Type::Struct(struct_type) => {
                let def = struct_type.definition(self.namespace);
                self.add_struct_definition(def, ast_type);
                IdlType::Defined(def.name.clone())
            }
            Type::Array(ty, dims) => {
                let mut idl_type = self.convert(ty);
                for item in dims {
                    match item {
                        ArrayLength::Fixed(number) => {
                            idl_type =
                                IdlType::Array(Box::new(idl_type), number.to_usize().unwrap());
                        }
                        ArrayLength::Dynamic => {
                            idl_type = IdlType::Vec(Box::new(idl_type));
                        }
                        ArrayLength::AnyFixed => {
                            unreachable!("A parameter cannot have an AnyFixed dimension")
                        }
                    }
                }
                idl_type
            }
            Type::Bytes(dim) => IdlType::Array(Box::new(IdlType::U8), *dim as usize),
            Type::Enum(enum_no) => {
                self.add_enum_definition(*enum_no, ast_type);
                IdlType::Defined(self.namespace.enums[*enum_no].name.clone())
            }
            Type::ExternalFunction { .. } => {
                self.convert(&Type::Struct(StructType::ExternalFunction))
            }
            Type::UserType(type_no) => self.convert(&self.namespace.user_types[*type_no].ty),
            _ => unreachable!("Type should not be in the IDL"),
        }
    }
}

/// Prepare the docs from doc comments.
fn idl_docs(tags: &[Tag]) -> Option<Vec<String>> {
    if tags.is_empty() {
        None
    } else {
        Some(
            tags.iter()
                .map(|tag| format!("{}: {}", tag.tag, tag.value))
                .collect::<Vec<String>>(),
        )
    }
}
