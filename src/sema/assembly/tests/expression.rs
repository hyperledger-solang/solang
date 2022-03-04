#![cfg(test)]
use crate::ast::{Namespace, Type};
use crate::sema::assembly::expression::{resolve_assembly_expression, AssemblyExpression};
use crate::Target;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use solang_parser::pt;
use solang_parser::pt::{HexLiteral, Identifier, Loc, StringLiteral};

#[test]
fn resolve_bool_literal() {
    let mut ns = Namespace::new(Target::Solana);
    let expr = pt::AssemblyExpression::BoolLiteral(
        Loc::File(0, 3, 5),
        false,
        Some(pt::Identifier {
            loc: Loc::File(0, 3, 4),
            name: "u32".to_string(),
        }),
    );

    let resolved_type = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved_type.is_ok());
    assert!(ns.diagnostics.is_empty());
    let unwrapped = resolved_type.unwrap();

    assert_eq!(
        unwrapped,
        AssemblyExpression::BoolLiteral(Loc::File(0, 3, 5), false, Type::Uint(32))
    );

    let expr = pt::AssemblyExpression::BoolLiteral(Loc::File(0, 3, 5), true, None);
    let resolved_type = resolve_assembly_expression(&expr, &mut ns);

    assert!(resolved_type.is_ok());
    assert!(ns.diagnostics.is_empty());
    let unwrapped = resolved_type.unwrap();
    assert_eq!(
        unwrapped,
        AssemblyExpression::BoolLiteral(Loc::File(0, 3, 5), true, Type::Bool)
    );
}

#[test]
fn resolve_number_literal() {
    let loc = Loc::File(0, 3, 5);
    let mut ns = Namespace::new(Target::Solana);
    let expr = pt::AssemblyExpression::NumberLiteral(
        loc,
        BigInt::from_u128(0xffffffffffffffffff).unwrap(),
        Some(Identifier {
            loc,
            name: "u64".to_string(),
        }),
    );
    let parsed = resolve_assembly_expression(&expr, &mut ns);
    assert!(parsed.is_ok());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided literal requires 72 bits, but the type only supports 64"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::NumberLiteral(
        loc,
        BigInt::from_i32(-50).unwrap(),
        Some(Identifier {
            loc,
            name: "u128".to_string(),
        }),
    );
    let parsed = resolve_assembly_expression(&expr, &mut ns);
    assert!(parsed.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "singed value cannot fit in unsigned type"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::NumberLiteral(loc, BigInt::from(20), None);
    let parsed = resolve_assembly_expression(&expr, &mut ns);
    assert!(parsed.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        parsed.unwrap(),
        AssemblyExpression::NumberLiteral(loc, BigInt::from(20), Type::Uint(256))
    );
}

#[test]
fn resolve_hex_number_literal() {
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::AssemblyExpression::HexNumberLiteral(
        loc,
        "0xf23456789a".to_string(),
        Some(Identifier {
            loc,
            name: "u32".to_string(),
        }),
    );

    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_ok());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided literal requires 40 bits, but the type only supports 32"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::HexNumberLiteral(
        loc,
        "0xff".to_string(),
        Some(Identifier {
            loc,
            name: "s64".to_string(),
        }),
    );
    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        AssemblyExpression::NumberLiteral(loc, BigInt::from(255), Type::Int(64))
    );
}

#[test]
fn resolve_hex_string_literal() {
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::AssemblyExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "3ca".to_string(),
        },
        None,
    );

    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "hex string \"3ca\" has odd number of characters"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "acdf".to_string(),
        },
        Some(Identifier {
            loc,
            name: "myType".to_string(),
        }),
    );
    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the specified type 'myType' does not exist"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "ffff".to_string(),
        },
        Some(Identifier {
            loc,
            name: "u256".to_string(),
        }),
    );
    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        AssemblyExpression::StringLiteral(loc, vec![255, 255], Type::Uint(256))
    );
}

#[test]
fn resolve_string_literal() {
    let mut ns = Namespace::new(Target::Solana);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::AssemblyExpression::StringLiteral(
        StringLiteral {
            loc,
            string: r#"ab\xffa\u00e0g"#.to_string(),
        },
        Some(Identifier {
            loc,
            name: "u128".to_string(),
        }),
    );

    let resolved = resolve_assembly_expression(&expr, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        AssemblyExpression::StringLiteral(
            loc,
            vec![97, 98, 255, 97, 0xc3, 0xa0, 103],
            Type::Uint(128)
        )
    );
}
