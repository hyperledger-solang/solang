use std::sync::Arc;
use indexmap::IndexMap;
use num_bigint::BigInt;
use solang_parser::pt::{FunctionTy, Identifier, Loc};
use crate::codegen::cfg::{ArrayLengthVars, ASTFunction, ReturnCode};
use crate::sema::ast::{Parameter, Type};
use crate::ssa_ir::expr::Expr;

pub struct Var {
    id: usize,
    ty: Type,
    name: String
}

pub struct Cfg {// FIXME: need some adjustments on the names and types
    pub name: String,
    pub function_no: ASTFunction,
    // TODO: define a new type for params?
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    pub vars: IndexMap<usize, Var>,
    pub blocks: Vec<Insn>,

    // ...
    pub nonpayable: bool,
    pub public: bool,
    pub ty: FunctionTy,
    pub selector: Vec<u8>,
    current: usize,
    pub array_lengths_temps: ArrayLengthVars,
    pub modifier: Option<usize>,
}

/// Three-address code type, which is a subset of the Solidity AST
// FIXME Be careful about the data types: pointers, primitives, and references.

/// Three-address code identifier
/// Variable and Literal
#[derive(Clone, Debug)]
pub enum Operand {
    Id { id: usize, },
    BoolLiteral { value: bool },
    NumberLiteral { value: BigInt },
    AddressLiteral { value: String }
}

/// binary operators
// LLVM doesn't diff signed and unsigned
pub enum BinaryOp {
    Add {
        overflow: bool
    },
    Sub {
        overflow: bool
    },
    Mul {
        overflow: bool
    },
    Pow {
        overflow: bool
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

/// unary operators
pub enum UnaryOp {
    Not,
    Neg {
        overflow: bool
    },
    BitNot,
}