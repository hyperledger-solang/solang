#![cfg(test)]

use crate::ast::{Namespace, Symbol, Type, Variable};
use crate::sema::assembly::builtin::AssemblyBuiltInFunction;
use crate::sema::assembly::expression::{
    check_type, resolve_assembly_expression, AssemblyExpression, AssemblySuffix,
};
use crate::sema::assembly::functions::{AssemblyFunction, AssemblyFunctionParameter};
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{Symtable, VariableUsage};
use crate::{ast, Target};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use solang_parser::pt;
use solang_parser::pt::{
    AssemblyFunctionCall, ContractTy, HexLiteral, Identifier, Loc, StorageLocation, StringLiteral,
    Visibility,
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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();

    let mut ns = Namespace::new(Target::Solana);
    let expr = pt::AssemblyExpression::BoolLiteral(
        Loc::File(0, 3, 5),
        false,
        Some(pt::Identifier {
            loc: Loc::File(0, 3, 4),
            name: "u32".to_string(),
        }),
    );

    let resolved_type = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
    assert!(resolved_type.is_ok());
    assert!(ns.diagnostics.is_empty());
    let unwrapped = resolved_type.unwrap();

    assert_eq!(
        unwrapped,
        AssemblyExpression::BoolLiteral(Loc::File(0, 3, 5), false, Type::Uint(32))
    );

    let expr = pt::AssemblyExpression::BoolLiteral(Loc::File(0, 3, 5), true, None);
    let resolved_type = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);

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
    let ctx = ExprContext {
        file_no: 0,
        contract_no: None,
        function_no: None,
        unchecked: false,
        constant: false,
        lvalue: false,
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();

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
    let parsed = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
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
    let parsed = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
    assert!(parsed.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "singed value cannot fit in unsigned type"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::NumberLiteral(loc, BigInt::from(20), None);
    let parsed = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
    assert!(parsed.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        parsed.unwrap(),
        AssemblyExpression::NumberLiteral(loc, BigInt::from(20), Type::Uint(256))
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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();

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

    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
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
    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        AssemblyExpression::NumberLiteral(loc, BigInt::from(255), Type::Int(64))
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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();

    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 3, 5);
    let expr = pt::AssemblyExpression::HexStringLiteral(
        HexLiteral {
            loc,
            hex: "3ca".to_string(),
        },
        None,
    );

    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
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
    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
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
    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
    assert!(resolved.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        resolved.unwrap(),
        AssemblyExpression::StringLiteral(loc, vec![255, 255], Type::Uint(256))
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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();

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

    let resolved = resolve_assembly_expression(&expr, &ctx, &symtable, &functions, &mut ns);
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

#[test]
fn resolve_variable_local() {
    let context = ExprContext {
        file_no: 0,
        contract_no: Some(0),
        function_no: Some(0),
        unchecked: false,
        constant: false,
        lvalue: false,
    };
    let mut symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();
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
            None,
            VariableUsage::AssemblyLocalVariable,
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
            None,
            VariableUsage::LocalVariable,
            None,
        )
        .unwrap();

    let expr1 = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "var1".to_string(),
    });
    let expr2 = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "var2".to_string(),
    });

    let expected_1 = AssemblyExpression::AssemblyLocalVariable(loc, Type::Uint(32), pos1);
    let expected_2 = AssemblyExpression::SolidityLocalVariable(loc, Type::Uint(32), None, pos2);

    let res1 = resolve_assembly_expression(&expr1, &context, &symtable, &functions, &mut ns);
    let res2 = resolve_assembly_expression(&expr2, &context, &symtable, &functions, &mut ns);

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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();
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
        (0, None, "var3".to_string()),
        Symbol::Variable(loc, None, 0),
    );
    ns.variable_symbols
        .insert((0, Some(0), "func".to_string()), Symbol::Function(vec![]));

    let expr = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "var1".to_string(),
    });
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        AssemblyExpression::ConstantVariable(loc, Type::Bool, Some(0), 0),
        res.unwrap()
    );

    let expr = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "var2".to_string(),
    });
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        AssemblyExpression::StorageVariable(loc, Type::Int(128), 0, 1),
        res.unwrap()
    );

    let expr = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "var3".to_string(),
    });
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        AssemblyExpression::ConstantVariable(loc, Type::Uint(32), None, 0),
        res.unwrap()
    );

    let expr = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "func".to_string(),
    });
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "only variables can be accessed inside assembly blocks"
    );

    ns.diagnostics.clear();
    let expr = pt::AssemblyExpression::Variable(Identifier {
        loc,
        name: "none".to_string(),
    });
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(ns.diagnostics[0].message, "'none' is not found");
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
    };
    let symtable = Symtable::new();
    let mut functions: IndexMap<String, AssemblyFunction> = IndexMap::new();
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "verbatim_1i_2o".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "verbatim functions are not yet supported in Solang"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "linkersymbol".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the internal EVM built-in 'linkersymbol' is not yet supported"
    );
    ns.diagnostics.clear();

    let arg = pt::AssemblyExpression::BoolLiteral(
        Loc::File(0, 3, 5),
        false,
        Some(pt::Identifier {
            loc: Loc::File(0, 3, 4),
            name: "u32".to_string(),
        }),
    );

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "add".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "builtin function 'add' requires 2 arguments, but 1 were provided"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        AssemblyExpression::BuiltInCall(
            loc,
            AssemblyBuiltInFunction::Not,
            vec![
                resolve_assembly_expression(&arg, &context, &symtable, &functions, &mut ns)
                    .unwrap()
            ]
        ),
        res.unwrap()
    );

    functions.insert(
        "myFunc".to_string(),
        AssemblyFunction {
            loc,
            name: "myFunc".to_string(),
            params: vec![],
            returns: vec![],
            body: vec![],
            functions: IndexMap::new(),
            symtable: Symtable::new(),
        },
    );

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "myFunc".to_string(),
        },
        arguments: vec![arg.clone()],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'myFunc' requires 0 arguments, but 1 were provided"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "myFunc".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert_eq!(
        AssemblyExpression::FunctionCall(loc, "myFunc".to_string(), vec![]),
        res.unwrap()
    );

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "none".to_string(),
        },
        arguments: vec![],
    }));
    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
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
    };
    let symtable = Symtable::new();
    let mut functions: IndexMap<String, AssemblyFunction> = IndexMap::new();
    let mut ns = Namespace::new(Target::Ewasm);
    let loc = Loc::File(0, 2, 3);

    functions.insert(
        "func1".to_string(),
        AssemblyFunction {
            loc,
            name: "func1".to_string(),
            params: vec![],
            returns: vec![],
            body: vec![],
            functions: IndexMap::new(),
            symtable: Symtable::new(),
        },
    );

    functions.insert(
        "func2".to_string(),
        AssemblyFunction {
            loc,
            name: "func2".to_string(),
            params: vec![],
            returns: vec![
                AssemblyFunctionParameter {
                    loc,
                    name: Identifier {
                        loc,
                        name: "ret1".to_string(),
                    },
                    ty: Type::Uint(256),
                },
                AssemblyFunctionParameter {
                    loc,
                    name: Identifier {
                        loc,
                        name: "ret2".to_string(),
                    },
                    ty: Type::Uint(256),
                },
            ],
            body: vec![],
            functions: IndexMap::new(),
            symtable: Symtable::new(),
        },
    );

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::AssemblyExpression::FunctionCall(Box::new(
            AssemblyFunctionCall {
                loc,
                id: Identifier {
                    loc,
                    name: "pop".to_string(),
                },
                arguments: vec![pt::AssemblyExpression::NumberLiteral(
                    loc,
                    BigInt::from(23),
                    None,
                )],
            },
        ))],
    }));

    let _ = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "builtin function 'pop' returns nothing"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::AssemblyExpression::FunctionCall(Box::new(
            AssemblyFunctionCall {
                loc,
                id: Identifier {
                    loc,
                    name: "func1".to_string(),
                },
                arguments: vec![],
            },
        ))],
    }));

    let _ = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'func1' returns nothing"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
        loc,
        id: Identifier {
            loc,
            name: "not".to_string(),
        },
        arguments: vec![pt::AssemblyExpression::FunctionCall(Box::new(
            AssemblyFunctionCall {
                loc,
                id: Identifier {
                    loc,
                    name: "func2".to_string(),
                },
                arguments: vec![],
            },
        ))],
    }));

    let _ = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(!ns.diagnostics.is_empty());
    assert_eq!(
        ns.diagnostics[0].message,
        "function 'func2' has multiple returns and cannot be used as argument"
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
    };
    let symtable = Symtable::new();
    let functions: IndexMap<String, AssemblyFunction> = IndexMap::new();
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

    let expr = pt::AssemblyExpression::Member(
        loc,
        Box::new(pt::AssemblyExpression::BoolLiteral(loc, true, None)),
        Identifier {
            loc,
            name: "pineapple".to_string(),
        },
    );

    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the provided suffix is not allowed in yul"
    );
    ns.diagnostics.clear();

    let expr = pt::AssemblyExpression::Member(
        loc,
        Box::new(pt::AssemblyExpression::BoolLiteral(loc, true, None)),
        Identifier {
            loc,
            name: "slot".to_string(),
        },
    );

    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_err());
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics[0].message,
        "the given expression does not support suffixes"
    );
    ns.diagnostics.clear();

    // TODO: tests with member access are difficult due to nested expressions.
    // Such tests must be done when Yul sema is complete.

    let expr = pt::AssemblyExpression::Member(
        loc,
        Box::new(pt::AssemblyExpression::Variable(Identifier {
            loc,
            name: "var1".to_string(),
        })),
        Identifier {
            loc,
            name: "slot".to_string(),
        },
    );

    let res = resolve_assembly_expression(&expr, &context, &symtable, &functions, &mut ns);
    assert!(res.is_ok());
    assert!(ns.diagnostics.is_empty());
    assert_eq!(
        AssemblyExpression::MemberAccess(
            loc,
            Box::new(AssemblyExpression::StorageVariable(loc, Type::Bool, 0, 0)),
            AssemblySuffix::Slot
        ),
        res.unwrap()
    );
}

#[test]
fn test_check_types() {
    let loc = Loc::File(0, 0, 0);
    let expr = AssemblyExpression::SolidityLocalVariable(
        loc,
        Type::Uint(32),
        Some(StorageLocation::Storage(loc)),
        0,
    );

    let res = check_type(&expr);
    assert!(res.is_some());
    assert_eq!(
        res.unwrap().message,
        "Storage variables must be accessed with \".slot\" or \".offset\""
    );

    let expr = AssemblyExpression::StorageVariable(loc, Type::Int(16), 0, 1);
    let res = check_type(&expr);
    assert!(res.is_some());
    assert_eq!(
        res.unwrap().message,
        "Storage variables must be accessed with \".slot\" or \".offset\""
    );

    let expr = AssemblyExpression::SolidityLocalVariable(
        loc,
        Type::Array(Box::new(Type::Int(8)), vec![None]),
        Some(StorageLocation::Calldata(loc)),
        2,
    );
    let res = check_type(&expr);
    assert!(res.is_some());
    assert_eq!(res.unwrap().message, "Calldata arrays must be accessed with \".offset\", \".length\" and the \"calldatacopy\" function");

    let expr = AssemblyExpression::StringLiteral(loc, vec![0, 255, 20], Type::Uint(256));
    let res = check_type(&expr);
    assert!(res.is_none());
}
