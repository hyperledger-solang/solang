//! Solidity file parser

use lalrpop_util::ParseError;

pub use diagnostics::Diagnostic;

pub mod diagnostics;
mod doc;
pub mod lexer;
pub mod pt;

#[allow(clippy::all)]
pub mod solidity {
    include!(concat!(env!("OUT_DIR"), "/solidity.rs"));
}

/// Parse soldiity file content
pub fn parse(
    src: &str,
    file_no: usize,
) -> Result<(pt::SourceUnit, Vec<pt::Comment>), Vec<Diagnostic>> {
    // parse phase
    let mut comments = Vec::new();

    let lex = lexer::Lexer::new(src, file_no, &mut comments);

    let s = solidity::SourceUnitParser::new().parse(src, file_no, lex);

    if let Err(e) = s {
        let errors = vec![match e {
            ParseError::InvalidToken { location } => Diagnostic::parser_error(
                pt::Loc::File(file_no, location, location),
                "invalid token".to_string(),
            ),
            ParseError::UnrecognizedToken {
                token: (l, token, r),
                expected,
            } => Diagnostic::parser_error(
                pt::Loc::File(file_no, l, r),
                format!(
                    "unrecognised token `{}', expected {}",
                    token,
                    expected.join(", ")
                ),
            ),
            ParseError::User { error } => Diagnostic::parser_error(*error.loc(), error.to_string()),
            ParseError::ExtraToken { token } => Diagnostic::parser_error(
                pt::Loc::File(file_no, token.0, token.2),
                format!("extra token `{}' encountered", token.0),
            ),
            ParseError::UnrecognizedEOF { location, expected } => Diagnostic::parser_error(
                pt::Loc::File(file_no, location, location),
                format!("unexpected end of file, expecting {}", expected.join(", ")),
            ),
        }];

        Err(errors)
    } else {
        Ok((s.unwrap(), comments))
    }
}

pub fn box_option<T>(o: Option<T>) -> Option<Box<T>> {
    o.map(Box::new)
}

#[cfg(test)]
mod test {
    use super::lexer;
    use super::pt::*;
    use super::solidity;

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

                    assembly {
                        let x := 0
                        for { let i := 0 } lt(i, 0x100) { i := add(i, 0x20) } {
                            x := /* meh */ add(x, mload(i))

                            if gt(i, 0x10) {
                                break
                            }
                        }

                        switch x
                        case 0 {
                            revert(0, 0)
                            // feh
                        }
                        default {
                            leave
                        }
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
                    loc: Loc::File(0, 735, 1770),
                    unchecked: false,
                    statements: vec![
                        Statement::Try(
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
                                        ty: Expression::Type(
                                            Loc::File(0, 780, 784),
                                            Type::Uint(256),
                                        ),
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
                                        ty: Expression::Type(
                                            Loc::File(0, 863, 868),
                                            Type::DynamicBytes,
                                        ),
                                        storage: Some(StorageLocation::Memory(Loc::File(
                                            0, 869, 875,
                                        ))),
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
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc::File(0, 912, 917),
                                                        string: "meh".to_string(),
                                                    },
                                                ])],
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
                                        storage: Some(StorageLocation::Memory(Loc::File(
                                            0, 961, 967,
                                        ))),
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
                                        ty: Expression::Type(
                                            Loc::File(0, 1050, 1054),
                                            Type::Uint(256),
                                        ),
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
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc::File(0, 1091, 1096),
                                                        string: "feh".to_string(),
                                                    },
                                                ])],
                                            ),
                                        )],
                                    },
                                ),
                            ],
                        ),
                        Statement::Assembly {
                            loc: Loc::File(0, 1142, 1752),
                            assembly: vec![
                                AssemblyStatement::LetAssign(
                                    Loc::File(0, 1177, 1187),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc::File(0, 1181, 1182),
                                        name: "x".to_string(),
                                    }),
                                    AssemblyExpression::NumberLiteral(
                                        Loc::File(0, 1186, 1187),
                                        0.into(),
                                    ),
                                ),
                                AssemblyStatement::For(
                                    Loc::File(0, 1212, 1467),
                                    vec![AssemblyStatement::LetAssign(
                                        Loc::File(0, 1218, 1228),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc::File(0, 1222, 1223),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::NumberLiteral(
                                            Loc::File(0, 1227, 1228),
                                            0.into(),
                                        ),
                                    )],
                                    AssemblyExpression::Function(
                                        Loc::File(0, 1231, 1243),
                                        Box::new(AssemblyExpression::Variable(Identifier {
                                            loc: Loc::File(0, 1231, 1233),
                                            name: "lt".to_string(),
                                        })),
                                        vec![
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc::File(0, 1234, 1235),
                                                name: "i".to_string(),
                                            }),
                                            AssemblyExpression::HexNumberLiteral(
                                                Loc::File(0, 1237, 1242),
                                                "0x100".to_string(),
                                            ),
                                        ],
                                    ),
                                    vec![AssemblyStatement::Assign(
                                        Loc::File(0, 1246, 1263),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc::File(0, 1246, 1247),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::Function(
                                            Loc::File(0, 1251, 1263),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc::File(0, 1251, 1254),
                                                name: "add".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc::File(0, 1255, 1256),
                                                    name: "i".to_string(),
                                                }),
                                                AssemblyExpression::HexNumberLiteral(
                                                    Loc::File(0, 1258, 1262),
                                                    "0x20".to_string(),
                                                ),
                                            ],
                                        ),
                                    )],
                                    Box::new(vec![
                                        AssemblyStatement::Assign(
                                            Loc::File(0, 1296, 1327),
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc::File(0, 1296, 1297),
                                                name: "x".to_string(),
                                            }),
                                            AssemblyExpression::Function(
                                                Loc::File(0, 1311, 1327),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc::File(0, 1311, 1314),
                                                        name: "add".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc::File(0, 1315, 1316),
                                                        name: "x".to_string(),
                                                    }),
                                                    AssemblyExpression::Function(
                                                        Loc::File(0, 1318, 1326),
                                                        Box::new(AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc::File(0, 1318, 1323),
                                                                name: "mload".to_string(),
                                                            },
                                                        )),
                                                        vec![AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc::File(0, 1324, 1325),
                                                                name: "i".to_string(),
                                                            },
                                                        )],
                                                    ),
                                                ],
                                            ),
                                        ),
                                        AssemblyStatement::If(
                                            Loc::File(0, 1357, 1441),
                                            AssemblyExpression::Function(
                                                Loc::File(0, 1360, 1371),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc::File(0, 1360, 1362),
                                                        name: "gt".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc::File(0, 1363, 1364),
                                                        name: "i".to_string(),
                                                    }),
                                                    AssemblyExpression::HexNumberLiteral(
                                                        Loc::File(0, 1366, 1370),
                                                        "0x10".to_string(),
                                                    ),
                                                ],
                                            ),
                                            Box::new(vec![AssemblyStatement::Break(Loc::File(
                                                0, 1406, 1411,
                                            ))]),
                                        ),
                                    ]),
                                ),
                                AssemblyStatement::Switch(
                                    Loc::File(0, 1493, 1730),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc::File(0, 1500, 1501),
                                        name: "x".to_string(),
                                    }),
                                    vec![AssemblySwitch::Case(
                                        AssemblyExpression::NumberLiteral(
                                            Loc::File(0, 1531, 1532),
                                            0.into(),
                                        ),
                                        Box::new(vec![AssemblyStatement::Expression(
                                            AssemblyExpression::Function(
                                                Loc::File(0, 1563, 1575),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc::File(0, 1563, 1569),
                                                        name: "revert".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc::File(0, 1570, 1571),
                                                        0.into(),
                                                    ),
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc::File(0, 1573, 1574),
                                                        0.into(),
                                                    ),
                                                ],
                                            ),
                                        )]),
                                    )],
                                    Some(AssemblySwitch::Default(Box::new(vec![
                                        AssemblyStatement::Leave(Loc::File(0, 1699, 1704)),
                                    ]))),
                                ),
                            ],
                        },
                    ],
                }),
            })),
        ]);

        assert_eq!(actual_parse_tree, expected_parse_tree);

        assert_eq!(
            comments,
            vec![
                Comment::Block(Loc::File(0, 1301, 1310), "/* meh */".to_string()),
                Comment::Line(Loc::File(0, 1604, 1610), "// feh".to_string())
            ]
        )
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
                    loc: Loc(
                        0,
                        10,
                        58,
                    ),
                    name: Identifier {
                        loc: Loc(
                            0,
                            16,
                            21,
                        ),
                        name: "Outer".to_string(),
                    },
                    fields: vec![
                        ErrorParameter {
                            ty: Expression::Type(
                                Loc(
                                    0,
                                    22,
                                    29,
                                ),
                                Type::Uint(
                                    256,
                                ),
                            ),
                            loc: Loc(
                                0,
                                22,
                                39,
                            ),
                            name: Some(
                                Identifier {
                                    loc: Loc(
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
                                Loc(
                                    0,
                                    41,
                                    48,
                                ),
                                Type::Uint(
                                    256,
                                ),
                            ),
                            loc: Loc(
                                0,
                                41,
                                57,
                            ),
                            name: Some(
                                Identifier {
                                    loc: Loc(
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
                        loc: Loc(
                            0,
                            69,
                            88,
                        ),
                        ty: ContractTy::Contract(
                            Loc(
                                0,
                                69,
                                77,
                            ),
                        ),
                        name: Identifier {
                            loc: Loc(
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
                                    loc: Loc(
                                        0,
                                        102,
                                        120,
                                    ),
                                    name: Identifier {
                                        loc: Loc(
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
                                    loc: Loc(
                                        0,
                                        365,
                                        427,
                                    ),
                                    name: Identifier {
                                        loc: Loc(
                                            0,
                                            371,
                                            390,
                                        ),
                                        name: "InsufficientBalance".to_string(),
                                    },
                                    fields: vec![
                                        ErrorParameter {
                                            ty: Expression::Type(
                                                Loc(
                                                    0,
                                                    391,
                                                    398,
                                                ),
                                                Type::Uint(
                                                    256,
                                                ),
                                            ),
                                            loc: Loc(
                                                0,
                                                391,
                                                408,
                                            ),
                                            name: Some(
                                                Identifier {
                                                    loc: Loc(
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
                                                Loc(
                                                    0,
                                                    410,
                                                    417,
                                                ),
                                                Type::Uint(
                                                    256,
                                                ),
                                            ),
                                            loc: Loc(
                                                0,
                                                410,
                                                426,
                                            ),
                                            name: Some(
                                                Identifier {
                                                    loc: Loc(
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
}
