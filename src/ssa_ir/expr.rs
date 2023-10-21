// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{FormatArg, StringLocation};
use crate::ssa_ir::ssa_type::Type;
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use std::fmt;
use std::fmt::Formatter;

/// Three-address code type, which is a subset of the Solidity AST
// FIXME Be careful about the data types: pointers, primitives, and references.

/// Three-address code identifier
/// Variable and Literal
#[derive(Clone, Debug)]
pub enum Operand {
    Id { id: usize },
    BoolLiteral { value: bool },
    NumberLiteral { value: BigInt, ty: Type },
}

/// binary operators
// LLVM doesn't diff signed and unsigned
#[derive(Debug)]
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

#[derive(Debug)]
/// unary operators
pub enum UnaryOperator {
    Not,
    Neg { overflowing: bool },
    BitNot,
}

#[derive(Debug)]
pub enum Expr {
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

    /*************************** Constants ***************************/
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
        // Dynamic type in array literal is impossible
        ty: Type,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    ConstArrayLiteral {
        loc: Loc,
        ty: Type,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    BytesLiteral {
        loc: Loc,
        ty: Type,
        value: Vec<u8>,
    },
    StructLiteral {
        loc: Loc,
        ty: Type,
        values: Vec<Operand>,
    },

    /*************************** Casts ***************************/
    Cast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    BytesCast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // Used for signed integers: int8 -> int16
    // https://en.wikipedia.org/wiki/Sign_extension
    SignExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // extending the length, only for unsigned int
    ZeroExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // truncating integer into a shorter one
    Trunc {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },

    /*************************** Memory Alloc ***************************/
    AllocDynamicBytes {
        loc: Loc,
        ty: Type,
        size: Box<Operand>,
        initializer: Option<Vec<u8>>,
    },

    /*************************** Memory Access ***************************/
    // address-of
    GetRef {
        loc: Loc,
        operand: Box<Operand>,
    },
    // value-of-address
    Load {
        loc: Loc,
        operand: Box<Operand>,
    },
    // Used for accessing struct member
    StructMember {
        loc: Loc,
        operand: Box<Operand>,
        member: usize,
    },
    // Array subscripting: <array>[<index>]
    Subscript {
        loc: Loc,
        arr: Box<Operand>,
        index: Box<Operand>,
    },
    // [b1, b2, b3]
    AdvancePointer {
        pointer: Box<Operand>,
        bytes_offset: Box<Operand>,
    },
    // get the nth param in the current function call stack
    FunctionArg {
        loc: Loc,
        ty: Type,
        arg_no: usize,
    },

    /*************************** Function Calls ***************************/
    FormatString {
        loc: Loc,
        args: Vec<(FormatArg, Operand)>,
    },
    InternalFunctionCfg {
        cfg_no: usize,
    },
    // hash function
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

    /*************************** RPC Calls ***************************/
    // a storage array is in the account
    // this func is a len() function
    StorageArrayLength {
        loc: Loc,
        array: Box<Operand>,
    },
    // External call: represents a hard coded mem location
    ReturnData {
        loc: Loc,
    },
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Id { id, .. } => write!(f, "%{}", id),
            Operand::BoolLiteral { value } => write!(f, "{}", value),
            Operand::NumberLiteral { value, ty } => write!(f, "{}({})", ty, value),
        }
    }
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
    pub fn get_id_or_err(&self) -> Result<usize, &'static str> {
        match self {
            Operand::Id { id } => Ok(*id),
            _ => Err("Operand is not an id"),
        }
    }
}
