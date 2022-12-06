// SPDX-License-Identifier: Apache-2.0

//! Solidity file parser
use crate::lexer::LexicalError;
use crate::lexer::Token;
use crate::pt::CodeLocation;
use crate::pt::Loc;
use diagnostics::Diagnostic;
use lalrpop_util::ParseError;

pub mod diagnostics;
pub mod doccomment;
pub mod lexer;
pub mod pt;
#[cfg(test)]
mod test;

#[allow(clippy::all)]
mod solidity {
    include!(concat!(env!("OUT_DIR"), "/solidity.rs"));
}

/// Parse solidity file
pub fn parse(
    src: &str,
    file_no: usize,
) -> Result<(pt::SourceUnit, Vec<pt::Comment>), Vec<Diagnostic>> {
    // parse phase
    let mut comments = Vec::new();
    let mut lexer_errors = Vec::new();
    let mut lex = lexer::Lexer::new(src, file_no, &mut comments, &mut lexer_errors);

    let parser_errors = &mut Vec::new();
    let diagnostics = &mut Vec::new();

    let s = solidity::SourceUnitParser::new().parse(src, file_no, parser_errors, &mut lex);

    for lexical_error in lex.errors {
        diagnostics.push(Diagnostic::parser_error(
            lexical_error.loc(),
            lexical_error.to_string(),
        ))
    }

    for e in parser_errors {
        diagnostics.push(parser_error_to_diagnostic(&e.error, file_no));
    }

    if let Err(e) = s {
        diagnostics.push(parser_error_to_diagnostic(&e, file_no));
        return Err(diagnostics.to_vec());
    }

    if !diagnostics.is_empty() {
        Err(diagnostics.to_vec())
    } else {
        Ok((s.unwrap(), comments))
    }
}

/// Convert lalrop parser error to a Diagnostic
fn parser_error_to_diagnostic(
    error: &ParseError<usize, Token, LexicalError>,
    file_no: usize,
) -> Diagnostic {
    match &error {
        ParseError::InvalidToken { location } => Diagnostic::parser_error(
            Loc::File(file_no, *location, *location),
            "invalid token".to_string(),
        ),
        ParseError::UnrecognizedToken {
            token: (l, token, r),
            expected,
        } => Diagnostic::parser_error(
            Loc::File(file_no, *l, *r),
            format!(
                "unrecognised token '{}', expected {}",
                token,
                expected.join(", ")
            ),
        ),
        ParseError::User { error } => Diagnostic::parser_error(error.loc(), error.to_string()),
        ParseError::ExtraToken { token } => Diagnostic::parser_error(
            Loc::File(file_no, token.0, token.2),
            format!("extra token '{}' encountered", token.0),
        ),
        ParseError::UnrecognizedEOF { expected, location } => Diagnostic::parser_error(
            Loc::File(file_no, *location, *location),
            format!("unexpected end of file, expecting {}", expected.join(", ")),
        ),
    }
}
