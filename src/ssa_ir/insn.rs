use solang_parser::pt::Loc;
use crate::codegen::cfg::ReturnCode;
use crate::sema::ast::Type;
use crate::ssa_ir::expr::{Expr, Operand};

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
        call: Operand,
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

impl Insn {
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
}