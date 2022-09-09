// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]
use crate::sema::ast::{Expression, Parameter, Statement, TryCatch, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::unescape;
use crate::sema::yul::ast::InlineAssembly;
use crate::{parse_and_resolve, sema::ast, FileResolver, Target};
use solang_parser::pt::Loc;
use std::ffi::OsStr;

pub(crate) fn parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::new();
    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);
    ns.print_diagnostics_in_plain(&cache, false);
    ns
}

#[test]
fn test_unescape() {
    let s = r#"\u00f3"#;
    let mut vec = Diagnostics::default();
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![0xc3, 0xb3]);
    let s = r#"\xff"#;
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![255]);
}

#[test]
fn test_statement_reachable() {
    let loc = Loc::File(0, 1, 2);
    let test_cases: Vec<(Statement, bool)> = vec![
        (Statement::Underscore(loc), true),
        (
            Statement::Destructure(loc, vec![], Expression::BoolLiteral(loc, true)),
            true,
        ),
        (
            Statement::VariableDecl(
                loc,
                0,
                Parameter {
                    loc,
                    id: None,
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                None,
            ),
            true,
        ),
        (
            Statement::Emit {
                loc,
                event_no: 0,
                event_loc: Loc::Builtin,
                args: vec![],
            },
            true,
        ),
        (
            Statement::Delete(loc, Type::Bool, Expression::BoolLiteral(loc, true)),
            true,
        ),
        (Statement::Continue(loc), false),
        (Statement::Break(loc), false),
        (Statement::Return(loc, None), false),
        (
            Statement::If(
                loc,
                false,
                Expression::BoolLiteral(loc, false),
                vec![],
                vec![],
            ),
            false,
        ),
        (
            Statement::While(loc, true, Expression::BoolLiteral(loc, false), vec![]),
            true,
        ),
        (
            Statement::DoWhile(loc, false, vec![], Expression::BoolLiteral(loc, true)),
            false,
        ),
        (
            Statement::Expression(loc, true, Expression::BoolLiteral(loc, false)),
            true,
        ),
        (
            Statement::For {
                loc,
                reachable: false,
                init: vec![],
                cond: None,
                next: vec![],
                body: vec![],
            },
            false,
        ),
        (
            Statement::TryCatch(
                loc,
                true,
                TryCatch {
                    expr: Expression::BoolLiteral(loc, false),
                    returns: vec![],
                    ok_stmt: vec![],
                    errors: vec![],
                    catch_param: None,
                    catch_param_pos: None,
                    catch_stmt: vec![],
                },
            ),
            true,
        ),
        (
            Statement::Assembly(
                InlineAssembly {
                    loc,
                    body: vec![],
                    functions: std::ops::Range { start: 0, end: 0 },
                },
                false,
            ),
            false,
        ),
    ];

    for (test_case, expected) in test_cases {
        assert_eq!(test_case.reachable(), expected);
    }
}

#[test]
fn constant_overflow() {
    let file = r#"
        contract test_contract {
            function test() public returns (int8) {
                int8 sesa_ovf = 127 + 6;
                int8 sesa_ovf
                return 1;
            }
        }
    
        "#;
    let ns = parse(file);
    assert!(ns.diagnostics.contains_message("Type int_const 133 is not implicitly convertible to expected type Int(8). Literal is too large to fit in Int(8)."));
}
// Add more test cases here
