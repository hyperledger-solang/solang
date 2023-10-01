use num_bigint::BigInt;
use solang_parser::pt::Loc;
use crate::sema::ast::{FormatArg, StringLocation, Type};

/// Three-address code type, which is a subset of the Solidity AST
// FIXME Be careful about the data types: pointers, primitives, and references.

/// Three-address code identifier
/// Variable and Literal
#[derive(Clone, Debug)]
pub enum Operand {
    Id { id: usize },
    BoolLiteral { value: bool },
    NumberLiteral { value: BigInt },
    AddressLiteral { value: String },
}

/// binary operators
// LLVM doesn't diff signed and unsigned
pub enum BinaryOperator {
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
pub enum UnaryOperator {
    Not,
    Neg {
        overflow: bool
    },
    BitNot,
}

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
        operand: Box<Operand>,
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
        expr: Box<Operand>,
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