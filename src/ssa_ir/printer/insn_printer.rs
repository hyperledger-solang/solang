use crate::sema::ast;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::printer::Printer;
use crate::ssa_ir::ssa_type::{InternalCallTy, PhiInput};
use crate::{stringfy_lhs_operand, stringfy_rhs_operand};
use std::io::Write;

#[macro_export]
macro_rules! stringfy_insn {
    ($printer:expr, $insn:expr) => {{
        let mut buf = Vec::new();
        $printer.print_insn(&mut buf, $insn).unwrap();
        String::from_utf8(buf).unwrap()
    }};
}

#[macro_export]
macro_rules! stringfy_phi {
    ($printer:expr, $phi:expr) => {{
        let mut buf = Vec::new();
        $printer.print_phi(&mut buf, $phi).unwrap();
        String::from_utf8(buf).unwrap()
    }};
}

impl Printer {
    pub fn print_phi(&self, f: &mut dyn Write, phi: &PhiInput) -> std::io::Result<()> {
        // write!(f, "[{}, block#{}]", phi.operand, phi.block_no)
        write!(f, "[")?;
        self.print_rhs_operand(f, &phi.operand)?;
        write!(f, ", block#{}]", phi.block_no)
    }

    pub fn print_insn(&self, f: &mut dyn Write, insn: &Insn) -> std::io::Result<()> {
        match insn {
            Insn::Nop => write!(f, "nop;"),
            Insn::ReturnData { data, data_len } => {
                // write!(f, "return_data {} of length {};", data, data_len)
                write!(f, "return_data ")?;
                self.print_rhs_operand(f, data)?;
                write!(f, " of length ",)?;
                self.print_rhs_operand(f, data_len)?;
                write!(f, ";")
            }
            Insn::ReturnCode { code, .. } => write!(f, "return_code \"{}\";", code),
            Insn::Set { res, expr, .. } => {
                let res_op = self.get_var_operand(res).unwrap();
                write!(f, "{} = ", stringfy_lhs_operand!(self, &res_op))?;
                self.print_expr(f, expr)?;
                write!(f, ";")
            }
            Insn::Store { dest, data, .. } => {
                // write!(f, "store {} to {};", data, dest)
                write!(f, "store ")?;
                self.print_rhs_operand(f, data)?;
                write!(f, " to ")?;
                self.print_rhs_operand(f, dest)?;
                write!(f, ";")
            }
            Insn::PushMemory {
                res, array, value, ..
            } => {
                // %101 = push_mem ptr<int32[10]> %3 uint32(1);
                let res_op = self.get_var_operand(res).unwrap();
                let array_op = self.get_var_operand(array).unwrap();
                write!(
                    f,
                    "{} = push_mem {} ",
                    stringfy_lhs_operand!(self, &res_op),
                    stringfy_rhs_operand!(self, &array_op)
                )?;
                self.print_rhs_operand(f, value)?;
                write!(f, ";")
            }
            Insn::PopMemory { res, array, .. } => {
                // %101 = pop_mem ptr<int32[10]> %3;
                let res_op = self.get_var_operand(res).unwrap();
                let array_op = self.get_var_operand(array).unwrap();
                write!(
                    f,
                    "{} = pop_mem {};",
                    stringfy_lhs_operand!(self, &res_op),
                    stringfy_rhs_operand!(self, &array_op)
                )
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
                    Some(success) => {
                        let res_op = self.get_var_operand(res).unwrap();
                        let success_op = self.get_var_operand(success).unwrap();
                        format!(
                            "{}, {}",
                            stringfy_lhs_operand!(self, &success_op),
                            stringfy_lhs_operand!(self, &res_op)
                        )
                    }
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
                    Some(salt) => format!("salt:{}", stringfy_rhs_operand!(self, salt)),
                    None => format!(""),
                };
                let rhs_value = match value {
                    Some(value) => format!("value:{}", stringfy_rhs_operand!(self, value)),
                    None => format!(""),
                };
                let rhs_gas = format!("gas:{}", stringfy_rhs_operand!(self, gas));
                let rhs_address = match address {
                    Some(address) => format!("address:{}", stringfy_rhs_operand!(self, address)),
                    None => format!(""),
                };
                let rhs_seeds = match seeds {
                    Some(seeds) => format!("seeds:{}", stringfy_rhs_operand!(self, seeds)),
                    None => format!(""),
                };
                let rhs_encoded_args = format!(
                    "encoded-buffer:{}",
                    stringfy_rhs_operand!(self, encoded_args)
                );
                let rhs_accounts = match accounts {
                    ast::ExternalCallAccounts::NoAccount => format!(""),
                    ast::ExternalCallAccounts::Present(acc) => {
                        format!("accounts:{}", stringfy_rhs_operand!(self, acc))
                    }
                    ast::ExternalCallAccounts::AbsentArgument => format!("accounts:_"),
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
                let res_op = self.get_var_operand(res).unwrap();
                // write!(f, "%{} = load_storage {};", res, storage)
                write!(
                    f,
                    "{} = load_storage ",
                    stringfy_lhs_operand!(self, &res_op)
                )?;
                self.print_rhs_operand(f, storage)?;
                write!(f, ";")
            }
            Insn::ClearStorage { storage, .. } => {
                // write!(f, "clear_storage {};", storage)
                write!(f, "clear_storage ")?;
                self.print_rhs_operand(f, storage)?;
                write!(f, ";")
            }
            Insn::SetStorage { value, storage, .. } => {
                // write!(f, "set_storage {} {};", storage, value)
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
                // set_storage_bytes {} offset:{} value:{}
                // write!(
                //     f,
                //     "set_storage_bytes {} offset:{} value:{};",
                //     storage, offset, value
                // )
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
                // "%{} = push storage ty:{} slot:{} = {}",
                let rhs = match value {
                    Some(value) => format!("{}", stringfy_rhs_operand!(self, value)),
                    None => format!("empty"),
                };
                // write!(f, "%{} = push_storage {} {};", res, storage, rhs)
                let res_op = self.get_var_operand(res).unwrap();
                write!(
                    f,
                    "{} = push_storage ",
                    stringfy_lhs_operand!(self, &res_op)
                )?;
                self.print_rhs_operand(f, storage)?;
                write!(f, " {};", rhs)
            }
            Insn::PopStorage { res, storage, .. } =>
            // "%{} = pop storage ty:{} slot({})"
            {
                match res {
                    Some(res) => {
                        let res_op = self.get_var_operand(res).unwrap();
                        write!(
                            f,
                            "{} = pop_storage {};",
                            stringfy_lhs_operand!(self, &res_op),
                            stringfy_rhs_operand!(self, storage)
                        )
                    }
                    None => write!(f, "pop_storage {};", stringfy_rhs_operand!(self, storage)),
                }
            }
            Insn::Call { res, call, args } => {
                // lhs: %0, %1, ...
                let lhs = res
                    .iter()
                    .map(|id| {
                        let res_op = self.get_var_operand(id).unwrap();
                        stringfy_lhs_operand!(self, &res_op)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                // rhs: call [builtin | static | dynamic] [call] args: %0, %1, ...
                let rhs_call = match call {
                    InternalCallTy::Builtin { ast_func_no, .. } => {
                        format!("builtin#{}", ast_func_no)
                    }
                    InternalCallTy::Static { cfg_no, .. } => format!("function#{}", cfg_no),
                    InternalCallTy::Dynamic(op) => stringfy_rhs_operand!(self, op),
                };

                let rhs_args = args
                    .iter()
                    .map(|arg| stringfy_rhs_operand!(self, arg))
                    .collect::<Vec<String>>()
                    .join(", ");

                write!(f, "{} = call {}({});", lhs, rhs_call, rhs_args)
            }
            Insn::Print { operand, .. } => {
                // "print {}"
                // write!(f, "print {};", operand)
                write!(f, "print ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, ";")
            }
            Insn::MemCopy {
                src, dest, bytes, ..
            } => {
                // memcopy %4 from %3 for uint8(11);
                // write!(f, "memcopy {} to {} for {} bytes;", src, dest, bytes)
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
                let lhs = match success {
                    Some(success) => {
                        let success_op = self.get_var_operand(success).unwrap();
                        format!("{}", stringfy_lhs_operand!(self, &success_op))
                    }
                    None => String::from(""),
                };
                let rhs_address = match address {
                    Some(address) => format!(" address:{}", stringfy_rhs_operand!(self, address)),
                    None => String::from(" _"),
                };
                let rhs_accounts = match accounts {
                    ast::ExternalCallAccounts::NoAccount => format!(""),
                    ast::ExternalCallAccounts::Present(acc) => {
                        format!("accounts:{}", stringfy_rhs_operand!(self, acc))
                    }
                    ast::ExternalCallAccounts::AbsentArgument => format!("accounts:_"),
                };
                let rhs_seeds = match seeds {
                    Some(seeds) => format!(" seeds:{}", stringfy_rhs_operand!(self, seeds)),
                    None => String::from(" _"),
                };
                let rhs_contract_function_no = match contract_function_no {
                    Some((contract_no, function_no)) => {
                        format!(" contract_no:{}, function_no:{}", contract_no, function_no)
                    }
                    None => String::from(" _"),
                };
                let rhs_flags = match flags {
                    Some(flags) => format!(" flags:{}", stringfy_rhs_operand!(self, flags)),
                    None => String::from(" _"),
                };
                // "{} = external call::{} address:{} payload:{} value:{} gas:{} accounts:{} seeds:{} contract|function:{} flags:{}",
                write!(
                    f,
                    "{}call_ext [{}]{}{}{}{}{}{}{}{};",
                    lhs,
                    callty,
                    rhs_address,
                    format!(" payload:{}", stringfy_rhs_operand!(self, payload)),
                    format!(" value:{}", stringfy_rhs_operand!(self, value)),
                    format!(" gas:{}", stringfy_rhs_operand!(self, gas)),
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
                    Some(success) => {
                        let success_op = self.get_var_operand(success).unwrap();
                        format!("{}", stringfy_lhs_operand!(self, &success_op))
                    }
                    None => String::from("_"),
                };
                // write!(f, "{} = transfer {} to {};", lhs, value, address)
                write!(f, "{} = value_transfer ", lhs)?;
                self.print_rhs_operand(f, value)?;
                write!(f, " to ")?;
                self.print_rhs_operand(f, address)?;
                write!(f, ";")
            }
            Insn::SelfDestruct { recipient, .. } => {
                // "selfdestruct {}",
                write!(
                    f,
                    "self_destruct {};",
                    stringfy_rhs_operand!(self, recipient)
                )
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
                    .map(|topic| stringfy_rhs_operand!(self, topic))
                    .collect::<Vec<String>>()
                    .join(", ");
                // write!(
                //     f,
                //     "emit event#{} to topics[{}], data: {};",
                //     event_no, rhs_topics, data
                // )
                write!(
                    f,
                    "emit event#{} to topics[{}], data: ",
                    event_no, rhs_topics
                )?;
                self.print_rhs_operand(f, data)?;
                write!(f, ";")
            }
            Insn::WriteBuffer {
                buf, offset, value, ..
            } => {
                // "write_buf {} offset:{} value:{}",
                // write!(f, "write_buf {} offset:{} value:{};", buf, offset, value)
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
                // write!(
                //     f,
                //     "cbr {} block#{} else block#{};",
                //     cond, true_block, false_block
                // )
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
                // switch %1 case: %4 => block#11, case: %5 => block#12, case: %6 => block#13, default: block#14;
                let rhs_cases = cases
                    .iter()
                    .map(|(cond, block)| {
                        format!(
                            "\n    case:    {} => block#{}",
                            stringfy_rhs_operand!(self, cond),
                            block
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                // write!(
                //     f,
                //     "switch {} cases: [{}] default: block#{};",
                //     cond, rhs_cases, default
                // )
                write!(f, "switch ")?;
                self.print_rhs_operand(f, cond)?;
                write!(f, ":{}\n    default: block#{};", rhs_cases, default)
            }
            Insn::Return { value, .. } => {
                let rhs = value
                    .iter()
                    .map(|value| stringfy_rhs_operand!(self, value))
                    .collect::<Vec<String>>()
                    .join(", ");
                if rhs.len() > 0 {
                    write!(f, "return {};", rhs)
                } else {
                    write!(f, "return;")
                }
            }
            Insn::AssertFailure { encoded_args, .. } => match encoded_args {
                Some(encoded_args) => {
                    write!(
                        f,
                        "assert_failure {};",
                        stringfy_rhs_operand!(self, encoded_args)
                    )
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
                let rhs_vars = vars
                    .iter()
                    .map(|input| stringfy_phi!(self, input))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(
                    f,
                    "{} = phi {};",
                    stringfy_lhs_operand!(self, &res_op),
                    rhs_vars
                )
            }
        }
    }
}
