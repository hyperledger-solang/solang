pub mod ast;
pub mod solidity;

use lalrpop_util::ParseError;
use output::Output;
use tiny_keccak::keccak256;

/// Returns true if hex number confirms to https://github.com/ethereum/EIPs/blob/master/EIPS/eip-55.md
pub fn is_hexstr_eip55(src: &str) -> bool {
    if !src.starts_with("0x") || src.len() != 42 {
        return false;
    }

    let address : String = src.chars().skip(2).map(|c| c.to_ascii_lowercase()).collect();

    let hash = keccak256(address.as_bytes());

    for (i, c) in src.chars().skip(2).enumerate() {
        let is_upper = match c {
            '0'..='9' => continue,
            'a'..='f' => false,
            'A'..='F' => true,
            _ => unreachable!()
        };

        // hash is 32 bytes; find the i'th "nibble"
        let nibble = hash[i >> 1] >> if (i & 1) != 0 {
            0
        } else {
            4
        };

        if ((nibble & 8) != 0) != is_upper {
            return false;
        }
    }

    true
}

pub fn parse(src: &str) -> Result<ast::SourceUnit, Vec<Output>> {
    // parse phase
    let nocomments = strip_comments(src);

    let s = solidity::SourceUnitParser::new().parse(&nocomments);

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
                    token.1,
                    expected.join(", ")
                ),
            ),
            ParseError::User { error } => Output::parser_error(ast::Loc(0, 0), error.to_string()),
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

//
// The lalrpop lexer cannot deal with comments, so you have to write your own lexer.
// Rather than do that let's just strip the comments before passing it to the lexer
// It's not great code but it's a stop-gap solution anyway
fn strip_comments(s: &str) -> String {
    let mut n = String::new();
    let mut single_line = false;
    let mut multi_line = false;
    let mut last = '\0';
    let mut c = '\0';

    for (i, j) in s.char_indices() {
        c = j;
        if single_line {
            if c == '\n' {
                single_line = false;
            }
            last = ' ';
        } else if multi_line {
            if last == '*' && c == '/' {
                c = ' ';
                multi_line = false;
            }
            if last != '\n' {
                last = ' ';
            }
        } else if last == '/' && c == '/' {
            single_line = true;
            last = ' ';
        } else if last == '/' && c == '*' {
            multi_line = true;
            last = ' ';
        }

        if i > 0 {
            n.push(last);
        }
        last = c;
    }

    if !single_line && !multi_line {
        n.push(c);
    }

    n
}

pub fn box_option<T>(o: Option<T>) -> Option<Box<T>> {
    match o {
        None => None,
        Some(x) => Some(Box::new(x)),
    }
}

#[test]
fn strip_comments_test() {
    assert_eq!(
        strip_comments(&("foo //Zabc\nbar".to_string())),
        "foo       \nbar"
    );
    assert_eq!(
        strip_comments(&("foo /*|x\ny&*/ bar".to_string())),
        "foo     \n     bar"
    );
}

#[test]
fn test_is_hexstr_eip55() {
    assert!(is_hexstr_eip55("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"));
    assert!(is_hexstr_eip55("0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359"));
    assert!(is_hexstr_eip55("0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB"));
    assert!(is_hexstr_eip55("0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb"));
}