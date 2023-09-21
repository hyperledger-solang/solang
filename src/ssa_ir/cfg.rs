use solang_parser::pt::{Identifier, Loc};

/// Three-address code type, which is a subset of the Solidity AST
/// This is a subset of the Solidity AST
/// Use struct to define the three-address code type
#[derive(Clone)]
pub enum TacType {
    // primitive types
    Bool,
    Int(u16, bool),
    Bytes(u16),
}

impl TacType {
    pub fn clone(&self) -> Self {
        match self {
            Self::Bool => Self::Bool,
            Self::Int(width, signed) => Self::Int(*width, *signed),
            Self::Bytes(width) => Self::Bytes(*width),
        }
    }

    pub fn get_type_size(&self) -> u16 {
        match self {
            TacType::Int(n, _) => *n,
            TacType::Bool => 1,
            _ => unimplemented!("size of type not known"),
        }
    }
}

/// Three-address code identifier
#[derive(Clone)]
pub struct TacId {
    pub id: usize,
    pub ty: TacType,
}

impl TacId {
    pub fn new(id: usize, ty: TacType) -> Self {
        Self { id, ty }
    }

    fn clone(&self) -> Self {
        Self {
            id: self.id,
            ty: self.ty.clone(),
        }
    }
}

/// Be careful about the data types: pointers, primitives, and references.
/// binary operations
pub enum Bop {
    Add, Sub, Mul, Div, UDiv, Mod, UMod, Pow,
    Eq, Neq,
    Lt, Lte,
    Gt, Gte,

    BitAnd, BitOr, BitXor,

    Shl, Shr,
}
pub struct Binop {
    pub loc: Loc,
    pub bop: Bop,
    pub left: Box<TacId>,
    pub right: Box<TacId>,
    pub overflow: bool,
    pub signed: bool
}
pub enum TacBinop {
    Add{
        loc: Loc,
        overflowing: bool,
        left: TacId,
        right: TacId,
    },
    Sub {
        loc: Loc,
        overflowing: bool,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Mul {
        loc: Loc,
        overflowing: bool,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Div {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    UDiv {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Mod {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    UMod {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Pow {
        loc: Loc,
        overflowing: bool,
        base: Box<TacId>,
        exp: Box<TacId>,
    },
    IAnd {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    IOr {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    IXor {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Eq {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Neq {
        loc: Loc,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Lt {
        loc: Loc,
        signed: bool,
        left: Box<TacId>,
        right: Box<TacId>,
    },
    Lte {
        loc: Loc,
        signed: bool,
        left: Box<TacId>,
        right: Box<TacId>,
    },
}

/// Control Flow Graph using three-address code format
pub enum TacInstr {
    Set {
        loc: Loc,
        res: usize,
        op: TacId,
    },
    Call {
        res: Vec<usize>,
        return_tys: Vec<TacType>,
        call: Identifier,
        args: Vec<TacId>,
    },
    Return { value: Vec<TacId> },
    BranchCond {
        cond: TacId,
        true_block: usize,
        false_block: usize,
    },
    Store { dest: TacId, data: TacId },
    AssertFailure { encoded_args: Option<TacId> },
    /// Print to log message
    Print { expr: TacId },
}