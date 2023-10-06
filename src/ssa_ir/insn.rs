use std::fmt;
use std::fmt::Formatter;
use solang_parser::pt::Loc;
use crate::codegen;
use crate::sema::ast::Type;
use crate::ssa_ir::expr::{Expr, Operand};

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
        code: codegen::cfg::ReturnCode
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
        value: Operand,
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

impl fmt::Display for Insn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Insn::Nop => write!(f, "nop"),
            Insn::ReturnData { data, data_len, } => {
                write!(f, "return {} of length {};", data, data_len)
            }
            Insn::ReturnCode { .. } => todo!("ReturnCode"),
            Insn::Set { .. } => todo!("Set"),
            Insn::Store { .. } => todo!("Store"),
            Insn::PushMemory { .. } => todo!("PushMemory"),
            Insn::PopMemory { .. } => todo!("PopMemory"),
            Insn::Constructor { .. } => todo!("Constructor"),
            Insn::LoadStorage { .. } => todo!("LoadStorage"),
            Insn::ClearStorage { .. } => todo!("ClearStorage"),
            Insn::SetStorage { .. } => todo!("SetStorage"),
            Insn::SetStorageBytes { .. } => todo!("SetStorageBytes"),
            Insn::PushStorage { .. } => todo!("PushStorage"),
            Insn::PopStorage { .. } => todo!("PopStorage"),
            Insn::Call { .. } => todo!("Call"),
            Insn::Print { .. } => todo!("Print"),
            Insn::MemCopy { .. } => todo!("MemCopy"),
            Insn::ExternalCall { .. } => todo!("ExternalCall"),
            Insn::ValueTransfer { .. } => todo!("ValueTransfer"),
            Insn::SelfDestruct { .. } => todo!("SelfDestruct"),
            Insn::EmitEvent { .. } => todo!("EmitEvent"),
            Insn::WriteBuffer { .. } => todo!("WriteBuffer"),
            Insn::Branch { .. } => todo!("Branch"),
            Insn::BranchCond { .. } => todo!("BranchCond"),
            Insn::Switch { .. } => todo!("Switch"),
            Insn::Return { .. } => todo!("Return"),
            Insn::AssertFailure { .. } => todo!("AssertFailure"),
            Insn::Unimplemented { .. } => todo!("Unimplemented"),
            Insn::Phi { .. } => todo!("Phi")
        }
    }
}

// pub type Insns = Vec<Insn>;

// impl Insns {
//
//     fn try_from_instr_nop() -> Result<Self, &'static str> {
//         Ok(Insns(vec![Insn::Nop]))
//     }
//
//     fn try_from_instr_set(loc: &Loc, res: &usize, expr: &Expression) -> Result<Self, &'static str> {
//         // [t] a = b + c * d
//         // translates to:
//         //   1. [t1] tmp_1 = c * d;
//         //   2. [t2] tmp_2 = b + tmp_1
//         //   3. [t] a = tmp_2;
//         let (mut insns, operand) = InsnsAndOperand::try_from(expr)?;
//         insns.push(
//             Set {
//                 loc: loc.clone(),
//                 res: res.clone(),
//                 expr: Expr::Cast {
//                     // FIXME: need to retrieve the variable type from var table
//                     ty: Type::Int(16),
//                     loc: Loc::Codegen,
//                     op: Box::new(operand)
//                 }
//             }
//         );
//
//         Ok(Insns(insns))
//     }
//
//     fn try_from_instr_store(p0: &Expression, p1: &Expression) -> Result<Insns, &'static str> {
//         todo!()
//     }
//
//     fn try_from_instr_push_memory(p0: &usize, p1: &Type, p2: &usize, p3: &Box<Expression>) -> Result<Insns, &'static str> {
//         todo!()
//     }
//
//     fn try_from_instr_pop_memory(p0: &usize, p1: &Type, p2: &usize, p3: &Loc) -> Result<Insns, &'static str> {
//         todo!()
//     }
//
//     fn try_from_instr_constructor(p0: &Option<usize>, p1: &usize, p2: &usize, p3: &Option<usize>, p4: &Expression, p5: &Option<Expression>, p6: &Expression, p7: &Option<Expression>, p8: &Option<Expression>, p9: &Option<Expression>, p10: &Option<Expression>, p11: &Loc) -> Result<Insns, &'static str> {
//         todo!()
//     }
// }

// impl TryFrom<&Instr> for Insns {
//     type Error = &'static str;
//
//     fn try_from(value: &Instr) -> Result<Self, Self::Error> {
//         match value {
//             Instr::Nop =>
//                 Insns::try_from_instr_nop(),
//             Instr::Set {
//                 loc,
//                 res,
//                 expr
//             } => Insns::try_from_instr_set(loc, res, expr),
//             Instr::Store {
//                 dest,
//                 data
//             } => Insns::try_from_instr_store(dest, data),
//             Instr::PushMemory {
//                 res,
//                 ty,
//                 array,
//                 value
//             } => Insns::try_from_instr_push_memory(res, ty, array, value),
//             Instr::PopMemory {
//                 res,
//                 ty,
//                 array,
//                 loc
//             } => Insns::try_from_instr_pop_memory(res, ty, array, loc),
//             Instr::Constructor {
//                 success,
//                 res,
//                 contract_no,
//                 constructor_no,
//                 encoded_args,
//                 value,
//                 gas,
//                 salt,
//                 address,
//                 seeds,
//                 accounts,
//                 loc } =>
//                 Insns::try_from_instr_constructor(
//                     success, res, contract_no, constructor_no,
//                     encoded_args, value, gas, salt,
//                     address, seeds, accounts, loc),
//             _ => Err("Not implemented yet")
//         }
//     }
// }

// pub type InsnsAndOperand = (Vec<Insn>, Operand);
// impl TryFrom<&Expression> for InsnsAndOperand {
//     type Error = &'static str;
//
//     fn try_from(value: &Expression) -> Result<Self, Self::Error> {
//         // InsnsAndOperand::try_from(b + c * d) gives:
//         //   insn1: [t1] tmp_1 = c * d;
//         //   insn2: [t2] tmp_2 = b + tmp_1
//         //   return [insn1, insn2], tmp_2
//         todo!()
//     }
// }

// impl Insn {

    // Create a new three-address code instruction from old Instr type
    // One Instr may be translated into multiple Insn
    // pub fn from(instr: &Instr) -> Vec<Insn> {
    //     match instr {
    //         Instr::Nop => vec![Insn::Nop],
    //         _ => unimplemented!()
    //     }
    // }

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