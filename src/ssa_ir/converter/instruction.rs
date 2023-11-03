// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::instructions::Insn;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub(crate) fn convert_instr(
        &self,
        instr: &Instr,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        match instr {
            Instr::Nop => Ok(vec![Insn::Nop]),
            Instr::Set { res, expr, loc, .. } => {
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let dest_operand = vartable.get_operand(res, *loc)?;
                self.convert_expression(&dest_operand, expr, vartable)
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()
                let (dest_op, dest_insns) = self.to_operand_and_insns(dest, vartable)?;
                let (data_op, data_insns) = self.to_operand_and_insns(data, vartable)?;
                let mut insns = vec![];
                insns.extend(dest_insns);
                insns.extend(data_insns);
                insns.push(Insn::Store {
                    dest: dest_op,
                    data: data_op,
                });
                Ok(insns)
            }
            Instr::PushMemory {
                res, array, value, ..
            } => {
                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                let mut insns = vec![];
                insns.extend(value_insns);
                insns.push(Insn::PushMemory {
                    res: *res,
                    array: *array,
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::PopMemory {
                res, array, loc, ..
            } => Ok(vec![Insn::PopMemory {
                res: *res,
                array: *array,
                loc: *loc,
            }]),

            Instr::Branch { block } => Ok(vec![Insn::Branch { block: *block }]),
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                let (cond_op, cond_insns) = self.to_operand_and_insns(cond, vartable)?;
                let mut insns = Vec::new();
                insns.extend(cond_insns);
                insns.push(Insn::BranchCond {
                    cond: cond_op,
                    true_block: *true_block,
                    false_block: *false_block,
                });
                Ok(insns)
            }
            Instr::Return { value } => {
                let mut operands = vec![];
                let mut insns = vec![];
                for v in value {
                    let (tmp, expr_insns) = self.to_operand_and_insns(v, vartable)?;
                    insns.extend(expr_insns);
                    operands.push(tmp);
                }
                insns.push(Insn::Return { value: operands });
                Ok(insns)
            }
            Instr::AssertFailure { encoded_args } => match encoded_args {
                Some(args) => {
                    let (tmp, expr_insns) = self.to_operand_and_insns(args, vartable)?;
                    let mut insns = vec![];
                    insns.extend(expr_insns);
                    insns.push(Insn::AssertFailure {
                        encoded_args: Some(tmp),
                    });
                    Ok(insns)
                }
                None => Ok(vec![Insn::AssertFailure { encoded_args: None }]),
            },
            Instr::Call {
                res, call, args, ..
            } => {
                let mut insns = vec![];

                // resolve the function
                let (callty, callty_insns) = self.to_internal_call_ty_and_insns(call, vartable)?;
                insns.extend(callty_insns);

                // resolve the arguments
                let mut arg_ops = vec![];
                for arg in args {
                    let (tmp, expr_insns) = self.to_operand_and_insns(arg, vartable)?;
                    insns.extend(expr_insns);
                    arg_ops.push(tmp);
                }

                let call_insn = Insn::Call {
                    res: res.clone(),
                    call: callty,
                    args: arg_ops,
                };

                insns.push(call_insn);
                Ok(insns)
            }
            Instr::Print { expr } => {
                let (tmp, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Print { operand: tmp });
                Ok(insns)
            }
            Instr::LoadStorage { res, storage, .. } => {
                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;
                let mut insns = vec![];
                insns.extend(storage_insns);
                insns.push(Insn::LoadStorage {
                    res: *res,
                    storage: storage_op,
                });
                Ok(insns)
            }
            Instr::ClearStorage { storage, .. } => {
                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;
                let mut insns = vec![];
                insns.extend(storage_insns);
                insns.push(Insn::ClearStorage {
                    storage: storage_op,
                });
                Ok(insns)
            }
            Instr::SetStorage { value, storage, .. } => {
                let mut insns = vec![];

                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;
                insns.extend(storage_insns);

                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                insns.extend(value_insns);

                insns.push(Insn::SetStorage {
                    value: value_op,
                    storage: storage_op,
                });
                Ok(insns)
            }
            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;
                let (offset_op, offset_insns) = self.to_operand_and_insns(offset, vartable)?;

                let mut insns = vec![];
                insns.extend(value_insns);
                insns.extend(storage_insns);
                insns.extend(offset_insns);

                insns.push(Insn::SetStorageBytes {
                    value: value_op,
                    storage: storage_op,
                    offset: offset_op,
                });
                Ok(insns)
            }
            Instr::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                let (value_op, value_insns) = self.to_operand_option_and_insns(value, vartable)?;
                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;

                let mut insns = vec![];
                insns.extend(storage_insns);
                insns.extend(value_insns);

                insns.push(Insn::PushStorage {
                    res: *res,
                    value: value_op,
                    storage: storage_op,
                });
                Ok(insns)
            }
            Instr::PopStorage { res, storage, .. } => {
                let (storage_op, storage_insns) = self.to_operand_and_insns(storage, vartable)?;
                let mut insns = vec![];
                insns.extend(storage_insns);
                insns.push(Insn::PopStorage {
                    res: *res,
                    storage: storage_op,
                });
                Ok(insns)
            }
            Instr::ExternalCall {
                loc,
                success,
                address,
                accounts,
                seeds,
                payload,
                value,
                gas,
                callty,
                contract_function_no,
                flags,
            } => {
                let (address_op, address_insns) =
                    self.to_operand_option_and_insns(address, vartable)?;
                let (accounts_op, accounts_insns) =
                    self.to_external_call_accounts_and_insns(accounts, vartable)?;
                let (seeds_op, seeds_insns) = self.to_operand_option_and_insns(seeds, vartable)?;
                let (payload_op, payload_insns) = self.to_operand_and_insns(payload, vartable)?;
                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                let (gas_op, gas_insns) = self.to_operand_and_insns(gas, vartable)?;
                let (flags_op, flags_insns) = self.to_operand_option_and_insns(flags, vartable)?;

                let mut insns = vec![];
                insns.extend(address_insns);
                insns.extend(accounts_insns);
                insns.extend(seeds_insns);
                insns.extend(payload_insns);
                insns.extend(value_insns);
                insns.extend(gas_insns);
                insns.extend(flags_insns);
                insns.push(Insn::ExternalCall {
                    loc: *loc,
                    success: *success,
                    address: address_op,
                    accounts: accounts_op,
                    seeds: seeds_op,
                    payload: payload_op,
                    value: value_op,
                    gas: gas_op,
                    callty: callty.clone(),
                    contract_function_no: *contract_function_no,
                    flags: flags_op,
                });
                Ok(insns)
            }
            Instr::ValueTransfer {
                success,
                address,
                value,
            } => {
                let (address_op, address_insns) = self.to_operand_and_insns(address, vartable)?;
                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                let mut insns = vec![];
                insns.extend(address_insns);
                insns.extend(value_insns);
                insns.push(Insn::ValueTransfer {
                    success: *success,
                    address: address_op,
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::SelfDestruct { recipient } => {
                let (tmp, expr_insns) = self.to_operand_and_insns(recipient, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::SelfDestruct { recipient: tmp });
                Ok(insns)
            }
            Instr::EmitEvent {
                event_no,
                data,
                topics,
            } => {
                let (data_op, data_insns) = self.to_operand_and_insns(data, vartable)?;
                let mut insns = vec![];
                insns.extend(data_insns);
                let mut topic_ops = vec![];
                for topic in topics {
                    let (topic_op, topic_insns) = self.to_operand_and_insns(topic, vartable)?;
                    insns.extend(topic_insns);
                    topic_ops.push(topic_op);
                }
                insns.push(Insn::EmitEvent {
                    event_no: *event_no,
                    data: data_op,
                    topics: topic_ops,
                });
                Ok(insns)
            }
            Instr::WriteBuffer { buf, offset, value } => {
                let (buf_op, buf_insns) = self.to_operand_and_insns(buf, vartable)?;
                let (offset_op, offset_insns) = self.to_operand_and_insns(offset, vartable)?;
                let (value_op, value_insns) = self.to_operand_and_insns(value, vartable)?;
                let mut insns = vec![];
                insns.extend(buf_insns);
                insns.extend(offset_insns);
                insns.extend(value_insns);
                insns.push(Insn::WriteBuffer {
                    buf: buf_op,
                    offset: offset_op,
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::MemCopy {
                source,
                destination,
                bytes,
            } => {
                let (source_op, source_insns) = self.to_operand_and_insns(source, vartable)?;
                let (dest_op, dest_insns) = self.to_operand_and_insns(destination, vartable)?;
                let (bytes_op, bytes_insns) = self.to_operand_and_insns(bytes, vartable)?;
                let mut insns = vec![];
                insns.extend(source_insns);
                insns.extend(dest_insns);
                insns.extend(bytes_insns);
                insns.push(Insn::MemCopy {
                    src: source_op,
                    dest: dest_op,
                    bytes: bytes_op,
                });
                Ok(insns)
            }
            Instr::Switch {
                cond,
                cases,
                default,
            } => {
                let mut insns = vec![];

                let (cond_op, cond_insns) = self.to_operand_and_insns(cond, vartable)?;
                insns.extend(cond_insns);

                let mut case_ops = vec![];
                for (case, block_no) in cases {
                    let (case_op, case_insns) = self.to_operand_and_insns(case, vartable)?;
                    insns.extend(case_insns);
                    case_ops.push((case_op, *block_no));
                }

                insns.push(Insn::Switch {
                    cond: cond_op,
                    cases: case_ops,
                    default: *default,
                });

                Ok(insns)
            }
            Instr::ReturnData { data, data_len } => {
                let (data_op, data_insns) = self.to_operand_and_insns(data, vartable)?;
                let (data_len_op, data_len_insns) =
                    self.to_operand_and_insns(data_len, vartable)?;
                let mut insns = vec![];
                insns.extend(data_insns);
                insns.extend(data_len_insns);
                insns.push(Insn::ReturnData {
                    data: data_op,
                    data_len: data_len_op,
                });
                Ok(insns)
            }
            Instr::ReturnCode { code } => Ok(vec![Insn::ReturnCode { code: code.clone() }]),
            Instr::Unimplemented { reachable } => Ok(vec![Insn::Unimplemented {
                reachable: *reachable,
            }]),
            Instr::AccountAccess { .. } => panic!("AccountAccess should be replaced by Subscript"),
            Instr::Constructor {
                success,
                res,
                contract_no,
                constructor_no,
                encoded_args,
                value,
                gas,
                salt,
                address,
                seeds,
                accounts,
                loc,
            } => {
                let mut insns = vec![];

                let (args_op, args_insns) = self.to_operand_and_insns(encoded_args, vartable)?;
                insns.extend(args_insns);

                let (value_op, value_insns) = self.to_operand_option_and_insns(value, vartable)?;
                insns.extend(value_insns);

                let (gas_op, gas_insns) = self.to_operand_and_insns(gas, vartable)?;
                insns.extend(gas_insns);

                let (salt_op, salt_insns) = self.to_operand_option_and_insns(salt, vartable)?;
                insns.extend(salt_insns);

                let (address_op, address_insns) =
                    self.to_operand_option_and_insns(address, vartable)?;
                insns.extend(address_insns);

                let (seeds_op, seeds_insns) = self.to_operand_option_and_insns(seeds, vartable)?;
                insns.extend(seeds_insns);

                let (accounts, accounts_insns) =
                    self.to_external_call_accounts_and_insns(accounts, vartable)?;
                insns.extend(accounts_insns);

                let constructor_insn = Insn::Constructor {
                    loc: *loc,
                    success: *success,
                    res: *res,
                    contract_no: *contract_no,
                    constructor_no: *constructor_no,
                    encoded_args: args_op,
                    value: value_op,
                    gas: gas_op,
                    salt: salt_op,
                    address: address_op,
                    seeds: seeds_op,
                    accounts,
                };
                insns.push(constructor_insn);

                Ok(insns)
            }
        }
    }
}
