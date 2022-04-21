#![cfg(test)]

use crate::ast::{Namespace, Parameter, Symbol, Type, Variable};
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{Symtable, VariableInitializer, VariableUsage};
use crate::sema::yul::ast::{YulExpression, YulSuffix};
use crate::sema::yul::builtin::YulBuiltInFunction;
use crate::sema::yul::expression::{check_type, resolve_yul_expression};
use crate::sema::yul::functions::FunctionsTable;
use crate::sema::yul::tests::{assert_message_in_diagnostics, parse};
use crate::{ast, Target};
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use solang_parser::pt;
use solang_parser::pt::{
    ContractTy, HexLiteral, Identifier, Loc, StorageLocation, StringLiteral, Visibility,
    YulFunctionCall,
};

#[test]
fn resolve_bool_literal() {
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);

    let mut ns = Namespace::new(Target::Solana);
    let expr = pt::YulExpression::BoolLiteral(
        Loc::File(0, 3, 5),
        false,
        Some(pt::Identifier {
            loc: Loc::File(0, 3, 4),
            name: "u32".to_string(),
        }),
    );

    let resolved_type =
        resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved_type.is_ok());
    assert!(ns.diagnostics.is_empty());
    let unwrapped = resolved_type.unwrap();

    assert_eq!(
        unwrapped,
        YulExpression::BoolLiteral(Loc::File(0, 3, 5), false, Type::Uint(32))
    );

    let expr = pt::YulExpression::BoolLiteral(Loc::File(0, 3, 5), true, None);
    let resolved_type =
        resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);

    assert!(resolved_type.is_ok());
    assert!(ns.diagnostics.is_empty());
    let unwrapped = resolved_type.unwrap();
    assert_eq!(
        unwrapped,
        YulExpression::BoolLiteral(Loc::File(0, 3, 5), true, Type::Bool)
    );
}

#[test]
fn resolve_number_literal() {
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);

    let loc = Loc::File(0, 3, 5);
    let mut ns = Namespace::new(Target::Solana);
    let expr = pt::YulExpression::NumberLiteral(
        loc,
        BigInt::from_u128(0xffffffffffffffffff).unwrap(),
        Some(Identifier {
            loc,
            name: "u64".to_string(),
        }),
    );
    let parsed = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(parsed.is_ok());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided literal requires 72 bits, but the type only supports 64"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::NumberLiteral(
        loc,
        BigInt::from_i32(-50).unwrap(),
        Some(Identifier {
            loc,
            name: "u128".to_string(),
        }),
    );
    let parsed = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(parsed.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "signed integer cannot fit in unsigned integer"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::NumberLiteral(loc, BigInt::from(20), None);
    let parsed = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(parsed.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        parsed.unwrap(),
        YulExpression::NumberLiteral(loc, BigInt::from(20), Type::Uint(256))
    );
}

#[test]
fn resolve_hex_number_literal() {
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);

    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::YulExpression::HexNumberLiteral(
        loc,
        "0xf23456789a".to_string(),
        Some(Identifier {
            loc,
            name: "u32".to_string(),
        }),
    );

    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_ok());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided literal requires 40 bits, but the type only supports 32"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::HexNumberLiteral(
        loc,
        "0xff".to_string(),
        Some(Identifier {
            loc,
            name: "s64".to_string(),
        }),
    );
    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        YulExpression::NumberLiteral(loc, BigInt::from(255), Type::Int(64))
    );
}

#[test]
fn resolve_hex_string_literal() {
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);

    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::YulExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "3ca".to_string(),
        },
        None,
    );

    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "hex string \"3ca\" has odd number of characters"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "acdf".to_string(),
        },
        Some(Identifier {
            loc,
            name: "myType".to_string(),
        }),
    );
    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the specified type 'myType' does not exist"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "ffff".to_string(),
        },
        Some(Identifier {
            loc,
            name: "u256".to_string(),
        }),
    );
    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        YulExpression::StringLiteral(loc, vec![255, 255], Type::Uint(256))
    );
}

#[test]
fn resolve_string_literal() {
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);

    let mut ns = Namespace::new(Target::Solana);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::YulExpression::StringLiteral(
        StringLiteral {
            loc,
            string: r#"ab\xffa\u00e0g"#.to_string(),
        },
        Some(Identifier {
            loc,
            name: "u128".to_string(),
        }),
    );

    let resolved = resolve_yul_expression(&expr, &ctx, &mut symtable, &mut function_table, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        YulExpression::StringLiteral(loc, vec![97, 98, 255, 97, 0xc3, 0xa0, 103], Type::Uint(128))
    );
}

#[test]
fn resolve_variable_local() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(1, 2, 3);

    let pos1 = symtable
        .add(
            &Identifier {
                loc,
                name: "var1".to_string(),
            },
            Type::Uint(32),
            &mut ns,
            VariableInitializer::Yul(false),
            VariableUsage::YulLocalVariable,
            None,
        )
        .unwrap();
    let pos2 = symtable
        .add(
            &Identifier {
                loc,
                name: "var2".to_string(),
            },
            Type::Uint(32),
            &mut ns,
            VariableInitializer::Yul(false),
            VariableUsage::LocalVariable,
            None,
        )
        .unwrap();

    let expr1 = pt::YulExpression::Variable(Identifier {
        loc,
        name: "var1".to_string(),
    });
    let expr2 = pt::YulExpression::Variable(Identifier {
        loc,
        name: "var2".to_string(),
    });

    let expected_1 = YulExpression::YulLocalVariable(loc, Type::Uint(32), pos1);
    let expected_2 = YulExpression::SolidityLocalVariable(loc, Type::Uint(32), None, pos2);

    let res1 = resolve_yul_expression(
        &expr1,
        &context,
        &mut symtable,
        &mut function_table,
        &mut ns,
    );
    let res2 = resolve_yul_expression(
        &expr2,
        &context,
        &mut symtable,
        &mut function_table,
        &mut ns,
    );

    assert!(res1.is_ok());
    assert!(res2.is_ok());
    assert!(ns.diagnostics.is_empty());

    assert_eq!(expected_1, res1.unwrap());
    assert_eq!(expected_2, res2.unwrap());
}

#[test]
fn resolve_variable_contract() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);
    let mut contract = ast::Contract::new("test", ContractTy::Contract(loc), vec![], loc);
    contract.variables.push(Variable {
        tags: vec![],
        name: "var1".to_string(),
        loc,
        ty: Type::Bool,
        visibility: Visibility::Public(None),
        constant: true,
        immutable: false,
        initializer: None,
        assigned: false,
        read: false,
    });
    contract.variables.push(Variable {
        tags: vec![],
        name: "var2".to_string(),
        loc,
        ty: Type::Int(128),
        visibility: Visibility::Public(None),
        constant: false,
        immutable: false,
        initializer: None,
        assigned: false,
        read: false,
    });

    contract.variables.push(Variable {
        tags: vec![],
        name: "imut".to_string(),
        loc,
        ty: Type::Int(128),
        visibility: Visibility::Public(None),
        constant: false,
        immutable: true,
        initializer: None,
        assigned: false,
        read: false,
    });

    ns.contracts.push(contract);

    ns.constants.push(Variable {
        tags: vec![],
        name: "var3".to_string(),
        loc,
        ty: Type::Uint(32),
        visibility: Visibility::Public(None),
        constant: true,
        immutable: false,
        initializer: None,
        assigned: false,
        read: false,
    });

    ns.variable_symbols.insert(
        (0, Some(0), "var1".to_string()),
        Symbol::Variable(loc, Some(0), 0),
    );
    ns.variable_symbols.insert(
        (0, Some(0), "var2".to_string()),
        Symbol::Variable(loc, Some(0), 1),
    );
    ns.variable_symbols.insert(
        (0, Some(0), "imut".to_string()),
        Symbol::Variable(loc, Some(0), 2),
    );
    ns.variable_symbols.insert(
        (0, None, "var3".to_string()),
        Symbol::Variable(loc, None, 0),
    );
    ns.variable_symbols
        .insert((0, Some(0), "func".to_string()), Symbol::Function(vec![]));

    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "var1".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        YulExpression::ConstantVariable(loc, Type::Bool, Some(0), 0),
        res.unwrap()
    );

    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "var2".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        YulExpression::StorageVariable(loc, Type::Int(128), 0, 1),
        res.unwrap()
    );

    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "var3".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        YulExpression::ConstantVariable(loc, Type::Uint(32), None, 0),
        res.unwrap()
    );

    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "func".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "only variables can be accessed inside assembly blocks"
    );

    ns.diagnostics.clear();
    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "none".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(ns.diagnostics[0].message, "'none' is not found");

    ns.diagnostics.clear();
    let expr = pt::YulExpression::Variable(Identifier {
        loc,
        name: "imut".to_string(),
    });
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "assembly access to immutable variables is not supported"
    );
}

#[test]
fn function_call() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);
    function_table.new_scope();
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "verbatim_1i_2o".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "verbatim functions are not yet supported in Solang"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "linkersymbol".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the internal EVM built-in 'linkersymbol' is not yet supported"
    );
    ns.diagnostics.clear();

    let arg = pt::YulExpression::BoolLiteral(
        Loc::File(0, 3, 5),
        false,
        Some(pt::Identifier {
            loc: Loc::File(0, 3, 4),
            name: "u32".to_string(),
        }),
    );

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "add".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "builtin function 'add' requires 2 arguments, but 1 were provided"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        YulExpression::BuiltInCall(
            loc,
            YulBuiltInFunction::Not,
            vec![resolve_yul_expression(
                &arg,
                &context,
                &mut symtable,
                &mut function_table,
                &mut ns
            )
            .unwrap()]
        ),
        res.unwrap()
    );

    function_table.add_function_header(
        &Identifier {
            loc,
            name: "myFunc".to_string(),
        },
        vec![],
        vec![],
    );

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "myFunc".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'myFunc' requires 0 arguments, but 1 were provided"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "myFunc".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert_eq!(YulExpression::FunctionCall(loc, 0, vec![]), res.unwrap());

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "none".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(ns.diagnostics[0].message, "function 'none' is not defined");
}

#[test]
fn check_arguments() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);
    function_table.new_scope();
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);

    function_table.add_function_header(
        &Identifier {
            loc,
            name: "func1".to_string(),
        },
        vec![],
        vec![],
    );

    function_table.add_function_header(
        &Identifier {
            loc,
            name: "func2".to_string(),
        },
        vec![],
        vec![
            Parameter {
                loc,
                id: Some(Identifier {
                    loc,
                    name: "ret1".to_string(),
                }),
                ty: Type::Uint(256),
                ty_loc: None,
                indexed: false,
                readonly: false,
            },
            Parameter {
                loc,
                id: Some(Identifier {
                    loc,
                    name: "ret2".to_string(),
                }),
                ty: Type::Uint(256),
                ty_loc: None,
                indexed: false,
                readonly: false,
            },
        ],
    );

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
            loc,
            id: Identifier {
                loc,
                name: "pop".to_string(),
            },
            arguments: vec![pt::YulExpression::NumberLiteral(
                loc,
                BigInt::from(23),
                None,
            )],
        }))],
    }));

    let _ = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "builtin function 'pop' returns nothing"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
            loc,
            id: Identifier {
                loc,
                name: "func1".to_string(),
            },
            arguments: vec![],
        }))],
    }));

    let _ = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'func1' returns nothing"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::YulExpression::FunctionCall(Box::new(YulFunctionCall {
            loc,
            id: Identifier {
                loc,
                name: "func2".to_string(),
            },
            arguments: vec![],
        }))],
    }));

    let _ = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'func2' has multiple returns and cannot be used in this scope"
    );
}

#[test]
fn test_member_access() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };
    let mut symtable = Symtable::new();
    let mut function_table = FunctionsTable::new(0);
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);

    let mut contract = ast::Contract::new("test", ContractTy::Contract(loc), vec![], loc);
    contract.variables.push(Variable {
        tags: vec![],
        name: "var1".to_string(),
        loc,
        ty: Type::Bool,
        visibility: Visibility::Public(None),
        constant: false,
        immutable: false,
        initializer: None,
        assigned: false,
        read: false,
    });

    ns.contracts.push(contract);

    ns.variable_symbols.insert(
        (0, Some(0), "var1".to_string()),
        Symbol::Variable(loc, Some(0), 0),
    );

    let expr = pt::YulExpression::Member(
        loc,
        Box::new(pt::YulExpression::BoolLiteral(loc, true, None)),
        Identifier {
            loc,
            name: "pineapple".to_string(),
        },
    );

    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided suffix is not allowed in yul"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::Member(
        loc,
        Box::new(pt::YulExpression::BoolLiteral(loc, true, None)),
        Identifier {
            loc,
            name: "slot".to_string(),
        },
    );

    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the given expression does not support ‘.slot‘ suffixes"
    );
    ns.diagnostics.clear();

    let expr = pt::YulExpression::Member(
        loc,
        Box::new(pt::YulExpression::Variable(Identifier {
            loc,
            name: "var1".to_string(),
        })),
        Identifier {
            loc,
            name: "slot".to_string(),
        },
    );

    let res = resolve_yul_expression(&expr, &context, &mut symtable, &mut function_table, &mut ns);
    assert!(res.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        YulExpression::MemberAccess(
            loc,
            Box::new(YulExpression::StorageVariable(loc, Type::Bool, 0, 0)),
            YulSuffix::Slot
        ),
        res.unwrap()
    );
}

#[test]
fn test_check_types() {
    let loc = Loc::File(0, 0, 0);
    let expr = YulExpression::SolidityLocalVariable(
        loc,
        Type::Uint(32),
        Some(StorageLocation::Storage(loc)),
        0,
    );

    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
        yul_function: false,
    };

    let mut ns = Namespace::new(Target::Ewasm);
    let mut contract = ast::Contract::new("test", ContractTy::Contract(loc), vec![], loc);
    contract.variables.push(Variable {
        tags: vec![],
        name: "var1".to_string(),
        loc,
        ty: Type::Bool,
        visibility: Visibility::Public(None),
        constant: true,
        immutable: false,
        initializer: None,
        assigned: false,
        read: false,
    });
    ns.contracts.push(contract);
    let mut symtable = Symtable::new();
    symtable.add(
        &Identifier {
            loc,
            name: "name".to_string(),
        },
        Type::Uint(32),
        &mut ns,
        VariableInitializer::Solidity(None),
        VariableUsage::YulLocalVariable,
        None,
    );
    let res = check_type(&expr, &context, &mut ns, &mut symtable);
    assert!(res.is_some());
    assert_eq!(
        res.unwrap().message,
        "Storage variables must be accessed with ‘.slot‘ or ‘.offset‘"
    );

    let expr = YulExpression::StorageVariable(loc, Type::Int(16), 0, 0);
    let res = check_type(&expr, &context, &mut ns, &mut symtable);
    assert!(res.is_some());
    assert_eq!(
        res.unwrap().message,
        "Storage variables must be accessed with ‘.slot‘ or ‘.offset‘"
    );

    let expr = YulExpression::SolidityLocalVariable(
        loc,
        Type::Array(Box::new(Type::Int(8)), vec![None]),
        Some(StorageLocation::Calldata(loc)),
        0,
    );
    let res = check_type(&expr, &context, &mut ns, &mut symtable);
    assert!(res.is_some());
    assert_eq!(res.unwrap().message, "Calldata arrays must be accessed with ‘.offset‘, ‘.length‘ and the ‘calldatacopy‘ function");

    let expr = YulExpression::StringLiteral(loc, vec![0, 255, 20], Type::Uint(256));
    let res = check_type(&expr, &context, &mut ns, &mut symtable);
    assert!(res.is_none());
}

#[test]
fn test_check_types_resolved() {
    let file = r#"
contract testTypes {
    uint256 b;
    function testAsm() public pure {
        assembly {
            {
                let x := 0
                b.offset := add(x, 2)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "cannot assign a value to offset"
    ));

    let file = r#"
    contract testTypes {
    uint256 b;
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let x := 0
                vl.length := add(x, 2)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "cannot assign a value to length"
    ));

    let file = r#"
contract testTypes {
    uint256 b;
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let x := 0
                pop(x) := add(x, 2)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        r#"unrecognised token `:=', expected ")", ",", "address", "bool", "break", "byte", "case", "continue", "default", "for", "function", "if", "leave", "let", "return", "revert", "switch", "{", "}", identifier"#
    ));

    let file = r#"
uint256 constant b = 1;
contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let x := 0
                b := add(x, 2)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "cannot assigned a value to a constant"
    ));

    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }

    test tt1;
    function testAsm(uint[] calldata vl) public view {
        test storage tt2 = tt1;
        assembly {
            {
                let x := 0
                tt1.slot := add(x, 2)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "cannot assign to slot of storage variable"
    ));

    let file = r#"
    contract testTypes {
    struct test {
        uint a;
        uint b;
    }

    test tt1;
    function testAsm(uint[] calldata vl) public view {
        test storage tt2 = tt1;
        assembly {
            {
                let x := 0
                tt2.slot := add(x, 2)
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "inline assembly is not yet supported"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "function parameter ‘vl‘ has never been read"
    ));

    let file = r#"
    contract testTypes {
    struct test {
        uint a;
        uint b;
    }

    test tt1;
    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        assembly {
            {
                let x := 0
                tt2 := add(x, 2)
            }
        }
    }
}   "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "storage variables cannot be assigned any value in assembly. You may use ‘sstore()‘"
    ));

    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }

    test tt1;
    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        assembly {
            {
                let x := 0
                let y := add(tt2, 2)
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "Storage variables must be accessed with ‘.slot‘ or ‘.offset‘"
    ));

    let file = r#"
contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let x := 4
                let y := add(vl, x)
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "Calldata arrays must be accessed with ‘.offset‘, ‘.length‘ and the ‘calldatacopy‘ function"
    ));
}

#[test]
fn test_member_access_resolved() {
    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }
    test tt1;
    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        assembly {
            {
                let x := tt1.slot.length
               // let y := tt2.length
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "there cannot be multiple suffixes to a name"
    ));

    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }
    test tt1;
    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        assembly {
            {
                //let x := tt1.slot.length
               let y := tt2.length
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        r#"state variables only support ".slot" and ".offset""#
    ));

    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }
    test tt1;
    function bar(uint a) public pure returns (uint) {
        return a + 3;
    }

    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        function(uint) internal pure returns (uint) func = bar;
        assembly {
            {
               let y := func.length
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "only variables of type external function pointer support suffixes"
    ));

    let file = r#"
contract testTypes {
    struct test {
        uint a;
        uint b;
    }
    test tt1;
    function bar(uint a) public pure returns (uint) {
        return a + 3;
    }

    function testAsm(uint[] calldata vl) public pure {
        test storage tt2 = tt1;
        function(uint) external pure returns (uint) func = this.bar;
        assembly {
            {
               let y := func.length
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        r#"variables of type function pointer only support ".selector" and ".address" suffixes"#
    ));

    let file = r#"
contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
               let y := vl.selector
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        r#"calldata variables only support ".offset" and ".length""#
    ));

    let file = r#"
contract testTypes {
    uint constant cte = 5;
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
               let y := cte.offset
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the suffixes .offset and .slot can only be used in non-constant storage variables"
    ));

    let file = r#"
contract testTypes {
    uint constant cte = 5;
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
               let y := cte.dada
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the provided suffix is not allowed in yul"
    ));
}

#[test]
fn test_check_argument() {
    let file = r#"
contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let a1 : u32 := 5
                let b1 : s32 := 6
                let c1 : u256 := 7
                let d1 : s256 := 8
                let f1 : u256 := 9

                let ret1 := testPars(a1, b1, c1, d1, f1)
                function testPars(a : s32, b:u32, c:u128, d:s128, f : bool) -> ret{
                    ret := add(a, b)
                    ret := add(c, ret)
                    ret := add(d, ret)
                    if f {
                        ret := 0
                    }
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "32 bit unsigned integer may not fit into 32 bit signed integer"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "signed integer may not be correctly represented as unsigned integer"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "256 bit type may not fit into 128 bit type"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "256 bit type may not fit into 128 bit type"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "Truncating argument to bool"
    ));
}

#[test]
fn address_member_access() {
    let file = r#"
contract C {
    // Assigns a new selector and address to the return variable @fun
    function combineToFunctionPointer(address newAddress, uint newSelector) public pure returns (function() external fun) {
        assembly {
            fun.selector := newSelector
            fun.address  := newAddress
        }
    }
}
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "inline assembly is not yet supported"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘C’"
    ));
}

#[test]
fn test_invalid_suffix() {
    let file = r#"
contract test {
    function testing() public pure returns (int) {
        int[4] memory vec = [int(1), 2, 3, 4];
        assembly {
            let x := vec.slot
            if lt(x, 0) {
                stop()
            }
        }

        return vec[1];
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘test’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the given expression does not support ‘.slot‘ suffixes"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘x‘ has never been read"
    ));

    let file = r#"
    contract test {
    struct tts {
        int a;
        string b;
    }
    function testing(tts calldata strcl) public pure returns (int) {
        assembly {
            let x := strcl.slot
        }

        return strcl.a;
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘test’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the given expression does not support ‘.slot‘ suffixes"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘x‘ has never been read"
    ));

    let file = r#"
    contract test {

    function testing() public pure returns (int) {
        int c = 8;
        assembly {
            let x := c.offset
        }

        return c;
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘test’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the given expression does not support ‘.offset‘ suffixes"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘x‘ has never been read"
    ));
}
