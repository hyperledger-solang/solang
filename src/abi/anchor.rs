// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{
    ArrayLength, Contract, Function, Namespace, Parameter, StructDecl, StructType, Tag, Type,
};
use anchor_syn::idl::types::{
    Idl, IdlAccount, IdlAccountItem, IdlEnumVariant, IdlEvent, IdlEventField, IdlField,
    IdlInstruction, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use base58::ToBase58;
use num_traits::ToPrimitive;
use semver::Version;
use std::collections::{HashMap, HashSet};

use convert_case::{Boundary, Case, Casing};
use serde_json::json;
use sha2::{Digest, Sha256};
use solang_parser::pt::FunctionTy;

/// Generate discriminator based on the name of the function. This is the 8 byte
/// value anchor uses to dispatch function calls on. This should match
/// anchor's behaviour - we need to match the discriminator exactly
pub fn function_discriminator(name: &str) -> Vec<u8> {
    // must match snake-case npm library, see
    // https://github.com/coral-xyz/anchor/blob/master/ts/packages/anchor/src/coder/borsh/instruction.ts#L389
    let normalized = name
        .from_case(Case::Camel)
        .without_boundaries(&[Boundary::LowerDigit])
        .to_case(Case::Snake);
    discriminator("global", &normalized)
}

/// Generate discriminator based on the name of the event. This is the 8 byte
/// value anchor uses for events. This should match anchor's behaviour,
///  generating the same discriminator
pub fn event_discriminator(name: &str) -> Vec<u8> {
    discriminator("event", name)
}

fn discriminator(namespace: &'static str, name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(format!("{namespace}:{name}"));
    hasher.finalize()[..8].to_vec()
}

/// Generate an Anchor IDL for a Solidity contract.
pub fn generate_anchor_idl(contract_no: usize, ns: &Namespace, contract_version: &str) -> Idl {
    let contract = &ns.contracts[contract_no];
    let docs = idl_docs(&contract.tags);
    let mut type_manager = TypeManager::new(ns, contract_no);

    let instructions = idl_instructions(contract_no, contract, &mut type_manager, ns);

    let events = idl_events(contract, &mut type_manager, ns);

    let metadata = contract
        .program_id
        .as_ref()
        .map(|id| json!({"address": id.to_base58()}));

    Idl {
        version: Version::parse(contract_version).unwrap().to_string(),
        name: ns.contracts[contract_no].id.name.clone(),
        docs,
        constants: vec![],
        instructions,
        accounts: vec![],
        types: type_manager.generate_custom_idl_types(),
        events,
        errors: None,
        metadata,
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
            let mut dedup = Deduplicate::new("field".to_owned());
            for item in &def.fields {
                let name = dedup.unique_name(item);
                fields.push(IdlEventField {
                    name,
                    ty: type_manager.convert(&item.ty),
                    index: item.indexed,
                });
            }

            events.push(IdlEvent {
                name: def.id.name.clone(),
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

    if contract.constructors(ns).is_empty() {
        instructions.push(IdlInstruction {
            name: "new".to_string(),
            docs: None,
            accounts: vec![IdlAccountItem::IdlAccount(IdlAccount {
                name: "dataAccount".to_string(),
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
        if !ns.function_externally_callable(contract_no, Some(*func_no))
            || matches!(
                ns.functions[*func_no].ty,
                FunctionTy::Fallback | FunctionTy::Receive | FunctionTy::Modifier
            )
        {
            continue;
        }

        let func = &ns.functions[*func_no];
        let tags = idl_docs(&func.tags);

        let mut args: Vec<IdlField> = Vec::with_capacity(func.params.len());
        let mut dedup = Deduplicate::new("arg".to_owned());
        for item in &*func.params {
            let name = dedup.unique_name(item);
            let normalized = name
                .from_case(Case::Snake)
                .without_boundaries(&[Boundary::LowerDigit])
                .to_case(Case::Camel);

            args.push(IdlField {
                name: normalized,
                docs: None,
                ty: type_manager.convert(&item.ty),
            });
        }

        let name = if func.is_constructor() {
            "new".to_string()
        } else if func.mangled_name_contracts.contains(&contract_no) {
            func.mangled_name.clone()
        } else {
            func.id.name.clone()
        };

        let accounts = func
            .solana_accounts
            .borrow()
            .iter()
            .map(|(account_name, account)| {
                IdlAccountItem::IdlAccount(IdlAccount {
                    name: account_name.clone(),
                    is_mut: account.is_writer,
                    is_signer: account.is_signer,
                    is_optional: Some(false),
                    docs: None,
                    pda: None,
                    relations: vec![],
                })
            })
            .collect::<Vec<IdlAccountItem>>();

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
    contract_no: usize,
    added_types: HashSet<Type>,
    /// This is a mapping between the IDL type and the tuple
    /// (index into the types vector, is the type from the current contract?, original type name)
    added_names: HashMap<String, (usize, Option<String>, String)>,
    returns_structs: Vec<IdlTypeDefinition>,
    types: Vec<IdlTypeDefinition>,
}

impl TypeManager<'_> {
    fn new(ns: &Namespace, contract_no: usize) -> TypeManager {
        TypeManager {
            namespace: ns,
            added_types: HashSet::new(),
            added_names: HashMap::new(),
            types: Vec::new(),
            returns_structs: Vec::new(),
            contract_no,
        }
    }

    /// Functions with multiple returns must return a struct in Anchor, so we build a return
    /// struct containing all the returned types.
    fn build_struct_for_return(&mut self, func: &Function, effective_name: &String) -> IdlType {
        let mut fields: Vec<IdlField> = Vec::with_capacity(func.returns.len());
        let mut dedup = Deduplicate::new("return".to_owned());
        for item in &*func.returns {
            let name = dedup.unique_name(item);

            fields.push(IdlField {
                name,
                docs: None,
                ty: self.convert(&item.ty),
            });
        }

        let name = format!("{effective_name}_returns");
        self.returns_structs.push(IdlTypeDefinition {
            name: name.clone(),
            docs: Some(vec![format!(
                "Data structure to hold the multiple returns of function {}",
                func.id
            )]),
            ty: IdlTypeDefinitionTy::Struct { fields },
            generics: None,
        });

        IdlType::Defined(name)
    }

    /// This function creates an unique name for either a custom struct or enum.
    fn unique_custom_type_name(&mut self, type_name: &String, contract: &Option<String>) -> String {
        let (idx, other_contract, real_name) =
            if let Some((idx, other_contract, real_name)) = self.added_names.get(type_name) {
                (*idx, other_contract.clone(), real_name.clone())
            } else {
                return type_name.clone();
            };

        // If the existing type was declared outside a contract or if it is from the current contract,
        // we should change the name of the type we are adding now.
        if other_contract.is_none()
            || other_contract.as_ref().unwrap()
                == &self.namespace.contracts[self.contract_no].id.name
        {
            let new_name = if let Some(this_name) = contract {
                format!("{this_name}_{type_name}")
            } else {
                type_name.clone()
            };
            self.unique_string(new_name)
        } else {
            // If the type we are adding now belongs to the current contract, we change the name
            // of a previously added IDL type
            let new_other_name = if let Some(other_name) = &other_contract {
                format!("{other_name}_{real_name}")
            } else {
                format!("_{real_name}")
            };
            let unique_name = self.unique_string(new_other_name);
            self.types[idx].name.clone_from(&unique_name);
            self.added_names
                .insert(unique_name, (idx, other_contract, real_name));
            type_name.clone()
        }
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

        let name = self.unique_custom_type_name(&def.id.name, &def.contract);

        self.added_names.insert(
            name.clone(),
            (self.types.len(), def.contract.clone(), def.id.name.clone()),
        );

        self.types.push(IdlTypeDefinition {
            name,
            docs,
            ty: IdlTypeDefinitionTy::Struct { fields },
            generics: None,
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
                generics: None,
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

        let name = self.unique_custom_type_name(&def.id.name, &def.contract);
        self.added_names.insert(
            name.clone(),
            (self.types.len(), def.contract.clone(), def.id.name.clone()),
        );

        let variants = def
            .values
            .iter()
            .map(|(name, _)| IdlEnumVariant {
                name: name.clone(),
                fields: None,
            })
            .collect::<Vec<IdlEnumVariant>>();

        self.types.push(IdlTypeDefinition {
            name,
            docs,
            ty: IdlTypeDefinitionTy::Enum { variants },
            generics: None,
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
                IdlType::Defined(def.id.name.clone())
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
                IdlType::Defined(self.namespace.enums[*enum_no].id.name.clone())
            }
            Type::ExternalFunction { .. } => {
                self.convert(&Type::Struct(StructType::ExternalFunction))
            }
            Type::UserType(type_no) => self.convert(&self.namespace.user_types[*type_no].ty),
            _ => unreachable!("Type should not be in the IDL"),
        }
    }

    /// This function ensures that the string we are generating is unique given the names we have in
    /// self.added_names
    fn unique_string(&mut self, name: String) -> String {
        let mut num = 0;
        let mut unique_name = name.clone();
        while self.added_names.contains_key(&unique_name) {
            num += 1;
            unique_name = format!("{name}_{num}");
        }

        unique_name
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

struct Deduplicate {
    prefix: String,
    counter: u16,
    existing_names: HashSet<String>,
}

impl Deduplicate {
    fn new(prefix: String) -> Deduplicate {
        Deduplicate {
            prefix,
            counter: 0,
            existing_names: HashSet::new(),
        }
    }

    fn unique_name(&mut self, param: &Parameter<Type>) -> String {
        if param.id.is_none() || param.id.as_ref().unwrap().name.is_empty() {
            self.try_prefix()
        } else {
            let mut name = param.id.as_ref().unwrap().name.clone();
            self.try_name(&mut name);
            name
        }
    }

    fn try_prefix(&mut self) -> String {
        let mut candidate = format!("{}_{}", self.prefix, self.counter);
        while self.existing_names.contains(&candidate) {
            self.counter += 1;
            candidate = format!("{}_{}", self.prefix, self.counter);
        }
        self.existing_names.insert(candidate.clone());
        candidate
    }

    fn try_name(&mut self, candidate: &mut String) {
        let mut counter = 0;
        let prefix = candidate.clone();
        while self.existing_names.contains(candidate) {
            counter += 1;
            *candidate = format!("{prefix}_{counter}");
        }
        self.existing_names.insert(candidate.clone());
    }
}
