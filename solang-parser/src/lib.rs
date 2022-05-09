//! Solidity file parser

use crate::pt::CodeLocation;
use lalrpop_util::ParseError;

pub use diagnostics::Diagnostic;

pub mod diagnostics;
pub mod lexer;
pub mod pt;
#[cfg(test)]
mod test;

#[allow(clippy::all)]
pub mod solidity {
    include!(concat!(env!("OUT_DIR"), "/solidity.rs"));
}

/// Parse solidity file content
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
                    "unrecognised token '{}', expected {}",
                    token,
                    expected.join(", ")
                ),
            ),
            ParseError::User { error } => Diagnostic::parser_error(error.loc(), error.to_string()),
            ParseError::ExtraToken { token } => Diagnostic::parser_error(
                pt::Loc::File(file_no, token.0, token.2),
                format!("extra token '{}' encountered", token.0),
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
