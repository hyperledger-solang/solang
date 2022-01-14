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
        let src = "/// @title Foo
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
                loc: Loc(0, 736, 751),
                ty: FunctionTy::Function,
                name: Some(Identifier {
                    loc: Loc(0, 745, 748),
                    name: "bar".to_string(),
                }),
                name_loc: Loc(0, 745, 748),
                params: vec![],
                attributes: vec![],
                return_not_returns: None,
                returns: vec![],
                body: Some(Statement::Block {
                    loc: Loc(0, 751, 1392),
                    unchecked: false,
                    statements: vec![Statement::Assembly {
                        loc: Loc(0, 773, 1374),
                        assembly: vec![
                            AssemblyStatement::LetAssign(
                                Loc(0, 808, 818),
                                AssemblyExpression::Variable(Identifier {
                                    loc: Loc(0, 812, 813),
                                    name: "x".to_string(),
                                }),
                                AssemblyExpression::NumberLiteral(Loc(0, 817, 818), 0.into()),
                            ),
                            AssemblyStatement::For(
                                Loc(0, 843, 1096),
                                vec![AssemblyStatement::LetAssign(
                                    Loc(0, 849, 859),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 853, 854),
                                        name: "i".to_string(),
                                    }),
                                    AssemblyExpression::NumberLiteral(Loc(0, 858, 859), 0.into()),
                                )],
                                AssemblyExpression::Function(
                                    Loc(0, 862, 874),
                                    Box::new(AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 862, 864),
                                        name: "lt".to_string(),
                                    })),
                                    vec![
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 865, 866),
                                            name: "i".to_string(),
                                        }),
                                        AssemblyExpression::HexNumberLiteral(
                                            Loc(0, 868, 873),
                                            "0x100".to_string(),
                                        ),
                                    ],
                                ),
                                vec![AssemblyStatement::Assign(
                                    Loc(0, 877, 894),
                                    AssemblyExpression::Variable(Identifier {
                                        loc: Loc(0, 877, 878),
                                        name: "i".to_string(),
                                    }),
                                    AssemblyExpression::Function(
                                        Loc(0, 882, 894),
                                        Box::new(AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 882, 885),
                                            name: "add".to_string(),
                                        })),
                                        vec![
                                            AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 886, 887),
                                                name: "i".to_string(),
                                            }),
                                            AssemblyExpression::HexNumberLiteral(
                                                Loc(0, 889, 893),
                                                "0x20".to_string(),
                                            ),
                                        ],
                                    ),
                                )],
                                Box::new(vec![
                                    AssemblyStatement::Assign(
                                        Loc(0, 927, 948),
                                        AssemblyExpression::Variable(Identifier {
                                            loc: Loc(0, 927, 928),
                                            name: "x".to_string(),
                                        }),
                                        AssemblyExpression::Function(
                                            Loc(0, 932, 948),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 932, 935),
                                                name: "add".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc(0, 936, 937),
                                                    name: "x".to_string(),
                                                }),
                                                AssemblyExpression::Function(
                                                    Loc(0, 939, 947),
                                                    Box::new(AssemblyExpression::Variable(
                                                        Identifier {
                                                            loc: Loc(0, 939, 944),
                                                            name: "mload".to_string(),
                                                        },
                                                    )),
                                                    vec![AssemblyExpression::Variable(
                                                        Identifier {
                                                            loc: Loc(0, 945, 946),
                                                            name: "i".to_string(),
                                                        },
                                                    )],
                                                ),
                                            ],
                                        ),
                                    ),
                                    AssemblyStatement::If(
                                        Loc(0, 986, 1070),
                                        AssemblyExpression::Function(
                                            Loc(0, 989, 1000),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 989, 991),
                                                name: "gt".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::Variable(Identifier {
                                                    loc: Loc(0, 992, 993),
                                                    name: "i".to_string(),
                                                }),
                                                AssemblyExpression::HexNumberLiteral(
                                                    Loc(0, 995, 999),
                                                    "0x10".to_string(),
                                                ),
                                            ],
                                        ),
                                        Box::new(vec![AssemblyStatement::Break(Loc(
                                            0, 1035, 1040,
                                        ))]),
                                    ),
                                ]),
                            ),
                            AssemblyStatement::Switch(
                                Loc(0, 1150, 1352),
                                AssemblyExpression::Variable(Identifier {
                                    loc: Loc(0, 1157, 1158),
                                    name: "x".to_string(),
                                }),
                                vec![AssemblySwitch::Case(
                                    AssemblyExpression::NumberLiteral(Loc(0, 1188, 1189), 0.into()),
                                    Box::new(vec![AssemblyStatement::Expression(
                                        AssemblyExpression::Function(
                                            Loc(0, 1220, 1232),
                                            Box::new(AssemblyExpression::Variable(Identifier {
                                                loc: Loc(0, 1220, 1226),
                                                name: "revert".to_string(),
                                            })),
                                            vec![
                                                AssemblyExpression::NumberLiteral(
                                                    Loc(0, 1227, 1228),
                                                    0.into(),
                                                ),
                                                AssemblyExpression::NumberLiteral(
                                                    Loc(0, 1230, 1231),
                                                    0.into(),
                                                ),
                                            ],
                                        ),
                                    )]),
                                )],
                                Some(AssemblySwitch::Default(Box::new(vec![
                                    AssemblyStatement::Leave(Loc(0, 1321, 1326)),
                                ]))),
                            ),
                        ],
                    }],
                }),
            })),
        ]);

        assert_eq!(actual_parse_tree, expected_parse_tree);
    }
}
