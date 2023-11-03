// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast;
use crate::ssa_ir::instructions::Insn;
use crate::ssa_ir::printer::Printer;
use crate::ssa_ir::ssa_type::{InternalCallTy, PhiInput};
use std::io::Write;

impl Printer {
    pub fn print_phi(&self, f: &mut dyn Write, phi: &PhiInput) -> std::io::Result<()> {
        write!(f, "[")?;
        self.print_rhs_operand(f, &phi.operand)?;
        write!(f, ", block#{}]", phi.block_no)
    }

    pub fn print_insn(&self, f: &mut dyn Write, insn: &Insn) -> std::io::Result<()> {
        match insn {
            Insn::Nop => write!(f, "nop;"),
            Insn::ReturnData { data, data_len } => {
                write!(f, "return_data ")?;
                self.print_rhs_operand(f, data)?;
                write!(f, " of length ",)?;
                self.print_rhs_operand(f, data_len)?;
                write!(f, ";")
            }
            Insn::ReturnCode { code, .. } => write!(f, "return_code \"{}\";", code),
            Insn::Set { res, expr, .. } => {
                let res_op = self.get_var_operand(res).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = ")?;
                self.print_expr(f, expr)?;
                write!(f, ";")
            }
            Insn::Store { dest, data, .. } => {
                write!(f, "store ")?;
                self.print_rhs_operand(f, data)?;
                write!(f, " to ")?;
                self.print_rhs_operand(f, dest)?;
                write!(f, ";")
            }
            Insn::PushMemory {
                res, array, value, ..
            } => {
                let res_op = self.get_var_operand(res).unwrap();
                let array_op = self.get_var_operand(array).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = push_mem ")?;
                self.print_rhs_operand(f, &array_op)?;
                write!(f, " ")?;
                self.print_rhs_operand(f, value)?;
                write!(f, ";")
            }
            Insn::PopMemory { res, array, .. } => {
                let res_op = self.get_var_operand(res).unwrap();
                let array_op = self.get_var_operand(array).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = pop_mem ")?;
                self.print_rhs_operand(f, &array_op)?;
                write!(f, ";")
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
                // success
                match success {
                    Some(success) => {
                        let res_op = self.get_var_operand(res).unwrap();
                        let success_op = self.get_var_operand(success).unwrap();
                        self.print_lhs_operand(f, &success_op)?;
                        write!(f, ", ")?;
                        self.print_lhs_operand(f, &res_op)?;
                    }
                    None => write!(f, "{}, _", res)?,
                };

                write!(f, " = ")?;

                // constructor
                match constructor_no {
                    Some(constructor_no) => write!(
                        f,
                        "constructor(no: {}, contract_no:{})",
                        constructor_no, contract_no
                    )?,
                    None => write!(f, "constructor(no: _, contract_no:{})", contract_no)?,
                };

                write!(f, " ")?;

                // salt
                match salt {
                    Some(salt) => {
                        write!(f, "salt:")?;
                        self.print_rhs_operand(f, salt)?;
                    }
                    None => write!(f, "salt:_")?,
                };

                write!(f, " ")?;

                // value
                match value {
                    Some(value) => {
                        write!(f, "value:")?;
                        self.print_rhs_operand(f, value)?;
                    }
                    None => write!(f, "value:_")?,
                };

                write!(f, " ")?;

                // gas
                write!(f, "gas:")?;
                self.print_rhs_operand(f, gas)?;

                write!(f, " ")?;

                match address {
                    Some(address) => {
                        write!(f, "address:")?;
                        self.print_rhs_operand(f, address)?;
                    }
                    None => write!(f, "address:_")?,
                };

                write!(f, " ")?;

                match seeds {
                    Some(seeds) => {
                        write!(f, "seeds:")?;
                        self.print_rhs_operand(f, seeds)?;
                    }
                    None => write!(f, "seeds:_")?,
                };

                write!(f, " ")?;

                write!(f, "encoded-buffer:")?;
                self.print_rhs_operand(f, encoded_args)?;

                write!(f, " ")?;

                match accounts {
                    ast::ExternalCallAccounts::NoAccount => write!(f, "accounts:none"),
                    ast::ExternalCallAccounts::Present(acc) => {
                        write!(f, "accounts:")?;
                        self.print_rhs_operand(f, acc)
                    }
                    ast::ExternalCallAccounts::AbsentArgument => write!(f, "accounts:absent"),
                }
            }
            Insn::LoadStorage { res, storage, .. } => {
                let res_op = self.get_var_operand(res).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = load_storage ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, ";")
            }
            Insn::ClearStorage { storage, .. } => {
                write!(f, "clear_storage ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, ";")
            }
            Insn::SetStorage { value, storage, .. } => {
                write!(f, "set_storage ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, " ")?;
                self.print_rhs_operand(f, value)?;
                write!(f, ";")
            }
            Insn::SetStorageBytes {
                value,
                storage,
                offset,
                ..
            } => {
                write!(f, "set_storage_bytes ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, " offset:")?;
                self.print_rhs_operand(f, offset)?;
                write!(f, " value:")?;
                self.print_rhs_operand(f, value)?;
                write!(f, ";")
            }
            Insn::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                let res_op = self.get_var_operand(res).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = push_storage ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, " ")?;
                match value {
                    Some(value) => self.print_rhs_operand(f, value)?,
                    None => write!(f, "empty")?,
                };
                write!(f, ";")
            }
            Insn::PopStorage { res, storage, .. } => match res {
                Some(res) => {
                    let res_op = self.get_var_operand(res).unwrap();
                    self.print_lhs_operand(f, &res_op)?;
                    write!(f, " = pop_storage ")?;
                    self.print_rhs_operand(f, storage)?;
                    write!(f, ";")
                }
                None => {
                    write!(f, "pop_storage ")?;
                    self.print_rhs_operand(f, storage)?;
                    write!(f, ";")
                }
            },
            Insn::Call { res, call, args } => {
                // lhs: %0, %1, ...
                for (i, id) in res.iter().enumerate() {
                    let res_op = self.get_var_operand(id).unwrap();
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.print_lhs_operand(f, &res_op)?;
                }

                write!(f, " = call ")?;

                match call {
                    InternalCallTy::Builtin { ast_func_no, .. } => {
                        write!(f, "builtin#{}", ast_func_no)?
                    }
                    InternalCallTy::Static { cfg_no, .. } => write!(f, "function#{}", cfg_no)?,
                    InternalCallTy::Dynamic(op) => self.print_rhs_operand(f, op)?,
                };

                write!(f, "(")?;

                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.print_rhs_operand(f, arg)?;
                }

                write!(f, ");")
            }
            Insn::Print { operand, .. } => {
                write!(f, "print ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, ";")
            }
            Insn::MemCopy {
                src, dest, bytes, ..
            } => {
                write!(f, "memcopy ")?;
                self.print_rhs_operand(f, src)?;
                write!(f, " to ")?;
                self.print_rhs_operand(f, dest)?;
                write!(f, " for ")?;
                self.print_rhs_operand(f, bytes)?;
                write!(f, " bytes;")
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
                // {} = call_ext ty:{} address:{} payload:{} value:{} gas:{} accounts:{} seeds:{} contract_no:{}, function_no:{} flags:{};
                match success {
                    Some(success) => {
                        let success_op = self.get_var_operand(success).unwrap();
                        self.print_lhs_operand(f, &success_op)?;
                    }
                    None => write!(f, "_")?,
                };

                write!(f, " = call_ext [{}] ", callty)?;

                match address {
                    Some(address) => {
                        write!(f, "address:")?;
                        self.print_rhs_operand(f, address)?;
                    }
                    None => write!(f, "address:_")?,
                };

                write!(f, " ")?;

                write!(f, "payload:")?;
                self.print_rhs_operand(f, payload)?;

                write!(f, " ")?;

                write!(f, "value:")?;
                self.print_rhs_operand(f, value)?;

                write!(f, " ")?;

                write!(f, "gas:")?;
                self.print_rhs_operand(f, gas)?;

                write!(f, " ")?;

                match accounts {
                    ast::ExternalCallAccounts::NoAccount => write!(f, "accounts:none")?,
                    ast::ExternalCallAccounts::Present(acc) => {
                        write!(f, "accounts:")?;
                        self.print_rhs_operand(f, acc)?;
                    }
                    ast::ExternalCallAccounts::AbsentArgument => write!(f, "accounts:absent")?,
                };

                write!(f, " ")?;

                match seeds {
                    Some(seeds) => {
                        write!(f, "seeds:")?;
                        self.print_rhs_operand(f, seeds)?;
                    }
                    None => write!(f, "seeds:_")?,
                };

                write!(f, " ")?;

                match contract_function_no {
                    Some((contract_no, function_no)) => {
                        write!(
                            f,
                            "contract_no:{}, function_no:{}",
                            contract_no, function_no
                        )?;
                    }
                    None => write!(f, "contract_no:_, function_no:_")?,
                };

                write!(f, " ")?;

                match flags {
                    Some(flags) => {
                        write!(f, "flags:")?;
                        self.print_rhs_operand(f, flags)?;
                    }
                    None => write!(f, "flags:_")?,
                }

                write!(f, ";")
            }
            Insn::ValueTransfer {
                success,
                address,
                value,
                ..
            } => {
                match success {
                    Some(success) => {
                        let success_op = self.get_var_operand(success).unwrap();
                        self.print_lhs_operand(f, &success_op)?;
                    }
                    None => write!(f, "_")?,
                };
                write!(f, " = value_transfer ")?;
                self.print_rhs_operand(f, value)?;
                write!(f, " to ")?;
                self.print_rhs_operand(f, address)?;
                write!(f, ";")
            }
            Insn::SelfDestruct { recipient, .. } => {
                write!(f, "self_destruct ")?;
                self.print_rhs_operand(f, recipient)?;
                write!(f, ";")
            }
            Insn::EmitEvent {
                data,
                topics,
                event_no,
                ..
            } => {
                write!(f, "emit event#{} to topics[", event_no)?;
                for (i, topic) in topics.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.print_rhs_operand(f, topic)?;
                }
                write!(f, "], data: ")?;
                self.print_rhs_operand(f, data)?;
                write!(f, ";")
            }
            Insn::WriteBuffer {
                buf, offset, value, ..
            } => {
                write!(f, "write_buf ")?;
                self.print_rhs_operand(f, buf)?;
                write!(f, " offset:")?;
                self.print_rhs_operand(f, offset)?;
                write!(f, " value:")?;
                self.print_rhs_operand(f, value)?;
                write!(f, ";")
            }
            Insn::Branch { block, .. } => write!(f, "br block#{};", block),
            Insn::BranchCond {
                cond,
                true_block,
                false_block,
                ..
            } => {
                write!(f, "cbr ")?;
                self.print_rhs_operand(f, cond)?;
                write!(f, " block#{} else block#{};", true_block, false_block)
            }
            Insn::Switch {
                cond,
                cases,
                default,
                ..
            } => {
                write!(f, "switch ")?;
                self.print_rhs_operand(f, cond)?;
                write!(f, ":")?;
                for (i, (cond, block)) in cases.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\n    case:    ")?;
                    self.print_rhs_operand(f, cond)?;
                    write!(f, " => block#{}", block)?;
                }
                write!(f, "\n    default: block#{};", default)
            }
            Insn::Return { value, .. } => {
                write!(f, "return")?;
                for (i, value) in value.iter().enumerate() {
                    if i == 0 {
                        write!(f, " ")?;
                    } else {
                        write!(f, ", ")?;
                    }
                    self.print_rhs_operand(f, value)?;
                }
                write!(f, ";")
            }
            Insn::AssertFailure { encoded_args, .. } => match encoded_args {
                Some(encoded_args) => {
                    write!(f, "assert_failure ")?;
                    self.print_rhs_operand(f, encoded_args)?;
                    write!(f, ";")
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
                let res_op = self.get_var_operand(res).unwrap();
                self.print_lhs_operand(f, &res_op)?;
                write!(f, " = phi ")?;
                for (i, var) in vars.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.print_phi(f, var)?;
                }
                write!(f, ";")
            }
        }
    }
}
