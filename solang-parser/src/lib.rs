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
                }";

        let lex = lexer::Lexer::new(src);

        let actual_parse_tree = solidity::SourceUnitParser::new()
            .parse(src, 0, lex)
            .unwrap();

        let expected_parse_tree = SourceUnit(vec![SourceUnitPart::ContractDefinition(Box::new(
            ContractDefinition {
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
                            value: "Foo Bar".to_string(),
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
                                        value: "Data for jurisdiction".to_string(),
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
            },
        ))]);

        assert_eq!(actual_parse_tree, expected_parse_tree);
    }
}
