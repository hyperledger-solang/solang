use super::tags::resolve_tags;
use super::SOLANA_BUCKET_SIZE;
use super::{
    ast::{
        BuiltinStruct, Contract, Diagnostic, EnumDecl, EventDecl, Namespace, Parameter, StructDecl,
        Symbol, Tag, Type, UserTypeDecl,
    },
    tags::{parse_doccomments, DocComment},
    SOLANA_SPARSE_ARRAY_SIZE,
};
use crate::Target;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use solang_parser::{pt, pt::CodeLocation};
use std::{collections::HashMap, fmt::Write, ops::Mul};

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
    s: &'a pt::SourceUnit,
    file_no: usize,
    ns: &mut Namespace,
) -> ResolveFields<'a> {
    let mut doccomments = Vec::new();
    let mut delay = ResolveFields {
        structs: Vec::new(),
        events: Vec::new(),
    };

    // Find all the types: contracts, enums, and structs. Either in a contract or not
    // We do not resolve the struct fields yet as we do not know all the possible types until we're
    // done
    for part in &s.0 {
        match part {
            pt::SourceUnitPart::DocComment(doccomment) => {
                doccomments.push(doccomment);
            }
            pt::SourceUnitPart::ContractDefinition(def) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                resolve_contract(def, &tags, file_no, &mut delay, ns);
            }
            pt::SourceUnitPart::EnumDefinition(def) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                let _ = enum_decl(def, file_no, &tags, None, ns);
            }
            pt::SourceUnitPart::StructDefinition(def) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                let struct_no = ns.structs.len();

                if ns.add_symbol(
                    file_no,
                    None,
                    &def.name,
                    Symbol::Struct(def.name.loc, struct_no),
                ) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: def.name.name.to_owned(),
                        builtin: BuiltinStruct::None,
                        loc: def.name.loc,
                        contract: None,
                        fields: Vec::new(),
                        offsets: Vec::new(),
                        storage_offsets: Vec::new(),
                    });

                    delay.structs.push(ResolveStructFields {
                        struct_no,
                        pt: def,
                        comments: tags,
                        contract: None,
                    });
                }
            }
            pt::SourceUnitPart::EventDefinition(def) => {
                let event_no = ns.events.len();

                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                if let Some(Symbol::Event(events)) =
                    ns.variable_symbols
                        .get_mut(&(file_no, None, def.name.name.to_owned()))
                {
                    events.push((def.name.loc, event_no));
                } else if !ns.add_symbol(
                    file_no,
                    None,
                    &def.name,
                    Symbol::Event(vec![(def.name.loc, event_no)]),
                ) {
                    continue;
                }

                ns.events.push(EventDecl {
                    tags: Vec::new(),
                    name: def.name.name.to_owned(),
                    loc: def.name.loc,
                    contract: None,
                    fields: Vec::new(),
                    anonymous: def.anonymous,
                    signature: String::new(),
                    used: false,
                });

                delay.events.push(ResolveEventFields {
                    event_no,
                    pt: def,
                    comments: tags,
                    contract: None,
                });
            }
            pt::SourceUnitPart::TypeDefinition(ty) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                type_decl(ty, file_no, &tags, None, ns);
            }
            _ => (),
        }
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
    let mut diagnostics = Vec::new();

    let ty = match ns.resolve_type(file_no, contract_no, false, &def.ty, &mut diagnostics) {
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
            format!("’{}’ is not an elementary value type", ty.to_string(ns)),
        ));
        return;
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

pub fn resolve_fields(delay: ResolveFields, file_no: usize, ns: &mut Namespace) {
    // now we can resolve the fields for the structs
    for resolve in delay.structs {
        if let Some((tags, fields)) =
            struct_decl(resolve.pt, file_no, &resolve.comments, resolve.contract, ns)
        {
            ns.structs[resolve.struct_no].tags = tags;
            ns.structs[resolve.struct_no].fields = fields;
        }
    }

    // struct can contain other structs, and we have to check for recursiveness,
    // i.e. "struct a { b f1; } struct b { a f1; }"
    for struct_no in 0..ns.structs.len() {
        fn check(struct_no: usize, structs_visited: &mut Vec<usize>, ns: &mut Namespace) {
            let def = ns.structs[struct_no].clone();
            let mut types_seen = Vec::new();

            for field in &def.fields {
                if let Type::Struct(struct_no) = field.ty {
                    if types_seen.contains(&struct_no) {
                        continue;
                    }

                    types_seen.push(struct_no);

                    if structs_visited.contains(&struct_no) {
                        ns.diagnostics.push(Diagnostic::error_with_note(
                            def.loc,
                            format!("struct '{}' has infinite size", def.name),
                            field.loc,
                            format!("recursive field '{}'", field.name_as_str()),
                        ));
                    } else {
                        structs_visited.push(struct_no);
                        check(struct_no, structs_visited, ns);
                        let _ = structs_visited.pop();
                    }
                }
            }
        }

        check(struct_no, &mut vec![struct_no], ns);
    }

    // Do not attempt to call struct offsets if there are any infinitely recursive structs
    if !ns.diagnostics.any_errors() {
        struct_offsets(ns);
    }

    // now we can resolve the fields for the events
    for event in delay.events {
        if let Some((tags, fields)) =
            event_decl(event.pt, file_no, &event.comments, event.contract, ns)
        {
            ns.events[event.event_no].signature =
                ns.signature(&ns.events[event.event_no].name, &fields);
            ns.events[event.event_no].fields = fields;
            ns.events[event.event_no].tags = tags;
        }
    }
}

/// Resolve all the types in a contract
fn resolve_contract<'a>(
    def: &'a pt::ContractDefinition,
    contract_tags: &[DocComment],
    file_no: usize,
    delay: &mut ResolveFields<'a>,
    ns: &mut Namespace,
) -> bool {
    let contract_no = ns.contracts.len();

    let doc = resolve_tags(
        def.name.loc.file_no(),
        "contract",
        contract_tags,
        None,
        None,
        None,
        ns,
    );

    ns.contracts
        .push(Contract::new(&def.name.name, def.ty.clone(), doc, def.loc));

    let mut broken = !ns.add_symbol(
        file_no,
        None,
        &def.name,
        Symbol::Contract(def.loc, contract_no),
    );

    if is_windows_reserved(&def.name.name) {
        ns.diagnostics.push(Diagnostic::error(
            def.name.loc,
            format!(
                "contract name '{}' is reserved file name on Windows",
                def.name.name
            ),
        ));
    }

    let mut doccomments = Vec::new();

    for parts in &def.parts {
        match parts {
            pt::ContractPart::DocComment(doccomment) => {
                doccomments.push(doccomment);
            }
            pt::ContractPart::EnumDefinition(ref e) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                if !enum_decl(e, file_no, &tags, Some(contract_no), ns) {
                    broken = true;
                }
            }
            pt::ContractPart::StructDefinition(ref pt) => {
                let struct_no = ns.structs.len();

                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                if ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    &pt.name,
                    Symbol::Struct(pt.name.loc, struct_no),
                ) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: pt.name.name.to_owned(),
                        builtin: BuiltinStruct::None,
                        loc: pt.name.loc,
                        contract: Some(def.name.name.to_owned()),
                        fields: Vec::new(),
                        offsets: Vec::new(),
                        storage_offsets: Vec::new(),
                    });

                    delay.structs.push(ResolveStructFields {
                        struct_no,
                        pt,
                        comments: tags,
                        contract: Some(contract_no),
                    });
                } else {
                    broken = true;
                }
            }
            pt::ContractPart::EventDefinition(ref pt) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                let event_no = ns.events.len();

                if let Some(Symbol::Event(events)) = ns.variable_symbols.get_mut(&(
                    file_no,
                    Some(contract_no),
                    pt.name.name.to_owned(),
                )) {
                    events.push((pt.name.loc, event_no));
                } else if !ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    &pt.name,
                    Symbol::Event(vec![(pt.name.loc, event_no)]),
                ) {
                    broken = true;
                    continue;
                }

                ns.events.push(EventDecl {
                    tags: Vec::new(),
                    name: pt.name.name.to_owned(),
                    loc: pt.name.loc,
                    contract: Some(contract_no),
                    fields: Vec::new(),
                    anonymous: pt.anonymous,
                    signature: String::new(),
                    used: false,
                });

                delay.events.push(ResolveEventFields {
                    event_no,
                    pt,
                    comments: tags,
                    contract: Some(contract_no),
                });
            }
            pt::ContractPart::TypeDefinition(ty) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                type_decl(ty, file_no, &tags, Some(contract_no), ns);
            }
            _ => (),
        }
    }

    broken
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
) -> Option<(Vec<Tag>, Vec<Parameter>)> {
    let mut valid = true;
    let mut fields: Vec<Parameter> = Vec::new();

    for field in &def.fields {
        let mut diagnostics = Vec::new();

        let ty = match ns.resolve_type(file_no, contract_no, false, &field.ty, &mut diagnostics) {
            Ok(s) => s,
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                valid = false;
                continue;
            }
        };

        if let Some(other) = fields
            .iter()
            .find(|f| f.id.as_ref().map(|id| id.name.as_str()) == Some(field.name.name.as_str()))
        {
            ns.diagnostics.push(Diagnostic::error_with_note(
                field.name.loc,
                format!(
                    "struct '{}' has duplicate struct field '{}'",
                    def.name.name, field.name.name
                ),
                other.loc,
                format!(
                    "location of previous declaration of '{}'",
                    other.name_as_str()
                ),
            ));
            valid = false;
            continue;
        }

        // memory/calldata make no sense for struct fields.
        // TODO: ethereum foundation solidity does not allow storage fields
        // in structs, but this is perfectly possible. The struct would not be
        // allowed as parameter/return types of public functions though.
        if let Some(storage) = &field.storage {
            ns.diagnostics.push(Diagnostic::error(
                storage.loc(),
                format!(
                    "storage location '{}' not allowed for struct field",
                    storage
                ),
            ));
            valid = false;
        }

        fields.push(Parameter {
            loc: field.loc,
            id: Some(pt::Identifier {
                name: field.name.name.to_string(),
                loc: field.name.loc,
            }),
            ty,
            ty_loc: Some(field.ty.loc()),
            indexed: false,
            readonly: false,
        });
    }

    if fields.is_empty() {
        if valid {
            ns.diagnostics.push(Diagnostic::error(
                def.name.loc,
                format!("struct definition for '{}' has no fields", def.name.name),
            ));
        }

        valid = false;
    }

    if valid {
        let doc = resolve_tags(
            def.name.loc.file_no(),
            "struct",
            tags,
            Some(&fields),
            None,
            None,
            ns,
        );

        Some((doc, fields))
    } else {
        None
    }
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
) -> Option<(Vec<Tag>, Vec<Parameter>)> {
    let mut valid = true;
    let mut fields: Vec<Parameter> = Vec::new();
    let mut indexed_fields = 0;

    for field in &def.fields {
        let mut diagnostics = Vec::new();

        let ty = match ns.resolve_type(file_no, contract_no, false, &field.ty, &mut diagnostics) {
            Ok(s) => s,
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                valid = false;
                continue;
            }
        };

        if ty.contains_mapping(ns) {
            ns.diagnostics.push(Diagnostic::error(
                field.loc,
                "mapping type is not permitted as event field".to_string(),
            ));
            valid = false;
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
                        def.name.name, name.name
                    ),
                    other.loc,
                    format!(
                        "location of previous declaration of '{}'",
                        other.name_as_str()
                    ),
                ));
                valid = false;
                continue;
            }
            Some(pt::Identifier {
                name: name.name.to_owned(),
                loc: name.loc,
            })
        } else {
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
        });
    }

    if def.anonymous && indexed_fields > 4 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.loc,
            format!(
                "anonymous event definition for '{}' has {} indexed fields where 4 permitted",
                def.name.name, indexed_fields
            ),
        ));

        valid = false;
    } else if !def.anonymous && indexed_fields > 3 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.loc,
            format!(
                "event definition for '{}' has {} indexed fields where 3 permitted",
                def.name.name, indexed_fields
            ),
        ));

        valid = false;
    }

    if valid {
        let doc = resolve_tags(
            def.name.loc.file_no(),
            "event",
            tags,
            Some(&fields),
            None,
            None,
            ns,
        );

        Some((doc, fields))
    } else {
        None
    }
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
            enum_.name.loc,
            format!("enum '{}' has no fields", enum_.name.name),
        ));
        valid = false;
    } else if enum_.values.len() > 256 {
        ns.diagnostics.push(Diagnostic::error(
            enum_.name.loc,
            format!(
                "enum '{}' has {} fields, which is more than the 256 limit",
                enum_.name.name,
                enum_.values.len()
            ),
        ));
        valid = false;
    }

    // check for duplicates
    let mut entries: HashMap<String, (pt::Loc, usize)> = HashMap::new();

    for (i, e) in enum_.values.iter().enumerate() {
        if let Some(prev) = entries.get(&e.name.to_string()) {
            ns.diagnostics.push(Diagnostic::error_with_note(
                e.loc,
                format!("duplicate enum value {}", e.name),
                prev.0,
                "location of previous definition".to_string(),
            ));
            valid = false;
            continue;
        }

        entries.insert(e.name.to_string(), (e.loc, i));
    }

    let tags = resolve_tags(enum_.name.loc.file_no(), "enum", tags, None, None, None, ns);

    let decl = EnumDecl {
        tags,
        name: enum_.name.name.to_string(),
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
        &enum_.name,
        Symbol::Enum(enum_.name.loc, pos),
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
                let alignment = field.ty.align_of(ns);
                largest_alignment = std::cmp::max(alignment, largest_alignment);
                let remainder = offset.clone() % alignment;

                if remainder > BigInt::zero() {
                    offset += alignment - remainder;
                }

                offsets.push(offset.clone());

                offset += field.ty.size_of(ns);
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
                let alignment = field.ty.storage_align(ns);
                largest_alignment = std::cmp::max(alignment.clone(), largest_alignment.clone());
                let remainder = offset.clone() % alignment.clone();

                if remainder > BigInt::zero() {
                    offset += alignment - remainder;
                }

                storage_offsets.push(offset.clone());

                offset += field.ty.storage_slots(ns);
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
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Rational => "rational".to_string(),
            Type::Value => format!("uint{}", ns.value_length * 8),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::String => "string".to_string(),
            Type::DynamicBytes => "bytes".to_string(),
            Type::Enum(n) => format!("enum {}", ns.enums[*n]),
            Type::Struct(n) => format!("struct {}", ns.structs[*n]),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_string(ns),
                len.iter()
                    .map(|l| match l {
                        None => "[]".to_string(),
                        Some(l) => format!("[{}]", l),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(k, v) => format!("mapping({} => {})", k.to_string(ns), v.to_string(ns)),
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
                    write!(s, " {}", mutability).unwrap();
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
            Type::StorageRef(_, ty) => format!("{} storage", ty.to_string(ns)),
            Type::Void => "void".to_owned(),
            Type::Unreachable => "unreachable".to_owned(),
            Type::Slice => "slice".to_owned(),
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
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Rational => "rational".to_string(),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(n) => ns.enums[*n].ty.to_signature_string(say_tuple, ns),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_signature_string(say_tuple, ns),
                len.iter()
                    .map(|l| match l {
                        None => "[]".to_string(),
                        Some(l) => format!("[{}]", l),
                    })
                    .collect::<String>()
            ),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(_, r) => r.to_string(ns),
            Type::Struct(_) if say_tuple => "tuple".to_string(),
            Type::Struct(struct_no) => {
                format!(
                    "({})",
                    ns.structs[*struct_no]
                        .fields
                        .iter()
                        .map(|f| f.ty.to_signature_string(say_tuple, ns))
                        .collect::<Vec<String>>()
                        .join(",")
                )
            }
            Type::InternalFunction { .. } | Type::ExternalFunction { .. } => "function".to_owned(),
            Type::UserType(n) => ns.user_types[*n].ty.to_signature_string(say_tuple, ns),
            _ => unreachable!(),
        }
    }

    /// Give the type of an memory array after dereference
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

    /// Is this a reference type of fixed size
    pub fn is_fixed_reference_type(&self) -> bool {
        match self {
            Type::Bool => false,
            Type::Address(_) => false,
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Rational => false,
            Type::Bytes(_) => false,
            Type::Enum(_) => false,
            Type::Struct(_) => true,
            Type::Array(_, dims) => dims[0].is_some(),
            Type::DynamicBytes => false,
            Type::String => false,
            Type::Mapping(..) => false,
            Type::Contract(_) => false,
            Type::Ref(_) => false,
            Type::StorageRef(..) => false,
            Type::InternalFunction { .. } => false,
            Type::ExternalFunction { .. } => false,
            Type::Slice => false,
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
            Type::Mapping(_, v) => Type::StorageRef(false, v.clone()),
            Type::DynamicBytes => Type::Bytes(1),
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
            Type::Array(_, dim) => dim.last().unwrap().as_ref(),
            _ => panic!("array_length on non-array"),
        }
    }

    /// Calculate how much memory we expect this type to use when allocated on the
    /// stack or on the heap. Depending on the llvm implementation there might be
    /// padding between elements which is not accounted for.
    pub fn size_of(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::Enum(_) => BigInt::one(),
            Type::Bool => BigInt::one(),
            Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
            Type::Bytes(n) => BigInt::from(*n),
            Type::Value => BigInt::from(ns.value_length),
            Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
            Type::Rational => unreachable!(),
            Type::Array(ty, dims) => {
                let pointer_size = BigInt::from(4);
                ty.size_of(ns).mul(
                    dims.iter()
                        .map(|d| match d {
                            None => &pointer_size,
                            Some(d) => d,
                        })
                        .product::<BigInt>(),
                )
            }
            Type::Struct(n) => ns.structs[*n]
                .offsets
                .last()
                .cloned()
                .unwrap_or_else(BigInt::zero),
            Type::String | Type::DynamicBytes => BigInt::from(4),
            Type::InternalFunction { .. } => BigInt::from(ns.target.ptr_size()),
            Type::ExternalFunction { .. } => {
                // Address and selector
                Type::Address(false).size_of(ns) + Type::Uint(32).size_of(ns)
            }
            Type::Mapping(..) => BigInt::zero(),
            Type::Ref(ty) | Type::StorageRef(_, ty) => ty.size_of(ns),
            Type::UserType(no) => ns.user_types[*no].ty.size_of(ns),
            _ => unimplemented!("sizeof on {:?}", self),
        }
    }

    /// Does this type fit into memory
    pub fn fits_in_memory(&self, ns: &Namespace) -> bool {
        self.size_of(ns) < BigInt::from(u16::MAX)
    }

    /// Calculate the alignment
    pub fn align_of(&self, ns: &Namespace) -> usize {
        match self {
            Type::Uint(8) | Type::Int(8) => 1,
            Type::Uint(n) | Type::Int(n) if *n <= 16 => 2,
            Type::Uint(n) | Type::Int(n) if *n <= 32 => 4,
            Type::Uint(_) | Type::Int(_) => 8,
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| f.ty.align_of(ns))
                .max()
                .unwrap(),
            Type::InternalFunction { .. } => ns.target.ptr_size().into(),
            _ => 1,
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
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Value => false,
            _ => false,
        }
    }

    /// Calculate how many storage slots a type occupies. Note that storage arrays can
    /// be very large
    pub fn storage_slots(&self, ns: &Namespace) -> BigInt {
        if ns.target == Target::Solana {
            match self {
                Type::Enum(_) => BigInt::one(),
                Type::Bool => BigInt::one(),
                Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
                Type::Bytes(n) => BigInt::from(*n),
                Type::Value => BigInt::from(ns.value_length),
                Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
                Type::Rational => unreachable!(),
                Type::Array(_, dims) if dims[0].is_none() => BigInt::from(4),
                Type::Array(ty, dims) => {
                    let pointer_size = BigInt::from(4);
                    if self.is_sparse_solana(ns) {
                        BigInt::from(SOLANA_BUCKET_SIZE) * BigInt::from(4)
                    } else {
                        ty.storage_slots(ns).mul(
                            dims.iter()
                                .map(|d| match d {
                                    None => &pointer_size,
                                    Some(d) => d,
                                })
                                .product::<BigInt>(),
                        )
                    }
                }
                Type::Struct(n) => ns.structs[*n]
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
                Type::Ref(ty) | Type::StorageRef(_, ty) => ty.storage_slots(ns),
                _ => unimplemented!(),
            }
        } else {
            match self {
                Type::StorageRef(_, r) | Type::Ref(r) => r.storage_slots(ns),
                Type::Struct(n) => ns.structs[*n]
                    .fields
                    .iter()
                    .map(|f| f.ty.storage_slots(ns))
                    .sum(),
                Type::Array(ty, dims) => {
                    let one = BigInt::one();

                    ty.storage_slots(ns)
                        * dims
                            .iter()
                            .map(|l| match l {
                                None => &one,
                                Some(l) => l,
                            })
                            .product::<BigInt>()
                }
                _ => BigInt::one(),
            }
        }
    }

    /// Alignment of elements in storage
    pub fn storage_align(&self, ns: &Namespace) -> BigInt {
        if ns.target == Target::Solana {
            let length = match self {
                Type::Enum(_) => BigInt::one(),
                Type::Bool => BigInt::one(),
                Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
                Type::Bytes(n) => BigInt::from(*n),
                Type::Value => BigInt::from(ns.value_length),
                Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
                Type::Rational => unreachable!(),
                Type::Array(_, dims) if dims[0].is_none() => BigInt::from(4),
                Type::Array(ty, _) => {
                    if self.is_sparse_solana(ns) {
                        BigInt::from(4)
                    } else {
                        ty.storage_align(ns)
                    }
                }
                Type::Struct(n) => ns.structs[*n]
                    .fields
                    .iter()
                    .map(|field| field.ty.storage_align(ns))
                    .max()
                    .unwrap(),
                Type::String | Type::DynamicBytes => BigInt::from(4),
                Type::InternalFunction { .. } => BigInt::from(ns.target.ptr_size()),
                Type::ExternalFunction { .. } => BigInt::from(ns.address_length),
                Type::Mapping(..) => BigInt::from(4),
                Type::Ref(ty) | Type::StorageRef(_, ty) => ty.storage_align(ns),
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
            Type::ExternalFunction { .. } => false,
            Type::UserType(no) => ns.user_types[*no].ty.is_reference_type(ns),
            _ => false,
        }
    }

    /// Does this type contain any types which are variable-length
    pub fn is_dynamic(&self, ns: &Namespace) -> bool {
        match self {
            Type::String | Type::DynamicBytes => true,
            Type::Ref(r) => r.is_dynamic(ns),
            Type::Array(ty, dim) => {
                if dim.iter().any(|d| d.is_none()) {
                    return true;
                }

                ty.is_dynamic(ns)
            }
            Type::Struct(n) => ns.structs[*n].fields.iter().any(|f| f.ty.is_dynamic(ns)),
            Type::StorageRef(_, r) => r.is_dynamic(ns),
            _ => false,
        }
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

    /// Is this a reference to contract storage?
    pub fn is_dynamic_memory(&self) -> bool {
        match self {
            Type::DynamicBytes => true,
            Type::Array(_, dim) if dim.len() == 1 && dim[0].is_none() => true,
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
        match self {
            Type::Mapping(..) => true,
            Type::Array(ty, _) => ty.contains_mapping(ns),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .any(|f| f.ty.contains_mapping(ns)),
            Type::StorageRef(_, r) | Type::Ref(r) => r.contains_mapping(ns),
            _ => false,
        }
    }

    /// Does the type contain any internal function type
    pub fn contains_internal_function(&self, ns: &Namespace) -> bool {
        match self {
            Type::InternalFunction { .. } => true,
            Type::Array(ty, _) => ty.contains_internal_function(ns),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .any(|f| f.ty.contains_internal_function(ns)),
            Type::StorageRef(_, r) | Type::Ref(r) => r.contains_internal_function(ns),
            _ => false,
        }
    }

    /// Is this structure a builtin
    pub fn builtin_struct(&self, ns: &Namespace) -> BuiltinStruct {
        match self {
            Type::Struct(n) => ns.structs[*n].builtin,
            Type::StorageRef(_, r) | Type::Ref(r) => r.builtin_struct(ns),
            _ => BuiltinStruct::None,
        }
    }

    /// Does the type contain any builtin type
    pub fn contains_builtins<'a>(
        &'a self,
        ns: &'a Namespace,
        builtin: BuiltinStruct,
    ) -> Option<&'a Type> {
        match self {
            Type::Array(ty, _) => ty.contains_builtins(ns, builtin),
            Type::Mapping(key, value) => key
                .contains_builtins(ns, builtin)
                .or_else(|| value.contains_builtins(ns, builtin)),
            Type::Struct(n) if ns.structs[*n].builtin == builtin => Some(self),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .find_map(|f| f.ty.contains_builtins(ns, builtin)),
            Type::StorageRef(_, r) | Type::Ref(r) => r.contains_builtins(ns, builtin),
            _ => None,
        }
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
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(i) => format!("{}", ns.enums[*i]),
            Type::Struct(i) => format!("{}", ns.structs[*i]),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_llvm_string(ns),
                len.iter()
                    .map(|r| match r {
                        None => ":".to_string(),
                        Some(r) => format!(":{}", r),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(k, v) => {
                format!("mapping:{}:{}", k.to_llvm_string(ns), v.to_llvm_string(ns))
            }
            Type::Contract(i) => ns.contracts[*i].name.to_owned(),
            Type::InternalFunction { .. } => "function".to_owned(),
            Type::ExternalFunction { .. } => "function".to_owned(),
            Type::Ref(r) => r.to_llvm_string(ns),
            Type::StorageRef(_, r) => r.to_llvm_string(ns),
            Type::UserType(no) => ns.user_types[*no].ty.to_llvm_string(ns),
            _ => unreachable!(),
        }
    }

    /// Is this type sparse on Solana
    pub fn is_sparse_solana(&self, ns: &Namespace) -> bool {
        match self.deref_any() {
            Type::Mapping(..) => true,
            Type::Array(_, dims) if dims[0].is_none() => false,
            Type::Array(ty, dims) => {
                let pointer_size = BigInt::from(4);
                let len = ty.storage_slots(ns).mul(
                    dims.iter()
                        .map(|d| match d {
                            None => &pointer_size,
                            Some(d) => d,
                        })
                        .product::<BigInt>(),
                );

                len >= BigInt::from(SOLANA_SPARSE_ARRAY_SIZE)
            }
            _ => false,
        }
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
