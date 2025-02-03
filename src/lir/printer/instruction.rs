// SPDX-License-Identifier: Apache-2.0

use crate::lir::instructions::Instruction;
use crate::lir::lir_type::{InternalCallTy, PhiInput};
use crate::lir::printer::Printer;
use crate::sema::ast;
use std::io::Write;

impl Printer<'_> {
    pub fn print_phi(&self, f: &mut dyn Write, phi: &PhiInput) {
        write!(f, "[").unwrap();
        self.print_rhs_operand(f, &phi.operand);
        write!(f, ", block#{}]", phi.block_no).unwrap();
    }

    pub fn print_instruction(&self, f: &mut dyn Write, insn: &Instruction) {
        match insn {
            Instruction::Nop => write!(f, "nop;").unwrap(),
            Instruction::ReturnData { data, data_len, .. } => {
                write!(f, "return_data ").unwrap();
                self.print_rhs_operand(f, data);
                write!(f, " of length ",).unwrap();
                self.print_rhs_operand(f, data_len);
                write!(f, ";").unwrap();
            }
            Instruction::ReturnCode { code, .. } => write!(f, "return_code \"{}\";", code).unwrap(),
            Instruction::Set { res, expr, .. } => {
                let res_op = self.get_var_operand(res);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = ").unwrap();
                self.print_expr(f, expr);
                write!(f, ";").unwrap();
            }
            Instruction::Store { dest, data, .. } => {
                write!(f, "store ").unwrap();
                self.print_rhs_operand(f, data);
                write!(f, " to ").unwrap();
                self.print_rhs_operand(f, dest);
                write!(f, ";").unwrap();
            }
            Instruction::PushMemory {
                res, array, value, ..
            } => {
                let res_op = self.get_var_operand(res);
                let array_op = self.get_var_operand(array);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = push_mem ").unwrap();
                self.print_rhs_operand(f, &array_op);
                write!(f, " ").unwrap();
                self.print_rhs_operand(f, value);
                write!(f, ";").unwrap();
            }
            Instruction::PopMemory { res, array, .. } => {
                let res_op = self.get_var_operand(res);
                let array_op = self.get_var_operand(array);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = pop_mem ").unwrap();
                self.print_rhs_operand(f, &array_op);
                write!(f, ";").unwrap();
            }
            Instruction::Constructor {
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
                        let res_op = self.get_var_operand(res);
                        let success_op = self.get_var_operand(success);
                        self.print_lhs_operand(f, &success_op);
                        write!(f, ", ").unwrap();
                        self.print_lhs_operand(f, &res_op);
                    }
                    None => write!(f, "{}, _", res).unwrap(),
                };

                write!(f, " = ").unwrap();

                // constructor
                match constructor_no {
                    Some(constructor_no) => write!(
                        f,
                        "constructor(no: {}, contract_no:{})",
                        constructor_no, contract_no
                    )
                    .unwrap(),
                    None => write!(f, "constructor(no: _, contract_no:{})", contract_no).unwrap(),
                };

                write!(f, " ").unwrap();

                // salt
                match salt {
                    Some(salt) => {
                        write!(f, "salt:").unwrap();
                        self.print_rhs_operand(f, salt);
                    }
                    None => write!(f, "salt:_").unwrap(),
                };

                write!(f, " ").unwrap();

                // value
                match value {
                    Some(value) => {
                        write!(f, "value:").unwrap();
                        self.print_rhs_operand(f, value);
                    }
                    None => write!(f, "value:_").unwrap(),
                };

                write!(f, " ").unwrap();

                // gas
                write!(f, "gas:").unwrap();
                self.print_rhs_operand(f, gas);

                write!(f, " ").unwrap();

                match address {
                    Some(address) => {
                        write!(f, "address:").unwrap();
                        self.print_rhs_operand(f, address);
                    }
                    None => write!(f, "address:_").unwrap(),
                };

                write!(f, " ").unwrap();

                match seeds {
                    Some(seeds) => {
                        write!(f, "seeds:").unwrap();
                        self.print_rhs_operand(f, seeds);
                    }
                    None => write!(f, "seeds:_").unwrap(),
                };

                write!(f, " ").unwrap();

                write!(f, "encoded-buffer:").unwrap();
                self.print_rhs_operand(f, encoded_args);

                write!(f, " ").unwrap();

                match accounts {
                    ast::ExternalCallAccounts::NoAccount => write!(f, "accounts:none").unwrap(),
                    ast::ExternalCallAccounts::Present(acc) => {
                        write!(f, "accounts:").unwrap();
                        self.print_rhs_operand(f, acc);
                    }
                    ast::ExternalCallAccounts::AbsentArgument => {
                        write!(f, "accounts:absent").unwrap()
                    }
                }
            }
            Instruction::LoadStorage { res, storage, .. } => {
                let res_op = self.get_var_operand(res);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = load_storage ").unwrap();
                self.print_rhs_operand(f, storage);
                write!(f, ";").unwrap();
            }
            Instruction::ClearStorage { storage, .. } => {
                write!(f, "clear_storage ").unwrap();
                self.print_rhs_operand(f, storage);
                write!(f, ";").unwrap();
            }
            Instruction::SetStorage { value, storage, .. } => {
                write!(f, "set_storage ").unwrap();
                self.print_rhs_operand(f, storage);
                write!(f, " ").unwrap();
                self.print_rhs_operand(f, value);
                write!(f, ";").unwrap();
            }
            Instruction::SetStorageBytes {
                value,
                storage,
                offset,
                ..
            } => {
                write!(f, "set_storage_bytes ").unwrap();
                self.print_rhs_operand(f, storage);
                write!(f, " offset:").unwrap();
                self.print_rhs_operand(f, offset);
                write!(f, " value:").unwrap();
                self.print_rhs_operand(f, value);
                write!(f, ";").unwrap();
            }
            Instruction::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                let res_op = self.get_var_operand(res);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = push_storage ").unwrap();
                self.print_rhs_operand(f, storage);
                write!(f, " ").unwrap();
                match value {
                    Some(value) => self.print_rhs_operand(f, value),
                    None => write!(f, "empty").unwrap(),
                };
                write!(f, ";").unwrap();
            }
            Instruction::PopStorage { res, storage, .. } => match res {
                Some(res) => {
                    let res_op = self.get_var_operand(res);
                    self.print_lhs_operand(f, &res_op);
                    write!(f, " = pop_storage ").unwrap();
                    self.print_rhs_operand(f, storage);
                    write!(f, ";").unwrap();
                }
                None => {
                    write!(f, "pop_storage ").unwrap();
                    self.print_rhs_operand(f, storage);
                    write!(f, ";").unwrap();
                }
            },
            Instruction::Call {
                res, call, args, ..
            } => {
                // lhs: %0, %1, ...
                for (i, id) in res.iter().enumerate() {
                    let res_op = self.get_var_operand(id);
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_lhs_operand(f, &res_op);
                }

                write!(f, " = call ").unwrap();

                match call {
                    InternalCallTy::Builtin { ast_func_no, .. } => {
                        write!(f, "builtin#{}", ast_func_no).unwrap();
                    }
                    InternalCallTy::Static { cfg_no, .. } => {
                        write!(f, "function#{}", cfg_no).unwrap()
                    }
                    InternalCallTy::Dynamic(op) => self.print_rhs_operand(f, op),
                    InternalCallTy::HostFunction { name } => {
                        write!(f, "host_function#{}", name).unwrap()
                    }
                };

                write!(f, "(").unwrap();

                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, arg);
                }

                write!(f, ");").unwrap();
            }
            Instruction::Print { operand, .. } => {
                write!(f, "print ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, ";").unwrap();
            }
            Instruction::MemCopy {
                src, dest, bytes, ..
            } => {
                write!(f, "memcopy ").unwrap();
                self.print_rhs_operand(f, src);
                write!(f, " to ").unwrap();
                self.print_rhs_operand(f, dest);
                write!(f, " for ").unwrap();
                self.print_rhs_operand(f, bytes);
                write!(f, " bytes;").unwrap();
            }
            Instruction::ExternalCall {
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
                        let success_op = self.get_var_operand(success);
                        self.print_lhs_operand(f, &success_op);
                    }
                    None => write!(f, "_").unwrap(),
                };

                write!(f, " = call_ext [{}] ", callty).unwrap();

                match address {
                    Some(address) => {
                        write!(f, "address:").unwrap();
                        self.print_rhs_operand(f, address);
                    }
                    None => write!(f, "address:_").unwrap(),
                };

                write!(f, " ").unwrap();

                write!(f, "payload:").unwrap();
                self.print_rhs_operand(f, payload);

                write!(f, " ").unwrap();

                write!(f, "value:").unwrap();
                self.print_rhs_operand(f, value);

                write!(f, " ").unwrap();

                write!(f, "gas:").unwrap();
                self.print_rhs_operand(f, gas);

                write!(f, " ").unwrap();

                match accounts {
                    ast::ExternalCallAccounts::NoAccount => write!(f, "accounts:none").unwrap(),
                    ast::ExternalCallAccounts::Present(acc) => {
                        write!(f, "accounts:").unwrap();
                        self.print_rhs_operand(f, acc);
                    }
                    ast::ExternalCallAccounts::AbsentArgument => {
                        write!(f, "accounts:absent").unwrap()
                    }
                };

                write!(f, " ").unwrap();

                match seeds {
                    Some(seeds) => {
                        write!(f, "seeds:").unwrap();
                        self.print_rhs_operand(f, seeds);
                    }
                    None => write!(f, "seeds:_").unwrap(),
                };

                write!(f, " ").unwrap();

                match contract_function_no {
                    Some((contract_no, function_no)) => {
                        write!(
                            f,
                            "contract_no:{}, function_no:{}",
                            contract_no, function_no
                        )
                        .unwrap();
                    }
                    None => write!(f, "contract_no:_, function_no:_").unwrap(),
                };

                write!(f, " ").unwrap();

                match flags {
                    Some(flags) => {
                        write!(f, "flags:").unwrap();
                        self.print_rhs_operand(f, flags);
                    }
                    None => write!(f, "flags:_").unwrap(),
                }

                write!(f, ";").unwrap();
            }
            Instruction::ValueTransfer {
                success,
                address,
                value,
                ..
            } => {
                match success {
                    Some(success) => {
                        let success_op = self.get_var_operand(success);
                        self.print_lhs_operand(f, &success_op);
                    }
                    None => write!(f, "_").unwrap(),
                };
                write!(f, " = value_transfer ").unwrap();
                self.print_rhs_operand(f, value);
                write!(f, " to ").unwrap();
                self.print_rhs_operand(f, address);
                write!(f, ";").unwrap();
            }
            Instruction::SelfDestruct { recipient, .. } => {
                write!(f, "self_destruct ").unwrap();
                self.print_rhs_operand(f, recipient);
                write!(f, ";").unwrap();
            }
            Instruction::EmitEvent {
                data,
                topics,
                event_no,
                ..
            } => {
                write!(f, "emit event#{} to topics[", event_no).unwrap();
                for (i, topic) in topics.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, topic);
                }
                write!(f, "], data: ").unwrap();
                self.print_rhs_operand(f, data);
                write!(f, ";").unwrap()
            }
            Instruction::WriteBuffer {
                buf, offset, value, ..
            } => {
                write!(f, "write_buf ").unwrap();
                self.print_rhs_operand(f, buf);
                write!(f, " offset:").unwrap();
                self.print_rhs_operand(f, offset);
                write!(f, " value:").unwrap();
                self.print_rhs_operand(f, value);
                write!(f, ";").unwrap();
            }
            Instruction::Branch { block, .. } => write!(f, "br block#{};", block).unwrap(),
            Instruction::BranchCond {
                cond,
                true_block,
                false_block,
                ..
            } => {
                write!(f, "cbr ").unwrap();
                self.print_rhs_operand(f, cond);
                write!(f, " block#{} else block#{};", true_block, false_block).unwrap();
            }
            Instruction::Switch {
                cond,
                cases,
                default,
                ..
            } => {
                write!(f, "switch ").unwrap();
                self.print_rhs_operand(f, cond);
                write!(f, ":").unwrap();
                for (i, (cond, block)) in cases.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "\n    case:    ").unwrap();
                    self.print_rhs_operand(f, cond);
                    write!(f, " => block#{}", block).unwrap();
                }
                write!(f, "\n    default: block#{};", default).unwrap();
            }
            Instruction::Return { value, .. } => {
                write!(f, "return").unwrap();
                for (i, value) in value.iter().enumerate() {
                    if i == 0 {
                        write!(f, " ").unwrap();
                    } else {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, value);
                }
                write!(f, ";").unwrap();
            }
            Instruction::AssertFailure { encoded_args, .. } => match encoded_args {
                Some(encoded_args) => {
                    write!(f, "assert_failure ").unwrap();
                    self.print_rhs_operand(f, encoded_args);
                    write!(f, ";").unwrap();
                }
                None => write!(f, "assert_failure;").unwrap(),
            },
            Instruction::Phi { res, vars, .. } => {
                let res_op = self.get_var_operand(res);
                self.print_lhs_operand(f, &res_op);
                write!(f, " = phi ").unwrap();
                for (i, var) in vars.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_phi(f, var);
                }
                write!(f, ";").unwrap();
            }
        }
    }
}
