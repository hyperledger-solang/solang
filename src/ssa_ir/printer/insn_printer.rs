use std::io::Write;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::printer::Printer;
use crate::ssa_ir::ssa_type::InternalCallTy;

#[macro_export]
macro_rules! stringfy_insn {
    ($vartable:expr, $insn:expr) => {{
        use solang::ssa_ir::printer::Printer;
        let mut printer = Printer { vartable: $vartable };
        let mut buf = Vec::new();
        printer.print_insn(&mut buf, $insn).unwrap();
        String::from_utf8(buf).unwrap()
    }}
}

impl Printer<'_> {
    pub fn print_insn(&self, f: &mut dyn Write, insn: &Insn) -> std::io::Result<()> {
        match insn {
            Insn::Nop => write!(f, "nop;"),
            Insn::ReturnData { data, data_len } => {
                write!(f, "return_data {} of length {};", data, data_len)
            }
            Insn::ReturnCode { code, .. } => write!(f, "return_code \"{}\";", code),
            Insn::Set { res, expr, .. } => {
                write!(f, "%{} = ", res)?;
                self.print_expr(f, expr)?;
                write!(f, ";")
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