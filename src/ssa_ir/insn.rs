// SPDX-License-Identifier: Apache-2.0
use crate::codegen;
use crate::sema::ast::CallTy;
use crate::ssa_ir::expr::{Expr, Operand};
use crate::ssa_ir::ssa_type::InternalCallTy;
use solang_parser::pt::Loc;
use std::fmt;
use std::fmt::Formatter;

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
        accounts: Option<Operand>,
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
        callty: CallTy,
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

    // TODO: AccountAccess should be replaced by Subscript

    /*************************** Phi Function ***************************/
    Phi {
        res: usize,
        vars: Vec<PhiInput>,
    },
}

impl fmt::Display for Insn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Insn::Nop => write!(f, "nop;"),
            Insn::ReturnData { data, data_len } => {
                write!(f, "return {} of length {};", data, data_len)
            }
            Insn::ReturnCode { code, .. } => write!(f, "return code \"{}\";", code),
            Insn::Set { res, expr, .. } => {
                write!(f, "%{} = {};", res, expr)
            }
            Insn::Store { dest, data, .. } => {
                write!(f, "store {} to {};", data, dest)
            }
            Insn::PushMemory {
                res, array, value, ..
            } => {
                // %101 = push_mem ptr<int32[10]> %3 uint32(1);
                write!(f, "%{} = push_mem %{} {};", res, array, value)
            }
            Insn::PopMemory { res, array, .. } => {
                // %101 = pop_mem ptr<int32[10]> %3;
                write!(f, "%{} = pop_mem %{};", res, array)
            }
            Insn::Constructor {
                success,
                res,
                contract_no,
                encoded_args,
                gas,
                salt,
                value,
                address,
                seeds,
                accounts,
                constructor_no,
                ..
            } => {
                let lhs = match success {
                    Some(success) => format!("%{}, %{}", res, success),
                    None => format!("%{}, _", res),
                };
                let rhs_constructor = match constructor_no {
                    Some(constructor_no) => {
                        format!(
                            "constructor(no: {}, contract_no:{})",
                            constructor_no, contract_no
                        )
                    }
                    None => format!("constructor(no: _, contract_no:{})", contract_no),
                };
                let rhs_salt = match salt {
                    Some(salt) => format!("salt:{}", salt),
                    None => format!(""),
                };
                let rhs_value = match value {
                    Some(value) => format!("value:{}", value),
                    None => format!(""),
                };
                let rhs_gas = format!("gas:{}", gas);
                let rhs_address = match address {
                    Some(address) => format!("address:{}", address),
                    None => format!(""),
                };
                let rhs_seeds = match seeds {
                    Some(seeds) => format!("seeds:{}", seeds),
                    None => format!(""),
                };
                let rhs_encoded_args = format!("encoded-buffer:{}", encoded_args);
                let rhs_accounts = match accounts {
                    Some(accounts) => format!("accounts:{}", accounts),
                    None => format!(""),
                };
                write!(
                    f,
                    "{} = {} {} {} {} {} {} {} {};",
                    lhs,
                    rhs_constructor,
                    rhs_salt,
                    rhs_value,
                    rhs_gas,
                    rhs_address,
                    rhs_seeds,
                    rhs_encoded_args,
                    rhs_accounts,
                )
            }
            Insn::LoadStorage { res, storage, .. } => {
                write!(f, "%{} = load_storage {};", res, storage)
            }
            Insn::ClearStorage { storage, .. } => {
                write!(f, "clear_storage {};", storage)
            }
            Insn::SetStorage { value, storage, .. } => {
                write!(f, "set_storage {} {};", storage, value)
            }
            Insn::SetStorageBytes {
                value,
                storage,
                offset,
                ..
            } => {
                // set_storage_bytes {} offset:{} value:{}
                write!(
                    f,
                    "set_storage_bytes {} offset:{} value:{};",
                    storage, offset, value
                )
            }
            Insn::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                // "%{} = push storage ty:{} slot:{} = {}",
                let rhs = match value {
                    Some(value) => format!("{}", value),
                    None => format!("empty"),
                };
                write!(f, "%{} = push_storage {} {};", res, storage, rhs)
            }
            Insn::PopStorage { res, storage, .. } =>
            // "%{} = pop storage ty:{} slot({})"
            {
                match res {
                    Some(res) => write!(f, "%{} = pop_storage {};", res, storage),
                    None => write!(f, "pop_storage {};", storage),
                }
            }
            Insn::Call { res, call, args } => {
                // lhs: %0, %1, ...
                let lhs = res
                    .iter()
                    .map(|id| format!("%{}", id))
                    .collect::<Vec<String>>()
                    .join(", ");

                // rhs: call [builtin | static | dynamic] [call] args: %0, %1, ...
                let rhs_call = match call {
                    InternalCallTy::Builtin { ast_func_no, .. } => {
                        format!("builtin#{}", ast_func_no)
                    }
                    InternalCallTy::Static { cfg_no, .. } => format!("function#{}", cfg_no),
                    InternalCallTy::Dynamic(op) => format!("{}", op),
                };

                let rhs_args = args
                    .iter()
                    .map(|arg| format!("{}", arg))
                    .collect::<Vec<String>>()
                    .join(", ");

                write!(f, "{} = call {}({});", lhs, rhs_call, rhs_args)
            }
            Insn::Print { operand, .. } => {
                // "print {}"
                write!(f, "print {};", operand)
            }
            Insn::MemCopy {
                src, dest, bytes, ..
            } => {
                // memcopy %4 from %3 for uint8(11);
                write!(f, "memcopy {} to {} for {} bytes;", src, dest, bytes)
            }
            Insn::ExternalCall {
                success,
                address,
                payload,
                value,
                accounts,
                seeds,
                gas,
                callty,
                contract_function_no,
                flags,
                ..
            } => {
                let lhs = match success {
                    Some(success) => format!("%{} = ", success),
                    None => String::from(""),
                };
                let rhs_address = match address {
                    Some(address) => format!(" address:{}", address),
                    None => String::from(" _"),
                };
                let rhs_accounts = match accounts {
                    Some(accounts) => format!(" accounts:{}", accounts),
                    None => String::from(" _"),
                };
                let rhs_seeds = match seeds {
                    Some(seeds) => format!(" seeds:{}", seeds),
                    None => String::from(" _"),
                };
                let rhs_contract_function_no = match contract_function_no {
                    Some((contract_no, function_no)) => {
                        format!(" contract_no:{}, function_no:{}", contract_no, function_no)
                    }
                    None => String::from(" _"),
                };
                let rhs_flags = match flags {
                    Some(flags) => format!(" flags:{}", flags),
                    None => String::from(" _"),
                };
                // "{} = external call::{} address:{} payload:{} value:{} gas:{} accounts:{} seeds:{} contract|function:{} flags:{}",
                write!(
                    f,
                    "{}call_ext [{}]{}{}{}{}{}{}{}{};",
                    lhs,
                    callty,
                    rhs_address,
                    format!(" payload:{}", payload),
                    format!(" value:{}", value),
                    format!(" gas:{}", gas),
                    rhs_accounts,
                    rhs_seeds,
                    rhs_contract_function_no,
                    rhs_flags
                )
            }
            Insn::ValueTransfer {
                success,
                address,
                value,
                ..
            } => {
                // "%{} = value_transfer {} to {}}",
                let lhs = match success {
                    Some(success) => success.to_string(),
                    None => String::from("_"),
                };
                write!(f, "%{} = transfer {} to {};", lhs, value, address)
            }
            Insn::SelfDestruct { recipient, .. } => {
                // "selfdestruct {}",
                write!(f, "self_destruct {};", recipient)
            }
            Insn::EmitEvent {
                data,
                topics,
                event_no,
                ..
            } => {
                // "emit event#{} to topics[{}], data: {};",
                let rhs_topics = topics
                    .iter()
                    .map(|topic| format!("{}", topic))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(
                    f,
                    "emit event#{} to topics[{}], data: {};",
                    event_no, rhs_topics, data
                )
            }
            Insn::WriteBuffer {
                buf, offset, value, ..
            } => {
                // "write_buf {} offset:{} value:{}",
                write!(f, "write_buf {} offset:{} value:{};", buf, offset, value)
            }
            Insn::Branch { block, .. } => write!(f, "br block#{};", block),
            Insn::BranchCond {
                cond,
                true_block,
                false_block,
                ..
            } => {
                write!(
                    f,
                    "cbr {} block#{} else block#{};",
                    cond, true_block, false_block
                )
            }
            Insn::Switch {
                cond,
                cases,
                default,
                ..
            } => {
                // switch %1 cases: [%4 => block#11, %5 => block#12, %6 => block#13] default: block#14;
                let rhs_cases = cases
                    .iter()
                    .map(|(cond, block)| format!("{} => block#{}", cond, block))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(
                    f,
                    "switch {} cases: [{}] default: block#{};",
                    cond, rhs_cases, default
                )
            }
            Insn::Return { value, .. } => {
                let rhs = value
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "return {};", rhs)
            }
            Insn::AssertFailure { encoded_args, .. } => match encoded_args {
                Some(encoded_args) => {
                    write!(f, "assert_failure {};", encoded_args)
                }
                None => write!(f, "assert_failure;"),
            },
            Insn::Unimplemented { reachable, .. } => {
                write!(
                    f,
                    "unimplemented: {};",
                    if *reachable {
                        "reachable"
                    } else {
                        "unreachable"
                    }
                )
            }
            Insn::Phi { res, vars, .. } => {
                let rhs_vars = vars
                    .iter()
                    .map(|input| format!("{}", input))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "%{} = phi {};", res, rhs_vars)
            }
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
