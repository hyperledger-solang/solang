use super::diagnostics::any_errors;
use super::tags::resolve_tags;
use super::SOLANA_BUCKET_SIZE;
use super::{
    ast::{
        Contract, Diagnostic, EnumDecl, EventDecl, Namespace, Parameter, StructDecl, Symbol, Tag,
        Type,
    },
    SOLANA_SPARSE_ARRAY_SIZE,
};
use crate::parser::pt;
use crate::Target;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::HashMap;
use std::ops::Mul;

/// List the types which should be resolved later
pub struct ResolveFields<'a> {
    pub structs: Vec<(usize, &'a pt::StructDefinition, Option<usize>)>,
    pub events: Vec<(usize, &'a pt::EventDefinition, Option<usize>)>,
}

/// Resolve all the types we can find (enums, structs, contracts). structs can have other
/// structs as fields, including ones that have not been declared yet.
pub fn resolve_typenames<'a>(
    s: &'a pt::SourceUnit,
    file_no: usize,
    ns: &mut Namespace,
) -> ResolveFields<'a> {
    let mut delay = ResolveFields {
        structs: Vec::new(),
        events: Vec::new(),
    };

    // Find all the types: contracts, enums, and structs. Either in a contract or not
    // We do not resolve the struct fields yet as we do not know all the possible types until we're
    // done
    for part in &s.0 {
        match part {
            pt::SourceUnitPart::ContractDefinition(def) => {
                resolve_contract(def, file_no, &mut delay, ns);
            }
            pt::SourceUnitPart::EnumDefinition(def) => {
                let _ = enum_decl(def, file_no, None, ns);
            }
            pt::SourceUnitPart::StructDefinition(def) => {
                let pos = ns.structs.len();

                if ns.add_symbol(file_no, None, &def.name, Symbol::Struct(def.name.loc, pos)) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: def.name.name.to_owned(),
                        loc: def.name.loc,
                        contract: None,
                        fields: Vec::new(),
                        offsets: Vec::new(),
                    });

                    delay.structs.push((pos, def, None));
                }
            }
            pt::SourceUnitPart::EventDefinition(def) => {
                let pos = ns.events.len();

                if let Some(Symbol::Event(events)) =
                    ns.variable_symbols
                        .get_mut(&(file_no, None, def.name.name.to_owned()))
                {
                    events.push((def.name.loc, pos));
                } else if !ns.add_symbol(
                    file_no,
                    None,
                    &def.name,
                    Symbol::Event(vec![(def.name.loc, pos)]),
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

                delay.events.push((pos, def, None));
            }
            _ => (),
        }
    }

    delay
}

pub fn resolve_fields(delay: ResolveFields, file_no: usize, ns: &mut Namespace) {
    // now we can resolve the fields for the structs
    for (pos, def, contract) in delay.structs {
        if let Some((tags, fields)) = struct_decl(def, file_no, contract, ns) {
            ns.structs[pos].tags = tags;
            ns.structs[pos].fields = fields;
        }
    }

    // struct can contain other structs, and we have to check for recursiveness,
    // i.e. "struct a { b f1; } struct b { a f1; }"
    for s in 0..ns.structs.len() {
        fn check(s: usize, file_no: usize, struct_fields: &mut Vec<usize>, ns: &mut Namespace) {
            let def = ns.structs[s].clone();
            let mut types_seen = Vec::new();

            for field in &def.fields {
                if let Type::Struct(n) = field.ty {
                    if types_seen.contains(&n) {
                        continue;
                    }

                    types_seen.push(n);

                    if struct_fields.contains(&n) {
                        ns.diagnostics.push(Diagnostic::error_with_note(
                            def.loc,
                            format!("struct ‘{}’ has infinite size", def.name),
                            field.loc,
                            format!("recursive field ‘{}’", field.name),
                        ));
                    } else {
                        struct_fields.push(n);
                        check(n, file_no, struct_fields, ns);
                    }
                }
            }
        }

        check(s, file_no, &mut vec![s], ns);
    }

    // Do not attempt to call struct offsets if there are any infinitely recursive structs
    if !any_errors(&ns.diagnostics) {
        struct_offsets(ns);
    }

    // now we can resolve the fields for the events
    for (pos, def, contract) in delay.events {
        if let Some((tags, fields)) = event_decl(def, file_no, contract, ns) {
            ns.events[pos].signature = ns.signature(&ns.events[pos].name, &fields);
            ns.events[pos].fields = fields;
            ns.events[pos].tags = tags;
        }
    }
}

/// Resolve all the types in a contract
fn resolve_contract<'a>(
    def: &'a pt::ContractDefinition,
    file_no: usize,
    delay: &mut ResolveFields<'a>,
    ns: &mut Namespace,
) -> bool {
    let contract_no = ns.contracts.len();

    let doc = resolve_tags(def.name.loc.0, "contract", &def.doc, None, None, None, ns);

    ns.contracts
        .push(Contract::new(&def.name.name, def.ty.clone(), doc, def.loc));

    let mut broken = !ns.add_symbol(
        file_no,
        None,
        &def.name,
        Symbol::Contract(def.loc, contract_no),
    );

    for parts in &def.parts {
        match parts {
            pt::ContractPart::EnumDefinition(ref e) => {
                if !enum_decl(e, file_no, Some(contract_no), ns) {
                    broken = true;
                }
            }
            pt::ContractPart::StructDefinition(ref s) => {
                let pos = ns.structs.len();

                if ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    &s.name,
                    Symbol::Struct(s.name.loc, pos),
                ) {
                    ns.structs.push(StructDecl {
                        tags: Vec::new(),
                        name: s.name.name.to_owned(),
                        loc: s.name.loc,
                        contract: Some(def.name.name.to_owned()),
                        fields: Vec::new(),
                        offsets: Vec::new(),
                    });

                    delay.structs.push((pos, s, Some(contract_no)));
                } else {
                    broken = true;
                }
            }
            pt::ContractPart::EventDefinition(ref s) => {
                let pos = ns.events.len();

                if let Some(Symbol::Event(events)) = ns.variable_symbols.get_mut(&(
                    file_no,
                    Some(contract_no),
                    s.name.name.to_owned(),
                )) {
                    events.push((s.name.loc, pos));
                } else if !ns.add_symbol(
                    file_no,
                    Some(contract_no),
                    &s.name,
                    Symbol::Event(vec![(s.name.loc, pos)]),
                ) {
                    broken = true;
                    continue;
                }

                ns.events.push(EventDecl {
                    tags: Vec::new(),
                    name: s.name.name.to_owned(),
                    loc: s.name.loc,
                    contract: Some(contract_no),
                    fields: Vec::new(),
                    anonymous: s.anonymous,
                    signature: String::new(),
                    used: false,
                });

                delay.events.push((pos, s, Some(contract_no)));
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

        if let Some(other) = fields.iter().find(|f| f.name == field.name.name) {
            ns.diagnostics.push(Diagnostic::error_with_note(
                field.name.loc,
                format!(
                    "struct ‘{}’ has duplicate struct field ‘{}’",
                    def.name.name, field.name.name
                ),
                other.loc,
                format!("location of previous declaration of ‘{}’", other.name),
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
                *storage.loc(),
                format!(
                    "storage location ‘{}’ not allowed for struct field",
                    storage
                ),
            ));
            valid = false;
        }

        fields.push(Parameter {
            loc: field.loc,
            name_loc: Some(field.name.loc),
            name: field.name.name.to_string(),
            ty,
            ty_loc: field.ty.loc(),
            indexed: false,
        });
    }

    if fields.is_empty() {
        if valid {
            ns.diagnostics.push(Diagnostic::error(
                def.name.loc,
                format!("struct definition for ‘{}’ has no fields", def.name.name),
            ));
        }

        valid = false;
    }

    if valid {
        let doc = resolve_tags(
            def.name.loc.0,
            "struct",
            &def.doc,
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

        let (name, name_loc) = if let Some(name) = &field.name {
            if let Some(other) = fields.iter().find(|f| f.name == name.name) {
                ns.diagnostics.push(Diagnostic::error_with_note(
                    name.loc,
                    format!(
                        "event ‘{}’ has duplicate field name ‘{}’",
                        def.name.name, name.name
                    ),
                    other.loc,
                    format!("location of previous declaration of ‘{}’", other.name),
                ));
                valid = false;
                continue;
            }
            (name.name.to_owned(), Some(name.loc))
        } else {
            (String::new(), None)
        };

        if field.indexed {
            indexed_fields += 1;
        }

        fields.push(Parameter {
            loc: field.loc,
            name,
            name_loc,
            ty,
            ty_loc: field.ty.loc(),
            indexed: field.indexed,
        });
    }

    if def.anonymous && indexed_fields > 4 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.loc,
            format!(
                "anonymous event definition for ‘{}’ has {} indexed fields where 4 permitted",
                def.name.name, indexed_fields
            ),
        ));

        valid = false;
    } else if !def.anonymous && indexed_fields > 3 {
        ns.diagnostics.push(Diagnostic::error(
            def.name.loc,
            format!(
                "event definition for ‘{}’ has {} indexed fields where 3 permitted",
                def.name.name, indexed_fields
            ),
        ));

        valid = false;
    }

    if valid {
        let doc = resolve_tags(
            def.name.loc.0,
            "event",
            &def.doc,
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
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> bool {
    let mut valid = true;

    let mut bits = if enum_.values.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            enum_.name.loc,
            format!("enum ‘{}’ is missing fields", enum_.name.name),
        ));
        valid = false;

        0
    } else {
        // Number of bits required to represent this enum
        std::mem::size_of::<usize>() as u32 * 8 - (enum_.values.len() - 1).leading_zeros()
    };

    // round it up to the next
    if bits <= 8 {
        bits = 8;
    } else {
        bits += 7;
        bits -= bits % 8;
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

    let tags = resolve_tags(enum_.name.loc.0, "enum", &enum_.doc, None, None, None, ns);

    let decl = EnumDecl {
        tags,
        name: enum_.name.name.to_string(),
        loc: enum_.loc,
        contract: match contract_no {
            Some(c) => Some(ns.contracts[c].name.to_owned()),
            None => None,
        },
        ty: Type::Uint(bits as u16),
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
    let mut changes;

    while {
        changes = false;
        for struct_no in 0..ns.structs.len() {
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

            offsets.push(offset.clone());

            if ns.structs[struct_no].offsets != offsets {
                ns.structs[struct_no].offsets = offsets;
                changes = true;
            }
        }

        changes
    } {}
}

#[test]
fn enum_256values_is_uint8() {
    let mut e = pt::EnumDefinition {
        doc: vec![],
        loc: pt::Loc(0, 0, 0),
        name: pt::Identifier {
            loc: pt::Loc(0, 0, 0),
            name: "foo".into(),
        },
        values: Vec::new(),
    };

    let mut ns = Namespace::new(Target::Ewasm, 20, 16);

    e.values.push(pt::Identifier {
        loc: pt::Loc(0, 0, 0),
        name: "first".into(),
    });

    assert!(enum_decl(&e, 0, None, &mut ns));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    for i in 1..256 {
        e.values.push(pt::Identifier {
            loc: pt::Loc(0, 0, 0),
            name: format!("val{}", i),
        })
    }

    assert_eq!(e.values.len(), 256);

    e.name.name = "foo2".to_owned();
    assert!(enum_decl(&e, 0, None, &mut ns));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    e.values.push(pt::Identifier {
        loc: pt::Loc(0, 0, 0),
        name: "another".into(),
    });

    e.name.name = "foo3".to_owned();
    assert!(enum_decl(&e, 0, None, &mut ns));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(16));
}

impl Type {
    pub fn to_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address(false) => "address".to_string(),
            Type::Address(true) => "address payable".to_string(),
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
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
                    s.push_str(&format!(" {}", mutability));
                }

                if !returns.is_empty() {
                    s.push_str(&format!(
                        " returns ({})",
                        returns
                            .iter()
                            .map(|ty| ty.to_string(ns))
                            .collect::<Vec<String>>()
                            .join(",")
                    ));
                }

                s
            }
            Type::Contract(n) => format!("contract {}", ns.contracts[*n].name),
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
            Type::Value => true,
            Type::Ref(r) => r.is_primitive(),
            Type::StorageRef(_, r) => r.is_primitive(),
            _ => false,
        }
    }

    pub fn to_signature_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Contract(_) | Type::Address(_) if ns.address_length == 20 => {
                "address".to_string()
            }
            Type::Contract(_) | Type::Address(_) => format!("bytes{}", ns.address_length),
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(n) => ns.enums[*n].ty.to_signature_string(ns),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_signature_string(ns),
                len.iter()
                    .map(|l| match l {
                        None => "[]".to_string(),
                        Some(l) => format!("[{}]", l),
                    })
                    .collect::<String>()
            ),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(_, r) => r.to_string(ns),
            Type::Struct(struct_no) => {
                format!(
                    "({})",
                    ns.structs[*struct_no]
                        .fields
                        .iter()
                        .map(|f| f.ty.to_signature_string(ns))
                        .collect::<Vec<String>>()
                        .join(",")
                )
            }
            Type::InternalFunction { .. } | Type::ExternalFunction { .. } => "function".to_owned(),
            _ => unreachable!(),
        }
    }

    /// Give the type of an memory array after dereference.
    pub fn array_deref(&self) -> Self {
        match self {
            Type::String | Type::DynamicBytes => Type::Ref(Box::new(Type::Uint(8))),
            Type::Ref(t) => t.array_deref(),
            Type::Array(ty, dim) if dim.len() > 1 => {
                Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())
            }
            Type::Array(ty, dim) if dim.len() == 1 => Type::Ref(ty.clone()),
            Type::Bytes(_) => Type::Bytes(1),
            _ => panic!("deref on non-array"),
        }
    }

    /// Given an array, return the type of its elements
    pub fn array_elem(&self) -> Self {
        match self {
            Type::Array(ty, dim) if dim.len() > 1 => {
                Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())
            }
            Type::Array(ty, dim) if dim.len() == 1 => *ty.clone(),
            _ => panic!("not an array"),
        }
    }

    /// Give the type of an storage array after dereference. This can only be used on
    /// array types and will cause a panic otherwise.
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
            Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
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
            Type::Mapping(_, _) => BigInt::zero(),
            _ => unimplemented!(),
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
            Type::InternalFunction { .. } => ns.target.ptr_size(),
            _ => 1,
        }
    }

    pub fn bits(&self, ns: &Namespace) -> u16 {
        match self {
            Type::Address(_) => ns.address_length as u16 * 8,
            Type::Bool => 1,
            Type::Int(n) => *n,
            Type::Uint(n) => *n,
            Type::Bytes(n) => *n as u16 * 8,
            Type::Enum(n) => ns.enums[*n].ty.bits(ns),
            Type::Value => ns.value_length as u16 * 8,
            Type::StorageRef(_, _) => ns.storage_type().bits(ns),
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

    /// Calculate how many storage slots a type occupies. Note that storage arrays can
    /// be very large
    pub fn storage_slots(&self, ns: &Namespace) -> BigInt {
        if ns.target == Target::Solana {
            if self.is_sparse_solana(ns) {
                BigInt::from(SOLANA_BUCKET_SIZE) * ns.storage_type().storage_slots(ns)
            } else {
                self.size_of(ns)
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

    /// Is this type an reference type in the solidity language? (struct, array, mapping)
    pub fn is_reference_type(&self) -> bool {
        match self {
            Type::Bool => false,
            Type::Address(_) => false,
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Bytes(_) => false,
            Type::Enum(_) => false,
            Type::Struct(_) => true,
            Type::Array(_, _) => true,
            Type::DynamicBytes => true,
            Type::String => true,
            Type::Mapping(_, _) => true,
            Type::Contract(_) => false,
            Type::Ref(r) => r.is_reference_type(),
            Type::StorageRef(_, r) => r.is_reference_type(),
            Type::InternalFunction { .. } => false,
            Type::ExternalFunction { .. } => false,
            _ => unreachable!(),
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
            Type::Array(_, _)
                | Type::Struct(_)
                | Type::Mapping(_, _)
                | Type::String
                | Type::DynamicBytes
        )
    }

    /// Is this a reference to contract storage?
    pub fn is_contract_storage(&self) -> bool {
        matches!(self, Type::StorageRef(_, _))
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
            Type::Mapping(_, _) => true,
            Type::StorageRef(_, ty) => ty.is_mapping(),
            _ => false,
        }
    }

    /// Does the type contain any mapping type
    pub fn contains_mapping(&self, ns: &Namespace) -> bool {
        match self {
            Type::Mapping(_, _) => true,
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

    /// If the type is Ref or StorageRef, get the underlying type
    pub fn deref_any(&self) -> &Self {
        match self {
            Type::StorageRef(_, r) => r,
            Type::Ref(r) => r,
            _ => self,
        }
    }

    /// If the type is Ref or StorageRef, get the underlying type
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
    pub fn to_wasm_string(&self, ns: &Namespace) -> String {
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
                ty.to_wasm_string(ns),
                len.iter()
                    .map(|r| match r {
                        None => ":".to_string(),
                        Some(r) => format!(":{}", r),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(k, v) => {
                format!("mapping:{}:{}", k.to_wasm_string(ns), v.to_wasm_string(ns))
            }
            Type::Contract(i) => ns.contracts[*i].name.to_owned(),
            Type::InternalFunction { .. } => "function".to_owned(),
            Type::ExternalFunction { .. } => "function".to_owned(),
            Type::Ref(r) => r.to_wasm_string(ns),
            Type::StorageRef(_, r) => r.to_wasm_string(ns),
            _ => unreachable!(),
        }
    }

    /// Is this type sparse on Solana
    pub fn is_sparse_solana(&self, ns: &Namespace) -> bool {
        match self.deref_any() {
            Type::Mapping(_, _) => true,
            Type::Array(ty, dims) => {
                if let Some(len) = &dims[0] {
                    ty.storage_slots(ns) * len >= BigInt::from(SOLANA_SPARSE_ARRAY_SIZE)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
