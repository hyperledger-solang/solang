// SPDX-License-Identifier: Apache-2.0

use crate::codegen;
use crate::lir::expressions::{Expression, Operand};
use crate::lir::lir_type::InternalCallTy;
use crate::sema::ast::{CallTy, ExternalCallAccounts};
use solang_parser::pt::Loc;

use super::lir_type::PhiInput;

/// Instructions using three-address code format
#[derive(Debug)]
pub enum Instruction {
    Nop,

    /// Return data to the outside callers
    ReturnData {
        loc: Loc,
        data: Operand,
        data_len: Operand,
    },
    ReturnCode {
        loc: Loc,
        code: codegen::cfg::ReturnCode,
    },

    Set {
        loc: Loc,
        res: usize,
        expr: Expression,
    },
    Store {
        loc: Loc,
        dest: Operand,
        data: Operand,
    },
    PushMemory {
        loc: Loc,
        res: usize,
        array: usize,
        value: Operand,
    },
    PopMemory {
        loc: Loc,
        res: usize,
        array: usize,
    },
    Constructor {
        loc: Loc,
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
    },

    LoadStorage {
        loc: Loc,
        res: usize,
        storage: Operand,
    },
    ClearStorage {
        loc: Loc,
        storage: Operand,
    },
    SetStorage {
        loc: Loc,
        value: Operand,
        storage: Operand,
    },
    SetStorageBytes {
        loc: Loc,
        value: Operand,
        storage: Operand,
        offset: Operand,
    },
    PushStorage {
        loc: Loc,
        res: usize,
        value: Option<Operand>,
        storage: Operand,
    },
    PopStorage {
        loc: Loc,
        res: Option<usize>,
        storage: Operand,
    },

    Call {
        loc: Loc,
        res: Vec<usize>,
        call: InternalCallTy,
        args: Vec<Operand>,
    },
    /// Print to log message
    Print {
        loc: Loc,
        operand: Operand,
    },
    MemCopy {
        loc: Loc,
        src: Operand,
        dest: Operand,
        bytes: Operand,
    },

    ExternalCall {
        loc: Loc,
        /// Polkadot specific
        success: Option<usize>,
        address: Option<Operand>,
        accounts: ExternalCallAccounts<Operand>,
        /// Solana specific:
        /// for deriving and proving the ownership of an account
        seeds: Option<Operand>,
        payload: Operand,
        /// Polkadot specific:
        /// holding tokens
        value: Operand,
        /// Polkadot specific.
        /// On Solana, charged by transaction
        gas: Operand,
        /// CallTy is polkadot specific:
        /// It involves difference code generation in emit.
        callty: CallTy,
        /// only used for analysis passes
        contract_function_no: Option<(usize, usize)>,
        /// Polkadot specific
        flags: Option<Operand>,
    },
    /// Value transfer; either address.send() or address.transfer()
    /// transfer tokens from one addr to another
    ValueTransfer {
        loc: Loc,
        success: Option<usize>,
        address: Operand,
        value: Operand,
    },
    /// Self destruct
    /// for destructing the contract from inside.
    /// Note: only available on Polkadot
    SelfDestruct {
        loc: Loc,
        recipient: Operand,
    },
    EmitEvent {
        loc: Loc,
        event_no: usize,
        data: Operand,
        topics: Vec<Operand>,
    },
    WriteBuffer {
        loc: Loc,
        buf: Operand,
        offset: Operand,
        value: Operand,
    },

    Branch {
        loc: Loc,
        block: usize,
    },
    BranchCond {
        loc: Loc,
        cond: Operand,
        true_block: usize,
        false_block: usize,
    },
    Switch {
        loc: Loc,
        cond: Operand,
        cases: Vec<(Operand, usize)>,
        default: usize,
    },
    Return {
        loc: Loc,
        value: Vec<Operand>,
    },

    AssertFailure {
        loc: Loc,
        encoded_args: Option<Operand>,
    },

    Phi {
        loc: Loc,
        res: usize,
        vars: Vec<PhiInput>,
    },
}
