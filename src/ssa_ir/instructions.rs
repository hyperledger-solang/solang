// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::sema::ast::{CallTy, ExternalCallAccounts};
use crate::ssa_ir::expressions::{Expr, Operand};
use crate::ssa_ir::ssa_type::InternalCallTy;
use solang_parser::pt::Loc;

use super::ssa_type::PhiInput;

/// Statements using three-address code format
#[derive(Debug)]
pub enum Insn {
    Nop,

    /*************************** Contract As Callee ***************************/
    // Return data to the outside callers
    ReturnData {
        data: Operand,
        data_len: Operand,
    },
    ReturnCode {
        code: codegen::cfg::ReturnCode,
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
        array: usize,
        value: Operand,
    },
    PopMemory {
        res: usize,
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
        accounts: ExternalCallAccounts<Operand>,
        loc: Loc,
    },

    /*************************** Storage Access ***************************/
    LoadStorage {
        res: usize,
        storage: Operand,
    },
    ClearStorage {
        storage: Operand,
    },
    SetStorage {
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
        value: Option<Operand>,
        storage: Operand,
    },
    PopStorage {
        res: Option<usize>,
        storage: Operand,
    },

    /*************************** Function Calls ***************************/
    // Call internal function, either static dispatch or dynamic dispatch
    Call {
        res: Vec<usize>,
        call: InternalCallTy,
        args: Vec<Operand>,
    },
    // Print to log message
    Print {
        operand: Operand,
    },
    MemCopy {
        src: Operand,
        dest: Operand,
        bytes: Operand,
    },

    /*************************** External Calls ***************************/
    ExternalCall {
        loc: Loc,
        // Polkadot specific
        success: Option<usize>,
        address: Option<Operand>,
        accounts: ExternalCallAccounts<Operand>,
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
        // delegate/regular/static
        // CallTy is polkadot specific.
        // It involves difference code generation in emit.
        callty: CallTy,
        // only used for analysis passes
        contract_function_no: Option<(usize, usize)>,
        // Polkadot specific
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
        recipient: Operand,
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
        block: usize,
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
        value: Vec<Operand>,
    },

    /*************************** Error Ctl ***************************/
    AssertFailure {
        encoded_args: Option<Operand>,
    },
    Unimplemented {
        reachable: bool,
    },

    // AccountAccess should be replaced by Subscript

    /*************************** Phi Function ***************************/
    Phi {
        res: usize,
        vars: Vec<PhiInput>,
    },
}
