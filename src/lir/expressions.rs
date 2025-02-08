// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::sema::ast::{FormatArg, StringLocation};
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use std::fmt;
use std::fmt::Formatter;

use super::lir_type::LIRType;

/// Operand: including variables and literals
#[derive(Clone, Debug)]
pub enum Operand {
    Id {
        loc: Loc,
        id: usize,
    },
    BoolLiteral {
        loc: Loc,
        value: bool,
    },
    NumberLiteral {
        loc: Loc,
        value: BigInt,
        ty: LIRType,
    },
}

/// Binary operators
#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add { overflowing: bool },
    Sub { overflowing: bool },
    Mul { overflowing: bool },
    Pow { overflowing: bool },

    Div,
    UDiv,

    Mod,
    UMod,

    Eq,
    Neq,

    Lt,
    ULt,

    Lte,
    ULte,

    Gt,
    UGt,

    Gte,
    UGte,

    BitAnd,
    BitOr,
    BitXor,

    Shl,
    Shr,
    UShr,
}

/// Unary operators
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
    Neg { overflowing: bool },
    BitNot,
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expression {
    BinaryExpr {
        loc: Loc,
        operator: BinaryOperator,
        left: Box<Operand>,
        right: Box<Operand>,
    },
    UnaryExpr {
        loc: Loc,
        operator: UnaryOperator,
        right: Box<Operand>,
    },

    Id {
        loc: Loc,
        id: usize,
    },

    BoolLiteral {
        loc: Loc,
        value: bool,
    },
    NumberLiteral {
        loc: Loc,
        value: BigInt,
    },
    ArrayLiteral {
        loc: Loc,
        ty: LIRType,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    ConstArrayLiteral {
        loc: Loc,
        ty: LIRType,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    BytesLiteral {
        loc: Loc,
        ty: LIRType,
        value: Vec<u8>,
    },
    StructLiteral {
        loc: Loc,
        ty: LIRType,
        values: Vec<Operand>,
    },

    Cast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: LIRType,
    },
    BytesCast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: LIRType,
    },
    /// Sign extending the length, only for signed int
    SignExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: LIRType,
    },
    /// Extending the length, only for unsigned int
    ZeroExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: LIRType,
    },
    // Truncating integer into a shorter one
    Trunc {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: LIRType,
    },

    AllocDynamicBytes {
        loc: Loc,
        ty: LIRType,
        size: Box<Operand>,
        initializer: Option<Vec<u8>>,
    },

    /// address-of
    GetRef {
        loc: Loc,
        operand: Box<Operand>,
    },
    /// value-of-address
    Load {
        loc: Loc,
        operand: Box<Operand>,
    },
    /// Used for accessing struct member
    StructMember {
        loc: Loc,
        operand: Box<Operand>,
        member: usize,
    },
    /// Array subscripting: <array>[<index>]
    Subscript {
        loc: Loc,
        arr: Box<Operand>,
        index: Box<Operand>,
    },
    AdvancePointer {
        loc: Loc,
        pointer: Box<Operand>,
        bytes_offset: Box<Operand>,
    },
    // Get the nth param in the current function call stack
    FunctionArg {
        loc: Loc,
        ty: LIRType,
        arg_no: usize,
    },

    FormatString {
        loc: Loc,
        args: Vec<(FormatArg, Operand)>,
    },
    InternalFunctionCfg {
        loc: Loc,
        cfg_no: usize,
    },
    /// Keccak256 hash
    Keccak256 {
        loc: Loc,
        args: Vec<Operand>,
    },
    StringCompare {
        loc: Loc,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },
    StringConcat {
        loc: Loc,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },
    Builtin {
        loc: Loc,
        kind: codegen::Builtin,
        args: Vec<Operand>,
    },

    StorageArrayLength {
        loc: Loc,
        array: Box<Operand>,
    },
    // This is designed for external calls: represents a hard coded mem location.
    ReturnData {
        loc: Loc,
    },

    VectorData {
        pointer: Box<Operand>,
    },
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Add { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)+" } else { "+" })
            }
            BinaryOperator::Sub { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)-" } else { "-" })
            }
            BinaryOperator::Mul { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)*" } else { "*" })
            }
            BinaryOperator::Pow { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)**" } else { "**" })
            }
            BinaryOperator::Div => write!(f, "/"),
            // example: uint8 a = b (u)/ c
            BinaryOperator::UDiv => write!(f, "(u)/"),
            BinaryOperator::Mod => write!(f, "%"),
            BinaryOperator::UMod => write!(f, "(u)%"),
            BinaryOperator::Eq => write!(f, "=="),
            BinaryOperator::Neq => write!(f, "!="),
            BinaryOperator::Lt => write!(f, "<"),
            BinaryOperator::ULt => write!(f, "(u)<"),
            BinaryOperator::Lte => write!(f, "<="),
            BinaryOperator::ULte => write!(f, "(u)<="),
            BinaryOperator::Gt => write!(f, ">"),
            BinaryOperator::UGt => write!(f, "(u)>"),
            BinaryOperator::Gte => write!(f, ">="),
            BinaryOperator::UGte => write!(f, "(u)>="),
            BinaryOperator::BitAnd => write!(f, "&"),
            BinaryOperator::BitOr => write!(f, "|"),
            BinaryOperator::BitXor => write!(f, "^"),
            BinaryOperator::Shl => write!(f, "<<"),
            BinaryOperator::Shr => write!(f, ">>"),
            BinaryOperator::UShr => write!(f, "(u)>>"),
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOperator::Not => write!(f, "!"),
            UnaryOperator::Neg { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)-" } else { "-" })
            }
            UnaryOperator::BitNot => write!(f, "~"),
        }
    }
}

impl Operand {
    /// Get id from operand, panic if operand is not an id
    pub fn get_id_or_error(&self) -> usize {
        match self {
            Operand::Id { id, .. } => *id,
            _ => panic!("Operand is not an id"),
        }
    }

    /// Create a new operand from an id
    pub fn new_id(id: usize, loc: Loc) -> Self {
        Operand::Id { id, loc }
    }

    /// Create a new operand from a bool literal
    pub fn new_bool_literal(value: bool, loc: Loc) -> Self {
        Operand::BoolLiteral { value, loc }
    }

    /// Create a new operand from a number literal
    pub fn new_number_literal(value: &BigInt, ty: LIRType, loc: Loc) -> Self {
        Operand::NumberLiteral {
            loc,
            value: value.clone(),
            ty,
        }
    }
}
