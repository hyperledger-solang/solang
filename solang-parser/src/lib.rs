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
                pt::Loc(file_no, location, location),
                "invalid token".to_string(),
            ),
            ParseError::UnrecognizedToken {
                token: (l, token, r),
                expected,
            } => Diagnostic::parser_error(
                pt::Loc(file_no, l, r),
                format!(
                    "unrecognised token `{}', expected {}",
                    token,
                    expected.join(", ")
                ),
            ),
            ParseError::User { error } => Diagnostic::parser_error(*error.loc(), error.to_string()),
            ParseError::ExtraToken { token } => Diagnostic::parser_error(
                pt::Loc(file_no, token.0, token.2),
                format!("extra token `{}' encountered", token.0),
            ),
            ParseError::UnrecognizedEOF { location, expected } => Diagnostic::parser_error(
                pt::Loc(file_no, location, location),
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
                loc: Loc(0, 92, 105),
                ty: ContractTy::Contract(Loc(0, 92, 100)),
                name: Identifier {
                    loc: Loc(0, 101, 104),
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
                            loc: Loc(0, 419, 431),
                            name: "Jurisdiction".to_string(),
                        },
                        loc: Loc(0, 412, 609),
                        fields: vec![
                            VariableDeclaration {
                                loc: Loc(0, 458, 469),
                                ty: Expression::Type(Loc(0, 458, 462), Type::Bool),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 463, 469),
                                    name: "exists".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 495, 506),
                                ty: Expression::Type(Loc(0, 495, 499), Type::Uint(256)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 500, 506),
                                    name: "keyIdx".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 532, 546),
                                ty: Expression::Type(Loc(0, 532, 538), Type::Bytes(2)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 539, 546),
                                    name: "country".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 572, 586),
                                ty: Expression::Type(Loc(0, 572, 579), Type::Bytes(32)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 580, 586),
                                    name: "region".to_string(),
                                },
                            },
                        ],
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 630, 636), Type::String),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 637, 645),
                            name: "__abba_$".to_string(),
                        },
                        loc: Loc(0, 630, 645),
                        initializer: None,
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 667, 672), Type::Int(64)),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 673, 683),
                            name: "$thing_102".to_string(),
                        },
                        loc: Loc(0, 667, 683),
                        initializer: None,
                    })),
                ],
            })),
            SourceUnitPart::FunctionDefinition(Box::new(FunctionDefinition {
                doc: vec![],
                loc: Loc(0, 720, 735),
                ty: FunctionTy::Function,
                name: Some(Identifier {
                    loc: Loc(0, 729, 732),
                    name: "bar".to_string(),
                }),
                name_loc: Loc(0, 729, 732),
                params: vec![],
                attributes: vec![],
                return_not_returns: None,
                returns: vec![],
                body: Some(Statement::Block {
                    loc: Loc(0, 735, 1770),
                    unchecked: false,
                    statements: vec![
                        Statement::Try(
                            Loc(0, 757, 1120),
                            Expression::FunctionCall(
                                Loc(0, 761, 770),
                                Box::new(Expression::Variable(Identifier {
                                    loc: Loc(0, 761, 764),
                                    name: "sum".to_string(),
                                })),
                                vec![
                                    Expression::NumberLiteral(Loc(0, 765, 766), 1.into()),
                                    Expression::NumberLiteral(Loc(0, 768, 769), 1.into()),
                                ],
                            ),
                            Some((
                                vec![(
                                    Loc(0, 780, 788),
                                    Some(Parameter {
                                        loc: Loc(0, 780, 788),
                                        ty: Expression::Type(Loc(0, 780, 784), Type::Uint(256)),
                                        storage: None,
                                        name: Some(Identifier {
                                            loc: Loc(0, 785, 788),
                                            name: "sum".to_string(),
                                        }),
                                    }),
                                )],
                                Box::new(Statement::Block {
                                    loc: Loc(0, 790, 855),
                                    unchecked: false,
                                    statements: vec![Statement::Expression(
                                        Loc(0, 816, 832),
                                        Expression::FunctionCall(
                                            Loc(0, 816, 832),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 816, 822),
                                                name: "assert".to_string(),
                                            })),
                                            vec![Expression::Equal(
                                                Loc(0, 827, 829),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 823, 826),
                                                    name: "sum".to_string(),
                                                })),
                                                Box::new(Expression::NumberLiteral(
                                                    Loc(0, 830, 831),
                                                    2.into(),
                                                )),
                                            )],
                                        ),
                                    )],
                                }),
                            )),
                            vec![
                                CatchClause::Simple(
                                    Loc(0, 856, 941),
                                    Some(Parameter {
                                        loc: Loc(0, 863, 877),
                                        ty: Expression::Type(Loc(0, 863, 868), Type::DynamicBytes),
                                        storage: Some(StorageLocation::Memory(Loc(0, 869, 875))),
                                        name: Some(Identifier {
                                            loc: Loc(0, 876, 877),
                                            name: "b".to_string(),
                                        }),
                                    }),
                                    Statement::Block {
                                        loc: Loc(0, 879, 941),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 905, 918),
                                            Expression::FunctionCall(
                                                Loc(0, 905, 918),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 905, 911),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc(0, 912, 917),
                                                        string: "meh".to_string(),
                                                    },
                                                ])],
                                            ),
                                        )],
                                    },
                                ),
                                CatchClause::Named(
                                    Loc(0, 942, 1037),
                                    Identifier {
                                        loc: Loc(0, 948, 953),
                                        name: "Error".to_string(),
                                    },
                                    Parameter {
                                        loc: Loc(0, 954, 973),
                                        ty: Expression::Type(Loc(0, 954, 960), Type::String),
                                        storage: Some(StorageLocation::Memory(Loc(0, 961, 967))),
                                        name: Some(Identifier {
                                            loc: Loc(0, 968, 973),
                                            name: "error".to_string(),
                                        }),
                                    },
                                    Statement::Block {
                                        loc: Loc(0, 975, 1037),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 1001, 1014),
                                            Expression::FunctionCall(
                                                Loc(0, 1001, 1014),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 1001, 1007),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::Variable(Identifier {
                                                    loc: Loc(0, 1008, 1013),
                                                    name: "error".to_string(),
                                                })],
                                            ),
                                        )],
                                    },
                                ),
                                CatchClause::Named(
                                    Loc(0, 1038, 1120),
                                    Identifier {
                                        loc: Loc(0, 1044, 1049),
                                        name: "Panic".to_string(),
                                    },
                                    Parameter {
                                        loc: Loc(0, 1050, 1056),
                                        ty: Expression::Type(Loc(0, 1050, 1054), Type::Uint(256)),
                                        storage: None,
                                        name: Some(Identifier {
                                            loc: Loc(0, 1055, 1056),
                                            name: "x".to_string(),
                                        }),
                                    },
                                    Statement::Block {
                                        loc: Loc(0, 1058, 1120),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 1084, 1097),
                                            Expression::FunctionCall(
                                                Loc(0, 1084, 1097),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 1084, 1090),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc(0, 1091, 1096),
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
                            loc: Loc(0, 1142, 1752),
                            assembly: vec![
                                AssemblyStatement::LetAssign(
                                    Loc(0, 1177, 1187),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 1181, 1182),
                                        name: "x".to_string(),
                                    }),
                                    AssemblyExpression::NumberLiteral(Loc(0, 1186, 1187), 0.into()),
                                ),
                                AssemblyStatement::For(
                                    Loc(0, 1212, 1467),
                                    vec![AssemblyStatement::LetAssign(
                                        Loc(0, 1218, 1228),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1222, 1223),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::NumberLiteral(
                                            Loc(0, 1227, 1228),
                                            0.into(),
                                        ),
                                    )],
                                    AssemblyExpression::Function(
                                        Loc(0, 1231, 1243),
                                        Box::new(AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1231, 1233),
                                            name: "lt".to_string(),
                                        })),
                                        vec![
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1234, 1235),
                                                name: "i".to_string(),
                                            }),
                                            AssemblyExpression::HexNumberLiteral(
                                                Loc(0, 1237, 1242),
                                                "0x100".to_string(),
                                            ),
                                        ],
                                    ),
                                    vec![AssemblyStatement::Assign(
                                        Loc(0, 1246, 1263),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1246, 1247),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::Function(
                                            Loc(0, 1251, 1263),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1251, 1254),
                                                name: "add".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc(0, 1255, 1256),
                                                    name: "i".to_string(),
                                                }),
                                                AssemblyExpression::HexNumberLiteral(
                                                    Loc(0, 1258, 1262),
                                                    "0x20".to_string(),
                                                ),
                                            ],
                                        ),
                                    )],
                                    Box::new(vec![
                                        AssemblyStatement::Assign(
                                            Loc(0, 1296, 1327),
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1296, 1297),
                                                name: "x".to_string(),
                                            }),
                                            AssemblyExpression::Function(
                                                Loc(0, 1311, 1327),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1311, 1314),
                                                        name: "add".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc(0, 1315, 1316),
                                                        name: "x".to_string(),
                                                    }),
                                                    AssemblyExpression::Function(
                                                        Loc(0, 1318, 1326),
                                                        Box::new(AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc(0, 1318, 1323),
                                                                name: "mload".to_string(),
                                                            },
                                                        )),
                                                        vec![AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc(0, 1324, 1325),
                                                                name: "i".to_string(),
                                                            },
                                                        )],
                                                    ),
                                                ],
                                            ),
                                        ),
                                        AssemblyStatement::If(
                                            Loc(0, 1357, 1441),
                                            AssemblyExpression::Function(
                                                Loc(0, 1360, 1371),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1360, 1362),
                                                        name: "gt".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc(0, 1363, 1364),
                                                        name: "i".to_string(),
                                                    }),
                                                    AssemblyExpression::HexNumberLiteral(
                                                        Loc(0, 1366, 1370),
                                                        "0x10".to_string(),
                                                    ),
                                                ],
                                            ),
                                            Box::new(vec![AssemblyStatement::Break(Loc(
                                                0, 1406, 1411,
                                            ))]),
                                        ),
                                    ]),
                                ),
                                AssemblyStatement::Switch(
                                    Loc(0, 1493, 1730),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 1500, 1501),
                                        name: "x".to_string(),
                                    }),
                                    vec![AssemblySwitch::Case(
                                        AssemblyExpression::NumberLiteral(
                                            Loc(0, 1531, 1532),
                                            0.into(),
                                        ),
                                        Box::new(vec![AssemblyStatement::Expression(
                                            AssemblyExpression::Function(
                                                Loc(0, 1563, 1575),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1563, 1569),
                                                        name: "revert".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc(0, 1570, 1571),
                                                        0.into(),
                                                    ),
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc(0, 1573, 1574),
                                                        0.into(),
                                                    ),
                                                ],
                                            ),
                                        )]),
                                    )],
                                    Some(AssemblySwitch::Default(Box::new(vec![
                                        AssemblyStatement::Leave(Loc(0, 1699, 1704)),
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
                Comment::Block(Loc(0, 1301, 1310), "/* meh */".to_string()),
                Comment::Line(Loc(0, 1604, 1610), "// feh".to_string())
            ]
        )
    }
}
