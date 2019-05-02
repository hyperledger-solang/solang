
use output::Output;
use solidity;
use ast;
use lalrpop_util::ParseError;

pub fn parse(src: &str) -> Result<ast::SourceUnit, Vec<Output>> {
    // parse phase
    let nocomments = strip_comments(src);

    let s = solidity::SourceUnitParser::new()
        .parse(&nocomments);

    let mut errors = Vec::new();

    if let Err(e) = s {
        errors.push(match e {
            ParseError::InvalidToken{location} => Output::parser_error(ast::Loc(location, location), "invalid token".to_string()),
            ParseError::UnrecognizedToken{token, expected} => {
                match token {
                    None => Output::parser_error(ast::Loc(0, 0), format!("unrecognised token, expected `{}'", expected.join(","))),
                    Some(t) => Output::parser_error(ast::Loc(t.0, t.2), format!("unrecognised token `{}'", t.1)),
                }
            },
            ParseError::User{error} => {
                Output::parser_error(ast::Loc(0, 0), error.to_string())
            },
            ParseError::ExtraToken{token} => {
                Output::parser_error(ast::Loc(token.0, token.2), format!("extra token `{}' encountered", token.0))
            }
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
        } else if last == '/' && c == '/'  {
            single_line = true;
            last = ' ';
        } else if last == '/' && c == '*'  {
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

#[test]
fn strip_comments_test() {
    assert_eq!(strip_comments(&("foo //Zabc\nbar".to_string())),
                              "foo       \nbar");
    assert_eq!(strip_comments(&("foo /*|x\ny&*/ bar".to_string())),
                              "foo     \n     bar");
}

