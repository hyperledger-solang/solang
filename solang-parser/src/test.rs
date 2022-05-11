use crate::lexer::Lexer;
use crate::pt::*;
use crate::solidity;

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
    let lex = Lexer::new(src, 0, &mut comments);

    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::DocComment(DocComment{ loc: Loc::File(0,3, 14), ty: CommentType::Line, comment: " @title Foo".to_string()}),
        SourceUnitPart::DocComment(DocComment{ loc: Loc::File(0,34,51), ty: CommentType::Line, comment: " @description Foo".to_string()}),
        SourceUnitPart::DocComment(DocComment{ loc: Loc::File(0,71,75), ty: CommentType::Line, comment: " Bar".to_string()}),
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 92, 105),
            ty: ContractTy::Contract(Loc::File(0, 92, 100)),
            name: Identifier {
                loc: Loc::File(0, 101, 104),
                name: "foo".to_string(),
            },
            base: Vec::new(),
            parts: vec![
                ContractPart::DocComment(DocComment{ loc: Loc::File(0,130,191), ty: CommentType::Block, comment: "\n                    @title Jurisdiction\n                    ".to_string()}),
                ContractPart::DocComment(DocComment{ loc: Loc::File(0,217,230), ty: CommentType::Line, comment: " @author Anon".to_string()}),
                ContractPart::DocComment(DocComment{ loc: Loc::File(0,254,389), ty: CommentType::Block, comment: "\n                    @description Data for\n                    jurisdiction\n                    @dev It's a struct\n                    ".to_string()}),
                ContractPart::StructDefinition(Box::new(StructDefinition {
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
                            Expression::NumberLiteral(Loc::File(0, 765, 766), "1".to_string(), "".to_string()),
                            Expression::NumberLiteral(Loc::File(0, 768, 769), "1".to_string(), "".to_string()),
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
                                            "2".to_string(), "".to_string(),
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
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 905, 918),
                                    None,
                                    vec![Expression::StringLiteral(vec![StringLiteral {
                                        loc: Loc::File(0, 912, 917),
                                        string: "meh".to_string(),
                                    }])],
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
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 1001, 1014),
                                    None,
                                    vec![Expression::Variable(Identifier {
                                        loc: Loc::File(0, 1008, 1013),
                                        name: "error".to_string(),
                                    })],
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
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 1084, 1097),
                                    None,
                                    vec![Expression::StringLiteral(vec![StringLiteral {
                                        loc: Loc::File(0, 1091, 1096),
                                        string: "feh".to_string(),
                                    }])],
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

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::ErrorDefinition(Box::new(ErrorDefinition {
            loc: Loc::File(0, 10, 58),
            name: Identifier {
                loc: Loc::File(0, 16, 21),
                name: "Outer".to_string(),
            },
            fields: vec![
                ErrorParameter {
                    ty: Expression::Type(Loc::File(0, 22, 29), Type::Uint(256)),
                    loc: Loc::File(0, 22, 39),
                    name: Some(Identifier {
                        loc: Loc::File(0, 30, 39),
                        name: "available".to_string(),
                    }),
                },
                ErrorParameter {
                    ty: Expression::Type(Loc::File(0, 41, 48), Type::Uint(256)),
                    loc: Loc::File(0, 41, 57),
                    name: Some(Identifier {
                        loc: Loc::File(0, 49, 57),
                        name: "required".to_string(),
                    }),
                },
            ],
        })),
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 69, 88),
            ty: ContractTy::Contract(Loc::File(0, 69, 77)),
            name: Identifier {
                loc: Loc::File(0, 78, 87),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 102, 120),
                    name: Identifier {
                        loc: Loc::File(0, 108, 118),
                        name: "NotPending".to_string(),
                    },
                    fields: vec![],
                })),
                ContractPart::DocComment(DocComment {
                    loc: Loc::File(0, 137, 199),
                    ty: CommentType::Line,
                    comment: " Insufficient balance for transfer. Needed `required` but only"
                        .to_string(),
                }),
                ContractPart::DocComment(DocComment {
                    loc: Loc::File(0, 215, 238),
                    ty: CommentType::Line,
                    comment: " `available` available.".to_string(),
                }),
                ContractPart::DocComment(DocComment {
                    loc: Loc::File(0, 254, 290),
                    ty: CommentType::Line,
                    comment: " @param available balance available.".to_string(),
                }),
                ContractPart::DocComment(DocComment {
                    loc: Loc::File(0, 306, 352),
                    ty: CommentType::Line,
                    comment: " @param required requested amount to transfer.".to_string(),
                }),
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 365, 427),
                    name: Identifier {
                        loc: Loc::File(0, 371, 390),
                        name: "InsufficientBalance".to_string(),
                    },
                    fields: vec![
                        ErrorParameter {
                            ty: Expression::Type(Loc::File(0, 391, 398), Type::Uint(256)),
                            loc: Loc::File(0, 391, 408),
                            name: Some(Identifier {
                                loc: Loc::File(0, 399, 408),
                                name: "available".to_string(),
                            }),
                        },
                        ErrorParameter {
                            ty: Expression::Type(Loc::File(0, 410, 417), Type::Uint(256)),
                            loc: Loc::File(0, 410, 426),
                            name: Some(Identifier {
                                loc: Loc::File(0, 418, 426),
                                name: "required".to_string(),
                            }),
                        },
                    ],
                })),
            ],
        })),
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
    let lex = Lexer::new(src, 0, &mut comments);
    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![SourceUnitPart::FunctionDefinition(Box::new(
        FunctionDefinition {
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
                        block: YulBlock {
                            loc: Loc::File(0, 72, 736),
                            statements: vec![
                                YulStatement::VariableDeclaration(
                                    Loc::File(0, 98, 108),
                                    vec![YulTypedIdentifier {
                                        loc: Loc::File(0, 102, 103),
                                        id: Identifier {
                                            loc: Loc::File(0, 102, 103),
                                            name: "x".to_string(),
                                        },
                                        ty: None,
                                    }],
                                    Some(YulExpression::NumberLiteral(
                                        Loc::File(0, 107, 108),
                                        "0".to_string(), "".to_string(),
                                        None,
                                    )),
                                ),
                                YulStatement::For(YulFor {
                                    loc: Loc::File(0, 133, 388),
                                    init_block: YulBlock {
                                        loc: Loc::File(0, 137, 151),
                                        statements: vec![YulStatement::VariableDeclaration(
                                            Loc::File(0, 139, 149),
                                            vec![YulTypedIdentifier {
                                                loc: Loc::File(0, 143, 144),
                                                id: Identifier {
                                                    loc: Loc::File(0, 143, 144),
                                                    name: "i".to_string(),
                                                },
                                                ty: None,
                                            }],
                                            Some(YulExpression::NumberLiteral(
                                                Loc::File(0, 148, 149),
                                                "0".to_string(), "".to_string(),
                                                None,
                                            )),
                                        )],
                                    },
                                    condition: YulExpression::FunctionCall(Box::new(YulFunctionCall {
                                        loc: Loc::File(0, 152, 164),
                                        id: Identifier {
                                            loc: Loc::File(0, 152, 154),
                                            name: "lt".to_string(),
                                        },
                                        arguments: vec![
                                            YulExpression::Variable(Identifier {
                                                loc: Loc::File(0, 155, 156),
                                                name: "i".to_string(),
                                            }),
                                            YulExpression::HexNumberLiteral(
                                                Loc::File(0, 158, 163),
                                                "0x100".to_string(),
                                                None,
                                            ),
                                        ],
                                    })),
                                    post_block: YulBlock {
                                        loc: Loc::File(0, 165, 186),
                                        statements: vec![YulStatement::Assign(
                                            Loc::File(0, 167, 184),
                                            vec![YulExpression::Variable(Identifier {
                                                loc: Loc::File(0, 167, 168),
                                                name: "i".to_string(),
                                            })],
                                            YulExpression::FunctionCall(Box::new(
                                                YulFunctionCall {
                                                    loc: Loc::File(0, 172, 184),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 172, 175),
                                                        name: "add".to_string(),
                                                    },
                                                    arguments: vec![
                                                        YulExpression::Variable(Identifier {
                                                            loc: Loc::File(0, 176, 177),
                                                            name: "i".to_string(),
                                                        }),
                                                        YulExpression::HexNumberLiteral(
                                                            Loc::File(0, 179, 183),
                                                            "0x20".to_string(),
                                                            None,
                                                        ),
                                                    ],
                                                },
                                            )),
                                        )],
                                    },
                                    execution_block: YulBlock {
                                        loc: Loc::File(0, 187, 388),
                                        statements: vec![
                                            YulStatement::Assign(
                                                Loc::File(0, 217, 248),
                                                vec![YulExpression::Variable(Identifier {
                                                    loc: Loc::File(0, 217, 218),
                                                    name: "x".to_string(),
                                                })],
                                                YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 232, 248),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 232, 235),
                                                            name: "add".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::Variable(Identifier {
                                                                loc: Loc::File(0, 236, 237),
                                                                name: "x".to_string(),
                                                            }),
                                                            YulExpression::FunctionCall(Box::new(
                                                                YulFunctionCall {
                                                                    loc: Loc::File(0, 239, 247),
                                                                    id: Identifier {
                                                                        loc: Loc::File(0, 239, 244),
                                                                        name: "mload".to_string(),
                                                                    },
                                                                    arguments: vec![
                                                                        YulExpression::Variable(
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
                                            YulStatement::If(
                                                Loc::File(0, 278, 362),
                                                YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 281, 292),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 281, 283),
                                                            name: "gt".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::Variable(Identifier {
                                                                loc: Loc::File(0, 284, 285),
                                                                name: "i".to_string(),
                                                            }),
                                                            YulExpression::HexNumberLiteral(
                                                                Loc::File(0, 287, 291),
                                                                "0x10".to_string(),
                                                                None,
                                                            ),
                                                        ],
                                                    },
                                                )),
                                                YulBlock {
                                                    loc: Loc::File(0, 293, 362),
                                                    statements: vec![YulStatement::Break(Loc::File(0, 327, 332))],
                                                },
                                            ),
                                        ],
                                    },
                            }),
                                YulStatement::VariableDeclaration(
                                    Loc::File(0, 414, 451),
                                    vec![
                                        YulTypedIdentifier {
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
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 427, 428),
                                            id: Identifier {
                                                loc: Loc::File(0, 427, 428),
                                                name: "y".to_string(),
                                            },
                                            ty: None,
                                        },
                                        YulTypedIdentifier {
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
                                    Some(YulExpression::FunctionCall(Box::new(
                                        YulFunctionCall {
                                            loc: Loc::File(0, 441, 451),
                                            id: Identifier {
                                                loc: Loc::File(0, 441, 449),
                                                name: "funcCall".to_string(),
                                            },
                                            arguments: vec![],
                                        },
                                    ))),
                                ),
                                YulStatement::Switch(YulSwitch {
                                    loc: Loc::File(0, 477, 714),
                                    condition: YulExpression::Variable(Identifier {
                                        loc: Loc::File(0, 484, 485),
                                        name: "x".to_string(),
                                    }),
                                    cases: vec![YulSwitchOptions::Case(
                                        Loc::File(0, 510, 620),
                                        YulExpression::NumberLiteral(
                                            Loc::File(0, 515, 516),
                                            "0".to_string(), "".to_string(),
                                            None,
                                        ),
                                        YulBlock {
                                            loc: Loc::File(0, 517, 620),
                                            statements: vec![YulStatement::FunctionCall(Box::new(
                                                YulFunctionCall {
                                                    loc: Loc::File(0, 547, 559),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 547, 553),
                                                        name: "revert".to_string(),
                                                    },
                                                    arguments: vec![
                                                        YulExpression::NumberLiteral(
                                                            Loc::File(0, 554, 555),
                                                            "0".to_string(), "".to_string(),
                                                            None,
                                                        ),
                                                        YulExpression::NumberLiteral(
                                                            Loc::File(0, 557, 558),
                                                            "0".to_string(), "".to_string(),
                                                            None,
                                                        ),
                                                    ],
                                                },
                                            ))],
                                        }
                                    )],
                                    default: Some(YulSwitchOptions::Default(
                                        Loc::File(0, 645, 714),
                                        YulBlock {
                                            loc: Loc::File(0, 653, 714),
                                            statements: vec![YulStatement::Leave(Loc::File(0, 683, 688))],
                                        }
                                    )),
                            }),
                            ],
                        },
                        dialect: Some(StringLiteral {
                            loc: Loc::File(0, 63, 71),
                            string: "evmasm".to_string(),
                        }),
                    },
                    Statement::Assembly {
                        loc: Loc::File(0, 758, 1027),
                        block: YulBlock {
                          loc: Loc::File(0, 767, 1027),
                            statements: vec![YulStatement::FunctionDefinition(Box::new(
                                YulFunctionDefinition {
                                    loc: Loc::File(0, 794, 1005),
                                    id: Identifier {
                                        loc: Loc::File(0, 803, 808),
                                        name: "power".to_string(),
                                    },
                                    params: vec![
                                        YulTypedIdentifier {
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
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 822, 830),
                                            id: Identifier {
                                                loc: Loc::File(0, 822, 830),
                                                name: "exponent".to_string(),
                                            },
                                            ty: None,
                                        },
                                    ],
                                    returns: vec![YulTypedIdentifier {
                                        loc: Loc::File(0, 835, 841),
                                        id: Identifier {
                                            loc: Loc::File(0, 835, 841),
                                            name: "result".to_string(),
                                        },
                                        ty: None,
                                    }],
                                    body: YulBlock {
                                        loc: Loc::File(0, 866, 1005),
                                        statements:  vec![
                                            YulStatement::VariableDeclaration(
                                                Loc::File(0, 896, 940),
                                                vec![YulTypedIdentifier {
                                                    loc: Loc::File(0, 900, 901),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 900, 901),
                                                        name: "y".to_string(),
                                                    },
                                                    ty: None,
                                                }],
                                                Some(YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 905, 940),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 905, 908),
                                                            name: "and".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::StringLiteral(
                                                                StringLiteral {
                                                                    loc: Loc::File(0, 909, 914),
                                                                    string: "abc".to_string(),
                                                                },
                                                                Some(Identifier {
                                                                    loc: Loc::File(0, 915, 918),
                                                                    name: "u32".to_string(),
                                                                }),
                                                            ),
                                                            YulExpression::FunctionCall(Box::new(
                                                                YulFunctionCall {
                                                                    loc: Loc::File(0, 920, 939),
                                                                    id: Identifier {
                                                                        loc: Loc::File(0, 920, 923),
                                                                        name: "add".to_string(),
                                                                    },
                                                                    arguments: vec![
                                                                        YulExpression::NumberLiteral(
                                                                            Loc::File(0, 924, 930),
                                                                            "3".to_string(), "".to_string(),
                                                                            Some(Identifier {
                                                                                loc: Loc::File(0, 926, 930),
                                                                                name: "u256".to_string(),
                                                                            }),
                                                                        ),
                                                                        YulExpression::NumberLiteral(
                                                                            Loc::File(0, 932, 938),
                                                                            "2".to_string(), "".to_string(),
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
                                            YulStatement::VariableDeclaration(
                                                Loc::File(0, 969, 979),
                                                vec![YulTypedIdentifier {
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
                                    }
                                },
                            ))],
                        },
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

#[test]
fn parse_revert_test() {
    let src = r#"
        contract TestToken {
            error BAR_ERROR();
            function foo()  {
                revert BAR_ERROR();
            }
        }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);

    let expected_parse_tree = SourceUnit(vec![SourceUnitPart::ContractDefinition(Box::new(
        ContractDefinition {
            loc: Loc::File(0, 9, 28),
            ty: ContractTy::Contract(Loc::File(0, 9, 17)),
            name: Identifier {
                loc: Loc::File(0, 18, 27),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 42, 59),
                    name: Identifier {
                        loc: Loc::File(0, 48, 57),
                        name: "BAR_ERROR".to_string(),
                    },
                    fields: vec![],
                })),
                ContractPart::FunctionDefinition(Box::new(FunctionDefinition {
                    loc: Loc::File(0, 73, 89),
                    ty: FunctionTy::Function,
                    name: Some(Identifier {
                        loc: Loc::File(0, 82, 85),
                        name: "foo".to_string(),
                    }),
                    name_loc: Loc::File(0, 82, 85),
                    params: vec![],
                    attributes: vec![],
                    return_not_returns: None,
                    returns: vec![],
                    body: Some(Statement::Block {
                        loc: Loc::File(0, 89, 140),
                        unchecked: false,
                        statements: vec![Statement::Revert(
                            Loc::File(0, 107, 125),
                            Some(Expression::Variable(Identifier {
                                loc: Loc::File(0, 114, 123),
                                name: "BAR_ERROR".to_string(),
                            })),
                            vec![],
                        )],
                    }),
                })),
            ],
        },
    ))]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn parse_byte_function_assembly() {
    let src = r#"
    contract ECDSA {
        function tryRecover() internal pure {
            assembly {
                v := byte(0, mload(add(signature, 0x60)))
            }
        }
    }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);
}

#[test]
fn parse_user_defined_value_type() {
    let src = r#"
        type Uint256 is uint256;
        contract TestToken {
            type Bytes32 is bytes32;
        }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 2);

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::TypeDefinition(Box::new(TypeDefinition {
            loc: Loc::File(0, 9, 32),
            name: Identifier {
                loc: Loc::File(0, 14, 21),
                name: "Uint256".to_string(),
            },
            ty: Expression::Type(Loc::File(0, 25, 32), Type::Uint(256)),
        })),
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 42, 61),
            ty: ContractTy::Contract(Loc::File(0, 42, 50)),
            name: Identifier {
                loc: Loc::File(0, 51, 60),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![ContractPart::TypeDefinition(Box::new(TypeDefinition {
                loc: Loc::File(0, 75, 98),
                name: Identifier {
                    loc: Loc::File(0, 80, 87),
                    name: "Bytes32".to_string(),
                },
                ty: Expression::Type(Loc::File(0, 91, 98), Type::Bytes(32)),
            }))],
        })),
    ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn parse_no_parameters_yul_function() {
    let src = r#"
contract C {
	function testing() pure public {
		assembly {
			function multiple() -> ret1, ret2 {
				ret1 := 1
				ret2 := 2
			}
		}
	}
}
    "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);
}
