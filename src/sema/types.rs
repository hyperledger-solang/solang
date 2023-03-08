// SPDX-License-Identifier: Apache-2.0

use super::tags::resolve_tags;
use super::{annotions_not_allowed, ast, SourceUnit, SOLANA_BUCKET_SIZE};
use super::{
    ast::{
        ArrayLength, Contract, Diagnostic, EnumDecl, EventDecl, Mapping, Namespace, Parameter,
        StructDecl, StructType, Symbol, Tag, Type, UserTypeDecl,
    },
    diagnostics::Diagnostics,
    ContractDefinition, SOLANA_SPARSE_ARRAY_SIZE,
};
use crate::Target;
use base58::{FromBase58, FromBase58Error};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use solang_parser::{doccomment::DocComment, pt, pt::CodeLocation};
use std::collections::HashSet;
use std::{fmt::Write, ops::Mul};

/// List the types which should be resolved later
pub struct ResolveFields<'a> {
    structs: Vec<ResolveStructFields<'a>>,
    events: Vec<ResolveEventFields<'a>>,
}

struct ResolveEventFields<'a> {
    event_no: usize,
    pt: &'a pt::EventDefinition,
    comments: Vec<DocComment>,
    contract: Option<usize>,
}

struct ResolveStructFields<'a> {
    struct_no: usize,
    pt: &'a pt::StructDefinition,
    comments: Vec<DocComment>,
    contract: Option<usize>,
}

/// Resolve all the types we can find (enums, structs, contracts). structs can have other
/// structs as fields, including ones that have not been declared yet.
pub fn resolve_typenames<'a>(
    tree: &'a SourceUnit,
    file_no: usize,
    ns: &mut Namespace,
) -> ResolveFields<'a> {
    let mut delay = ResolveFields {
        structs: Vec::new(),
        events: Vec::new(),
    };

    for item in &tree.items {
        match &item.part {
            pt::SourceUnitPart::EnumDefinition(def) => {
                annotions_not_allowed(&item.annotations, "enum", ns);

                let _ = enum_decl(def, file_no, &item.doccomments, None, ns);
            }
            pt::SourceUnitPart::StructDefinition(def) => {
                annotions_not_allowed(&item.annotations, "struct", ns);

                let struct_no = ns.structs.len();

                if ns.add_symbol(
                    file_no,
                    None,
                    def.name.as_ref().unwrap(),
                    Symbol::Struct(
                        def.name.as_ref().unwrap().loc,
                        StructType::UserDefined(struct_no),
                    ),
                ) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: def.name.as_ref().unwrap().name.to_owned(),
                        loc: def.name.as_ref().unwrap().loc,
                        contract: None,
                        fields: Vec::new(),
                        offsets: Vec::new(),
                        storage_offsets: Vec::new(),
                    });

                    delay.structs.push(ResolveStructFields {
                        struct_no,
                        pt: def,
                        comments: item.doccomments.clone(),
                        contract: None,
                    });
                }
            }
            pt::SourceUnitPart::EventDefinition(def) => {
                annotions_not_allowed(&item.annotations, "event", ns);

                let event_no = ns.events.len();

                if let Some(Symbol::Event(events)) = ns.variable_symbols.get_mut(&(
                    file_no,
                    None,
                    def.name.as_ref().unwrap().name.to_owned(),
                )) {
                    events.push((def.name.as_ref().unwrap().loc, event_no));
                } else if !ns.add_symbol(
                    file_no,
                    None,
                    def.name.as_ref().unwrap(),
                    Symbol::Event(vec![(def.name.as_ref().unwrap().loc, event_no)]),
                ) {
                    continue;
                }

                ns.events.push(EventDecl {
                    tags: Vec::new(),
                    name: def.name.as_ref().unwrap().name.to_owned(),
                    loc: def.name.as_ref().unwrap().loc,
                    contract: None,
                    fields: Vec::new(),
                    anonymous: def.anonymous,
                    signature: String::new(),
                    used: false,
                });

                delay.events.push(ResolveEventFields {
                    event_no,
                    pt: def,
                    comments: item.doccomments.clone(),
                    contract: None,
                });
            }
            pt::SourceUnitPart::TypeDefinition(ty) => {
                annotions_not_allowed(&item.annotations, "type", ns);

                type_decl(ty, file_no, &item.doccomments, None, ns);
            }
            _ => (),
        }
    }

    for contract in &tree.contracts {
        resolve_contract(contract, file_no, &mut delay, ns);
    }

    delay
}

fn type_decl(
    def: &pt::TypeDefinition,
    file_no: usize,
    tags: &[DocComment],
    contract_no: Option<usize>,
    ns: &mut Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    let mut ty = match ns.resolve_type(file_no, contract_no, false, &def.ty, &mut diagnostics) {
        Ok(ty) => ty,
        Err(_) => {
            ns.diagnostics.extend(diagnostics);
            return;
        }
    };

    // We could permit all types to be defined here, however:
    // - This would require resolving the types definition after all other types are resolved
    // - Need for circular checks (type a is b; type b is a;)
    if !matches!(
        ty,
        Type::Address(_) | Type::Bool | Type::Int(_) | Type::Uint(_) | Type::Bytes(_)
    ) {
        ns.diagnostics.push(Diagnostic::error(
            def.ty.loc(),
            format!("'{}' is not an elementary value type", ty.to_string(ns)),
        ));
        ty = Type::Unresolved;
    }

    let pos = ns.user_types.len();

    if !ns.add_symbol(
        file_no,
        contract_no,
        &def.name,
        Symbol::UserType(def.name.loc, pos),
    ) {
        return;
    }

    let tags = resolve_tags(def.name.loc.file_no(), "type", tags, None, None, None, ns);

    ns.user_types.push(UserTypeDecl {
        tags,
        loc: def.loc,
        name: def.name.name.to_string(),
        ty,
        contract: contract_no.map(|no| ns.contracts[no].name.to_string()),
    });
}

/// Check if a struct contains itself. This function calls itself recursively.
fn find_struct_recursion(struct_no: usize, structs_visited: &mut Vec<usize>, ns: &mut Namespace) {
    let def = ns.structs[struct_no].clone();
    let mut types_seen: HashSet<usize> = HashSet::new();

    for (field_no, field) in def.fields.iter().enumerate() {
        let field_struct_no = match &field.ty {
            Type::Mapping(m) => match m.value.as_ref() {
                Type::Struct(StructType::UserDefined(n)) => *n,
                _ => continue,
            },
            Type::Struct(StructType::UserDefined(n)) => *n,
            Type::Array(ty, dim) => match (ty.as_ref(), dim.last()) {
                (Type::Struct(StructType::UserDefined(n)), Some(_)) => *n,
                _ => continue,
            },
            _ => continue,
        };

        if !types_seen.insert(field_struct_no) {
            continue;
        }

        if structs_visited.contains(&field_struct_no) {
            ns.structs[struct_no].fields[field_no].recursive = true;
            match &field.ty {
                Type::Struct(StructType::UserDefined(_)) => {}
                Type::Array(ty, dim) => match (ty.as_ref(), dim.last()) {
                    (Type::Struct(StructType::UserDefined(_)), Some(ArrayLength::Fixed(_))) => {}
                    _ => continue,
                },
                _ => continue,
            }
            ns.diagnostics.push(Diagnostic::error_with_note(
                def.loc,
                format!("struct '{}' has infinite size", def.name),
                field.loc,
                format!("recursive field '{}'", field.name_as_str()),
            ));
            ns.structs[struct_no].fields[field_no].infinite_size = true;
        } else {
            structs_visited.push(field_struct_no);
            find_struct_recursion(field_struct_no, structs_visited, ns);
            structs_visited.pop();
        }
    }
}

pub fn resolve_fields(delay: ResolveFields, file_no: usize, ns: &mut Namespace) {
    // now we can resolve the fields for the structs
    for resolve in delay.structs {
        let (tags, fields) =
            struct_decl(resolve.pt, file_no, &resolve.comments, resolve.contract, ns);

        ns.structs[resolve.struct_no].tags = tags;
        ns.structs[resolve.struct_no].fields = fields;
    }

    // struct can contain other structs, and we have to check for recursiveness,
    // i.e. "struct a { b f1; } struct b { a f1; }"
    (0..ns.structs.len())
        .for_each(|struct_no| find_struct_recursion(struct_no, &mut vec![struct_no], ns));

    // Calculate the offset of each field in all the struct types
    struct_offsets(ns);

    // now we can resolve the fields for the events
    for event in delay.events {
        let (tags, fields) = event_decl(event.pt, file_no, &event.comments, event.contract, ns);

        ns.events[event.event_no].signature =
            ns.signature(&ns.events[event.event_no].name, &fields);
        ns.events[event.event_no].fields = fields;
        ns.events[event.event_no].tags = tags;
    }
}

/// Resolve all the types in a contract
fn resolve_contract<'a>(
    def: &'a ContractDefinition,
    file_no: usize,
    delay: &mut ResolveFields<'a>,
    ns: &mut Namespace,
) -> bool {
    let name = def.name.as_ref().unwrap();

    let contract_no = def.contract_no;

    let doc = resolve_tags(
        name.loc.file_no(),
        "contract",
        &def.doccomments,
        None,
        None,
        None,
        ns,
    );

    ns.contracts
        .push(Contract::new(&name.name, def.ty.clone(), doc, def.loc));

    contract_annotations(contract_no, &def.annotations, ns);

    let mut broken = !ns.add_symbol(
        file_no,
        None,
        def.name.as_ref().unwrap(),
        Symbol::Contract(def.loc, contract_no),
    );

    if is_windows_reserved(&def.name.as_ref().unwrap().name) {
        ns.diagnostics.push(Diagnostic::error(
            def.name.as_ref().unwrap().loc,
            format!(
                "contract name '{}' is reserved file name on Windows",
                def.name.as_ref().unwrap().name
            ),
        ));
    }

    for parts in &def.parts {
        match parts.part {
            pt::ContractPart::EnumDefinition(ref e) => {
                annotions_not_allowed(&parts.annotations, "enum", ns);

                if !enum_decl(e, file_no, &parts.doccomments, Some(contract_no), ns) {
                    broken = true;
                }
            }
            pt::ContractPart::StructDefinition(ref pt) => {
                annotions_not_allowed(&parts.annotations, "struct", ns);

                let struct_no = ns.structs.len();

                if ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    pt.name.as_ref().unwrap(),
                    Symbol::Struct(
                        pt.name.as_ref().unwrap().loc,
                        StructType::UserDefined(struct_no),
                    ),
                ) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: pt.name.as_ref().unwrap().name.to_owned(),
                        loc: pt.name.as_ref().unwrap().loc,
                        contract: Some(def.name.as_ref().unwrap().name.to_owned()),
                        fields: Vec::new(),
                        offsets: Vec::new(),
                        storage_offsets: Vec::new(),
                    });

                    delay.structs.push(ResolveStructFields {
                        struct_no,
                        pt,
                        comments: parts.doccomments.clone(),
                        contract: Some(contract_no),
                    });
                } else {
                    broken = true;
                }
            }
            pt::ContractPart::EventDefinition(ref pt) => {
                annotions_not_allowed(&parts.annotations, "event", ns);

                let event_no = ns.events.len();

                if let Some(Symbol::Event(events)) = ns.variable_symbols.get_mut(&(
                    file_no,
                    Some(contract_no),
                    pt.name.as_ref().unwrap().name.to_owned(),
                )) {
                    events.push((pt.name.as_ref().unwrap().loc, event_no));
                } else if !ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    pt.name.as_ref().unwrap(),
                    Symbol::Event(vec![(pt.name.as_ref().unwrap().loc, event_no)]),
                ) {
                    broken = true;
                    continue;
                }

                ns.events.push(EventDecl {
                    tags: Vec::new(),
                    name: pt.name.as_ref().unwrap().name.to_owned(),
                    loc: pt.name.as_ref().unwrap().loc,
                    contract: Some(contract_no),
                    fields: Vec::new(),
                    anonymous: pt.anonymous,
                    signature: String::new(),
                    used: false,
                });

                delay.events.push(ResolveEventFields {
                    event_no,
                    pt,
                    comments: parts.doccomments.clone(),
                    contract: Some(contract_no),
                });
            }
            pt::ContractPart::TypeDefinition(ty) => {
                annotions_not_allowed(&parts.annotations, "type", ns);

                type_decl(ty, file_no, &parts.doccomments, Some(contract_no), ns);
            }
            _ => (),
        }
    }

    broken
}

/// Resolve annotations attached to a contract
fn contract_annotations(
    contract_no: usize,
    annotations: &[&pt::Annotation],
    ns: &mut ast::Namespace,
) {
    let mut seen_program_id = None;

    for note in annotations {
        if ns.target != Target::Solana || note.id.name != "program_id" {
            ns.diagnostics.push(Diagnostic::error(
                note.loc,
                format!(
                    "unknown annotation '{}' on contract {}",
                    note.id.name, ns.contracts[contract_no].name,
                ),
            ));
            continue;
        }

        if let Some(prev_loc) = seen_program_id {
            ns.diagnostics.push(Diagnostic::error_with_note(
                note.loc,
                "duplicate program_id annotation".into(),
                prev_loc,
                "location of previous program_id annotation".into(),
            ));

            continue;
        }

        match &note.value {
            pt::Expression::StringLiteral(values) if values.len() == 1 => {
                let string = &values[0].string;
                let mut loc = values[0].loc;

                match string.from_base58() {
                    Ok(v) => {
                        if v.len() != ns.address_length {
                            ns.diagnostics.push(Diagnostic::error(
                                loc,
                                format!(
                                    "address literal {} incorrect length of {}",
                                    string,
                                    v.len()
                                ),
                            ));
                        } else {
                            seen_program_id = Some(note.loc);

                            ns.contracts[contract_no].program_id = Some(v);
                        }
                    }
                    Err(FromBase58Error::InvalidBase58Length) => {
                        ns.diagnostics.push(Diagnostic::error(
                            loc,
                            format!("address literal {string} invalid base58 length"),
                        ));
                    }
                    Err(FromBase58Error::InvalidBase58Character(ch, pos)) => {
                        if let pt::Loc::File(_, start, end) = &mut loc {
                            *start += pos + 1; // location includes quotes
                            *end = *start;
                        }
                        ns.diagnostics.push(Diagnostic::error(
                            loc,
                            format!("address literal {string} invalid character '{ch}'"),
                        ));
                    }
                }
            }
            _ => {
                ns.diagnostics.push(Diagnostic::error(
                        note.loc,
                            r#"annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'"#
                        .into(),
                    ));
            }
        }
    }
}

/// Resolve a parsed struct definition. The return value will be true if the entire
/// definition is valid; however, whatever could be parsed will be added to the resolved
/// contract, so that we can continue producing compiler messages for the remainder
/// of the contract, even if the struct contains an invalid definition.
pub fn struct_decl(
    def: &pt::StructDefinition,
    file_no: usize,
    tags: &[DocComment],
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> (Vec<Tag>, Vec<Parameter>) {
    let mut fields: Vec<Parameter> = Vec::new();

    for field in &def.fields {
        let mut diagnostics = Diagnostics::default();

        let ty = match ns.resolve_type(file_no, contract_no, false, &field.ty, &mut diagnostics) {
            Ok(s) => s,
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                Type::Unresolved
            }
        };

        if let Some(other) = fields.iter().find(|f| {
            f.id.as_ref().map(|id| id.name.as_str())
                == Some(field.name.as_ref().unwrap().name.as_str())
        }) {
            ns.diagnostics.push(Diagnostic::error_with_note(
                field.name.as_ref().unwrap().loc,
                format!(
                    "struct '{}' has duplicate struct field '{}'",
                    def.name.as_ref().unwrap().name,
                    field.name.as_ref().unwrap().name
                ),
                other.loc,
                format!(
                    "location of previous declaration of '{}'",
                    other.name_as_str()
                ),
            ));
            continue;
        }

        // memory/calldata make no sense for struct fields.
        // TODO: ethereum foundation solidity does not allow storage fields
        // in structs, but this is perfectly possible. The struct would not be
        // allowed as parameter/return types of public functions though.
        if let Some(storage) = &field.storage {
            ns.diagnostics.push(Diagnostic::error(
                storage.loc(),
                format!("storage location '{storage}' not allowed for struct field"),
            ));
        }

        fields.push(Parameter {
            loc: field.loc,
            id: Some(pt::Identifier {
                name: field.name.as_ref().unwrap().name.to_string(),
                loc: field.name.as_ref().unwrap().loc,
            }),
            ty,
            ty_loc: Some(field.ty.loc()),
            indexed: false,
            readonly: false,
            infinite_size: false,
            recursive: false,
        });
    }

    if fields.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            def.name.as_ref().unwrap().loc,
            format!(
                "struct definition for '{}' has no fields",
                def.name.as_ref().unwrap().name
            ),
        ));
    }

    let doc = resolve_tags(
        def.name.as_ref().unwrap().loc.file_no(),
        "struct",
        tags,
        Some(&fields),
        None,
        None,
        ns,
    );

    (doc, fields)
}

/// Resolve a parsed event definition. The return value will be true if the entire
/// definition is valid; however, whatever could be parsed will be added to the resolved
/// contract, so that we can continue producing compiler messages for the remainder
/// of the contract, even if the struct contains an invalid definition.
fn event_decl(
    def: &pt::EventDefinition,
    file_no: usize,
    tags: &[DocComment],
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> (Vec<Tag>, Vec<Parameter>) {
    let mut fields: Vec<Parameter> = Vec::new();
    let mut indexed_fields = 0;

    for field in &def.fields {
        let mut diagnostics = Diagnostics::default();

        let mut ty = match ns.resolve_type(file_no, contract_no, false, &field.ty, &mut diagnostics)
        {
            Ok(s) => s,
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                Type::Unresolved
            }
        };

        if ty.contains_mapping(ns) {
            ns.diagnostics.push(Diagnostic::error(
                field.loc,
                "mapping type is not permitted as event field".to_string(),
            ));
            ty = Type::Unresolved;
        }

        let name = if let Some(name) = &field.name {
            if let Some(other) = fields
                .iter()
                .find(|f| f.id.as_ref().map(|id| id.name.as_str()) == Some(name.name.as_str()))
            {
                ns.diagnostics.push(Diagnostic::error_with_note(
                    name.loc,
                    format!(
                        "event '{}' has duplicate field name '{}'",
                        def.name.as_ref().unwrap().name,
                        name.name
                    ),
                    other.loc,
                    format!(
                        "location of previous declaration of '{}'",
                        other.name_as_str()
                    ),
                ));
                continue;
            }
            Some(pt::Identifier {
                name: name.name.to_owned(),
                loc: name.loc,
            })
        } else {
            if ns.target.is_substrate() && field.indexed {
                ns.diagnostics.push(Diagnostic::error(
                    field.loc,
                    "indexed event fields must have a name on substrate".into(),
                ));
            }
            None
        };

        if field.indexed {
            indexed_fields += 1;
        }

        fields.push(Parameter {
            loc: field.loc,
            id: name,
            ty,
            ty_loc: Some(field.ty.loc()),
            indexed: field.indexed,
            readonly: false,
            infinite_size: false,
            recursive: false,
        });
    }

    if def.anonymous && indexed_fields > 4 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.as_ref().unwrap().loc,
            format!(
                "anonymous event definition for '{}' has {} indexed fields where 4 permitted",
                def.name.as_ref().unwrap().name,
                indexed_fields
            ),
        ));
    } else if !def.anonymous && indexed_fields > 3 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.as_ref().unwrap().loc,
            format!(
                "event definition for '{}' has {} indexed fields where 3 permitted",
                def.name.as_ref().unwrap().name,
                indexed_fields
            ),
        ));
    }

    let doc = resolve_tags(
        def.name.as_ref().unwrap().loc.file_no(),
        "event",
        tags,
        Some(&fields),
        None,
        None,
        ns,
    );

    (doc, fields)
}

/// Parse enum declaration. If the declaration is invalid, it is still generated
/// so that we can continue parsing, with errors recorded.
fn enum_decl(
    enum_: &pt::EnumDefinition,
    file_no: usize,
    tags: &[DocComment],
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> bool {
    let mut valid = true;

    if enum_.values.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            enum_.name.as_ref().unwrap().loc,
            format!("enum '{}' has no fields", enum_.name.as_ref().unwrap().name),
        ));
        valid = false;
    } else if enum_.values.len() > 256 {
        ns.diagnostics.push(Diagnostic::error(
            enum_.name.as_ref().unwrap().loc,
            format!(
                "enum '{}' has {} fields, which is more than the 256 limit",
                enum_.name.as_ref().unwrap().name,
                enum_.values.len()
            ),
        ));
        valid = false;
    }

    // check for duplicates
    let mut entries: IndexMap<String, pt::Loc> = IndexMap::new();

    for e in enum_.values.iter() {
        if let Some(prev) = entries.get(&e.as_ref().unwrap().name.to_string()) {
            ns.diagnostics.push(Diagnostic::error_with_note(
                e.as_ref().unwrap().loc,
                format!("duplicate enum value {}", e.as_ref().unwrap().name),
                *prev,
                "location of previous definition".to_string(),
            ));
            valid = false;
            continue;
        }

        entries.insert(
            e.as_ref().unwrap().name.to_string(),
            e.as_ref().unwrap().loc,
        );
    }

    let tags = resolve_tags(
        enum_.name.as_ref().unwrap().loc.file_no(),
        "enum",
        tags,
        None,
        None,
        None,
        ns,
    );

    let decl = EnumDecl {
        tags,
        name: enum_.name.as_ref().unwrap().name.to_string(),
        loc: enum_.loc,
        contract: match contract_no {
            Some(c) => Some(ns.contracts[c].name.to_owned()),
            None => None,
        },
        ty: Type::Uint(8),
        values: entries,
    };

    let pos = ns.enums.len();

    ns.enums.push(decl);

    if !ns.add_symbol(
        file_no,
        contract_no,
        enum_.name.as_ref().unwrap(),
        Symbol::Enum(enum_.name.as_ref().unwrap().loc, pos),
    ) {
        valid = false;
    }

    valid
}

/// Calculate the offsets for the fields in structs, and also the size of a struct overall.
///
/// Structs can be recursive, and we may not know the size of a field if the field is a struct
/// and we have not calculated yet. In this case we will get size 0. So, loop over all the structs
/// until all the offsets are unchanged.
fn struct_offsets(ns: &mut Namespace) {
    loop {
        let mut changes = false;
        for struct_no in 0..ns.structs.len() {
            // first in-memory
            let mut offsets = Vec::new();
            let mut offset = BigInt::zero();
            let mut largest_alignment = 0;

            for field in &ns.structs[struct_no].fields {
                let mut alignment = field.ty.align_of(ns);
                if alignment == 0 {
                    alignment = 1
                }
                largest_alignment = std::cmp::max(alignment, largest_alignment);
                let remainder = offset.clone() % alignment;

                if remainder > BigInt::zero() {
                    offset += alignment - remainder;
                }

                offsets.push(offset.clone());

                if !field.infinite_size && ns.target == Target::Solana {
                    offset += field.ty.solana_storage_size(ns);
                }
            }

            // add entry for overall size
            if largest_alignment > 1 {
                let remainder = offset.clone() % largest_alignment;

                if remainder > BigInt::zero() {
                    offset += largest_alignment - remainder;
                }
            }

            offsets.push(offset);

            if ns.structs[struct_no].offsets != offsets {
                ns.structs[struct_no].offsets = offsets;
                changes = true;
            }

            let mut storage_offsets = Vec::new();
            let mut offset = BigInt::zero();
            let mut largest_alignment = BigInt::zero();

            for field in &ns.structs[struct_no].fields {
                if !field.infinite_size {
                    let alignment = field.ty.storage_align(ns);
                    largest_alignment = std::cmp::max(alignment.clone(), largest_alignment.clone());
                    let remainder = offset.clone() % alignment.clone();

                    if remainder > BigInt::zero() {
                        offset += alignment - remainder;
                    }

                    storage_offsets.push(offset.clone());

                    offset += field.ty.storage_slots(ns);
                }
            }

            // add entry for overall size
            if largest_alignment > BigInt::one() {
                let remainder = offset.clone() % largest_alignment.clone();

                if remainder > BigInt::zero() {
                    offset += largest_alignment - remainder;
                }
            }

            storage_offsets.push(offset);

            if ns.structs[struct_no].storage_offsets != storage_offsets {
                ns.structs[struct_no].storage_offsets = storage_offsets;
                changes = true;
            }
        }

        if !changes {
            break;
        }
    }
}

impl Type {
    pub fn to_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address(false) => "address".to_string(),
            Type::Address(true) => "address payable".to_string(),
            Type::Int(n) => format!("int{n}"),
            Type::Uint(n) => format!("uint{n}"),
            Type::Rational => "rational".to_string(),
            Type::Value => format!("uint{}", ns.value_length * 8),
            Type::Bytes(n) => format!("bytes{n}"),
            Type::String => "string".to_string(),
            Type::DynamicBytes => "bytes".to_string(),
            Type::Enum(n) => format!("enum {}", ns.enums[*n]),
            Type::Struct(str_ty) => format!("struct {}", str_ty.definition(ns)),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_string(ns),
                len.iter()
                    .map(|len| match len {
                        ArrayLength::Fixed(len) => format!("[{len}]"),
                        _ => "[]".to_string(),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(Mapping {
                key,
                key_name,
                value,
                value_name,
            }) => {
                format!(
                    "mapping({}{}{} => {}{}{})",
                    key.to_string(ns),
                    if key_name.is_some() { " " } else { "" },
                    key_name.as_ref().map(|id| id.name.as_str()).unwrap_or(""),
                    value.to_string(ns),
                    if value_name.is_some() { " " } else { "" },
                    value_name.as_ref().map(|id| id.name.as_str()).unwrap_or(""),
                )
            }
            Type::ExternalFunction {
                params,
                mutability,
                returns,
            }
            | Type::InternalFunction {
                params,
                mutability,
                returns,
            } => {
                let mut s = format!(
                    "function({}) {}",
                    params
                        .iter()
                        .map(|ty| ty.to_string(ns))
                        .collect::<Vec<String>>()
                        .join(","),
                    if matches!(self, Type::InternalFunction { .. }) {
                        "internal"
                    } else {
                        "external"
                    }
                );

                if !mutability.is_default() {
                    write!(s, " {mutability}").unwrap();
                }

                if !returns.is_empty() {
                    write!(
                        s,
                        " returns ({})",
                        returns
                            .iter()
                            .map(|ty| ty.to_string(ns))
                            .collect::<Vec<String>>()
                            .join(",")
                    )
                    .unwrap();
                }

                s
            }
            Type::Contract(n) => format!("contract {}", ns.contracts[*n].name),
            Type::UserType(n) => format!("usertype {}", ns.user_types[*n]),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(_, ty) => {
                format!("{} storage", ty.to_string(ns))
            }
            Type::Void => "void".into(),
            Type::Unreachable => "unreachable".into(),
            // A slice of bytes1 is like bytes
            Type::Slice(ty) if **ty == Type::Bytes(1) => "bytes".into(),
            Type::Slice(ty) => format!("{} slice", ty.to_string(ns)),
            Type::Unresolved => "unresolved".into(),
            Type::BufferPointer => "buffer_pointer".into(),
            Type::FunctionSelector => "function_selector".into(),
        }
    }

    /// Is this a primitive, i.e. bool, address, int, uint, bytes
    pub fn is_primitive(&self) -> bool {
        match self {
            Type::Bool => true,
            Type::Address(_) => true,
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Bytes(_) => true,
            Type::Rational => true,
            Type::Value => true,
            Type::Ref(r) => r.is_primitive(),
            Type::StorageRef(_, r) => r.is_primitive(),
            _ => false,
        }
    }

    /// The eth abi file wants to hear "tuple" rather than "(ty, ty)"
    pub fn to_signature_string(&self, say_tuple: bool, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Contract(_) | Type::Address(_) if ns.target == Target::Solana => {
                format!("bytes{}", ns.address_length)
            }
            Type::Contract(_) | Type::Address(_) => "address".to_string(),
            Type::Int(n) => format!("int{n}"),
            Type::Uint(n) => format!("uint{n}"),
            Type::Rational => "rational".to_string(),
            Type::Bytes(n) => format!("bytes{n}"),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(n) => ns.enums[*n].ty.to_signature_string(say_tuple, ns),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_signature_string(say_tuple, ns),
                len.iter()
                    .map(|len| match len {
                        ArrayLength::Fixed(len) => format!("[{len}]"),
                        _ => "[]".to_string(),
                    })
                    .collect::<String>()
            ),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(_, r) => r.to_string(ns),
            Type::Struct(_) if say_tuple => "tuple".to_string(),
            Type::Struct(struct_type) => {
                format!(
                    "({})",
                    struct_type
                        .definition(ns)
                        .fields
                        .iter()
                        .map(|f| f.ty.to_signature_string(say_tuple, ns))
                        .collect::<Vec<String>>()
                        .join(",")
                )
            }
            Type::InternalFunction { .. } | Type::ExternalFunction { .. } => "function".to_owned(),
            Type::UserType(n) => ns.user_types[*n].ty.to_signature_string(say_tuple, ns),
            // TODO: should an unresolved type not match another unresolved type?
            Type::Unresolved => "unresolved".to_owned(),
            Type::Slice(ty) => format!("{} slice", ty.to_string(ns)),
            _ => unreachable!(),
        }
    }

    /// Give the type of a memory array after dereference
    #[must_use]
    pub fn array_deref(&self) -> Self {
        match self {
            Type::String | Type::DynamicBytes => Type::Ref(Box::new(Type::Uint(8))),
            Type::Ref(t) => t.array_deref(),
            Type::Array(ty, dim) if dim.len() > 1 => Type::Ref(Box::new(Type::Array(
                ty.clone(),
                dim[..dim.len() - 1].to_vec(),
            ))),
            Type::Array(ty, dim) if dim.len() == 1 => Type::Ref(ty.clone()),
            Type::Bytes(_) => Type::Bytes(1),
            _ => panic!("deref on non-array"),
        }
    }

    /// Fetch the type of an array element
    pub fn elem_ty(&self) -> Self {
        match self {
            Type::Array(ty, _) => *ty.clone(),
            _ => unreachable!("Type is not an array"),
        }
    }

    /// Is this a reference type of fixed size
    pub fn is_fixed_reference_type(&self, ns: &Namespace) -> bool {
        match self {
            Type::Bool => false,
            Type::Address(_) => false,
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Rational => false,
            Type::Bytes(_) => false,
            Type::Enum(_) => false,
            Type::Struct(_) => true,
            Type::Array(_, dims) => matches!(dims.last(), Some(ArrayLength::Fixed(_))),
            Type::DynamicBytes => false,
            Type::String => false,
            Type::Mapping(..) => false,
            Type::Contract(_) => false,
            Type::Ref(_) => false,
            Type::StorageRef(..) => false,
            Type::InternalFunction { .. } => false,
            // On EVM, an external function is saved on an 256-bit register, so it is not
            // a reference type.
            Type::ExternalFunction { .. } => ns.target != Target::EVM,
            Type::Slice(_) => false,
            Type::Unresolved => false,
            Type::FunctionSelector => false,
            _ => unreachable!("{:?}", self),
        }
    }

    /// Given an array, return the type of its elements
    #[must_use]
    pub fn array_elem(&self) -> Self {
        match self {
            Type::Array(ty, dim) if dim.len() > 1 => {
                Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())
            }
            Type::Array(ty, dim) if dim.len() == 1 => *ty.clone(),
            Type::DynamicBytes => Type::Bytes(1),
            _ => panic!("not an array"),
        }
    }

    /// Give the type of an storage array after dereference. This can only be used on
    /// array types and will cause a panic otherwise.
    #[must_use]
    pub fn storage_array_elem(&self) -> Self {
        match self {
            Type::Mapping(Mapping { value, .. }) => Type::StorageRef(false, value.clone()),
            Type::DynamicBytes | Type::String | Type::Bytes(_) => Type::Bytes(1),
            Type::Array(ty, dim) if dim.len() > 1 => Type::StorageRef(
                false,
                Box::new(Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())),
            ),
            Type::Array(ty, dim) if dim.len() == 1 => Type::StorageRef(false, ty.clone()),
            Type::StorageRef(_, ty) => ty.storage_array_elem(),
            _ => panic!("deref on non-array"),
        }
    }

    /// Give the length of the outer array. This can only be called on array types
    /// and will panic otherwise.
    pub fn array_length(&self) -> Option<&BigInt> {
        match self {
            Type::StorageRef(_, ty) => ty.array_length(),
            Type::Ref(ty) => ty.array_length(),
            Type::Array(_, dim) => dim.last().unwrap().array_length(),
            _ => panic!("array_length on non-array"),
        }
    }

    /// Returns the size a type occupies in memory
    pub fn memory_size_of(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::Enum(_) => BigInt::one(),
            Type::Bool => BigInt::one(),
            Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
            Type::Bytes(n) => BigInt::from(*n),
            Type::Value => BigInt::from(ns.value_length),
            Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
            Type::Rational => unreachable!(),
            Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => {
                (ns.target.ptr_size() / 8).into()
            }
            Type::Array(ty, dims) => {
                let pointer_size = (ns.target.ptr_size() / 8).into();
                ty.memory_size_of(ns).mul(
                    dims.iter()
                        .map(|d| match d {
                            ArrayLength::Fixed(n) => n,
                            ArrayLength::Dynamic => &pointer_size,
                            ArrayLength::AnyFixed => unreachable!(),
                        })
                        .product::<BigInt>(),
                )
            }
            Type::Struct(str_ty) => str_ty
                .definition(ns)
                .fields
                .iter()
                .map(|d| d.ty.memory_size_of(ns))
                .sum::<BigInt>(),
            Type::String
            | Type::DynamicBytes
            | Type::InternalFunction { .. }
            | Type::Ref(_)
            | Type::StorageRef(..) => BigInt::from(ns.target.ptr_size() / 8),
            Type::ExternalFunction { .. } => {
                // Address and selector
                Type::Address(false).memory_size_of(ns) + Type::Uint(32).memory_size_of(ns)
            }
            Type::Unresolved | Type::Mapping(..) => BigInt::zero(),
            Type::UserType(no) => ns.user_types[*no].ty.memory_size_of(ns),
            Type::FunctionSelector => BigInt::from(ns.target.selector_length()),
            _ => unimplemented!("sizeof on {:?}", self),
        }
    }

    /// Retrieve the alignment for each type, if it is a struct member.
    /// Arrays are always reference types when declared as local variables. Inside structs, however,
    /// they are the object itself, if they are of fixed length.
    pub fn struct_elem_alignment(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::Bool
            // Contract and address are arrays of u8, so they align with one.
            | Type::Contract(_)
            | Type::Address(_)
            | Type::Enum(_) => BigInt::one(),

            // Bytes are custom width type in LLVM, so they fit in the smallest integer type
            // whose bitwidth is larger than what is needed.
            Type::Bytes(n) => {
                BigInt::from(n.next_power_of_two())
            }

            // The same reasoning as above applies for value
            Type::Value => {
                BigInt::from(ns.value_length.next_power_of_two())
            }
            Type::Int(n) | Type::Uint(n) => BigInt::from(n / 8),
            Type::Rational => unreachable!(),
            Type::Array(ty, dims) => {
                if dims.iter().any(|d| *d == ArrayLength::Dynamic) {
                    BigInt::from(ns.target.ptr_size() / 8)
                } else {
                    ty.struct_elem_alignment(ns)
                }
            }

            Type::Struct(def) => {
                def.definition(ns).fields.iter().map(|d| d.ty.struct_elem_alignment(ns)).max().unwrap()
            }

            Type::String
            | Type::DynamicBytes
            | Type::InternalFunction { .. }
            | Type::Ref(_)
            | Type::StorageRef(..) => BigInt::from(ns.target.ptr_size() / 8),

            Type::ExternalFunction { .. } => {
                Type::Address(false).struct_elem_alignment(ns)
            }
            Type::UserType(no) => ns.user_types[*no].ty.struct_elem_alignment(ns),

            _ => unreachable!("Type should not appear on a struct"),

        }
    }

    /// Calculate how much memory this type occupies in Solana's storage.
    /// Depending on the llvm implementation there might be padding between elements
    /// which is not accounted for.
    pub fn solana_storage_size(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => 4.into(),
            Type::Array(ty, dims) => ty.solana_storage_size(ns).mul(
                dims.iter()
                    .map(|d| match d {
                        ArrayLength::Fixed(d) => d,
                        _ => panic!("unknown length"),
                    })
                    .product::<BigInt>(),
            ),
            Type::Struct(str_ty) => str_ty
                .definition(ns)
                .offsets
                .last()
                .cloned()
                .unwrap_or_else(BigInt::zero),
            Type::String | Type::DynamicBytes => BigInt::from(4),
            Type::Ref(ty) | Type::StorageRef(_, ty) => ty.solana_storage_size(ns),
            Type::UserType(no) => ns.user_types[*no].ty.solana_storage_size(ns),
            // Other types have the same size both in storage and in memory
            _ => self.memory_size_of(ns),
        }
    }

    /// Does this type fit into memory
    pub fn fits_in_memory(&self, ns: &Namespace) -> bool {
        self.memory_size_of(ns) < BigInt::from(u16::MAX)
    }

    /// Calculate the alignment
    pub fn align_of(&self, ns: &Namespace) -> usize {
        self.align_of_internal(ns, &mut HashSet::new())
    }

    fn align_of_internal(&self, ns: &Namespace, structs_visited: &mut HashSet<usize>) -> usize {
        self.recurse(structs_visited, 1, |structs_visited| match self {
            Type::Uint(8) | Type::Int(8) => 1,
            Type::Uint(n) | Type::Int(n) if *n <= 16 => 2,
            Type::Uint(n) | Type::Int(n) if *n <= 32 => 4,
            Type::Uint(_) | Type::Int(_) => 8,
            Type::Struct(str_ty) => str_ty
                .definition(ns)
                .fields
                .iter()
                .map(|f| f.ty.align_of_internal(ns, structs_visited))
                .max()
                .unwrap(),
            Type::InternalFunction { .. } => ns.target.ptr_size().into(),
            _ => 1,
        })
    }

    pub fn bytes(&self, ns: &Namespace) -> u8 {
        match self {
            Type::Contract(_) | Type::Address(_) => ns.address_length as u8,
            Type::Bool => 1,
            Type::Int(n) => ((*n + 7) / 8) as u8,
            Type::Uint(n) => ((*n + 7) / 8) as u8,
            Type::Rational => unreachable!(),
            Type::Bytes(n) => *n,
            Type::Enum(n) => ns.enums[*n].ty.bytes(ns),
            Type::Value => ns.value_length as u8,
            Type::StorageRef(..) => ns.storage_type().bytes(ns),
            Type::Ref(ty) => ty.bytes(ns),
            Type::FunctionSelector => ns.target.selector_length(),
            _ => panic!("type not allowed"),
        }
    }

    pub fn bits(&self, ns: &Namespace) -> u16 {
        match self {
            Type::Contract(_) | Type::Address(_) => ns.address_length as u16 * 8,
            Type::Bool => 1,
            Type::Int(n) => *n,
            Type::Uint(n) => *n,
            Type::Rational => unreachable!(),
            Type::Bytes(n) => *n as u16 * 8,
            Type::Enum(n) => ns.enums[*n].ty.bits(ns),
            Type::Value => ns.value_length as u16 * 8,
            Type::StorageRef(..) => ns.storage_type().bits(ns),
            Type::Ref(ty) => ty.bits(ns),
            _ => panic!("type not allowed"),
        }
    }

    pub fn is_signed_int(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Ref(r) => r.is_signed_int(),
            Type::StorageRef(_, r) => r.is_signed_int(),
            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Value => true,
            Type::Bytes(1) => true,
            Type::Ref(r) => r.is_integer(),
            Type::StorageRef(_, r) => r.is_integer(),
            _ => false,
        }
    }

    pub fn is_rational(&self) -> bool {
        match self {
            Type::Rational => true,
            Type::Ref(r) => r.is_rational(),
            Type::StorageRef(_, r) => r.is_rational(),
            _ => false,
        }
    }

    /// Calculate how many storage slots a type occupies. Note that storage arrays can
    /// be very large
    pub fn storage_slots(&self, ns: &Namespace) -> BigInt {
        self.storage_slots_internal(ns, &mut HashSet::new())
    }

    fn storage_slots_internal(
        &self,
        ns: &Namespace,
        structs_visited: &mut HashSet<usize>,
    ) -> BigInt {
        self.recurse(structs_visited, 1.into(), |structs_visited| {
            if ns.target == Target::Solana {
                match self {
                    Type::Enum(_) => BigInt::one(),
                    Type::Bool => BigInt::one(),
                    Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
                    Type::Bytes(n) => BigInt::from(*n),
                    Type::Value => BigInt::from(ns.value_length),
                    Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
                    Type::Rational => unreachable!(),
                    Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => {
                        BigInt::from(4)
                    }
                    Type::Array(ty, dims) => {
                        let pointer_size = BigInt::from(4);
                        if self.is_sparse_solana(ns) {
                            BigInt::from(SOLANA_BUCKET_SIZE) * BigInt::from(4)
                        } else {
                            ty.storage_slots_internal(ns, structs_visited).mul(
                                dims.iter()
                                    .map(|d| match d {
                                        ArrayLength::Dynamic => &pointer_size,
                                        ArrayLength::Fixed(d) => d,
                                        ArrayLength::AnyFixed => {
                                            panic!("unknown length");
                                        }
                                    })
                                    .product::<BigInt>(),
                            )
                        }
                    }
                    Type::Struct(str_ty) => str_ty
                        .definition(ns)
                        .storage_offsets
                        .last()
                        .cloned()
                        .unwrap_or_else(BigInt::zero),
                    Type::String | Type::DynamicBytes => BigInt::from(4),
                    Type::InternalFunction { .. } => BigInt::from(ns.target.ptr_size()),
                    Type::ExternalFunction { .. } => {
                        // Address and selector
                        BigInt::from(ns.address_length + 4)
                    }
                    Type::Mapping(..) => BigInt::from(SOLANA_BUCKET_SIZE) * BigInt::from(4),
                    Type::Ref(ty) | Type::StorageRef(_, ty) => {
                        ty.storage_slots_internal(ns, structs_visited)
                    }
                    Type::Unresolved => BigInt::one(),
                    Type::UserType(no) => ns.user_types[*no]
                        .ty
                        .storage_slots_internal(ns, structs_visited),
                    _ => unimplemented!(),
                }
            } else {
                match self {
                    Type::StorageRef(_, r) | Type::Ref(r) => {
                        r.storage_slots_internal(ns, structs_visited)
                    }
                    Type::Struct(str_ty) => str_ty
                        .definition(ns)
                        .fields
                        .iter()
                        .map(|f| f.ty.storage_slots_internal(ns, structs_visited))
                        .sum(),
                    Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => 1.into(),
                    Type::Array(ty, dims) => {
                        let one = 1.into();
                        ty.storage_slots_internal(ns, structs_visited)
                            * dims
                                .iter()
                                .map(|len| match len {
                                    ArrayLength::Dynamic => &one,
                                    ArrayLength::Fixed(len) => len,
                                    ArrayLength::AnyFixed => {
                                        unreachable!("unknown length")
                                    }
                                })
                                .product::<BigInt>()
                    }
                    _ => BigInt::one(),
                }
            }
        })
    }

    /// Alignment of elements in storage
    pub fn storage_align(&self, ns: &Namespace) -> BigInt {
        self.storage_align_internal(ns, &mut HashSet::new())
    }

    fn storage_align_internal(
        &self,
        ns: &Namespace,
        structs_visited: &mut HashSet<usize>,
    ) -> BigInt {
        self.recurse(structs_visited, 1.into(), |structs_visited| {
            if ns.target == Target::Solana {
                let length = match self {
                    Type::Enum(_) => BigInt::one(),
                    Type::Bool => BigInt::one(),
                    Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
                    Type::Bytes(n) => BigInt::from(*n),
                    Type::Value => BigInt::from(ns.value_length),
                    Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
                    Type::Rational => unreachable!(),
                    Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => {
                        BigInt::from(4)
                    }
                    Type::Array(ty, _) => {
                        if self.is_sparse_solana(ns) {
                            BigInt::from(4)
                        } else {
                            ty.storage_align_internal(ns, structs_visited)
                        }
                    }
                    Type::Struct(str_ty) => str_ty
                        .definition(ns)
                        .fields
                        .iter()
                        .map(|field| field.ty.storage_align_internal(ns, structs_visited))
                        .max()
                        .unwrap(),
                    Type::String | Type::DynamicBytes => BigInt::from(4),
                    Type::InternalFunction { .. } => BigInt::from(ns.target.ptr_size()),
                    Type::ExternalFunction { .. } => BigInt::from(ns.address_length),
                    Type::Mapping(..) => BigInt::from(4),
                    Type::Ref(ty) | Type::StorageRef(_, ty) => {
                        ty.storage_align_internal(ns, structs_visited)
                    }
                    Type::Unresolved => BigInt::one(),
                    _ => unimplemented!(),
                };

                if length > BigInt::from(8) {
                    BigInt::from(8)
                } else {
                    length
                }
            } else {
                BigInt::one()
            }
        })
    }

    /// Is this type an reference type in the solidity language? (struct, array, mapping)
    pub fn is_reference_type(&self, ns: &Namespace) -> bool {
        match self {
            Type::Bool => false,
            Type::Address(_) => false,
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Rational => false,
            Type::Bytes(_) => false,
            Type::Enum(_) => false,
            Type::Struct(_) => true,
            Type::Array(..) => true,
            Type::DynamicBytes => true,
            Type::String => true,
            Type::Mapping(..) => true,
            Type::Contract(_) => false,
            Type::Ref(r) => r.is_reference_type(ns),
            Type::StorageRef(_, r) => r.is_reference_type(ns),
            Type::InternalFunction { .. } => false,
            // On EVM, an external function is saved on an 256-bit register, so it is not
            // a reference type.
            Type::ExternalFunction { .. } => ns.target != Target::EVM,
            Type::UserType(no) => ns.user_types[*no].ty.is_reference_type(ns),
            _ => false,
        }
    }

    /// Does this type contain any types which are variable-length
    pub fn is_dynamic(&self, ns: &Namespace) -> bool {
        self.is_dynamic_internal(ns, &mut HashSet::new())
    }

    fn is_dynamic_internal(&self, ns: &Namespace, structs_visited: &mut HashSet<usize>) -> bool {
        self.recurse(structs_visited, false, |structs_visited| match self {
            Type::String | Type::DynamicBytes => true,
            Type::Ref(r) => r.is_dynamic_internal(ns, structs_visited),
            Type::Array(ty, dim) => {
                if dim.iter().any(|d| d == &ArrayLength::Dynamic) {
                    return true;
                }
                ty.is_dynamic_internal(ns, structs_visited)
            }
            Type::Struct(str_ty) => str_ty
                .definition(ns)
                .fields
                .iter()
                .any(|f| f.ty.is_dynamic_internal(ns, structs_visited)),
            Type::StorageRef(_, r) => r.is_dynamic_internal(ns, structs_visited),
            Type::Slice(_) => true,
            _ => false,
        })
    }

    /// Can this type have a calldata, memory, or storage location. This is to be
    /// compatible with ethereum solidity. Opinions on whether other types should be
    /// allowed be storage are welcome.
    pub fn can_have_data_location(&self) -> bool {
        matches!(
            self,
            Type::Array(..)
                | Type::Struct(_)
                | Type::Mapping(..)
                | Type::String
                | Type::DynamicBytes
        )
    }

    /// Is this a reference to contract storage?
    pub fn is_contract_storage(&self) -> bool {
        matches!(self, Type::StorageRef(..))
    }

    /// Is this a reference to dynamic memory (arrays, strings)
    pub fn is_dynamic_memory(&self) -> bool {
        match self {
            Type::String | Type::DynamicBytes | Type::Slice(_) => true,
            Type::Array(_, dim) if dim.last() == Some(&ArrayLength::Dynamic) => true,
            Type::Ref(ty) => ty.is_dynamic_memory(),
            _ => false,
        }
    }

    /// Is this a storage bytes string
    pub fn is_storage_bytes(&self) -> bool {
        if let Type::StorageRef(_, ty) = self {
            if let Type::DynamicBytes = ty.as_ref() {
                return true;
            }
        }

        false
    }

    /// Is this a mapping
    pub fn is_mapping(&self) -> bool {
        match self {
            Type::Mapping(..) => true,
            Type::StorageRef(_, ty) => ty.is_mapping(),
            _ => false,
        }
    }

    /// Is it an address (with some sugar)
    pub fn is_address(&self) -> bool {
        matches!(self, Type::Address(_) | Type::Contract(_))
    }

    /// Does the type contain any mapping type
    pub fn contains_mapping(&self, ns: &Namespace) -> bool {
        self.contains_mapping_internal(ns, &mut HashSet::new())
    }

    fn contains_mapping_internal(
        &self,
        ns: &Namespace,
        structs_visited: &mut HashSet<usize>,
    ) -> bool {
        self.recurse(structs_visited, false, |structs_visited| match self {
            Type::Mapping(..) => true,
            Type::Array(ty, _) => ty.contains_mapping_internal(ns, structs_visited),
            Type::Struct(str_ty) => str_ty
                .definition(ns)
                .fields
                .iter()
                .any(|f| f.ty.contains_mapping_internal(ns, structs_visited)),
            Type::StorageRef(_, r) | Type::Ref(r) => {
                r.contains_mapping_internal(ns, structs_visited)
            }
            _ => false,
        })
    }

    /// Does the type contain any internal function type
    pub fn contains_internal_function(&self, ns: &Namespace) -> bool {
        self.contains_internal_function_internal(ns, &mut HashSet::new())
    }

    fn contains_internal_function_internal(
        &self,
        ns: &Namespace,
        structs_visited: &mut HashSet<usize>,
    ) -> bool {
        self.recurse(structs_visited, false, |structs_visited| match self {
            Type::InternalFunction { .. } => true,
            Type::Array(ty, _) => ty.contains_internal_function_internal(ns, structs_visited),
            Type::Struct(str_ty) => str_ty.definition(ns).fields.iter().any(|f| {
                f.ty.contains_internal_function_internal(ns, structs_visited)
            }),
            Type::StorageRef(_, r) | Type::Ref(r) => {
                r.contains_internal_function_internal(ns, structs_visited)
            }
            _ => false,
        })
    }

    /// Is this structure a builtin
    pub fn is_builtin_struct(&self) -> Option<StructType> {
        self.is_builtin_struct_internal(&mut HashSet::new())
    }

    fn is_builtin_struct_internal(
        &self,
        structs_visited: &mut HashSet<usize>,
    ) -> Option<StructType> {
        self.recurse(structs_visited, None, |structs_visited| match self {
            Type::Struct(str_ty) => {
                if matches!(str_ty, StructType::UserDefined(_)) {
                    None
                } else {
                    Some(*str_ty)
                }
            }
            Type::StorageRef(_, r) | Type::Ref(r) => r.is_builtin_struct_internal(structs_visited),
            _ => None,
        })
    }

    /// Does the type contain any builtin type
    pub fn contains_builtins<'a>(
        &'a self,
        ns: &'a Namespace,
        builtin: &StructType,
    ) -> Option<&'a Type> {
        self.contains_builtins_internal(ns, builtin, &mut HashSet::new())
    }

    fn contains_builtins_internal<'a>(
        &'a self,
        ns: &'a Namespace,
        builtin: &StructType,
        structs_visited: &mut HashSet<usize>,
    ) -> Option<&'a Type> {
        self.recurse(structs_visited, None, |structs_visited| match self {
            Type::Array(ty, _) => ty.contains_builtins_internal(ns, builtin, structs_visited),
            Type::Mapping(Mapping { key, value, .. }) => key
                .contains_builtins_internal(ns, builtin, structs_visited)
                .or_else(|| value.contains_builtins_internal(ns, builtin, structs_visited)),
            Type::Struct(str_ty) if str_ty == builtin => Some(self),
            Type::Struct(str_ty) => str_ty.definition(ns).fields.iter().find_map(|f| {
                f.ty.contains_builtins_internal(ns, builtin, structs_visited)
            }),
            Type::StorageRef(_, r) | Type::Ref(r) => {
                r.contains_builtins_internal(ns, builtin, structs_visited)
            }
            _ => None,
        })
    }

    /// If the type is Ref or StorageRef, get the underlying type
    pub fn deref_any(&self) -> &Self {
        match self {
            Type::StorageRef(_, r) => r,
            Type::Ref(r) => r,
            _ => self,
        }
    }

    /// If the type is Ref or StorageRef, get the underlying type
    #[must_use]
    pub fn deref_into(self) -> Self {
        match self {
            Type::StorageRef(_, r) => *r,
            Type::Ref(r) => *r,
            _ => self,
        }
    }

    /// If the type is Ref, get the underlying type
    pub fn deref_memory(&self) -> &Self {
        match self {
            Type::Ref(r) => r,
            _ => self,
        }
    }

    /// Give a valid name for the type which is
    pub fn to_llvm_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address(_) => "address".to_string(),
            Type::Int(n) => format!("int{n}"),
            Type::Uint(n) => format!("uint{n}"),
            Type::Bytes(n) => format!("bytes{n}"),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(i) => format!("{}", ns.enums[*i]),
            Type::Struct(str_ty) => format!("{}", str_ty.definition(ns)),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_llvm_string(ns),
                len.iter()
                    .map(|r| match r {
                        ArrayLength::Dynamic | ArrayLength::AnyFixed => ":".to_string(),
                        ArrayLength::Fixed(r) => format!(":{r}"),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(Mapping { key, value, .. }) => {
                format!(
                    "mapping:{}:{}",
                    key.to_llvm_string(ns),
                    value.to_llvm_string(ns)
                )
            }
            Type::Contract(i) => ns.contracts[*i].name.to_owned(),
            Type::InternalFunction { .. } => "function".to_owned(),
            Type::ExternalFunction { .. } => "function".to_owned(),
            Type::Ref(r) => r.to_llvm_string(ns),
            Type::StorageRef(_, r) => r.to_llvm_string(ns),
            Type::UserType(no) => ns.user_types[*no].ty.to_llvm_string(ns),
            Type::Slice(ty) => format!("slice:{}", ty.to_llvm_string(ns)),
            _ => unreachable!(),
        }
    }

    /// Is this type sparse on Solana
    pub fn is_sparse_solana(&self, ns: &Namespace) -> bool {
        match self.deref_any() {
            Type::Mapping(..) => true,
            Type::Array(_, dims) if dims.last() == Some(&ArrayLength::Dynamic) => false,
            Type::Array(ty, dims) => {
                let pointer_size = BigInt::from(4);
                let len = ty.storage_slots(ns).mul(
                    dims.iter()
                        .map(|d| match d {
                            ArrayLength::Fixed(d) => d,
                            _ => &pointer_size,
                        })
                        .product::<BigInt>(),
                );

                len >= BigInt::from(SOLANA_SPARSE_ARRAY_SIZE)
            }
            _ => false,
        }
    }

    /// Recursively walk over a type, protected against overflows on infinite recursive types.
    /// `structs_visited` is the set of already visited structs.
    /// `bail` is the value that should be returned in case an infinite recursion occured.
    /// `f` is the closure applied on each iteration. `f` can be recursive.
    fn recurse<F, O>(&self, structs_visited: &mut HashSet<usize>, bail: O, f: F) -> O
    where
        F: FnOnce(&mut HashSet<usize>) -> O,
    {
        if let Type::Struct(StructType::UserDefined(n)) = self {
            if !structs_visited.insert(*n) {
                return bail;
            }
        }
        f(structs_visited)
    }
}

/// These names cannot be used on Windows, even with an extension.
/// shamelessly stolen from cargo
fn is_windows_reserved(name: &str) -> bool {
    [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ]
    .contains(&name.to_ascii_lowercase().as_str())
}
