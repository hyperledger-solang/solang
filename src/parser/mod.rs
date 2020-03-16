pub mod ast;
pub mod lexer;

#[allow(clippy::all,unused_parens)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub mod solidity;

use lalrpop_util::ParseError;
use output::Output;

pub fn parse(src: &str) -> Result<ast::SourceUnit, Vec<Output>> {
    // parse phase
    let lex = lexer::Lexer::new(src);

    let s = solidity::SourceUnitParser::new().parse(src, lex);

    let mut errors = Vec::new();

    if let Err(e) = s {
        errors.push(match e {
            ParseError::InvalidToken { location } => {
                Output::parser_error(ast::Loc(location, location), "invalid token".to_string())
            }
            ParseError::UnrecognizedToken {
                token: (l, token, r),
                expected,
            } => Output::parser_error(
                ast::Loc(l, r),
                format!(
                    "unrecognised token `{}', expected {}",
                    token,
                    expected.join(", ")
                ),
            ),
            ParseError::User { error } => Output::parser_error(error.loc(), error.to_string()),
            ParseError::ExtraToken { token } => Output::parser_error(
                ast::Loc(token.0, token.2),
                format!("extra token `{}' encountered", token.0),
            ),
            ParseError::UnrecognizedEOF { location, expected } => Output::parser_error(
                ast::Loc(location, location),
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
    use parser::ast::*;
    use parser::lexer;
    use parser::solidity;

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

        let e = solidity::SourceUnitParser::new().parse(&src, lex).unwrap();

        let a = SourceUnit(vec![SourceUnitPart::ContractDefinition(Box::new(
            ContractDefinition {
                doc: vec![],
                loc: Loc(0, 325),
                ty: ContractType::Contract,
                name: Identifier {
                    loc: Loc(9, 12),
                    name: "foo".to_string(),
                },
                parts: vec![
                    ContractPart::StructDefinition(Box::new(StructDefinition {
                        doc: vec![],
                        name: Identifier {
                            loc: Loc(42, 54),
                            name: "Jurisdiction".to_string(),
                        },
                        fields: vec![
                            VariableDeclaration {
                                ty: ComplexType::Primitive(Type::Bool, Vec::new()),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(86, 92),
                                    name: "exists".to_string(),
                                },
                            },
                            VariableDeclaration {
                                ty: ComplexType::Primitive(Type::Uint(256), Vec::new()),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(123, 129),
                                    name: "keyIdx".to_string(),
                                },
                            },
                            VariableDeclaration {
                                ty: ComplexType::Primitive(Type::Bytes(2), Vec::new()),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(162, 169),
                                    name: "country".to_string(),
                                },
                            },
                            VariableDeclaration {
                                ty: ComplexType::Primitive(Type::Bytes(32), Vec::new()),
                                storage: None,
                                name: Identifier {
                                    loc: Loc(203, 209),
                                    name: "region".to_string(),
                                },
                            },
                        ],
                    })),
                    ContractPart::ContractVariableDefinition(Box::new(
                        ContractVariableDefinition {
                            doc: vec![],
                            ty: ComplexType::Primitive(Type::String, Vec::new()),
                            attrs: vec![],
                            name: Identifier {
                                loc: Loc(260, 268),
                                name: "__abba_$".to_string(),
                            },
                            loc: Loc(253, 268),
                            initializer: None,
                        },
                    )),
                    ContractPart::ContractVariableDefinition(Box::new(
                        ContractVariableDefinition {
                            doc: vec![],
                            ty: ComplexType::Primitive(Type::Int(64), Vec::new()),
                            attrs: vec![],
                            name: Identifier {
                                loc: Loc(296, 306),
                                name: "$thing_102".to_string(),
                            },
                            loc: Loc(290, 306),
                            initializer: None,
                        },
                    )),
                ],
            },
        ))]);

        assert_eq!(e, a);
    }
}
