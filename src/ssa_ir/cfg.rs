use std::sync::Arc;
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_rational::BigRational;
use solang_parser::pt::{FunctionTy, Identifier, Loc};
use crate::codegen::cfg::{ArrayLengthVars, ASTFunction, ReturnCode};
use crate::sema::ast::{FormatArg, Parameter, StringLocation, Type};

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
    BoolLiteral { val: bool },
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

pub enum Expr {
    BinaryExpr {
        loc: Loc,
        op: BinaryOp,
        left: Box<Operand>,
        right: Box<Operand>,
    },
    UnaryExpr {
        loc: Loc,
        op: UnaryOp,
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

/// Statements using three-address code format
pub enum Insn {
    Nop,

    /*************************** Contract As Callee ***************************/
    // Return data to the outside callers
    ReturnData {
        data: Operand,
        data_len: Operand,
    },
    ReturnCode {
        code: ReturnCode
    },

    /*************************** Memory Alloc/Access ***************************/
    // Set variable
    Set {
        loc: Loc,
        res: usize,
        expr: Expr,
    },
    Store {
        dest: Operand,
        data: Operand,
    },
    PushMemory {
        res: usize,
        ty: Type,
        array: usize,
        value: Box<Operand>,
    },
    PopMemory {
        res: usize,
        ty: Type,
        array: usize,
        loc: Loc,
    },
    Constructor {
        success: Option<usize>,
        res: usize,
        contract_no: usize,
        constructor_no: Option<usize>,
        encoded_args: Operand,
        value: Option<Operand>,
        gas: Operand,
        salt: Option<Operand>,
        address: Option<Operand>,
        seeds: Option<Operand>,
        accounts: Option<Operand>,
        loc: Loc,
    },

    /*************************** Storage Access ***************************/
    LoadStorage {
        res: usize,
        ty: Type,
        storage: Operand,
    },
    ClearStorage {
        ty: Type,
        storage: Operand,
    },
    SetStorage {
        ty: Type,
        value: Operand,
        storage: Operand,
    },
    SetStorageBytes {
        value: Operand,
        storage: Operand,
        offset: Operand,
    },
    PushStorage {
        res: usize,
        ty: Type,
        value: Option<Operand>,
        storage: Operand,
    },
    PopStorage {
        res: Option<usize>,
        ty: Type,
        storage: Operand,
    },

    /*************************** Function Calls ***************************/
    // Call internal function, either static dispatch or dynamic dispatch
    Call {
        res: Vec<usize>,
        return_tys: Vec<Type>,
        call: Identifier,
        args: Vec<Operand>,
    },
    // Print to log message
    Print {
        expr: Operand
    },
    MemCopy {
        source: Operand,
        destination: Operand,
        bytes: Operand,
    },

    /*************************** External Calls ***************************/
    ExternalCall {
        loc: Loc,
        // Polkadot specific
        success: Option<usize>,
        address: Option<Operand>,
        accounts: Option<Operand>,
        // Solana specific
        // for deriving and proving the ownership of an account
        seeds: Option<Operand>,
        payload: Operand,
        // Polkadot specific
        // holding tokens
        value: Operand,
        // Polkadot specific
        // On Solana, charged by transaction
        gas: Operand,
        // TODO: What is callty? delegate/regular/static
        callty: Operand,
        // only used for analysis passes
        contract_function_no: Option<(usize, usize)>,
        // Polkadot specific
        // TODO: ask on discord
        flags: Option<Operand>,
    },
    /// Value transfer; either address.send() or address.transfer()
    // transfer tokens from one addr to another
    ValueTransfer {
        success: Option<usize>,
        address: Operand,
        value: Operand,
    },
    /// Self destruct
    // for destructing the contract from inside
    // Note: only available on Polkadot
    SelfDestruct {
        recipient: Operand
    },
    EmitEvent {
        event_no: usize,
        data: Operand,
        topics: Vec<Operand>,
    },
    WriteBuffer {
        buf: Operand,
        offset: Operand,
        value: Operand,
    },

    /*************************** Branching ***************************/
    Branch {
        block: usize
    },
    BranchCond {
        cond: Operand,
        true_block: usize,
        false_block: usize,
    },
    Switch {
        cond: Operand,
        cases: Vec<(Operand, usize)>,
        default: usize,
    },
    Return {
        value: Vec<Operand>
    },

    /*************************** Error Ctl ***************************/
    AssertFailure {
        encoded_args: Option<Operand>
    },
    Unimplemented {
        reachable: bool
    },

    // TODO: AccountAccess should be replaced by Subscript

    /*************************** Phi Function ***************************/
    Phi {
        vars: Vec<usize>
    },
}

// impl Insn {
//     pub fn test() -> Self {
//         // translate: int x = a + b + c; to three-address code
//         let plus_a_b = Insn::Set {
//             loc: Loc::Codegen,
//             res: 3,
//             expr: Expr::BinaryExpr {
//                 loc: Loc::Codegen,
//                 op: BinaryOp::Add {
//                     overflow: false
//                 },
//                 left: Box::new(Operand::Var {
//                     id: 0,
//                     ty: Type::Int(32)
//                 }),
//                 right: Box::new(Operand::Var {
//                     id: 1,
//                     ty: Type::Int(32)
//                 }),
//             }
//         };
//         let plus_a_b_c = Insn::Set {
//             loc: Loc::Codegen,
//             res: 4,
//             expr: Expr::BinaryExpr {
//                 loc: Loc::Codegen,
//                 op: BinaryOp::Add {
//                     overflow: false
//                 },
//                 left: Box::new(Operand::Var {
//                     id: 2,
//                     ty: Type::Int(32)
//                 }),
//                 right: Box::new(Operand::Var {
//                     id: 3,
//                     ty: Type::Int(32)
//                 }),
//             }
//         };
//     }
// }