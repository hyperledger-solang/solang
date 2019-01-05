#![feature(box_patterns)]

extern crate lalrpop;
extern crate num_bigint;
extern crate lalrpop_util;
extern crate llvm_sys;
extern crate num_traits;

mod ast;
mod solidity;
mod resolve;
mod emit;
mod vartable;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use lalrpop_util::ParseError;

fn main() {
    for filename in env::args().skip(1) {
        let mut f = File::open(&filename).expect("file not found");

        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");


        // parse phase
        let nocomments = strip_comments(&contents);

        let s = solidity::SourceUnitParser::new()
            .parse(&nocomments);

        let mut past;

        match s {
            Ok(s) => past = s,
            Err(e) => {
                match e {
                    ParseError::InvalidToken{location} => println!("{}: error: invalid token token at {}", filename, offset_to_line_column(&contents, location)),
                    ParseError::UnrecognizedToken{token, expected} => {
                        match token {
                            None => println!("{}: error: unrecognised token, expected {}", filename, expected.join(",")),
                            Some(t) => println!("{}: error: unrecognised token `{}' from {} to {}", filename, t.1, offset_to_line_column(&contents, t.0), offset_to_line_column(&contents, t.2)),
                        }
                    },
                    ParseError::User{error} => {
                        println!("{}: error: {}", filename, error)
                    },
                    ParseError::ExtraToken{token} => {
                        println!("{}: extra token `{}' encountered at {}-{}", filename, token.1, token.0, token.2)
                    }
                }
                return;
            }
        }

        past.name = filename.clone();

        // resolve phase
        if let Err(s) = resolve::resolve(&mut past) {
            println!("{}: {}", filename, s);
            break;
        }

        // emit phase
        emit::emit(past);
    }
}

fn offset_to_line_column(s: &String, offset: usize) -> String {
    let mut line = 1;
    let mut column = 1;

    for (o, c) in s.char_indices() {
        if o == offset {
            break;
        }
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    format!("{}:{}", line, column)
}

//
// The lalrpop lexer cannot deal with comments, so you have to write your own lexer.
// Rather than do that let's just strip the comments before passing it to the lexer
// It's not great code but it's a stop-gap solution anyway
fn strip_comments(s: &String) -> String {
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

