use std::fmt;
use std::fmt::Formatter;
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use crate::sema::ast::{FormatArg, StringLocation};
use crate::ssa_ir::ssa_type::Type;

/// Three-address code type, which is a subset of the Solidity AST
// FIXME Be careful about the data types: pointers, primitives, and references.

/// Three-address code identifier
/// Variable and Literal
#[derive(Clone, Debug)]
pub enum Operand {
    Id { id: usize, name: String },
    BoolLiteral { value: bool },
    NumberLiteral { value: BigInt },
}

/// binary operators
// LLVM doesn't diff signed and unsigned
#[derive(Debug)]
pub enum BinaryOperator {
    Add {
        overflowing: bool
    },
    Sub {
        overflowing: bool
    },
    Mul {
        overflowing: bool
    },
    Pow {
        overflowing: bool
    },

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
    Neg {
        overflowing: bool
    },
    BitNot,
}

#[derive(Debug)]
pub enum Expr {
    BinaryExpr {
        loc: Loc,
        op: BinaryOperator,
        left: Box<Operand>,
        right: Box<Operand>,
    },
    UnaryExpr {
        loc: Loc,
        op: UnaryOperator,
        right: Box<Operand>,
    },

    Id {
        loc: Loc,
        ty: Type,
        var_no: usize,
    },

    /*************************** Constants ***************************/
    ArrayLiteral {
        loc: Loc,
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
        ty: Type,
        op: Box<Operand>,
    },
    BytesCast {
        loc: Loc,
        ty: Type,
        from: Type,
        expr: Box<Operand>,
    },
    // Used for signed integers: int8 -> int16
    // https://en.wikipedia.org/wiki/Sign_extension
    SignExt {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
    },
    // extending the length, only for unsigned int
    ZeroExt {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
    },
    // truncating integer into a shorter one
    Trunc {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
    },

    /*************************** Memory Alloc/Access ***************************/
    AllocDynamicBytes {
        loc: Loc,
        ty: Type,
        size: Box<Operand>,
        initializer: Option<Vec<Operand>>,
    },
    // address-of
    GetRef {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
    },
    // value-of-address
    Load {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
    },
    // Used for accessing struct member
    StructMember {
        loc: Loc,
        ty: Type,
        expr: Box<Operand>,
        member: usize,
    },
    // Array subscripting: <array>[<index>]
    Subscript {
        loc: Loc,
        ty: Type,
        array_ty: Type,
        expr: Box<Operand>,
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
        ty: Type,
        cfg_no: usize,
    },
    // hash function
    Keccak256 {
        loc: Loc,
        ty: Type,
        exprs: Vec<Operand>,
    },
    StringCompare {
        loc: Loc,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },
    StringConcat {
        loc: Loc,
        ty: Type,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },

    /*************************** RPC Calls ***************************/
    // a storage array is in the account
    // this func is a len() function
    StorageArrayLength {
        loc: Loc,
        ty: Type,
        array: Box<Operand>,
        elem_ty: Type,
    },
    // External call: represents a hard coded mem location
    ReturnData {
        loc: Loc,
    },
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Id { id, name } => write!(f, "{}", name),
            Operand::BoolLiteral { value } => write!(f, "{}", value),
            Operand::NumberLiteral { value } => write!(f, "{}", value),
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Add {
                overflowing
            } => write!(f, "{}", if *overflowing { "(of)+" } else { "+" }),
            BinaryOperator::Sub {
                overflowing
            } => write!(f, "{}", if *overflowing { "(of)-" } else { "-" }),
            BinaryOperator::Mul {
                overflowing
            } => write!(f, "{}", if *overflowing { "(of)*" } else { "*" }),
            BinaryOperator::Pow {
                overflowing
            } => write!(f, "{}", if *overflowing { "(of)**" } else { "**" }),
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
            UnaryOperator::Neg {
                overflowing
            } => write!(f, "{}", if *overflowing { "(of)-" } else { "-" }),
            UnaryOperator::BitNot => write!(f, "~"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Expr::BinaryExpr {
                loc, op, left, right
            } => write!(f, "{} {} {}", left, op, right),
            Expr::UnaryExpr {
                loc, op, right
            } => write!(f, "{}{}", op, right),
            Expr::Id { var_no, .. } => write!(f, "%{}", var_no),
            Expr::ArrayLiteral { .. } => todo!("Implement this function"),
            Expr::ConstArrayLiteral { .. } => todo!("Implement this function"),
            Expr::BytesLiteral { .. } => todo!("Implement this function"),
            Expr::StructLiteral { .. } => todo!("Implement this function"),
            Expr::Cast { .. } => todo!("Implement this function"),
            Expr::BytesCast { .. } => todo!("Implement this function"),
            Expr::SignExt { .. } => todo!("Implement this function"),
            Expr::ZeroExt { .. } => todo!("Implement this function"),
            Expr::Trunc { .. } => todo!("Implement this function"),
            Expr::AllocDynamicBytes { .. } => todo!("Implement this function"),
            Expr::GetRef { .. } => todo!("Implement this function"),
            Expr::Load { .. } => todo!("Implement this function"),
            Expr::StructMember { .. } => todo!("Implement this function"),
            Expr::Subscript { .. } => todo!("Implement this function"),
            Expr::AdvancePointer { .. } => todo!("Implement this function"),
            Expr::FunctionArg { .. } => todo!("Implement this function"),
            Expr::FormatString { .. } => todo!("Implement this function"),
            Expr::InternalFunctionCfg { .. } => todo!("Implement this function"),
            Expr::Keccak256 { .. } => todo!("Implement this function"),
            Expr::StringCompare { .. } => todo!("Implement this function"),
            Expr::StringConcat { .. } => todo!("Implement this function"),
            Expr::StorageArrayLength { .. } => todo!("Implement this function"),
            Expr::ReturnData { .. } => todo!("Implement this function"),
        }
    }
}