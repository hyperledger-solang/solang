use super::{Contract, EnumDecl, Namespace, StructDecl, StructField, Symbol, Type};
use output::Output;
use parser::ast;
use std::collections::HashMap;
use Target;

/// Resolve all the types we can find (enums, structs, contracts). structs can have other
/// structs as fields, including ones that have not been declared yet.
pub fn resolve(s: &ast::SourceUnit, target: Target) -> (Namespace, Vec<Output>) {
    let mut errors = Vec::new();
    let mut ns = Namespace::new(
        target,
        match target {
            Target::Ewasm => 20,
            Target::Substrate => 32,
            Target::Sabre => 0, // substrate has no address type
        },
    );
    let mut structs = Vec::new();

    // Find all the types: contracts, enums, and structs. Either in a contract or not
    // We do not resolve the fields yet as we do not know all the possible types until we're
    // done
    for part in &s.0 {
        match part {
            ast::SourceUnitPart::PragmaDirective(name, value) => {
                if name.name == "solidity" {
                    errors.push(Output::info(
                        ast::Loc(name.loc.0, value.loc.1),
                        "pragma ‘solidity’ is ignored".to_string(),
                    ));
                } else if name.name == "experimental" && value.string == "ABIEncoderV2" {
                    errors.push(Output::info(
                        ast::Loc(name.loc.0, value.loc.1),
                        "pragma ‘experimental’ with value ‘ABIEncoderV2’ is ignored".to_string(),
                    ));
                } else {
                    errors.push(Output::warning(
                        ast::Loc(name.loc.0, value.loc.1),
                        format!(
                            "unknown pragma ‘{}’ with value ‘{}’ ignored",
                            name.name, value.string
                        ),
                    ));
                }
            }
            ast::SourceUnitPart::ContractDefinition(def) => {
                resolve_contract(&def, &mut structs, &mut errors, &mut ns);
            }
            ast::SourceUnitPart::EnumDefinition(def) => {
                let _ = enum_decl(&def, None, &mut ns, &mut errors);
            }
            ast::SourceUnitPart::StructDefinition(def) => {
                if ns.add_symbol(
                    None,
                    &def.name,
                    Symbol::Struct(def.name.loc, ns.structs.len()),
                    &mut errors,
                ) {
                    let s = StructDecl {
                        name: def.name.name.to_owned(),
                        loc: def.name.loc,
                        contract: None,
                        fields: Vec::new(),
                    };

                    structs.push((s, def, None));
                }
            }
            _ => (),
        }
    }

    // now we can resolve the fields for the structs
    for (mut decl, def, contract) in structs {
        if let Some(fields) = struct_decl(def, contract, &mut ns, &mut errors) {
            decl.fields = fields;
            ns.structs.push(decl);
        }
    }

    // struct can contain other structs, and we have to check for recursiveness,
    // i.e. "struct a { b f1; } struct b { a f1; }"
    for s in 0..ns.structs.len() {
        fn check(
            s: usize,
            struct_fields: &mut Vec<usize>,
            ns: &Namespace,
            errors: &mut Vec<Output>,
        ) {
            let def = &ns.structs[s];
            let mut types_seen = Vec::new();

            for field in &def.fields {
                if let Type::Struct(n) = field.ty {
                    if types_seen.contains(&n) {
                        continue;
                    }

                    types_seen.push(n);

                    if struct_fields.contains(&n) {
                        errors.push(Output::error_with_note(
                            def.loc,
                            format!("struct ‘{}’ has infinite size", def.name),
                            field.loc,
                            format!("recursive field ‘{}’", field.name),
                        ));
                    } else {
                        struct_fields.push(n);
                        check(n, struct_fields, ns, errors);
                    }
                }
            }
        };

        check(s, &mut vec![s], &ns, &mut errors);
    }

    (ns, errors)
}

/// Resolve all the types in a contract
fn resolve_contract<'a>(
    def: &'a ast::ContractDefinition,
    structs: &mut Vec<(StructDecl, &'a ast::StructDefinition, Option<usize>)>,
    errors: &mut Vec<Output>,
    ns: &mut Namespace,
) -> bool {
    let contract_no = ns.contracts.len();
    ns.contracts.push(Contract::new(&def.name.name));

    let mut broken = !ns.add_symbol(
        None,
        &def.name,
        Symbol::Contract(def.loc, contract_no),
        errors,
    );

    for parts in &def.parts {
        match parts {
            ast::ContractPart::EnumDefinition(ref e) => {
                if !enum_decl(e, Some(contract_no), ns, errors) {
                    broken = true;
                }
            }
            ast::ContractPart::StructDefinition(ref s) => {
                if ns.add_symbol(
                    Some(contract_no),
                    &s.name,
                    Symbol::Struct(s.name.loc, structs.len()),
                    errors,
                ) {
                    let decl = StructDecl {
                        name: s.name.name.to_owned(),
                        loc: s.name.loc,
                        contract: Some(def.name.name.to_owned()),
                        fields: Vec::new(),
                    };

                    structs.push((decl, s, Some(contract_no)));
                } else {
                    broken = true;
                }
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
    def: &ast::StructDefinition,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> Option<Vec<StructField>> {
    let mut valid = true;
    let mut fields: Vec<StructField> = Vec::new();

    for field in &def.fields {
        let ty = match ns.resolve_type(contract_no, false, &field.ty, errors) {
            Ok(s) => s,
            Err(()) => {
                valid = false;
                continue;
            }
        };

        if let Some(other) = fields.iter().find(|f| f.name == field.name.name) {
            errors.push(Output::error_with_note(
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
            errors.push(Output::error(
                *storage.loc(),
                format!(
                    "storage location ‘{}’ not allowed for struct field",
                    storage
                ),
            ));
            valid = false;
        }

        fields.push(StructField {
            loc: field.name.loc,
            name: field.name.name.to_string(),
            ty,
        });
    }

    if fields.is_empty() {
        if valid {
            errors.push(Output::error(
                def.name.loc,
                format!("struct definition for ‘{}’ has no fields", def.name.name),
            ));
        }

        valid = false;
    }

    if valid {
        Some(fields)
    } else {
        None
    }
}

/// Parse enum declaration. If the declaration is invalid, it is still generated
/// so that we can continue parsing, with errors recorded.
fn enum_decl(
    enum_: &ast::EnumDefinition,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> bool {
    let mut valid = true;

    let mut bits = if enum_.values.is_empty() {
        errors.push(Output::error(
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
    let mut entries: HashMap<String, (ast::Loc, usize)> = HashMap::new();

    for (i, e) in enum_.values.iter().enumerate() {
        if let Some(prev) = entries.get(&e.name.to_string()) {
            errors.push(Output::error_with_note(
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

    let decl = EnumDecl {
        name: enum_.name.name.to_string(),
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
        contract_no,
        &enum_.name,
        Symbol::Enum(enum_.name.loc, pos),
        errors,
    ) {
        valid = false;
    }

    valid
}

#[test]
fn enum_256values_is_uint8() {
    let mut e = ast::EnumDefinition {
        doc: vec![],
        name: ast::Identifier {
            loc: ast::Loc(0, 0),
            name: "foo".into(),
        },
        values: Vec::new(),
    };

    let mut ns = Namespace::new(Target::Ewasm, 20);

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "first".into(),
    });

    assert!(enum_decl(&e, None, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    for i in 1..256 {
        e.values.push(ast::Identifier {
            loc: ast::Loc(0, 0),
            name: format!("val{}", i),
        })
    }

    assert_eq!(e.values.len(), 256);

    e.name.name = "foo2".to_owned();
    assert!(enum_decl(&e, None, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "another".into(),
    });

    e.name.name = "foo3".to_owned();
    assert!(enum_decl(&e, None, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(16));
}
