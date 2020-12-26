mod doc;
pub mod lexer;
pub mod pt;

#[allow(clippy::all)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub mod solidity;

use crate::sema::ast::Diagnostic;
use lalrpop_util::ParseError;

pub fn parse(src: &str, file_no: usize) -> Result<pt::SourceUnit, Vec<Diagnostic>> {
    // parse phase
    let lex = lexer::Lexer::new(src);

    let s = solidity::SourceUnitParser::new().parse(src, file_no, lex);

    let mut errors = Vec::new();

    if let Err(e) = s {
        errors.push(match e {
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
        });

        Err(errors)
    } else {
        Ok(s.unwrap())
    }
}

pub fn box_option<T>(o: Option<T>) -> Option<Box<T>> {
    match o {
        None => None,
        Some(x) => Some(Box::new(x)),
    }
}

#[cfg(test)]
mod test {
    use super::lexer;
    use super::pt::*;
    use super::solidity;

    #[test]
    fn parse_test() {
        let src = "contract foo {
                    struct Jurisdiction {
                        bool exists;
                        uint keyIdx;
                        bytes2 country;
                        bytes32 region;
                    }
                    string __abba_$;
                    int64 $thing_102;
                }";

        let lex = lexer::Lexer::new(&src);

        let e = solidity::SourceUnitParser::new()
            .parse(&src, 0, lex)
            .unwrap();

        let a = SourceUnit(vec![SourceUnitPart::ContractDefinition(Box::new(
            ContractDefinition {
                doc: vec![],
                loc: Loc(0, 0, 13),
                ty: ContractTy::Contract(Loc(0, 0, 8)),
                name: Identifier {
                    loc: Loc(0, 9, 12),
                    name: "foo".to_string(),
                },
                base: Vec::new(),
                parts: vec![
                    ContractPart::StructDefinition(Box::new(StructDefinition {
                        doc: vec![],
                        name: Identifier {
                            loc: Loc(0, 42, 54),
                            name: "Jurisdiction".to_string(),
                        },
                        loc: Loc(0, 35, 232),
                        fields: vec![
                            VariableDeclaration {
                                loc: Loc(0, 81, 92),
                                ty: Expression::Type(Loc(0, 81, 85), Type::Bool),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 86, 92),
                                    name: "exists".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 118, 129),
                                ty: Expression::Type(Loc(0, 118, 122), Type::Uint(256)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 123, 129),
                                    name: "keyIdx".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 155, 169),
                                ty: Expression::Type(Loc(0, 155, 161), Type::Bytes(2)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 162, 169),
                                    name: "country".to_string(),
                                },
                            },
                            VariableDeclaration {
                                loc: Loc(0, 195, 209),
                                ty: Expression::Type(Loc(0, 195, 202), Type::Bytes(32)),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(0, 203, 209),
                                    name: "region".to_string(),
                                },
                            },
                        ],
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 253, 259), Type::String),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 260, 268),
                            name: "__abba_$".to_string(),
                        },
                        loc: Loc(0, 253, 268),
                        initializer: None,
                    })),
                    ContractPart::VariableDefinition(Box::new(VariableDefinition {
                        doc: vec![],
                        ty: Expression::Type(Loc(0, 290, 295), Type::Int(64)),
                        attrs: vec![],
                        name: Identifier {
                            loc: Loc(0, 296, 306),
                            name: "$thing_102".to_string(),
                        },
                        loc: Loc(0, 290, 306),
                        initializer: None,
                    })),
                ],
            },
        ))]);

        assert_eq!(e, a);
    }
}
