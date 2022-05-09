//
// Solidity custom lexer. Solidity needs a custom lexer for two reasons:
//  - comments and doc comments
//  - pragma value is [^;]+
//
use phf::phf_map;
use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;
use unicode_xid::UnicodeXID;

use crate::pt::{CodeLocation, Comment, CommentType, Loc};

pub type Spanned<Token, Loc, Error> = Result<(Loc, Token, Loc), Error>;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Token<'input> {
    Identifier(&'input str),
    StringLiteral(&'input str),
    AddressLiteral(&'input str),
    HexLiteral(&'input str),
    Number(&'input str, &'input str),
    RationalNumber(&'input str, &'input str, &'input str),
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
    Error,
    Enum,
    Type,

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
    Revert,

    Uint(u16),
    Int(u16),
    Bytes(u8),
    // prior to 0.8.0 `byte` used to be an alias for `bytes1`
    Byte,
    DynamicBytes,
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
    ColonAssign,

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
    Complement,
    Question,

    Mapping,
    Arrow,

    Try,
    Catch,

    Receive,
    Fallback,

    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Gwei,
    Wei,
    Szabo,
    Finney,
    Ether,

    This,
    As,
    Is,
    Abstract,
    Virtual,
    Override,
    Using,
    Modifier,
    Immutable,
    Unchecked,

    Assembly,
    Let,
    Leave,
    Switch,
    Case,
    Default,
    YulArrow,
}

impl<'input> fmt::Display for Token<'input> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::DocComment(CommentType::Line, s) => write!(f, "///{}", s),
            Token::DocComment(CommentType::Block, s) => write!(f, "/**{}\n*/", s),
            Token::Identifier(id) => write!(f, "{}", id),
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::HexLiteral(hex) => write!(f, "{}", hex),
            Token::AddressLiteral(address) => write!(f, "{}", address),
            Token::Number(base, exp) if exp.is_empty() => write!(f, "{}", base),
            Token::Number(base, exp) => write!(f, "{}e{}", base, exp),
            Token::RationalNumber(significand, mantissa, exp) if exp.is_empty() => {
                write!(f, "{}.{}", significand, mantissa)
            }
            Token::RationalNumber(significand, mantissa, exp) => {
                write!(f, "{}.{}e{}", significand, mantissa, exp)
            }
            Token::HexNumber(n) => write!(f, "{}", n),
            Token::Uint(w) => write!(f, "uint{}", w),
            Token::Int(w) => write!(f, "int{}", w),
            Token::Bytes(w) => write!(f, "bytes{}", w),
            Token::Byte => write!(f, "byte"),
            Token::DynamicBytes => write!(f, "bytes"),
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
            Token::ColonAssign => write!(f, ":="),
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
            Token::Error => write!(f, "error"),
            Token::Enum => write!(f, "enum"),
            Token::Type => write!(f, "type"),
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
            Token::Revert => write!(f, "revert"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Else => write!(f, "else"),
            Token::Anonymous => write!(f, "anonymous"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::If => write!(f, "if"),
            Token::Constructor => write!(f, "constructor"),
            Token::Indexed => write!(f, "indexed"),
            Token::Mapping => write!(f, "mapping"),
            Token::Arrow => write!(f, "=>"),
            Token::Try => write!(f, "try"),
            Token::Catch => write!(f, "catch"),
            Token::Receive => write!(f, "receive"),
            Token::Fallback => write!(f, "fallback"),
            Token::Seconds => write!(f, "seconds"),
            Token::Minutes => write!(f, "minutes"),
            Token::Hours => write!(f, "hours"),
            Token::Days => write!(f, "days"),
            Token::Weeks => write!(f, "weeks"),
            Token::Gwei => write!(f, "gwei"),
            Token::Wei => write!(f, "wei"),
            Token::Szabo => write!(f, "szabo"),
            Token::Finney => write!(f, "finney"),
            Token::Ether => write!(f, "ether"),
            Token::This => write!(f, "this"),
            Token::As => write!(f, "as"),
            Token::Is => write!(f, "is"),
            Token::Abstract => write!(f, "abstract"),
            Token::Virtual => write!(f, "virtual"),
            Token::Override => write!(f, "override"),
            Token::Using => write!(f, "using"),
            Token::Modifier => write!(f, "modifier"),
            Token::Immutable => write!(f, "immutable"),
            Token::Unchecked => write!(f, "unchecked"),
            Token::Assembly => write!(f, "assembly"),
            Token::Let => write!(f, "let"),
            Token::Leave => write!(f, "leave"),
            Token::Switch => write!(f, "switch"),
            Token::Case => write!(f, "case"),
            Token::Default => write!(f, "default"),
            Token::YulArrow => write!(f, "->"),
        }
    }
}

pub struct Lexer<'input> {
    input: &'input str,
    chars: Peekable<CharIndices<'input>>,
    comments: &'input mut Vec<Comment>,
    file_no: usize,
    last_tokens: [Option<Token<'input>>; 2],
}

#[derive(Debug, PartialEq)]
pub enum LexicalError {
    EndOfFileInComment(Loc),
    EndOfFileInString(Loc),
    EndofFileInHex(Loc),
    MissingNumber(Loc),
    InvalidCharacterInHexLiteral(Loc, char),
    UnrecognisedToken(Loc, String),
    MissingExponent(Loc),
    DoublePoints(Loc),
    UnrecognisedDecimal(Loc),
    ExpectedFrom(Loc, String),
}

impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::EndOfFileInComment(..) => write!(f, "end of file found in comment"),
            LexicalError::EndOfFileInString(..) => {
                write!(f, "end of file found in string literal")
            }
            LexicalError::EndofFileInHex(..) => {
                write!(f, "end of file found in hex literal string")
            }
            LexicalError::MissingNumber(..) => write!(f, "missing number"),
            LexicalError::InvalidCharacterInHexLiteral(_, ch) => {
                write!(f, "invalid character '{}' in hex literal string", ch)
            }
            LexicalError::UnrecognisedToken(_, t) => write!(f, "unrecognised token '{}'", t),
            LexicalError::ExpectedFrom(_, t) => write!(f, "'{}' found where 'from' expected", t),
            LexicalError::MissingExponent(..) => write!(f, "missing number"),
            LexicalError::DoublePoints(..) => write!(f, "found two dots in number"),
            LexicalError::UnrecognisedDecimal(..) => {
                write!(f, "expected number after decimal point")
            }
        }
    }
}

impl CodeLocation for LexicalError {
    fn loc(&self) -> Loc {
        match self {
            LexicalError::EndOfFileInComment(loc, ..)
            | LexicalError::EndOfFileInString(loc, ..)
            | LexicalError::EndofFileInHex(loc, ..)
            | LexicalError::MissingNumber(loc, ..)
            | LexicalError::InvalidCharacterInHexLiteral(loc, _)
            | LexicalError::UnrecognisedToken(loc, ..)
            | LexicalError::ExpectedFrom(loc, ..)
            | LexicalError::MissingExponent(loc, ..)
            | LexicalError::DoublePoints(loc, ..)
            | LexicalError::UnrecognisedDecimal(loc, ..) => *loc,
        }
    }
}

static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "address" => Token::Address,
    "anonymous" => Token::Anonymous,
    "bool" => Token::Bool,
    "break" => Token::Break,
    "bytes1" => Token::Bytes(1),
    "bytes2" => Token::Bytes(2),
    "bytes3" => Token::Bytes(3),
    "bytes4" => Token::Bytes(4),
    "bytes5" => Token::Bytes(5),
    "bytes6" => Token::Bytes(6),
    "bytes7" => Token::Bytes(7),
    "bytes8" => Token::Bytes(8),
    "bytes9" => Token::Bytes(9),
    "bytes10" => Token::Bytes(10),
    "bytes11" => Token::Bytes(11),
    "bytes12" => Token::Bytes(12),
    "bytes13" => Token::Bytes(13),
    "bytes14" => Token::Bytes(14),
    "bytes15" => Token::Bytes(15),
    "bytes16" => Token::Bytes(16),
    "bytes17" => Token::Bytes(17),
    "bytes18" => Token::Bytes(18),
    "bytes19" => Token::Bytes(19),
    "bytes20" => Token::Bytes(20),
    "bytes21" => Token::Bytes(21),
    "bytes22" => Token::Bytes(22),
    "bytes23" => Token::Bytes(23),
    "bytes24" => Token::Bytes(24),
    "bytes25" => Token::Bytes(25),
    "bytes26" => Token::Bytes(26),
    "bytes27" => Token::Bytes(27),
    "bytes28" => Token::Bytes(28),
    "bytes29" => Token::Bytes(29),
    "bytes30" => Token::Bytes(30),
    "bytes31" => Token::Bytes(31),
    "bytes32" => Token::Bytes(32),
    "bytes" => Token::DynamicBytes,
    "byte" => Token::Byte,
    "calldata" => Token::Calldata,
    "case" => Token::Case,
    "constant" => Token::Constant,
    "constructor" => Token::Constructor,
    "continue" => Token::Continue,
    "contract" => Token::Contract,
    "default" => Token::Default,
    "delete" => Token::Delete,
    "do" => Token::Do,
    "else" => Token::Else,
    "emit" => Token::Emit,
    "enum" => Token::Enum,
    "event" => Token::Event,
    "error" => Token::Error,
    "external" => Token::External,
    "false" => Token::False,
    "for" => Token::For,
    "function" => Token::Function,
    "if" => Token::If,
    "import" => Token::Import,
    "indexed" => Token::Indexed,
    "int8" => Token::Int(8),
    "int16" => Token::Int(16),
    "int24" => Token::Int(24),
    "int32" => Token::Int(32),
    "int40" => Token::Int(40),
    "int48" => Token::Int(48),
    "int56" => Token::Int(56),
    "int64" => Token::Int(64),
    "int72" => Token::Int(72),
    "int80" => Token::Int(80),
    "int88" => Token::Int(88),
    "int96" => Token::Int(96),
    "int104" => Token::Int(104),
    "int112" => Token::Int(112),
    "int120" => Token::Int(120),
    "int128" => Token::Int(128),
    "int136" => Token::Int(136),
    "int144" => Token::Int(144),
    "int152" => Token::Int(152),
    "int160" => Token::Int(160),
    "int168" => Token::Int(168),
    "int176" => Token::Int(176),
    "int184" => Token::Int(184),
    "int192" => Token::Int(192),
    "int200" => Token::Int(200),
    "int208" => Token::Int(208),
    "int216" => Token::Int(216),
    "int224" => Token::Int(224),
    "int232" => Token::Int(232),
    "int240" => Token::Int(240),
    "int248" => Token::Int(248),
    "int256" => Token::Int(256),
    "interface" => Token::Interface,
    "internal" => Token::Internal,
    "int" => Token::Int(256),
    "leave" => Token::Leave,
    "library" => Token::Library,
    "mapping" => Token::Mapping,
    "memory" => Token::Memory,
    "new" => Token::New,
    "payable" => Token::Payable,
    "pragma" => Token::Pragma,
    "private" => Token::Private,
    "public" => Token::Public,
    "pure" => Token::Pure,
    "returns" => Token::Returns,
    "return" => Token::Return,
    "revert" => Token::Revert,
    "storage" => Token::Storage,
    "string" => Token::String,
    "struct" => Token::Struct,
    "switch" => Token::Switch,
    "throw" => Token::Throw,
    "true" => Token::True,
    "type" => Token::Type,
    "uint8" => Token::Uint(8),
    "uint16" => Token::Uint(16),
    "uint24" => Token::Uint(24),
    "uint32" => Token::Uint(32),
    "uint40" => Token::Uint(40),
    "uint48" => Token::Uint(48),
    "uint56" => Token::Uint(56),
    "uint64" => Token::Uint(64),
    "uint72" => Token::Uint(72),
    "uint80" => Token::Uint(80),
    "uint88" => Token::Uint(88),
    "uint96" => Token::Uint(96),
    "uint104" => Token::Uint(104),
    "uint112" => Token::Uint(112),
    "uint120" => Token::Uint(120),
    "uint128" => Token::Uint(128),
    "uint136" => Token::Uint(136),
    "uint144" => Token::Uint(144),
    "uint152" => Token::Uint(152),
    "uint160" => Token::Uint(160),
    "uint168" => Token::Uint(168),
    "uint176" => Token::Uint(176),
    "uint184" => Token::Uint(184),
    "uint192" => Token::Uint(192),
    "uint200" => Token::Uint(200),
    "uint208" => Token::Uint(208),
    "uint216" => Token::Uint(216),
    "uint224" => Token::Uint(224),
    "uint232" => Token::Uint(232),
    "uint240" => Token::Uint(240),
    "uint248" => Token::Uint(248),
    "uint256" => Token::Uint(256),
    "uint" => Token::Uint(256),
    "view" => Token::View,
    "while" => Token::While,
    "try" => Token::Try,
    "catch" => Token::Catch,
    "receive" => Token::Receive,
    "fallback" => Token::Fallback,
    "seconds" => Token::Seconds,
    "minutes" => Token::Minutes,
    "hours" => Token::Hours,
    "days" => Token::Days,
    "weeks" => Token::Weeks,
    "wei" => Token::Wei,
    "gwei" => Token::Gwei,
    "szabo" => Token::Szabo,
    "finney" => Token::Finney,
    "ether" => Token::Ether,
    "this" => Token::This,
    "as" => Token::As,
    "is" => Token::Is,
    "abstract" => Token::Abstract,
    "virtual" => Token::Virtual,
    "override" => Token::Override,
    "using" => Token::Using,
    "modifier" => Token::Modifier,
    "immutable" => Token::Immutable,
    "unchecked" => Token::Unchecked,
    "assembly" => Token::Assembly,
    "let" => Token::Let,
};

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str, file_no: usize, comments: &'input mut Vec<Comment>) -> Self {
        Lexer {
            input,
            chars: input.char_indices().peekable(),
            comments,
            file_no,
            last_tokens: [None, None],
        }
    }

    fn parse_number(
        &mut self,
        start: usize,
        end: usize,
        ch: char,
    ) -> Result<(usize, Token<'input>, usize), LexicalError> {
        let mut is_rational = false;
        if ch == '0' {
            if let Some((_, 'x')) = self.chars.peek() {
                // hex number
                self.chars.next();

                let mut end = match self.chars.next() {
                    Some((end, ch)) if ch.is_ascii_hexdigit() => end,
                    Some((..)) => {
                        return Err(LexicalError::MissingNumber(Loc::File(
                            self.file_no,
                            start,
                            start + 1,
                        )));
                    }
                    None => {
                        return Err(LexicalError::EndofFileInHex(Loc::File(
                            self.file_no,
                            start,
                            self.input.len(),
                        )));
                    }
                };

                while let Some((i, ch)) = self.chars.peek() {
                    if !ch.is_ascii_hexdigit() && *ch != '_' {
                        break;
                    }
                    end = *i;
                    self.chars.next();
                }

                return Ok((start, Token::HexNumber(&self.input[start..=end]), end + 1));
            }
        }

        let mut start = start;
        if ch == '.' {
            is_rational = true;
            start -= 1;
        }

        let mut end = end;
        while let Some((i, ch)) = self.chars.peek() {
            if !ch.is_ascii_digit() && *ch != '_' {
                break;
            }
            end = *i;
            self.chars.next();
        }
        let mut rational_end = end;
        let mut end_before_rational = end;
        let mut rational_start = end;
        if is_rational {
            end_before_rational = start - 1;
            rational_start = start + 1;
        }

        if let Some((i, '.')) = self.chars.peek() {
            if is_rational {
                return Err(LexicalError::DoublePoints(Loc::File(
                    self.file_no,
                    start,
                    self.input.len(),
                )));
            }
            rational_start = *i + 1;
            rational_end = *i + 1;
            let mut has_number = false;
            is_rational = true;
            self.chars.next();
            while let Some((i, ch)) = self.chars.peek() {
                if *ch == '.' {
                    return Err(LexicalError::DoublePoints(Loc::File(
                        self.file_no,
                        start,
                        self.input.len(),
                    )));
                }
                if !ch.is_ascii_digit() {
                    break;
                }
                has_number = true;
                rational_end = *i;
                end = *i;
                self.chars.next();
            }
            if !has_number {
                return Err(LexicalError::UnrecognisedDecimal(Loc::File(
                    self.file_no,
                    start,
                    self.input.len(),
                )));
            }
        }

        let old_end = end;
        let mut exp_start = end + 1;

        if let Some((i, 'e' | 'E')) = self.chars.peek() {
            exp_start = *i + 1;
            self.chars.next();
            while let Some((i, ch)) = self.chars.peek() {
                if !ch.is_ascii_digit() && *ch != '_' && *ch != '-' {
                    break;
                }
                end = *i;
                self.chars.next();
            }

            if exp_start > end {
                return Err(LexicalError::MissingExponent(Loc::File(
                    self.file_no,
                    start,
                    self.input.len(),
                )));
            }
        }

        if is_rational {
            let significand = &self.input[start..=end_before_rational];
            let mantissa = &self.input[rational_start..=rational_end];

            if mantissa.is_empty() {
                return Err(LexicalError::UnrecognisedDecimal(Loc::File(
                    self.file_no,
                    start,
                    self.input.len(),
                )));
            }
            let exp = &self.input[exp_start..=end];
            return Ok((
                start,
                Token::RationalNumber(significand, mantissa, exp),
                end + 1,
            ));
        }

        let base = &self.input[start..=old_end];
        let exp = &self.input[exp_start..=end];

        Ok((start, Token::Number(base, exp), end + 1))
    }

    fn string(
        &mut self,
        token_start: usize,
        string_start: usize,
        quote_char: char,
    ) -> Result<(usize, Token<'input>, usize), LexicalError> {
        let mut end;

        let mut last_was_escape = false;

        loop {
            if let Some((i, ch)) = self.chars.next() {
                end = i;
                if !last_was_escape {
                    if ch == quote_char {
                        break;
                    }
                    last_was_escape = ch == '\\';
                } else {
                    last_was_escape = false;
                }
            } else {
                return Err(LexicalError::EndOfFileInString(Loc::File(
                    self.file_no,
                    token_start,
                    self.input.len(),
                )));
            }
        }

        Ok((
            token_start,
            Token::StringLiteral(&self.input[string_start..end]),
            end + 1,
        ))
    }

    fn next(&mut self) -> Option<Result<(usize, Token<'input>, usize), LexicalError>> {
        loop {
            match self.chars.next() {
                Some((start, ch)) if ch == '_' || ch == '$' || UnicodeXID::is_xid_start(ch) => {
                    let end;

                    loop {
                        if let Some((i, ch)) = self.chars.peek() {
                            if !UnicodeXID::is_xid_continue(*ch) && *ch != '$' {
                                end = *i;
                                break;
                            }
                            self.chars.next();
                        } else {
                            end = self.input.len();
                            break;
                        }
                    }

                    let id = &self.input[start..end];

                    if id == "unicode" {
                        match self.chars.peek() {
                            Some((_, quote_char @ '"')) | Some((_, quote_char @ '\'')) => {
                                let quote_char = *quote_char;

                                self.chars.next();

                                return Some(self.string(start, start + 8, quote_char));
                            }
                            _ => (),
                        }
                    }

                    if id == "hex" {
                        match self.chars.peek() {
                            Some((_, quote_char @ '"')) | Some((_, quote_char @ '\'')) => {
                                let quote_char = *quote_char;

                                self.chars.next();

                                for (i, ch) in &mut self.chars {
                                    if ch == quote_char {
                                        return Some(Ok((
                                            start,
                                            Token::HexLiteral(&self.input[start..=i]),
                                            i + 1,
                                        )));
                                    }

                                    if !ch.is_ascii_hexdigit() && ch != '_' {
                                        // Eat up the remainer of the string
                                        for (_, ch) in &mut self.chars {
                                            if ch == quote_char {
                                                break;
                                            }
                                        }

                                        return Some(Err(
                                            LexicalError::InvalidCharacterInHexLiteral(
                                                Loc::File(self.file_no, i, i + 1),
                                                ch,
                                            ),
                                        ));
                                    }
                                }

                                return Some(Err(LexicalError::EndOfFileInString(Loc::File(
                                    self.file_no,
                                    start,
                                    self.input.len(),
                                ))));
                            }
                            _ => (),
                        }
                    }

                    if id == "address" {
                        match self.chars.peek() {
                            Some((_, quote_char @ '"')) | Some((_, quote_char @ '\'')) => {
                                let quote_char = *quote_char;

                                self.chars.next();

                                for (i, ch) in &mut self.chars {
                                    if ch == quote_char {
                                        return Some(Ok((
                                            start,
                                            Token::AddressLiteral(&self.input[start..=i]),
                                            i + 1,
                                        )));
                                    }
                                }

                                return Some(Err(LexicalError::EndOfFileInString(Loc::File(
                                    self.file_no,
                                    start,
                                    self.input.len(),
                                ))));
                            }
                            _ => (),
                        }
                    }

                    return if let Some(w) = KEYWORDS.get(id) {
                        Some(Ok((start, *w, end)))
                    } else {
                        Some(Ok((start, Token::Identifier(id), end)))
                    };
                }
                Some((start, quote_char @ '"')) | Some((start, quote_char @ '\'')) => {
                    return Some(self.string(start, start + 1, quote_char));
                }
                Some((start, '/')) => {
                    match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            return Some(Ok((start, Token::DivideAssign, start + 2)));
                        }
                        Some((_, '/')) => {
                            // line comment
                            self.chars.next();

                            let mut newline = false;

                            let doc_comment_start = match self.chars.next() {
                                Some((i, '/')) => match self.chars.peek() {
                                    // ///(/)+ is still a line comment
                                    Some((_, '/')) => None,
                                    _ => Some(i + 1),
                                },
                                Some((_, ch)) if ch == '\n' || ch == '\r' => {
                                    newline = true;
                                    None
                                }
                                _ => None,
                            };

                            let mut last = start + 3;

                            if !newline {
                                loop {
                                    match self.chars.next() {
                                        None => {
                                            last = self.input.len();
                                            break;
                                        }
                                        Some((offset, '\n' | '\r')) => {
                                            last = offset;
                                            break;
                                        }
                                        Some(_) => (),
                                    }
                                }
                            }

                            if let Some(doc_start) = doc_comment_start {
                                if last > doc_start {
                                    return Some(Ok((
                                        start + 3,
                                        Token::DocComment(
                                            CommentType::Line,
                                            &self.input[doc_start..last],
                                        ),
                                        last,
                                    )));
                                }
                            } else {
                                self.comments.push(Comment::Line(
                                    Loc::File(self.file_no, start, last),
                                    self.input[start..last].to_owned(),
                                ));
                            }
                        }
                        Some((_, '*')) => {
                            // multiline comment
                            self.chars.next();

                            let doc_comment_start = match self.chars.peek() {
                                Some((i, '*')) => Some(i + 1),
                                _ => None,
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
                                    return Some(Err(LexicalError::EndOfFileInComment(Loc::File(
                                        self.file_no,
                                        start,
                                        self.input.len(),
                                    ))));
                                }
                            }

                            if let Some(doc_start) = doc_comment_start {
                                if last > doc_start {
                                    let comment = &self.input[doc_start..last];

                                    if comment.chars().any(|ch| !ch.is_whitespace() && ch != '*') {
                                        return Some(Ok((
                                            start + 3,
                                            Token::DocComment(
                                                CommentType::Block,
                                                &self.input[doc_start..last],
                                            ),
                                            last,
                                        )));
                                    }
                                }
                            }

                            self.comments.push(Comment::Block(
                                Loc::File(self.file_no, start, last + 2),
                                self.input[start..last + 2].to_owned(),
                            ));
                        }
                        _ => {
                            return Some(Ok((start, Token::Divide, start + 1)));
                        }
                    }
                }
                Some((start, ch)) if ch.is_ascii_digit() => {
                    return Some(self.parse_number(start, start, ch))
                }
                Some((i, ';')) => return Some(Ok((i, Token::Semicolon, i + 1))),
                Some((i, ',')) => return Some(Ok((i, Token::Comma, i + 1))),
                Some((i, '(')) => return Some(Ok((i, Token::OpenParenthesis, i + 1))),
                Some((i, ')')) => return Some(Ok((i, Token::CloseParenthesis, i + 1))),
                Some((i, '{')) => return Some(Ok((i, Token::OpenCurlyBrace, i + 1))),
                Some((i, '}')) => return Some(Ok((i, Token::CloseCurlyBrace, i + 1))),
                Some((i, '~')) => return Some(Ok((i, Token::Complement, i + 1))),
                Some((i, '=')) => match self.chars.peek() {
                    Some((_, '=')) => {
                        self.chars.next();
                        return Some(Ok((i, Token::Equal, i + 2)));
                    }
                    Some((_, '>')) => {
                        self.chars.next();
                        return Some(Ok((i, Token::Arrow, i + 2)));
                    }
                    _ => {
                        return Some(Ok((i, Token::Assign, i + 1)));
                    }
                },
                Some((i, '!')) => {
                    if let Some((_, '=')) = self.chars.peek() {
                        self.chars.next();
                        return Some(Ok((i, Token::NotEqual, i + 2)));
                    } else {
                        return Some(Ok((i, Token::Not, i + 1)));
                    }
                }
                Some((i, '|')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseOrAssign, i + 2)))
                        }
                        Some((_, '|')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Or, i + 2)))
                        }
                        _ => Some(Ok((i, Token::BitwiseOr, i + 1))),
                    };
                }
                Some((i, '&')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseAndAssign, i + 2)))
                        }
                        Some((_, '&')) => {
                            self.chars.next();
                            Some(Ok((i, Token::And, i + 2)))
                        }
                        _ => Some(Ok((i, Token::BitwiseAnd, i + 1))),
                    };
                }
                Some((i, '^')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::BitwiseXorAssign, i + 2)))
                        }
                        _ => Some(Ok((i, Token::BitwiseXor, i + 1))),
                    };
                }
                Some((i, '+')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::AddAssign, i + 2)))
                        }
                        Some((_, '+')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Increment, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Add, i + 1))),
                    };
                }
                Some((i, '-')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::SubtractAssign, i + 2)))
                        }
                        Some((_, '-')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Decrement, i + 2)))
                        }
                        Some((_, '>')) => {
                            self.chars.next();
                            Some(Ok((i, Token::YulArrow, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Subtract, i + 1))),
                    };
                }
                Some((i, '*')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::MulAssign, i + 2)))
                        }
                        Some((_, '*')) => {
                            self.chars.next();
                            Some(Ok((i, Token::Power, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Mul, i + 1))),
                    };
                }
                Some((i, '%')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::ModuloAssign, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Modulo, i + 1))),
                    };
                }
                Some((i, '<')) => {
                    return match self.chars.peek() {
                        Some((_, '<')) => {
                            self.chars.next();
                            if let Some((_, '=')) = self.chars.peek() {
                                self.chars.next();
                                Some(Ok((i, Token::ShiftLeftAssign, i + 3)))
                            } else {
                                Some(Ok((i, Token::ShiftLeft, i + 2)))
                            }
                        }
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::LessEqual, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Less, i + 1))),
                    };
                }
                Some((i, '>')) => {
                    return match self.chars.peek() {
                        Some((_, '>')) => {
                            self.chars.next();
                            if let Some((_, '=')) = self.chars.peek() {
                                self.chars.next();
                                Some(Ok((i, Token::ShiftRightAssign, i + 3)))
                            } else {
                                Some(Ok((i, Token::ShiftRight, i + 2)))
                            }
                        }
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::MoreEqual, i + 2)))
                        }
                        _ => Some(Ok((i, Token::More, i + 1))),
                    };
                }
                Some((i, '.')) => {
                    if let Some((_, a)) = self.chars.peek() {
                        if a.is_ascii_digit() {
                            return Some(self.parse_number(i + 1, i + 1, '.'));
                        }
                    }
                    return Some(Ok((i, Token::Member, i + 1)));
                }
                Some((i, '[')) => return Some(Ok((i, Token::OpenBracket, i + 1))),
                Some((i, ']')) => return Some(Ok((i, Token::CloseBracket, i + 1))),
                Some((i, ':')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some(Ok((i, Token::ColonAssign, i + 2)))
                        }
                        _ => Some(Ok((i, Token::Colon, i + 1))),
                    };
                }
                Some((i, '?')) => return Some(Ok((i, Token::Question, i + 1))),
                Some((_, ch)) if ch.is_whitespace() => (),
                Some((start, _)) => {
                    let mut end;

                    loop {
                        if let Some((i, ch)) = self.chars.next() {
                            end = i;

                            if ch.is_whitespace() {
                                break;
                            }
                        } else {
                            end = self.input.len();
                            break;
                        }
                    }

                    return Some(Err(LexicalError::UnrecognisedToken(
                        Loc::File(self.file_no, start, end),
                        self.input[start..end].to_owned(),
                    )));
                }
                None => return None, // End of file
            }
        }
    }

    /// Next token is pragma value. Return it
    fn pragma_value(&mut self) -> Option<Result<(usize, Token<'input>, usize), LexicalError>> {
        // special parser for pragma solidity >=0.4.22 <0.7.0;
        let mut start = None;
        let mut end = 0;

        // solc will include anything upto the next semicolon, whitespace
        // trimmed on left and right
        loop {
            match self.chars.peek() {
                Some((_, ';')) | None => {
                    return if let Some(start) = start {
                        Some(Ok((
                            start,
                            Token::StringLiteral(&self.input[start..end]),
                            end,
                        )))
                    } else {
                        self.next()
                    };
                }
                Some((_, ch)) if ch.is_whitespace() => {
                    self.chars.next();
                }
                Some((i, _)) => {
                    if start.is_none() {
                        start = Some(*i);
                    }
                    self.chars.next();

                    // end should point to the byte _after_ the character
                    end = match self.chars.peek() {
                        Some((i, _)) => *i,
                        None => self.input.len(),
                    }
                }
            }
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token<'input>, usize, LexicalError>;

    /// Return the next token
    fn next(&mut self) -> Option<Self::Item> {
        // Lexer should be aware of whether the last two tokens were
        // pragma followed by identifier. If this is true, then special parsing should be
        // done for the pragma value
        let token = if let [Some(Token::Pragma), Some(Token::Identifier(_))] = self.last_tokens {
            self.pragma_value()
        } else {
            self.next()
        };

        self.last_tokens = [
            self.last_tokens[1],
            match token {
                Some(Ok((_, n, _))) => Some(n),
                _ => None,
            },
        ];

        token
    }
}

#[test]
fn lexertest() {
    let mut comments = Vec::new();

    let tokens = Lexer::new("bool", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Bool, 4))));

    let tokens = Lexer::new("uint8", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Uint(8), 5))));

    let tokens = Lexer::new("hex", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::Identifier("hex"), 3))));

    let tokens = Lexer::new("hex\"cafe_dead\" /* adad*** */", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Ok((0, Token::HexLiteral("hex\"cafe_dead\""), 14)))
    );

    let tokens = Lexer::new("// foo bar\n0x00fead0_12 00090 0_0", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((11, Token::HexNumber("0x00fead0_12"), 23)),
            Ok((24, Token::Number("00090", ""), 29)),
            Ok((30, Token::Number("0_0", ""), 33))
        )
    );

    let tokens = Lexer::new("// foo bar\n0x00fead0_12 9.0008 0_0", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((11, Token::HexNumber("0x00fead0_12"), 23)),
            Ok((24, Token::RationalNumber("9", "0008", ""), 30)),
            Ok((31, Token::Number("0_0", ""), 34))
        )
    );

    let tokens = Lexer::new("// foo bar\n0x00fead0_12 .0008 0.9e2", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((11, Token::HexNumber("0x00fead0_12"), 23)),
            Ok((24, Token::RationalNumber("", "0008", ""), 29)),
            Ok((30, Token::RationalNumber("0", "9", "2"), 35))
        )
    );

    let tokens = Lexer::new("// foo bar\n0x00fead0_12 .0008 0.9e-2", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((11, Token::HexNumber("0x00fead0_12"), 23)),
            Ok((24, Token::RationalNumber("", "0008", ""), 29)),
            Ok((30, Token::RationalNumber("0", "9", "-2"), 36))
        )
    );

    let tokens = Lexer::new("\"foo\"", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::StringLiteral("foo"), 5)),));

    let tokens = Lexer::new("pragma solidity >=0.5.0 <0.7.0;", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Pragma, 6)),
            Ok((7, Token::Identifier("solidity"), 15)),
            Ok((16, Token::StringLiteral(">=0.5.0 <0.7.0"), 30)),
            Ok((30, Token::Semicolon, 31)),
        )
    );

    let tokens = Lexer::new("pragma solidity \t>=0.5.0 <0.7.0 \n ;", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Pragma, 6)),
            Ok((7, Token::Identifier("solidity"), 15)),
            Ok((17, Token::StringLiteral(">=0.5.0 <0.7.0"), 31)),
            Ok((34, Token::Semicolon, 35)),
        )
    );

    let tokens = Lexer::new("pragma solidity 赤;", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Pragma, 6)),
            Ok((7, Token::Identifier("solidity"), 15)),
            Ok((16, Token::StringLiteral("赤"), 19)),
            Ok((19, Token::Semicolon, 20))
        )
    );

    let tokens = Lexer::new(">>= >> >= >", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::ShiftRightAssign, 3)),
            Ok((4, Token::ShiftRight, 6)),
            Ok((7, Token::MoreEqual, 9)),
            Ok((10, Token::More, 11)),
        )
    );

    let tokens = Lexer::new("<<= << <= <", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::ShiftLeftAssign, 3)),
            Ok((4, Token::ShiftLeft, 6)),
            Ok((7, Token::LessEqual, 9)),
            Ok((10, Token::Less, 11)),
        )
    );

    let tokens = Lexer::new("-16 -- - -=", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Subtract, 1)),
            Ok((1, Token::Number("16", ""), 3)),
            Ok((4, Token::Decrement, 6)),
            Ok((7, Token::Subtract, 8)),
            Ok((9, Token::SubtractAssign, 11)),
        )
    );

    let tokens = Lexer::new("-4 ", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Subtract, 1)),
            Ok((1, Token::Number("4", ""), 2)),
        )
    );

    let tokens = Lexer::new(r#"hex"abcdefg""#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Err(LexicalError::InvalidCharacterInHexLiteral(
            Loc::File(0, 10, 11),
            'g'
        )))
    );

    let tokens = Lexer::new(r#" € "#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Err(LexicalError::UnrecognisedToken(
            Loc::File(0, 1, 4),
            "€".to_owned()
        )))
    );

    let tokens = Lexer::new(r#"€"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Err(LexicalError::UnrecognisedToken(
            Loc::File(0, 0, 3),
            "€".to_owned()
        )))
    );

    let tokens = Lexer::new(r#"pragma foo bar"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Pragma, 6)),
            Ok((7, Token::Identifier("foo"), 10)),
            Ok((11, Token::StringLiteral("bar"), 14)),
        )
    );

    let tokens = Lexer::new(r#"/// foo"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Ok((3, Token::DocComment(CommentType::Line, " foo"), 7)))
    );

    let tokens = Lexer::new("/// jadajadadjada\n// bar", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Ok((
            3,
            Token::DocComment(CommentType::Line, " jadajadadjada"),
            17
        )))
    );

    let tokens = Lexer::new(r#"/** foo */"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Ok((3, Token::DocComment(CommentType::Block, " foo "), 8)))
    );

    let tokens = Lexer::new("/** jadajadadjada */\n/* bar */", 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(Ok((
            3,
            Token::DocComment(CommentType::Block, " jadajadadjada "),
            18
        )))
    );

    let tokens = Lexer::new("/************/", 0, &mut comments).next();
    assert_eq!(tokens, None);

    let tokens = Lexer::new("/**", 0, &mut comments).next();
    assert_eq!(
        tokens,
        Some(Err(LexicalError::EndOfFileInComment(Loc::File(0, 0, 3))))
    );

    let tokens = Lexer::new("//////////////", 0, &mut comments).next();
    assert_eq!(tokens, None);

    // some unicode tests
    let tokens = Lexer::new(
        ">=\u{a0} . très\u{2028}αβγδεζηθικλμνξοπρστυφχψω\u{85}カラス",
        0,
        &mut comments,
    )
    .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::MoreEqual, 2)),
            Ok((5, Token::Member, 6)),
            Ok((7, Token::Identifier("très"), 12)),
            Ok((15, Token::Identifier("αβγδεζηθικλμνξοπρστυφχψω"), 63)),
            Ok((65, Token::Identifier("カラス"), 74))
        )
    );

    let tokens = Lexer::new(r#"unicode"€""#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((0, Token::StringLiteral("€"), 12)),));

    let tokens = Lexer::new(r#"unicode "€""#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((0, Token::Identifier("unicode"), 7)),
            Ok((8, Token::StringLiteral("€"), 13)),
        )
    );

    // scientific notation
    let tokens = Lexer::new(r#" 1e0 "#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(tokens, vec!(Ok((1, Token::Number("1", "0"), 4)),));

    let tokens = Lexer::new(r#" -9e0123"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((1, Token::Subtract, 2)),
            Ok((2, Token::Number("9", "0123"), 8)),
        )
    );

    let tokens = Lexer::new(r#" -9e"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Ok((1, Token::Subtract, 2)),
            Err(LexicalError::MissingExponent(Loc::File(0, 2, 4)))
        )
    );

    let tokens = Lexer::new(r#"9ea"#, 0, &mut comments)
        .collect::<Vec<Result<(usize, Token, usize), LexicalError>>>();

    assert_eq!(
        tokens,
        vec!(
            Err(LexicalError::MissingExponent(Loc::File(0, 0, 3))),
            Ok((2, Token::Identifier("a"), 3))
        )
    );
}
