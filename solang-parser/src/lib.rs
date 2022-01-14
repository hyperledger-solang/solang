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
                    try sum(1, 1) returns (uint sum) {
                        assert(sum == 2);
                    } catch (bytes memory b) {
                        revert('meh');
                    } catch Error(string memory error) {
                        revert(error);
                    } catch Panic(uint x) {
                        revert('feh');
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
                    loc: Loc(0, 751, 1154),
                    unchecked: false,
                    statements: vec![Statement::Try(
                        Loc(0, 773, 1136),
                        Expression::FunctionCall(
                            Loc(0, 777, 786),
                            Box::new(Expression::Variable(Identifier {
                                loc: Loc(0, 777, 780),
                                name: "sum".to_string(),
                            })),
                            vec![
                                Expression::NumberLiteral(Loc(0, 781, 782), 1.into()),
                                Expression::NumberLiteral(Loc(0, 784, 785), 1.into()),
                            ],
                        ),
                        Some((
                            vec![(
                                Loc(0, 796, 804),
                                Some(Parameter {
                                    loc: Loc(0, 796, 804),
                                    ty: Expression::Type(Loc(0, 796, 800), Type::Uint(256)),
                                    storage: None,
                                    name: Some(Identifier {
                                        loc: Loc(0, 801, 804),
                                        name: "sum".to_string(),
                                    }),
                                }),
                            )],
                            Box::new(Statement::Block {
                                loc: Loc(0, 806, 871),
                                unchecked: false,
                                statements: vec![Statement::Expression(
                                    Loc(0, 832, 848),
                                    Expression::FunctionCall(
                                        Loc(0, 832, 848),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc(0, 832, 838),
                                            name: "assert".to_string(),
                                        })),
                                        vec![Expression::Equal(
                                            Loc(0, 843, 845),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 839, 842),
                                                name: "sum".to_string(),
                                            })),
                                            Box::new(Expression::NumberLiteral(
                                                Loc(0, 846, 847),
                                                2.into(),
                                            )),
                                        )],
                                    ),
                                )],
                            }),
                        )),
                        vec![
                            CatchClause::Simple(
                                Loc(0, 872, 957),
                                Some(Parameter {
                                    loc: Loc(0, 879, 893),
                                    ty: Expression::Type(Loc(0, 879, 884), Type::DynamicBytes),
                                    storage: Some(StorageLocation::Memory(Loc(0, 885, 891))),
                                    name: Some(Identifier {
                                        loc: Loc(0, 892, 893),
                                        name: "b".to_string(),
                                    }),
                                }),
                                Statement::Block {
                                    loc: Loc(0, 895, 957),
                                    unchecked: false,
                                    statements: vec![Statement::Expression(
                                        Loc(0, 921, 934),
                                        Expression::FunctionCall(
                                            Loc(0, 921, 934),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 921, 927),
                                                name: "revert".to_string(),
                                            })),
                                            vec![Expression::StringLiteral(vec![StringLiteral {
                                                loc: Loc(0, 928, 933),
                                                string: "meh".to_string(),
                                            }])],
                                        ),
                                    )],
                                },
                            ),
                            CatchClause::Named(
                                Loc(0, 958, 1053),
                                Identifier {
                                    loc: Loc(0, 964, 969),
                                    name: "Error".to_string(),
                                },
                                Parameter {
                                    loc: Loc(0, 970, 989),
                                    ty: Expression::Type(Loc(0, 970, 976), Type::String),
                                    storage: Some(StorageLocation::Memory(Loc(0, 977, 983))),
                                    name: Some(Identifier {
                                        loc: Loc(0, 984, 989),
                                        name: "error".to_string(),
                                    }),
                                },
                                Statement::Block {
                                    loc: Loc(0, 991, 1053),
                                    unchecked: false,
                                    statements: vec![Statement::Expression(
                                        Loc(0, 1017, 1030),
                                        Expression::FunctionCall(
                                            Loc(0, 1017, 1030),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 1017, 1023),
                                                name: "revert".to_string(),
                                            })),
                                            vec![Expression::Variable(Identifier {
                                                loc: Loc(0, 1024, 1029),
                                                name: "error".to_string(),
                                            })],
                                        ),
                                    )],
                                },
                            ),
                            CatchClause::Named(
                                Loc(0, 1054, 1136),
                                Identifier {
                                    loc: Loc(0, 1060, 1065),
                                    name: "Panic".to_string(),
                                },
                                Parameter {
                                    loc: Loc(0, 1066, 1072),
                                    ty: Expression::Type(Loc(0, 1066, 1070), Type::Uint(256)),
                                    storage: None,
                                    name: Some(Identifier {
                                        loc: Loc(0, 1071, 1072),
                                        name: "x".to_string(),
                                    }),
                                },
                                Statement::Block {
                                    loc: Loc(0, 1074, 1136),
                                    unchecked: false,
                                    statements: vec![Statement::Expression(
                                        Loc(0, 1100, 1113),
                                        Expression::FunctionCall(
                                            Loc(0, 1100, 1113),
                                            Box::new(Expression::Variable(Identifier {
                                                loc: Loc(0, 1100, 1106),
                                                name: "revert".to_string(),
                                            })),
                                            vec![Expression::StringLiteral(vec![StringLiteral {
                                                loc: Loc(0, 1107, 1112),
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
}
