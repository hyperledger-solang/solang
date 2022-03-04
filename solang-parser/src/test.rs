use crate::lexer;
use crate::pt::*;
use crate::solidity;
use num_bigint::BigInt;

#[test]
fn parse_test() {
    let src = r#"/// @title Foo
                /// @description Foo
                /// Bar
                contract foo {
                    /**
                    @title Jurisdiction
                    */
                    /// @author Anon
                    /**
                    @description Data for
                    jurisdiction
                    @dev It's a struct
                    */
                    struct Jurisdiction {
                        bool exists;
                        uint keyIdx;
                        bytes2 country;
                        bytes32 region;
                    }
                    string __abba_$;
                    int64 $thing_102;
                }

                function bar() {
                    try sum(1, 1) returns (uint sum) {
                        assert(sum == 2);
                    } catch (bytes memory b) {
                        revert('meh');
                    } catch Error(string memory error) {
                        revert(error);
                    } catch Panic(uint x) {
                        revert('feh');
                    }
                }"#;

    let mut comments = Vec::new();
    let lex = lexer::Lexer::new(src, 0, &mut comments);

    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            doc: vec![
                DocComment::Line {
                    comment: SingleDocComment {
                        offset: 0,
                        tag: "title".to_string(),
                        value: "Foo".to_string(),
                    },
                },
                DocComment::Line {
                    comment: SingleDocComment {
                        offset: 0,
                        tag: "description".to_string(),
                        value: "Foo\nBar".to_string(),
                    },
                },
            ],
            loc: Loc::File(0, 92, 105),
            ty: ContractTy::Contract(Loc::File(0, 92, 100)),
            name: Identifier {
                loc: Loc::File(0, 101, 104),
                name: "foo".to_string(),
            },
            base: Vec::new(),
            parts: vec![
                ContractPart::StructDefinition(Box::new(StructDefinition {
                    doc: vec![
                        DocComment::Block {
                            comments: vec![SingleDocComment {
                                offset: 0,
                                tag: "title".to_string(),
                                value: "Jurisdiction".to_string(),
                            }],
                        },
                        DocComment::Line {
                            comment: SingleDocComment {
                                offset: 0,
                                tag: "author".to_string(),
                                value: "Anon".to_string(),
                            },
                        },
                        DocComment::Block {
                            comments: vec![
                                SingleDocComment {
                                    offset: 0,
                                    tag: "description".to_string(),
                                    value: "Data for\njurisdiction".to_string(),
                                },
                                SingleDocComment {
                                    offset: 0,
                                    tag: "dev".to_string(),
                                    value: "It's a struct".to_string(),
                                },
                            ],
                        },
                    ],
                    name: Identifier {
                        loc: Loc::File(0, 419, 431),
                        name: "Jurisdiction".to_string(),
                    },
                    loc: Loc::File(0, 412, 609),
                    fields: vec![
                        VariableDeclaration {
                            loc: Loc::File(0, 458, 469),
                            ty: Expression::Type(Loc::File(0, 458, 462), Type::Bool),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 463, 469),
                                name: "exists".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 495, 506),
                            ty: Expression::Type(Loc::File(0, 495, 499), Type::Uint(256)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 500, 506),
                                name: "keyIdx".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 532, 546),
                            ty: Expression::Type(Loc::File(0, 532, 538), Type::Bytes(2)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 539, 546),
                                name: "country".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 572, 586),
                            ty: Expression::Type(Loc::File(0, 572, 579), Type::Bytes(32)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 580, 586),
                                name: "region".to_string(),
                            },
                        },
                    ],
                })),
                ContractPart::VariableDefinition(Box::new(VariableDefinition {
                    doc: vec![],
                    ty: Expression::Type(Loc::File(0, 630, 636), Type::String),
                    attrs: vec![],
                    name: Identifier {
                        loc: Loc::File(0, 637, 645),
                        name: "__abba_$".to_string(),
                    },
                    loc: Loc::File(0, 630, 645),
                    initializer: None,
                })),
                ContractPart::VariableDefinition(Box::new(VariableDefinition {
                    doc: vec![],
                    ty: Expression::Type(Loc::File(0, 667, 672), Type::Int(64)),
                    attrs: vec![],
                    name: Identifier {
                        loc: Loc::File(0, 673, 683),
                        name: "$thing_102".to_string(),
                    },
                    loc: Loc::File(0, 667, 683),
                    initializer: None,
                })),
            ],
        })),
        SourceUnitPart::FunctionDefinition(Box::new(FunctionDefinition {
            doc: vec![],
            loc: Loc::File(0, 720, 735),
            ty: FunctionTy::Function,
            name: Some(Identifier {
                loc: Loc::File(0, 729, 732),
                name: "bar".to_string(),
            }),
            name_loc: Loc::File(0, 729, 732),
            params: vec![],
            attributes: vec![],
            return_not_returns: None,
            returns: vec![],
            body: Some(Statement::Block {
                loc: Loc::File(0, 735, 1138),
                unchecked: false,
                statements: vec![Statement::Try(
                    Loc::File(0, 757, 1120),
                    Expression::FunctionCall(
                        Loc::File(0, 761, 770),
                        Box::new(Expression::Variable(Identifier {
                            loc: Loc::File(0, 761, 764),
                            name: "sum".to_string(),
                        })),
                        vec![
                            Expression::NumberLiteral(Loc::File(0, 765, 766), 1.into()),
                            Expression::NumberLiteral(Loc::File(0, 768, 769), 1.into()),
                        ],
                    ),
                    Some((
                        vec![(
                            Loc::File(0, 780, 788),
                            Some(Parameter {
                                loc: Loc::File(0, 780, 788),
                                ty: Expression::Type(Loc::File(0, 780, 784), Type::Uint(256)),
                                storage: None,
                                name: Some(Identifier {
                                    loc: Loc::File(0, 785, 788),
                                    name: "sum".to_string(),
                                }),
                            }),
                        )],
                        Box::new(Statement::Block {
                            loc: Loc::File(0, 790, 855),
                            unchecked: false,
                            statements: vec![Statement::Expression(
                                Loc::File(0, 816, 832),
                                Expression::FunctionCall(
                                    Loc::File(0, 816, 832),
                                    Box::new(Expression::Variable(Identifier {
                                        loc: Loc::File(0, 816, 822),
                                        name: "assert".to_string(),
                                    })),
                                    vec![Expression::Equal(
                                        Loc::File(0, 827, 829),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc::File(0, 823, 826),
                                            name: "sum".to_string(),
                                        })),
                                        Box::new(Expression::NumberLiteral(
                                            Loc::File(0, 830, 831),
                                            2.into(),
                                        )),
                                    )],
                                ),
                            )],
                        }),
                    )),
                    vec![
                        CatchClause::Simple(
                            Loc::File(0, 856, 941),
                            Some(Parameter {
                                loc: Loc::File(0, 863, 877),
                                ty: Expression::Type(Loc::File(0, 863, 868), Type::DynamicBytes),
                                storage: Some(StorageLocation::Memory(Loc::File(0, 869, 875))),
                                name: Some(Identifier {
                                    loc: Loc::File(0, 876, 877),
                                    name: "b".to_string(),
                                }),
                            }),
                            Statement::Block {
                                loc: Loc::File(0, 879, 941),
                                unchecked: false,
                                statements: vec![Statement::Expression(
                                    Loc::File(0, 905, 918),
                                    Expression::FunctionCall(
                                        Loc::File(0, 905, 918),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc::File(0, 905, 911),
                                            name: "revert".to_string(),
                                        })),
                                        vec![Expression::StringLiteral(vec![StringLiteral {
                                            loc: Loc::File(0, 912, 917),
                                            string: "meh".to_string(),
                                        }])],
                                    ),
                                )],
                            },
                        ),
                        CatchClause::Named(
                            Loc::File(0, 942, 1037),
                            Identifier {
                                loc: Loc::File(0, 948, 953),
                                name: "Error".to_string(),
                            },
                            Parameter {
                                loc: Loc::File(0, 954, 973),
                                ty: Expression::Type(Loc::File(0, 954, 960), Type::String),
                                storage: Some(StorageLocation::Memory(Loc::File(0, 961, 967))),
                                name: Some(Identifier {
                                    loc: Loc::File(0, 968, 973),
                                    name: "error".to_string(),
                                }),
                            },
                            Statement::Block {
                                loc: Loc::File(0, 975, 1037),
                                unchecked: false,
                                statements: vec![Statement::Expression(
                                    Loc::File(0, 1001, 1014),
                                    Expression::FunctionCall(
                                        Loc::File(0, 1001, 1014),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc::File(0, 1001, 1007),
                                            name: "revert".to_string(),
                                        })),
                                        vec![Expression::Variable(Identifier {
                                            loc: Loc::File(0, 1008, 1013),
                                            name: "error".to_string(),
                                        })],
                                    ),
                                )],
                            },
                        ),
                        CatchClause::Named(
                            Loc::File(0, 1038, 1120),
                            Identifier {
                                loc: Loc::File(0, 1044, 1049),
                                name: "Panic".to_string(),
                            },
                            Parameter {
                                loc: Loc::File(0, 1050, 1056),
                                ty: Expression::Type(Loc::File(0, 1050, 1054), Type::Uint(256)),
                                storage: None,
                                name: Some(Identifier {
                                    loc: Loc::File(0, 1055, 1056),
                                    name: "x".to_string(),
                                }),
                            },
                            Statement::Block {
                                loc: Loc::File(0, 1058, 1120),
                                unchecked: false,
                                statements: vec![Statement::Expression(
                                    Loc::File(0, 1084, 1097),
                                    Expression::FunctionCall(
                                        Loc::File(0, 1084, 1097),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc::File(0, 1084, 1090),
                                            name: "revert".to_string(),
                                        })),
                                        vec![Expression::StringLiteral(vec![StringLiteral {
                                            loc: Loc::File(0, 1091, 1096),
                                            string: "feh".to_string(),
                                        }])],
                                    ),
                                )],
                            },
                        ),
                    ],
                )],
            }),
        })),
    ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn parse_error_test() {
    let src = r#"

        error Outer(uint256 available, uint256 required);

        contract TestToken {
            error NotPending();
            /// Insufficient balance for transfer. Needed `required` but only
            /// `available` available.
            /// @param available balance available.
            /// @param required requested amount to transfer.
            error InsufficientBalance(uint256 available, uint256 required);
        }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 2);

    let expected_parse_tree = SourceUnit
            (vec![
                SourceUnitPart::ErrorDefinition(Box::new(ErrorDefinition {
                    doc: vec![],
                    loc: Loc::File(
                        0,
                        10,
                        58,
                    ),
                    name: Identifier {
                        loc: Loc::File(
                            0,
                            16,
                            21,
                        ),
                        name: "Outer".to_string(),
                    },
                    fields: vec![
                        ErrorParameter {
                            ty: Expression::Type(
                                Loc::File(
                                    0,
                                    22,
                                    29,
                                ),
                                Type::Uint(
                                    256,
                                ),
                            ),
                            loc: Loc::File(
                                0,
                                22,
                                39,
                            ),
                            name: Some(
                                Identifier {
                                    loc: Loc::File(
                                        0,
                                        30,
                                        39,
                                    ),
                                    name: "available".to_string(),
                                },
                            ),
                        },
                        ErrorParameter {
                            ty: Expression::Type(
                                Loc::File(
                                    0,
                                    41,
                                    48,
                                ),
                                Type::Uint(
                                    256,
                                ),
                            ),
                            loc: Loc::File(
                                0,
                                41,
                                57,
                            ),
                            name: Some(
                                Identifier {
                                    loc: Loc::File(
                                        0,
                                        49,
                                        57,
                                    ),
                                    name: "required".to_string(),
                                },
                            ),
                        },
                    ],
                })),
                SourceUnitPart::ContractDefinition(Box::new(
                    ContractDefinition {
                        doc: vec![],
                        loc: Loc::File(
                            0,
                            69,
                            88,
                        ),
                        ty: ContractTy::Contract(
                            Loc::File(
                                0,
                                69,
                                77,
                            ),
                        ),
                        name: Identifier {
                            loc: Loc::File(
                                0,
                                78,
                                87,
                            ),
                            name: "TestToken".to_string(),
                        },
                        base: vec![],
                        parts: vec![
                            ContractPart::ErrorDefinition(Box::new(
                                ErrorDefinition {
                                    doc: vec![],
                                    loc: Loc::File(
                                        0,
                                        102,
                                        120,
                                    ),
                                    name: Identifier {
                                        loc: Loc::File(
                                            0,
                                            108,
                                            118,
                                        ),
                                        name: "NotPending".to_string(),
                                    },
                                    fields: vec![],
                                },
                            )),
                            ContractPart::ErrorDefinition(Box::new(
                                ErrorDefinition {
                                    doc: vec![
                                        DocComment::Line {
                                            comment: SingleDocComment {
                                                offset: 137,
                                                tag: "notice".to_string(),
                                                value: "Insufficient balance for transfer. Needed `required` but only\n`available` available.".to_string(),
                                            },
                                        },
                                        DocComment::Line {
                                            comment: SingleDocComment {
                                                offset: 0,
                                                tag: "param".to_string(),
                                                value: "available balance available.".to_string(),
                                            },
                                        },
                                        DocComment::Line {
                                            comment: SingleDocComment {
                                                offset: 0,
                                                tag: "param".to_string(),
                                                value: "required requested amount to transfer.".to_string(),
                                            },
                                        },
                                    ],
                                    loc: Loc::File(
                                        0,
                                        365,
                                        427,
                                    ),
                                    name: Identifier {
                                        loc: Loc::File(
                                            0,
                                            371,
                                            390,
                                        ),
                                        name: "InsufficientBalance".to_string(),
                                    },
                                    fields: vec![
                                        ErrorParameter {
                                            ty: Expression::Type(
                                                Loc::File(
                                                    0,
                                                    391,
                                                    398,
                                                ),
                                                Type::Uint(
                                                    256,
                                                ),
                                            ),
                                            loc: Loc::File(
                                                0,
                                                391,
                                                408,
                                            ),
                                            name: Some(
                                                Identifier {
                                                    loc: Loc::File(
                                                        0,
                                                        399,
                                                        408,
                                                    ),
                                                    name: "available".to_string(),
                                                },
                                            ),
                                        },
                                        ErrorParameter {
                                            ty: Expression::Type(
                                                Loc::File(
                                                    0,
                                                    410,
                                                    417,
                                                ),
                                                Type::Uint(
                                                    256,
                                                ),
                                            ),
                                            loc: Loc::File(
                                                0,
                                                410,
                                                426,
                                            ),
                                            name: Some(
                                                Identifier {
                                                    loc: Loc::File(
                                                        0,
                                                        418,
                                                        426,
                                                    ),
                                                    name: "required".to_string(),
                                                },
                                            ),
                                        },
                                    ],
                                },
                            )),
                        ],
                    },
                ))
            ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn test_assembly_parser() {
    let src = r#"
                function bar() {
                    assembly "evmasm" {
                        let x := 0
                        for { let i := 0 } lt(i, 0x100) { i := add(i, 0x20) } {
                            x := /* meh */ add(x, mload(i))

                            if gt(i, 0x10) {
                                break
                            }
                        }

                        let h : u32, y, z : u16 := funcCall()

                        switch x
                        case 0 {
                            revert(0, 0)
                            // feh
                        }
                        default {
                            leave
                        }
                    }

                    assembly {

                        function power(base : u256, exponent) -> result
                        {
                            let y := and("abc":u32, add(3:u256, 2:u256))
                            let result
                        }
                    }
                }"#;

    let mut comments = Vec::new();
    let lex = lexer::Lexer::new(src, 0, &mut comments);
    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![SourceUnitPart::FunctionDefinition(Box::new(
        FunctionDefinition {
            doc: vec![],
            loc: Loc::File(0, 17, 32),
            ty: FunctionTy::Function,
            name: Some(Identifier {
                loc: Loc::File(0, 26, 29),
                name: "bar".to_string(),
            }),
            name_loc: Loc::File(0, 26, 29),
            params: vec![],
            attributes: vec![],
            return_not_returns: None,
            returns: vec![],
            body: Some(Statement::Block {
                loc: Loc::File(0, 32, 1045),
                unchecked: false,
                statements: vec![
                    Statement::Assembly {
                        loc: Loc::File(0, 54, 736),
                        statements: vec![
                            AssemblyStatement::VariableDeclaration(
                                Loc::File(0, 98, 108),
                                vec![AssemblyTypedIdentifier {
                                    loc: Loc::File(0, 102, 103),
                                    id: Identifier {
                                        loc: Loc::File(0, 102, 103),
                                        name: "x".to_string(),
                                    },
                                    ty: None,
                                }],
                                Some(AssemblyExpression::NumberLiteral(
                                    Loc::File(0, 107, 108),
                                    BigInt::from(0),
                                    None,
                                )),
                            ),
                            AssemblyStatement::For(
                                Loc::File(0, 133, 388),
                                vec![AssemblyStatement::VariableDeclaration(
                                    Loc::File(0, 139, 149),
                                    vec![AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 143, 144),
                                        id: Identifier {
                                            loc: Loc::File(0, 143, 144),
                                            name: "i".to_string(),
                                        },
                                        ty: None,
                                    }],
                                    Some(AssemblyExpression::NumberLiteral(
                                        Loc::File(0, 148, 149),
                                        BigInt::from(0),
                                        None,
                                    )),
                                )],
                                AssemblyExpression::FunctionCall(Box::new(AssemblyFunctionCall {
                                    loc: Loc::File(0, 152, 164),
                                    function_name: Identifier {
                                        loc: Loc::File(0, 152, 154),
                                        name: "lt".to_string(),
                                    },
                                    arguments: vec![
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc::File(0, 155, 156),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::HexNumberLiteral(
                                            Loc::File(0, 158, 163),
                                            "0x100".to_string(),
                                            None,
                                        ),
                                    ],
                                })),
                                vec![AssemblyStatement::Assign(
                                    Loc::File(0, 167, 184),
                                    vec![AssemblyExpression::Variable(Identifier {
                                        loc: Loc::File(0, 167, 168),
                                        name: "i".to_string(),
                                    })],
                                    AssemblyExpression::FunctionCall(Box::new(
                                        AssemblyFunctionCall {
                                            loc: Loc::File(0, 172, 184),
                                            function_name: Identifier {
                                                loc: Loc::File(0, 172, 175),
                                                name: "add".to_string(),
                                            },
                                            arguments: vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc::File(0, 176, 177),
                                                    name: "i".to_string(),
                                                }),
                                                AssemblyExpression::HexNumberLiteral(
                                                    Loc::File(0, 179, 183),
                                                    "0x20".to_string(),
                                                    None,
                                                ),
                                            ],
                                        },
                                    )),
                                )],
                                vec![
                                    AssemblyStatement::Assign(
                                        Loc::File(0, 217, 248),
                                        vec![AssemblyExpression::Variable(Identifier {
                                            loc: Loc::File(0, 217, 218),
                                            name: "x".to_string(),
                                        })],
                                        AssemblyExpression::FunctionCall(Box::new(
                                            AssemblyFunctionCall {
                                                loc: Loc::File(0, 232, 248),
                                                function_name: Identifier {
                                                    loc: Loc::File(0, 232, 235),
                                                    name: "add".to_string(),
                                                },
                                                arguments: vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc::File(0, 236, 237),
                                                        name: "x".to_string(),
                                                    }),
                                                    AssemblyExpression::FunctionCall(Box::new(
                                                        AssemblyFunctionCall {
                                                            loc: Loc::File(0, 239, 247),
                                                            function_name: Identifier {
                                                                loc: Loc::File(0, 239, 244),
                                                                name: "mload".to_string(),
                                                            },
                                                            arguments: vec![
                                                                AssemblyExpression::Variable(
                                                                    Identifier {
                                                                        loc: Loc::File(0, 245, 246),
                                                                        name: "i".to_string(),
                                                                    },
                                                                ),
                                                            ],
                                                        },
                                                    )),
                                                ],
                                            },
                                        )),
                                    ),
                                    AssemblyStatement::If(
                                        Loc::File(0, 278, 362),
                                        AssemblyExpression::FunctionCall(Box::new(
                                            AssemblyFunctionCall {
                                                loc: Loc::File(0, 281, 292),
                                                function_name: Identifier {
                                                    loc: Loc::File(0, 281, 283),
                                                    name: "gt".to_string(),
                                                },
                                                arguments: vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc::File(0, 284, 285),
                                                        name: "i".to_string(),
                                                    }),
                                                    AssemblyExpression::HexNumberLiteral(
                                                        Loc::File(0, 287, 291),
                                                        "0x10".to_string(),
                                                        None,
                                                    ),
                                                ],
                                            },
                                        )),
                                        vec![AssemblyStatement::Break(Loc::File(0, 327, 332))],
                                    ),
                                ],
                            ),
                            AssemblyStatement::VariableDeclaration(
                                Loc::File(0, 414, 451),
                                vec![
                                    AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 418, 425),
                                        id: Identifier {
                                            loc: Loc::File(0, 418, 419),
                                            name: "h".to_string(),
                                        },
                                        ty: Some(Identifier {
                                            loc: Loc::File(0, 422, 425),
                                            name: "u32".to_string(),
                                        }),
                                    },
                                    AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 427, 428),
                                        id: Identifier {
                                            loc: Loc::File(0, 427, 428),
                                            name: "y".to_string(),
                                        },
                                        ty: None,
                                    },
                                    AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 430, 437),
                                        id: Identifier {
                                            loc: Loc::File(0, 430, 431),
                                            name: "z".to_string(),
                                        },
                                        ty: Some(Identifier {
                                            loc: Loc::File(0, 434, 437),
                                            name: "u16".to_string(),
                                        }),
                                    },
                                ],
                                Some(AssemblyExpression::FunctionCall(Box::new(
                                    AssemblyFunctionCall {
                                        loc: Loc::File(0, 441, 451),
                                        function_name: Identifier {
                                            loc: Loc::File(0, 441, 449),
                                            name: "funcCall".to_string(),
                                        },
                                        arguments: vec![],
                                    },
                                ))),
                            ),
                            AssemblyStatement::Switch(
                                Loc::File(0, 477, 714),
                                AssemblyExpression::Variable(Identifier {
                                    loc: Loc::File(0, 484, 485),
                                    name: "x".to_string(),
                                }),
                                vec![AssemblySwitch::Case(
                                    AssemblyExpression::NumberLiteral(
                                        Loc::File(0, 515, 516),
                                        BigInt::from(0),
                                        None,
                                    ),
                                    vec![AssemblyStatement::FunctionCall(Box::new(
                                        AssemblyFunctionCall {
                                            loc: Loc::File(0, 547, 559),
                                            function_name: Identifier {
                                                loc: Loc::File(0, 547, 553),
                                                name: "revert".to_string(),
                                            },
                                            arguments: vec![
                                                AssemblyExpression::NumberLiteral(
                                                    Loc::File(0, 554, 555),
                                                    BigInt::from(0),
                                                    None,
                                                ),
                                                AssemblyExpression::NumberLiteral(
                                                    Loc::File(0, 557, 558),
                                                    BigInt::from(0),
                                                    None,
                                                ),
                                            ],
                                        },
                                    ))],
                                )],
                                Some(AssemblySwitch::Default(vec![AssemblyStatement::Leave(
                                    Loc::File(0, 683, 688),
                                )])),
                            ),
                        ],
                        dialect: Some(StringLiteral {
                            loc: Loc::File(0, 63, 71),
                            string: "evmasm".to_string(),
                        }),
                    },
                    Statement::Assembly {
                        loc: Loc::File(0, 758, 1027),
                        statements: vec![AssemblyStatement::FunctionDefinition(Box::new(
                            AssemblyFunctionDefinition {
                                loc: Loc::File(0, 794, 1005),
                                id: Identifier {
                                    loc: Loc::File(0, 803, 808),
                                    name: "power".to_string(),
                                },
                                params: vec![
                                    AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 809, 820),
                                        id: Identifier {
                                            loc: Loc::File(0, 809, 813),
                                            name: "base".to_string(),
                                        },
                                        ty: Some(Identifier {
                                            loc: Loc::File(0, 816, 820),
                                            name: "u256".to_string(),
                                        }),
                                    },
                                    AssemblyTypedIdentifier {
                                        loc: Loc::File(0, 822, 830),
                                        id: Identifier {
                                            loc: Loc::File(0, 822, 830),
                                            name: "exponent".to_string(),
                                        },
                                        ty: None,
                                    },
                                ],
                                returns: vec![AssemblyTypedIdentifier {
                                    loc: Loc::File(0, 835, 841),
                                    id: Identifier {
                                        loc: Loc::File(0, 835, 841),
                                        name: "result".to_string(),
                                    },
                                    ty: None,
                                }],
                                body: vec![
                                    AssemblyStatement::VariableDeclaration(
                                        Loc::File(0, 896, 940),
                                        vec![AssemblyTypedIdentifier {
                                            loc: Loc::File(0, 900, 901),
                                            id: Identifier {
                                                loc: Loc::File(0, 900, 901),
                                                name: "y".to_string(),
                                            },
                                            ty: None,
                                        }],
                                        Some(AssemblyExpression::FunctionCall(Box::new(
                                            AssemblyFunctionCall {
                                                loc: Loc::File(0, 905, 940),
                                                function_name: Identifier {
                                                    loc: Loc::File(0, 905, 908),
                                                    name: "and".to_string(),
                                                },
                                                arguments: vec![
                                                    AssemblyExpression::StringLiteral(
                                                        StringLiteral {
                                                            loc: Loc::File(0, 909, 914),
                                                            string: "abc".to_string(),
                                                        },
                                                        Some(Identifier {
                                                            loc: Loc::File(0, 915, 918),
                                                            name: "u32".to_string(),
                                                        }),
                                                    ),
                                                    AssemblyExpression::FunctionCall(Box::new(
                                                        AssemblyFunctionCall {
                                                            loc: Loc::File(0, 920, 939),
                                                            function_name: Identifier {
                                                                loc: Loc::File(0, 920, 923),
                                                                name: "add".to_string(),
                                                            },
                                                            arguments: vec![
                                                                AssemblyExpression::NumberLiteral(
                                                                    Loc::File(0, 924, 930),
                                                                    BigInt::from(3),
                                                                    Some(Identifier {
                                                                        loc: Loc::File(0, 926, 930),
                                                                        name: "u256".to_string(),
                                                                    }),
                                                                ),
                                                                AssemblyExpression::NumberLiteral(
                                                                    Loc::File(0, 932, 938),
                                                                    BigInt::from(2),
                                                                    Some(Identifier {
                                                                        loc: Loc::File(0, 934, 938),
                                                                        name: "u256".to_string(),
                                                                    }),
                                                                ),
                                                            ],
                                                        },
                                                    )),
                                                ],
                                            },
                                        ))),
                                    ),
                                    AssemblyStatement::VariableDeclaration(
                                        Loc::File(0, 969, 979),
                                        vec![AssemblyTypedIdentifier {
                                            loc: Loc::File(0, 973, 979),
                                            id: Identifier {
                                                loc: Loc::File(0, 973, 979),
                                                name: "result".to_string(),
                                            },
                                            ty: None,
                                        }],
                                        None,
                                    ),
                                ],
                            },
                        ))],
                        dialect: None,
                    },
                ],
            }),
        },
    ))]);

    assert_eq!(expected_parse_tree, actual_parse_tree);

    assert_eq!(
        comments,
        vec![
            Comment::Block(Loc::File(0, 222, 231), "/* meh */".to_string()),
            Comment::Line(Loc::File(0, 588, 594), "// feh".to_string())
        ]
    );
}
