//! Solidity file parser

use lalrpop_util::ParseError;

pub use diagnostics::Diagnostic;

mod comments;
pub mod diagnostics;
pub mod lexer;
pub mod pt;

#[allow(clippy::all)]
pub mod solidity {
    include!(concat!(env!("OUT_DIR"), "/solidity.rs"));
}

/// Parse soldiity file content
pub fn parse(src: &str, file_no: usize) -> Result<pt::SourceUnit, Vec<Diagnostic>> {
    // parse phase
    let lex = lexer::Lexer::new(src);

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
            ParseError::User { error } => {
                Diagnostic::parser_error(error.loc(file_no), error.to_string())
            }
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
        Ok(s.unwrap())
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
        let src = "// SPDX-License-Identifier: MIT
                /// @title Foo
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
                
                //// nice bar function
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
                            x := add(x, mload(i))
        
                            if gt(i, 0x10) {
                                break
                            }
                        }    
                        
                        switch x
                        case 0 {
                            revert(0, 0)
                        }
                        default {
                            leave
                        }
                    }
                }";

        let lex = lexer::Lexer::new(src);

        let actual_parse_tree = solidity::SourceUnitParser::new()
            .parse(src, 0, lex)
            .unwrap();

        let expected_parse_tree = SourceUnit(vec![
            SourceUnitPart::Comment(Comment::Line {
                comment: " SPDX-License-Identifier: MIT".to_string(),
            }),
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
                loc: Loc(0, 140, 153),
                ty: ContractTy::Contract(Loc(0, 140, 148)),
                name: Identifier {
                    loc: Loc(0, 149, 152),
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
                            loc: Loc(0, 467, 479),
                            name: "Jurisdiction".to_string(),
                        },
                        loc: Loc(0, 460, 657),
                        fields: vec![
                            VariableDeclaration {
                                loc: Loc(0, 506, 517),
                                ty: Expression::Type(Loc(0, 506, 510), Type::Bool),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 511, 517),
                                    name: "exists".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 543, 554),
                                ty: Expression::Type(Loc(0, 543, 547), Type::Uint(256)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 548, 554),
                                    name: "keyIdx".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 580, 594),
                                ty: Expression::Type(Loc(0, 580, 586), Type::Bytes(2)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 587, 594),
                                    name: "country".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 620, 634),
                                ty: Expression::Type(Loc(0, 620, 627), Type::Bytes(32)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 628, 634),
                                    name: "region".to_string(),
                                },
                            },
                        ],
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 678, 684), Type::String),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 685, 693),
                            name: "__abba_$".to_string(),
                        },
                        loc: Loc(0, 678, 693),
                        initializer: None,
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 715, 720), Type::Int(64)),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 721, 731),
                            name: "$thing_102".to_string(),
                        },
                        loc: Loc(0, 715, 731),
                        initializer: None,
                    })),
                ],
            })),
            SourceUnitPart::Comment(Comment::Line {
                comment: "// nice bar function".to_string(),
            }),
            SourceUnitPart::FunctionDefinition(Box::new(FunctionDefinition {
                doc: vec![],
                loc: Loc(0, 823, 838),
                ty: FunctionTy::Function,
                name: Some(Identifier {
                    loc: Loc(0, 832, 835),
                    name: "bar".to_string(),
                }),
                name_loc: Loc(0, 832, 835),
                params: vec![],
                attributes: vec![],
                return_not_returns: None,
                returns: vec![],
                body: Some(Statement::Block {
                    loc: Loc(0, 838, 1864),
                    unchecked: false,
                    statements: vec![
                        Statement::Try(
                            Loc(0, 860, 1223),
                            Expression::FunctionCall(
                                Loc(0, 864, 873),
                                Box::new(Expression::Variable(Identifier {
                                    loc: Loc(0, 864, 867),
                                    name: "sum".to_string(),
                                })),
                                vec![
                                    Expression::NumberLiteral(Loc(0, 868, 869), 1.into()),
                                    Expression::NumberLiteral(Loc(0, 871, 872), 1.into()),
                                ],
                            ),
                            Some((
                                vec![(
                                    Loc(0, 883, 891),
                                    Some(Parameter {
                                        loc: Loc(0, 883, 891),
                                        ty: Expression::Type(Loc(0, 883, 887), Type::Uint(256)),
                                        storage: None,
                                        name: Some(Identifier {
                                            loc: Loc(0, 888, 891),
                                            name: "sum".to_string(),
                                        }),
                                    }),
                                )],
                                Box::new(Statement::Block {
                                    loc: Loc(0, 893, 958),
                                    unchecked: false,
                                    statements: vec![Statement::Expression(
                                        Loc(0, 919, 935),
                                        Expression::FunctionCall(
                                            Loc(0, 919, 935),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 919, 925),
                                                name: "assert".to_string(),
                                            })),
                                            vec![Expression::Equal(
                                                Loc(0, 930, 932),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 926, 929),
                                                    name: "sum".to_string(),
                                                })),
                                                Box::new(Expression::NumberLiteral(
                                                    Loc(0, 933, 934),
                                                    2.into(),
                                                )),
                                            )],
                                        ),
                                    )],
                                }),
                            )),
                            vec![
                                CatchClause::Simple(
                                    Loc(0, 959, 1044),
                                    Some(Parameter {
                                        loc: Loc(0, 966, 980),
                                        ty: Expression::Type(Loc(0, 966, 971), Type::DynamicBytes),
                                        storage: Some(StorageLocation::Memory(Loc(0, 972, 978))),
                                        name: Some(Identifier {
                                            loc: Loc(0, 979, 980),
                                            name: "b".to_string(),
                                        }),
                                    }),
                                    Statement::Block {
                                        loc: Loc(0, 982, 1044),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 1008, 1021),
                                            Expression::FunctionCall(
                                                Loc(0, 1008, 1021),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 1008, 1014),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc(0, 1015, 1020),
                                                        string: "meh".to_string(),
                                                    },
                                                ])],
                                            ),
                                        )],
                                    },
                                ),
                                CatchClause::Named(
                                    Loc(0, 1045, 1140),
                                    Identifier {
                                        loc: Loc(0, 1051, 1056),
                                        name: "Error".to_string(),
                                    },
                                    Parameter {
                                        loc: Loc(0, 1057, 1076),
                                        ty: Expression::Type(Loc(0, 1057, 1063), Type::String),
                                        storage: Some(StorageLocation::Memory(Loc(0, 1064, 1070))),
                                        name: Some(Identifier {
                                            loc: Loc(0, 1071, 1076),
                                            name: "error".to_string(),
                                        }),
                                    },
                                    Statement::Block {
                                        loc: Loc(0, 1078, 1140),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 1104, 1117),
                                            Expression::FunctionCall(
                                                Loc(0, 1104, 1117),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 1104, 1110),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::Variable(Identifier {
                                                    loc: Loc(0, 1111, 1116),
                                                    name: "error".to_string(),
                                                })],
                                            ),
                                        )],
                                    },
                                ),
                                CatchClause::Named(
                                    Loc(0, 1141, 1223),
                                    Identifier {
                                        loc: Loc(0, 1147, 1152),
                                        name: "Panic".to_string(),
                                    },
                                    Parameter {
                                        loc: Loc(0, 1153, 1159),
                                        ty: Expression::Type(Loc(0, 1153, 1157), Type::Uint(256)),
                                        storage: None,
                                        name: Some(Identifier {
                                            loc: Loc(0, 1158, 1159),
                                            name: "x".to_string(),
                                        }),
                                    },
                                    Statement::Block {
                                        loc: Loc(0, 1161, 1223),
                                        unchecked: false,
                                        statements: vec![Statement::Expression(
                                            Loc(0, 1187, 1200),
                                            Expression::FunctionCall(
                                                Loc(0, 1187, 1200),
                                                Box::new(Expression::Variable(Identifier {
                                                    loc: Loc(0, 1187, 1193),
                                                    name: "revert".to_string(),
                                                })),
                                                vec![Expression::StringLiteral(vec![
                                                    StringLiteral {
                                                        loc: Loc(0, 1194, 1199),
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
                            loc: Loc(0, 1245, 1846),
                            assembly: vec![
                                AssemblyStatement::LetAssign(
                                    Loc(0, 1280, 1290),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 1284, 1285),
                                        name: "x".to_string(),
                                    }),
                                    AssemblyExpression::NumberLiteral(Loc(0, 1289, 1290), 0.into()),
                                ),
                                AssemblyStatement::For(
                                    Loc(0, 1315, 1568),
                                    vec![AssemblyStatement::LetAssign(
                                        Loc(0, 1321, 1331),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1325, 1326),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::NumberLiteral(
                                            Loc(0, 1330, 1331),
                                            0.into(),
                                        ),
                                    )],
                                    AssemblyExpression::Function(
                                        Loc(0, 1334, 1346),
                                        Box::new(AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1334, 1336),
                                            name: "lt".to_string(),
                                        })),
                                        vec![
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1337, 1338),
                                                name: "i".to_string(),
                                            }),
                                            AssemblyExpression::HexNumberLiteral(
                                                Loc(0, 1340, 1345),
                                                "0x100".to_string(),
                                            ),
                                        ],
                                    ),
                                    vec![AssemblyStatement::Assign(
                                        Loc(0, 1349, 1366),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 1349, 1350),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::Function(
                                            Loc(0, 1354, 1366),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1354, 1357),
                                                name: "add".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc(0, 1358, 1359),
                                                    name: "i".to_string(),
                                                }),
                                                AssemblyExpression::HexNumberLiteral(
                                                    Loc(0, 1361, 1365),
                                                    "0x20".to_string(),
                                                ),
                                            ],
                                        ),
                                    )],
                                    Box::new(vec![
                                        AssemblyStatement::Assign(
                                            Loc(0, 1399, 1420),
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1399, 1400),
                                                name: "x".to_string(),
                                            }),
                                            AssemblyExpression::Function(
                                                Loc(0, 1404, 1420),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1404, 1407),
                                                        name: "add".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc(0, 1408, 1409),
                                                        name: "x".to_string(),
                                                    }),
                                                    AssemblyExpression::Function(
                                                        Loc(0, 1411, 1419),
                                                        Box::new(AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc(0, 1411, 1416),
                                                                name: "mload".to_string(),
                                                            },
                                                        )),
                                                        vec![AssemblyExpression::Variable(
                                                            Identifier {
                                                                loc: Loc(0, 1417, 1418),
                                                                name: "i".to_string(),
                                                            },
                                                        )],
                                                    ),
                                                ],
                                            ),
                                        ),
                                        AssemblyStatement::If(
                                            Loc(0, 1458, 1542),
                                            AssemblyExpression::Function(
                                                Loc(0, 1461, 1472),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1461, 1463),
                                                        name: "gt".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::Variable(Identifier {
                                                        loc: Loc(0, 1464, 1465),
                                                        name: "i".to_string(),
                                                    }),
                                                    AssemblyExpression::HexNumberLiteral(
                                                        Loc(0, 1467, 1471),
                                                        "0x10".to_string(),
                                                    ),
                                                ],
                                            ),
                                            Box::new(vec![AssemblyStatement::Break(Loc(
                                                0, 1507, 1512,
                                            ))]),
                                        ),
                                    ]),
                                ),
                                AssemblyStatement::Switch(
                                    Loc(0, 1622, 1824),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 1629, 1630),
                                        name: "x".to_string(),
                                    }),
                                    vec![AssemblySwitch::Case(
                                        AssemblyExpression::NumberLiteral(
                                            Loc(0, 1660, 1661),
                                            0.into(),
                                        ),
                                        Box::new(vec![AssemblyStatement::Expression(
                                            AssemblyExpression::Function(
                                                Loc(0, 1692, 1704),
                                                Box::new(AssemblyExpression::Variable(
                                                    Identifier {
                                                        loc: Loc(0, 1692, 1698),
                                                        name: "revert".to_string(),
                                                    },
                                                )),
                                                vec![
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc(0, 1699, 1700),
                                                        0.into(),
                                                    ),
                                                    AssemblyExpression::NumberLiteral(
                                                        Loc(0, 1702, 1703),
                                                        0.into(),
                                                    ),
                                                ],
                                            ),
                                        )]),
                                    )],
                                    Some(AssemblySwitch::Default(Box::new(vec![
                                        AssemblyStatement::Leave(Loc(0, 1793, 1798)),
                                    ]))),
                                ),
                            ],
                        },
                    ],
                }),
            })),
        ]);

        assert_eq!(actual_parse_tree, expected_parse_tree);
    }
}
