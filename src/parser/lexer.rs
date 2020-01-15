//
// Solidity custom lexer. Solidity needs a custom lexer for two reasons:
//  - comments and doc comments
//  - pragma value is [^;]+
//
use std::iter::Peekable;
use std::str::CharIndices;
use std::collections::HashMap;
use std::fmt;

use super::ast::Loc;

pub type Spanned<Token, Loc, Error> = Result<(Loc, Token, Loc), Error>;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommentType {
    Line,
    Block
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Token<'input> {
    Identifier(&'input str),
    StringLiteral(&'input str),
    HexLiteral(&'input str),
    Number(&'input str),
    HexNumber(&'input str),
    DocComment(CommentType, &'input str),
    Divide,
    Contract,
    Library,
    Interface,
    Function,
    Pragma,
    Import,

    Struct,
    Event,
    Enum,

    Memory,
    Storage,
    Calldata,

    Public,
    Private,
    Internal,
    External,

    Constant,

    New,
    Delete,

    Pure,
    View,
    Payable,

    Do,
    Continue,
    Break,

    Throw,
    Emit,
    Return,
    Returns,

    Uint(u16),
    Int(u16),
    Bytes(u8),
    Bool,
    Address,
    String,

    Semicolon,
    Comma,
    OpenParenthesis,
    CloseParenthesis,
    OpenCurlyBrace,
    CloseCurlyBrace,

    BitwiseOr,
    BitwiseOrAssign,
    Or,

    BitwiseXor,
    BitwiseXorAssign,

    BitwiseAnd,
    BitwiseAndAssign,
    And,

    AddAssign,
    Increment,
    Add,

    SubtractAssign,
    Decrement,
    Subtract,

    MulAssign,
    Mul,
    Power,
    DivideAssign,
    ModuloAssign,
    Modulo,

    Equal,
    Assign,

    NotEqual,
    Not,

    True,
    False,
    Else,
    Anonymous,
    For,
    While,
    If,

    ShiftRight,
    ShiftRightAssign,
    Less,
    LessEqual,

    ShiftLeft,
    ShiftLeftAssign,
    More,
    MoreEqual,

    Constructor,
    Indexed,

    Member,
    Colon,
    OpenBracket,
    CloseBracket,
    Underscore,
    Complement,
    Question,
}

impl<'input> fmt::Display for Token<'input> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::DocComment(CommentType::Line, s) => write!(f, "///{}", s),
            Token::DocComment(CommentType::Block, s) => write!(f, "/**{}\n*/", s),
            Token::Identifier(id) => write!(f, "{}", id),
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::HexLiteral(hex) => write!(f, "{}", hex),
            Token::Number(n) => write!(f, "{}", n),
            Token::HexNumber(n) => write!(f, "{}", n),
            Token::Uint(w) => write!(f, "uint{}", w),
            Token::Int(w) => write!(f, "int{}", w),
            Token::Bytes(w) => write!(f, "bytes{}", w),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::OpenParenthesis => write!(f, "("),
            Token::CloseParenthesis => write!(f, ")"),
            Token::OpenCurlyBrace => write!(f, "{{"),
            Token::CloseCurlyBrace => write!(f, "}}"),
            Token::BitwiseOr => write!(f, "|"),
            Token::BitwiseOrAssign => write!(f, "|="),
            Token::Or => write!(f, "||"),
            Token::BitwiseXor => write!(f, "^"),
            Token::BitwiseXorAssign => write!(f, "^="),
            Token::BitwiseAnd => write!(f, "&"),
            Token::BitwiseAndAssign => write!(f, "&="),
            Token::And => write!(f, "&&"),
            Token::AddAssign => write!(f, "+="),
            Token::Increment => write!(f, "++"),
            Token::Add => write!(f, "+"),
            Token::SubtractAssign => write!(f, "-="),
            Token::Decrement => write!(f, "--"),
            Token::Subtract => write!(f, "-"),
            Token::MulAssign => write!(f, "*="),
            Token::Mul => write!(f, "*"),
            Token::Power => write!(f, "**"),
            Token::Divide => write!(f, "/"),
            Token::DivideAssign => write!(f, "/="),
            Token::ModuloAssign => write!(f, "%="),
            Token::Modulo => write!(f, "%"),
            Token::Equal => write!(f, "=="),
            Token::Assign => write!(f, "="),
            Token::NotEqual => write!(f, "!="),
            Token::Not => write!(f, "!"),
            Token::ShiftLeft => write!(f, "<<"),
            Token::ShiftLeftAssign => write!(f, "<<="),
            Token::More => write!(f, ">"),
            Token::MoreEqual => write!(f, ">="),
            Token::Member => write!(f, "."),
            Token::Colon => write!(f, ":"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::Underscore => write!(f, "_"),
            Token::Complement => write!(f, "~"),
            Token::Question => write!(f, "?"),
            Token::ShiftRightAssign => write!(f, "<<="),
            Token::ShiftRight => write!(f, "<<"),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Bool => write!(f, "bool"),
            Token::Address => write!(f, "address"),
            Token::String => write!(f, "string"),
            Token::Contract => write!(f, "contract"),
            Token::Library => write!(f, "library"),
            Token::Interface => write!(f, "interface"),
            Token::Function => write!(f, "function"),
            Token::Pragma => write!(f, "pragma"),
            Token::Import => write!(f, "import"),
            Token::Struct => write!(f, "struct"),
            Token::Event => write!(f, "event"),
            Token::Enum => write!(f, "enum"),
            Token::Memory => write!(f, "memory"),
            Token::Storage => write!(f, "storage"),
            Token::Calldata => write!(f, "calldata"),
            Token::Public => write!(f, "public"),
            Token::Private => write!(f, "private"),
            Token::Internal => write!(f, "internal"),
            Token::External => write!(f, "external"),
            Token::Constant => write!(f, "constant"),
            Token::New => write!(f, "new"),
            Token::Delete => write!(f, "delete"),
            Token::Pure => write!(f, "pure"),
            Token::View => write!(f, "view"),
            Token::Payable => write!(f, "payable"),
            Token::Do => write!(f, "do"),
            Token::Continue => write!(f, "continue"),
            Token::Break => write!(f, "break"),
            Token::Throw => write!(f, "throw"),
            Token::Emit => write!(f, "emit"),
            Token::Return => write!(f, "return"),
            Token::Returns => write!(f, "returns"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Else => write!(f, "else"),
            Token::Anonymous => write!(f, "anonymous"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::If => write!(f, "if"),
            Token::Constructor => write!(f, "constructor"),
            Token::Indexed => write!(f, "indexed"),
        }
    }
}

pub struct Lexer<'input> {
    input: &'input str,
    chars: Peekable<CharIndices<'input>>,
    keywords: HashMap<String, Token<'input>>,
    pragma_state: PragmaParserState
}

#[derive(Debug,PartialEq)]
pub enum LexicalError {
    EndOfFileInComment(usize, usize),
    EndOfFileInString(usize, usize),
    EndofFileInHex(usize, usize),
    MissingNumber(usize, usize),
    InvalidCharacterInHexLiteral(usize, char),
    UnrecognisedToken(usize, usize, String),
    PragmaMissingSemiColon(usize, usize),
}

impl LexicalError {
    pub fn to_string(&self) -> String {
        match self {
            LexicalError::EndOfFileInComment(_, _) => "end of file found in comment".to_string(),
            LexicalError::EndOfFileInString(_, _) => "end of file found in string literal".to_string(),
            LexicalError::EndofFileInHex(_, _) => "end of file found in hex literal string".to_string(),
            LexicalError::MissingNumber(_, _) => "missing number".to_string(),
            LexicalError::InvalidCharacterInHexLiteral(_, ch) => format!("invalid character ‘{}’ in hex literal string", ch),
            LexicalError::UnrecognisedToken(_, _, t) => format!("unrecognised token ‘{}’", t),
            LexicalError::PragmaMissingSemiColon(_, _) => "pragma is missing terminating ‘;’".to_string(),
        }
    }

    pub fn loc(&self) -> Loc {
        match self {
            LexicalError::EndOfFileInComment(start, end) => Loc(*start, *end),
            LexicalError::EndOfFileInString(start, end) => Loc(*start, *end),
            LexicalError::EndofFileInHex(start, end) => Loc(*start, *end),
            LexicalError::MissingNumber(start, end) => Loc(*start, *end),
            LexicalError::InvalidCharacterInHexLiteral(pos, _) => Loc(*pos, *pos),
            LexicalError::UnrecognisedToken(start, end, _) => Loc(*start, *end),
            LexicalError::PragmaMissingSemiColon(start, end) => Loc(*start, *end),
        }
    }
}

// Lexer should be aware of whether the last two tokens were
// pragma followed by identifier. If this is true, then special parsing should be
// done for the pragma value
pub enum PragmaParserState {
    NotParsingPragma,
    SeenPragma,
    SeenPragmaIdentifier
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        let mut keywords = HashMap::new();

        for w in 1..=32 {
            keywords.insert(format!("bytes{}", w).to_owned(), Token::Bytes(w));
            let w = w as u16 * 8;
            keywords.insert(format!("uint{}", w).to_owned(), Token::Uint(w));
            keywords.insert(format!("int{}", w).to_owned(), Token::Int(w));
        }

        keywords.insert(String::from("byte"), Token::Bytes(1));
        keywords.insert(String::from("uint"), Token::Uint(256));
        keywords.insert(String::from("int"), Token::Int(256));
        keywords.insert(String::from("bool"), Token::Bool);
        keywords.insert(String::from("address"), Token::Address);
        keywords.insert(String::from("string"), Token::String);

        keywords.insert(String::from("struct"), Token::Struct);
        keywords.insert(String::from("event"), Token::Event);
        keywords.insert(String::from("enum"), Token::Enum);

        keywords.insert(String::from("memory"), Token::Memory);
        keywords.insert(String::from("calldata"), Token::Calldata);
        keywords.insert(String::from("storage"), Token::Storage);

        keywords.insert(String::from("public"), Token::Public);
        keywords.insert(String::from("private"), Token::Private);
        keywords.insert(String::from("external"), Token::External);
        keywords.insert(String::from("internal"), Token::Internal);

        keywords.insert(String::from("constant"), Token::Constant);

        keywords.insert(String::from("pragma"), Token::Pragma);
        keywords.insert(String::from("import"), Token::Import);
        keywords.insert(String::from("contract"), Token::Contract);
        keywords.insert(String::from("interface"), Token::Interface);
        keywords.insert(String::from("library"), Token::Library);
        keywords.insert(String::from("function"), Token::Function);

        keywords.insert(String::from("new"), Token::New);
        keywords.insert(String::from("delete"), Token::Delete);

        keywords.insert(String::from("pure"), Token::Pure);
        keywords.insert(String::from("view"), Token::View);
        keywords.insert(String::from("payable"), Token::Payable);

        keywords.insert(String::from("do"), Token::Do);
        keywords.insert(String::from("continue"), Token::Continue);
        keywords.insert(String::from("break"), Token::Break);

        keywords.insert(String::from("throw"), Token::Throw);
        keywords.insert(String::from("emit"), Token::Emit);
        keywords.insert(String::from("return"), Token::Return);
        keywords.insert(String::from("returns"), Token::Returns);

        keywords.insert(String::from("true"), Token::True);
        keywords.insert(String::from("false"), Token::False);
        keywords.insert(String::from("anonymous"), Token::Anonymous);
        keywords.insert(String::from("constructor"), Token::Constructor);
        keywords.insert(String::from("indexed"), Token::Indexed);
        keywords.insert(String::from("for"), Token::For);
        keywords.insert(String::from("while"), Token::While);
        keywords.insert(String::from("if"), Token::If);
        keywords.insert(String::from("else"), Token::Else);
        keywords.insert(String::from("_"), Token::Underscore);

        Lexer {
            input: input,
            chars: input.char_indices().peekable(),
            keywords: keywords,
            pragma_state: PragmaParserState::NotParsingPragma
        }
    }

    fn parse_number(&mut self, start: usize, end: usize, ch: char) -> Option<Result<(usize, Token<'input>, usize), LexicalError>> {
        if ch == '0' {
            if let Some((_, 'x')) = self.chars.peek() {
                // hex number
                self.chars.next();

                let mut end = match self.chars.next() {
                    Some((end, ch)) if ch.is_ascii_hexdigit() => end,
                    Some((_, _)) => {
                        return Some(Err(LexicalError::MissingNumber(start, start + 1)));
                    },
                    None => {
                        return Some(Err(LexicalError::EndofFileInHex(start, self.input.len())));
                    }
                };

                while let Some((i, ch)) = self.chars.peek() {
                    if !ch.is_ascii_hexdigit() && *ch != '_' {
                        break;
                    }
                    end = *i;
                    self.chars.next();
                }

                return Some(Ok((start, Token::HexNumber(&self.input[start..=end]), end+1)));
            }
        }

        let mut end = end;
        while let Some((i, ch)) = self.chars.peek() {
            if !ch.is_ascii_digit() && *ch != '_' {
                break;
            }
            end = *i;
            self.chars.next();
        }

        return Some(Ok((start, Token::Number(&self.input[start..=end]), end+1)));
    }

    fn next(&mut self) -> Option<Result<(usize, Token<'input>, usize), LexicalError>> {
        loop {
            match self.chars.next() {
                Some((start, ch)) if ch == '_' || ch == '$' || ch.is_alphabetic() => {
                    let mut end = start;

                    while let Some((i, ch)) = self.chars.peek() {
                        if !ch.is_alphanumeric() && *ch != '_' && *ch != '$' {
                            break;
                        }
                        end = *i;
                        self.chars.next();
                    }

                    let id = &self.input[start..=end];

                    if id == "hex" {
                        if let Some((_, '"')) = self.chars.peek() {
                            self.chars.next();

                            while let Some((i, ch)) = self.chars.next() {
                                if ch == '"' {
                                    return Some(Ok((start, Token::HexLiteral(&self.input[start..=i]), i+1)));
                                }

                                if !ch.is_ascii_hexdigit() && ch != '_' {
                                    // Eat up the remainer of the string
                                    while let Some((_, ch)) = self.chars.next() {
                                        if ch == '"' {
                                            break;
                                        }
                                    }

                                    return Some(Err(LexicalError::InvalidCharacterInHexLiteral(i, ch)));
                                }
                            }

                            return Some(Err(LexicalError::EndOfFileInString(start, self.input.len())));
                        }
                    }

                    return if let Some(w) = self.keywords.get(id) {
                        Some(Ok((start, *w, end+1)))
                    } else {
                        Some(Ok((start, Token::Identifier(id), end+1)))
                    };
                },
                Some((start, '"')) => {
                    let mut end;

                    let mut last_was_escape = false;

                    loop {
                        if let Some((i, ch)) = self.chars.next() {
                            end = i;
                            if !last_was_escape {
                                if ch == '"' {
                                    break;
                                }
                                last_was_escape = ch == '\\';
                            } else {
                                last_was_escape = false;
                            }
                        } else {
                            return Some(Err(LexicalError::EndOfFileInString(start, self.input.len())));
                        }
                    }

                    return Some(Ok((start, Token::StringLiteral(&self.input[start+1..=end-1]), end+1)));
                },
                Some((start, '/')) => {
                    match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            return Some(Ok((start, Token::DivideAssign, start+2)));
                        }
                        Some((_, '/')) => {
                            // line comment
                            self.chars.next();

                            let doc_comment_start = match self.chars.peek() {
                                Some((i, '/')) => Some(i + 1),
                                _ => None
                            };

                            let mut last = start + 3;

                            while let Some((i, ch)) = self.chars.next() {
                                if ch == '\n' || ch == '\r' {
                                    break;
                                }
                                last = i;
                            }

                            if let Some(doc_start) = doc_comment_start {
                                if last > doc_start {
                                    return Some(Ok((start + 3,
                                        Token::DocComment(CommentType::Line, &self.input[doc_start..=last]),
                                        last + 1)));
                                }
                            }
                        },
                        Some((_, '*')) => {
                            // multiline comment
                            self.chars.next();

                            let doc_comment_start = match self.chars.peek() {
                                Some((i, '*')) => Some(i + 1),
                                _ => None
                            };

                            let mut last = start + 3;
                            let mut seen_star = false;

                            loop {
                                if let Some((i, ch)) = self.chars.next() {
                                    if seen_star && ch == '/' {
                                        break;
                                    }
                                    seen_star = ch == '*';
                                    last = i;
                                } else {
                                    return Some(Err(LexicalError::EndOfFileInComment(start, self.input.len())));
                                }
                            }

                            if let Some(doc_start) = doc_comment_start {
                                if last > doc_start {
                                    return Some(Ok((start + 3,
                                        Token::DocComment(CommentType::Block, &self.input[doc_start..last]),
                                        last)));
                                }
                            }
                       },
                        _ => {
                            return Some(Ok((start, Token::Divide, start+1)));
                        }
                    }
                }
                Some((start, ch)) if ch.is_ascii_digit() => return self.parse_number(start, start, ch),
                Some((i, ';')) => return Some(Ok((i, Token::Semicolon, i+1))),
                Some((i, ',')) => return Some(Ok((i, Token::Comma, i+1))),
                Some((i, '(')) => return Some(Ok((i, Token::OpenParenthesis, i+1))),
                Some((i, ')')) => return Some(Ok((i, Token::CloseParenthesis, i+1))),
                Some((i, '{')) => return Some(Ok((i, Token::OpenCurlyBrace, i+1))),
                Some((i, '}')) => return Some(Ok((i, Token::CloseCurlyBrace, i+1))),
                Some((i, '~')) => return Some(Ok((i, Token::Complement, i+1))),
                Some((i, '=')) => {
                    if let Some((_, '=')) = self.chars.peek() {
                        self.chars.next();
                        return Some(Ok((i, Token::Equal, i+2)));
                    } else {
                        return Some(Ok((i, Token::Assign, i+1)));
                    }
                }
                Some((i, '!')) => {
                    if let Some((_, '=')) = self.chars.peek() {
                        self.chars.next();
                        return Some(Ok((i, Token::NotEqual, i+2)));
                    } else {
                        return Some(Ok((i, Token::Not, i+1)));
                    }
                }
                Some((i, '|')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseOrAssign, i+2)))
                        },
                        Some((_, '|')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Or, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::BitwiseOr, i+1)))
                        }
                    };
                }
                Some((i, '&')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseAndAssign, i+2)))
                        },
                        Some((_, '&')) => {
                            self.chars.next();
                            Some(Ok((i, Token::And, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::BitwiseAnd, i+1)))
                        }
                    };
                }
                Some((i, '^')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseXorAssign, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::BitwiseXor, i+1)))
                        }
                    };
                }
                Some((i, '+')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::AddAssign, i+2)))
                        },
                        Some((_, '+')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Increment, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::Add, i+1)))
                        }
                    };
                }
                Some((i, '-')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::SubtractAssign, i+2)))
                        },
                        Some((_, '-')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Decrement, i+2)))
                        },
                        Some((end, ch)) if ch.is_ascii_digit() => {
                            let ch = *ch;
                            let end = *end;
                            self.chars.next();
                            self.parse_number(i, end, ch)
                        },
                        _ => {
                            Some(Ok((i, Token::Subtract, i+1)))
                        }
                    };
                }
                Some((i, '*')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::MulAssign, i+2)))
                        },
                        Some((_, '*')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Power, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::Mul, i+1)))
                        }
                    };
                }
                Some((i, '%')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::ModuloAssign, i+2)))
                        },
                        _ => {
                            Some(Ok((i, Token::Modulo, i+1)))
                        }
                    };
                }
                Some((i, '<')) => {
                    return match self.chars.peek() {
                        Some((_, '<')) => {
                            self.chars.next();
                            if let Some((_, '=')) = self.chars.peek() {
                                self.chars.next();
                                Some(Ok((i, Token::ShiftLeftAssign, i+3)))
                            } else {
                                Some(Ok((i, Token::ShiftLeft, i+2)))
                            }
                        },
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::LessEqual, i+2)))
                        }
                        _ => {
                           Some(Ok((i, Token::Less, i+1)))
                        }
                    };
                }
                Some((i, '>')) => {
                    return match self.chars.peek() {
                        Some((_, '>')) => {
                            self.chars.next();
                            if let Some((_, '=')) = self.chars.peek() {
                                self.chars.next();
                                Some(Ok((i, Token::ShiftRightAssign, i+3)))
                            } else {
                                Some(Ok((i, Token::ShiftRight, i+2)))
                            }
                        },
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token:: MoreEqual, i+2)))
                        }
                        _ => {
                           Some(Ok((i, Token::More, i+1)))
                        }
                    };
                }
                Some((i, '.')) => return Some(Ok((i, Token::Member, i+1))),
                Some((i, '[')) => return Some(Ok((i, Token::OpenBracket, i+1))),
                Some((i, ']')) => return Some(Ok((i, Token::CloseBracket, i+1))),
                Some((i, ':')) => return Some(Ok((i, Token::Colon, i+1))),
                Some((i, '?')) => return Some(Ok((i, Token::Question, i+1))),
                Some((_, '\t')) |
                Some((_, '\r')) |
                Some((_, ' ')) |
                Some((_, '\n')) => (),
                Some((start, _)) => {
                    let mut end;

                    loop {
                        if let Some((i, ch)) = self.chars.next() {
                            end = i;

                            if ch.is_ascii_whitespace() {
                                break;
                            }
                        } else {
                            end = self.input.len();
                            break;
                        }
                    }

                    return Some(Err(LexicalError::UnrecognisedToken(start, end, self.input[start..end].to_owned())));
                }
                None => return None, // End of file
            }
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token<'input>, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let PragmaParserState::SeenPragmaIdentifier = self.pragma_state {
            // special parser for pragma solidity >=0.4.22 <0.7.0;
            self.pragma_state = PragmaParserState::NotParsingPragma;
            let start;

            // eat all whitespace
            loop {
                if let Some((i, ch)) = self.chars.next() {
                    if !ch.is_ascii_whitespace() {
                        start = i;
                        break;
                    }
                } else {
                    return None;
                }
            }

            loop {
                match self.chars.next() {
                    Some((i, ';')) => {
                        return Some(Ok((start, Token::StringLiteral(&self.input[start..i]), i-1)));
                    },
                    Some(_) => (),
                    None => {
                        return Some(Err(LexicalError::PragmaMissingSemiColon(start, self.input.len())));
                    }
                }
            }
        }

        let token = self.next();

        self.pragma_state = match self.pragma_state {
            PragmaParserState::NotParsingPragma => {
                if let Some(Ok((_, Token::Pragma, _))) = token {
                    PragmaParserState::SeenPragma
                } else {
                    PragmaParserState::NotParsingPragma
                }
            },
            PragmaParserState::SeenPragma => {
                if let Some(Ok((_, Token::Identifier(_), _))) = token {
                    PragmaParserState::SeenPragmaIdentifier
                } else {
                    PragmaParserState::NotParsingPragma
                }
            },
            PragmaParserState::SeenPragmaIdentifier => {
                unreachable!();
            }
        };

        token
    }
}

/// Given an array of DocComments, fold them into one String
/// If the last entry is a block comment, return that.
/// If the last entry is a line comment, return that and any
/// preceding line comments. Any block comments preceding line
/// comments are ignored.
pub fn fold_doc_comments(docs: Vec<(CommentType, &str)>) -> String {
    if docs.is_empty() {
        return String::new();
    }

    let mut comment = String::new();

    for d in docs.iter().rev() {
        match d {
            (CommentType::Block, s) => {
                return if comment.is_empty() {
                    s.trim().chars().map(|c| if c == '\n' { ' ' } else { c }).collect()
                } else {
                    comment
                };
            },
            (CommentType::Line, s) => {
                if comment.is_empty() {
                    comment = s.trim().to_string();
                } else {
                    comment = format!("{} {}", s.trim().to_string(), comment);
                }
            },
        }
    }

    comment
}

#[test]
fn lexertest() {
    let tokens = Lexer::new("bool").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Bool, 4))));

    let tokens = Lexer::new("uint8").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Uint(8), 5))));

    let tokens = Lexer::new("hex").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Identifier("hex"), 3))));

    let tokens = Lexer::new("hex\"cafe_dead\" /* adad*** */").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::HexLiteral("hex\"cafe_dead\""), 14))));

    let tokens = Lexer::new("// foo bar\n0x00fead0_12 00090 0_0").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((11, Token::HexNumber("0x00fead0_12"), 23)),
        Ok((24, Token::Number("00090"), 29)),
        Ok((30, Token::Number("0_0"), 33))
    ));

    let tokens = Lexer::new("\"foo\"").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::StringLiteral("foo"), 5)),
    ));

    let tokens = Lexer::new("pragma solidity >=0.5.0 <0.7.0;").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::Pragma, 6)),
        Ok((7, Token::Identifier("solidity"), 15)),
        Ok((16, Token::StringLiteral(">=0.5.0 <0.7.0"), 29)),
    ));

    let tokens = Lexer::new(">>= >> >= >").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::ShiftRightAssign, 3)),
        Ok((4, Token::ShiftRight, 6)),
        Ok((7, Token::MoreEqual, 9)),
        Ok((10, Token::More, 11)),
    ));

    let tokens = Lexer::new("<<= << <= <").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::ShiftLeftAssign, 3)),
        Ok((4, Token::ShiftLeft, 6)),
        Ok((7, Token::LessEqual, 9)),
        Ok((10, Token::Less, 11)),
    ));

    let tokens = Lexer::new("-16 -- - -=").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::Number("-16"), 3)),
        Ok((4, Token::Decrement, 6)),
        Ok((7, Token::Subtract, 8)),
        Ok((9, Token::SubtractAssign, 11)),
    ));

    let tokens = Lexer::new("-4 ").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::Number("-4"), 2)),
    ));

    let tokens = Lexer::new(r#"hex"abcdefg""#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Err(LexicalError::InvalidCharacterInHexLiteral(10, 'g'))
    ));

    let tokens = Lexer::new(r#" € "#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Err(LexicalError::UnrecognisedToken(1, 4, "€".to_owned()))
    ));

    let tokens = Lexer::new(r#"€"#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Err(LexicalError::UnrecognisedToken(0, 3, "€".to_owned()))
    ));

    let tokens = Lexer::new(r#"pragma foo bar"#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((0, Token::Pragma, 6)),
        Ok((7, Token::Identifier("foo"), 10)),
        Err(LexicalError::PragmaMissingSemiColon(11, 14))
    ));

    let tokens = Lexer::new(r#"/// foo"#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((3, Token::DocComment(CommentType::Line, " foo"), 7))
    ));

    let tokens = Lexer::new("/// jadajadadjada\n// bar").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((3, Token::DocComment(CommentType::Line, " jadajadadjada"), 17))
    ));

    let tokens = Lexer::new(r#"/** foo */"#).collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((3, Token::DocComment(CommentType::Block, " foo "), 8))
    ));

    let tokens = Lexer::new("/** jadajadadjada */\n/* bar */").collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(
        Ok((3, Token::DocComment(CommentType::Block, " jadajadadjada "), 18))
    ));
}

#[test]
fn doc_comment_lexer() {
    let tokens = Lexer::new("/** jadajadad\njada */\n/* bar */").map(|e| match e {
        Ok((_, Token::DocComment(t, s), _)) => (t, s),
        _ => unreachable!()
    }).collect();

    assert_eq!(fold_doc_comments(tokens), "jadajadad jada");

    let tokens = Lexer::new("/** bar *//** jadajadad\njada */\n/* bar */").map(|e| match e {
        Ok((_, Token::DocComment(t, s), _)) => (t, s),
        _ => unreachable!()
    }).collect();

    assert_eq!(fold_doc_comments(tokens), "jadajadad jada");

    let tokens = Lexer::new("/// bar   \n///    jadajadad\n\n/* bar */").map(|e| match e {
        Ok((_, Token::DocComment(t, s), _)) => (t, s),
        _ => unreachable!()
    }).collect();

    assert_eq!(fold_doc_comments(tokens), "bar jadajadad");
}