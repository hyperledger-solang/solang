#![cfg(test)]
use crate::ast::{Expression, Parameter, Statement, TryCatch, Type};
use crate::sema::expression::unescape;
use crate::sema::yul::ast::InlineAssembly;
use solang_parser::pt::Loc;
use solang_parser::Diagnostic;

#[test]
fn test_unescape() {
    let s = r#"\u00f3"#;
    let mut vec: Vec<Diagnostic> = Vec::new();
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
            Statement::Destructure(loc, vec![], Expression::Undefined(Type::Bool)),
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
            Statement::Delete(loc, Type::Bool, Expression::Undefined(Type::Bool)),
            true,
        ),
        (Statement::Continue(loc), false),
        (Statement::Break(loc), false),
        (Statement::Return(loc, None), false),
        (
            Statement::If(
                loc,
                false,
                Expression::Undefined(Type::Bool),
                vec![],
                vec![],
            ),
            false,
        ),
        (
            Statement::While(loc, true, Expression::Undefined(Type::Bool), vec![]),
            true,
        ),
        (
            Statement::DoWhile(loc, false, vec![], Expression::Undefined(Type::Bool)),
            false,
        ),
        (
            Statement::Expression(loc, true, Expression::Undefined(Type::Bool)),
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
                    expr: Expression::Poison,
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
