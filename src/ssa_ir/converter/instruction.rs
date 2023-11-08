// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::instructions::Instruction;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub(crate) fn lowering_instr(
        &self,
        instr: &Instr,
        vartable: &mut Vartable,
        mut results: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        match instr {
            Instr::Nop => {
                results.push(Instruction::Nop);
                Ok(())
            }
            Instr::Set { res, expr, loc, .. } => {
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let dest_operand = vartable.get_operand(res, *loc)?;
                self.lowering_expression(&dest_operand, expr, vartable, results)
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()
                let dest_op = self.to_operand_and_insns(dest, vartable, &mut results)?;
                let data_op = self.to_operand_and_insns(data, vartable, &mut results)?;
                results.push(Instruction::Store {
                    dest: dest_op,
                    data: data_op,
                });
                Ok(())
            }
            Instr::PushMemory {
                res, array, value, ..
            } => {
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                results.push(Instruction::PushMemory {
                    res: *res,
                    array: *array,
                    value: value_op,
                });
                Ok(())
            }
            Instr::PopMemory {
                res, array, loc, ..
            } => {
                results.push(Instruction::PopMemory {
                    res: *res,
                    array: *array,
                    loc: *loc,
                });
                Ok(())
            }

            Instr::Branch { block } => {
                results.push(Instruction::Branch { block: *block });
                Ok(())
            }
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                let cond_op = self.to_operand_and_insns(cond, vartable, &mut results)?;
                results.push(Instruction::BranchCond {
                    cond: cond_op,
                    true_block: *true_block,
                    false_block: *false_block,
                });
                Ok(())
            }
            Instr::Return { value } => {
                let mut operands = vec![];
                for v in value {
                    let tmp = self.to_operand_and_insns(v, vartable, &mut results)?;
                    operands.push(tmp);
                }
                results.push(Instruction::Return { value: operands });
                Ok(())
            }
            Instr::AssertFailure { encoded_args } => match encoded_args {
                Some(args) => {
                    let tmp = self.to_operand_and_insns(args, vartable, &mut results)?;
                    results.push(Instruction::AssertFailure {
                        encoded_args: Some(tmp),
                    });
                    Ok(())
                }
                None => {
                    results.push(Instruction::AssertFailure { encoded_args: None });
                    Ok(())
                }
            },
            Instr::Call {
                res, call, args, ..
            } => {
                // resolve the function
                let callty = self.to_internal_call_ty_and_insns(call, vartable, &mut results)?;

                // resolve the arguments
                let mut arg_ops = vec![];
                for arg in args {
                    let tmp = self.to_operand_and_insns(arg, vartable, &mut results)?;
                    arg_ops.push(tmp);
                }

                results.push(Instruction::Call {
                    res: res.clone(),
                    call: callty,
                    args: arg_ops,
                });
                Ok(())
            }
            Instr::Print { expr } => {
                let tmp = self.to_operand_and_insns(expr, vartable, &mut results)?;
                results.push(Instruction::Print { operand: tmp });
                Ok(())
            }
            Instr::LoadStorage { res, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                results.push(Instruction::LoadStorage {
                    res: *res,
                    storage: storage_op,
                });
                Ok(())
            }
            Instr::ClearStorage { storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                results.push(Instruction::ClearStorage {
                    storage: storage_op,
                });
                Ok(())
            }
            Instr::SetStorage { value, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                results.push(Instruction::SetStorage {
                    value: value_op,
                    storage: storage_op,
                });
                Ok(())
            }
            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                let offset_op = self.to_operand_and_insns(offset, vartable, &mut results)?;
                results.push(Instruction::SetStorageBytes {
                    value: value_op,
                    storage: storage_op,
                    offset: offset_op,
                });
                Ok(())
            }
            Instr::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                let value_op = self.to_operand_option_and_insns(value, vartable, &mut results)?;
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                results.push(Instruction::PushStorage {
                    res: *res,
                    value: value_op,
                    storage: storage_op,
                });
                Ok(())
            }
            Instr::PopStorage { res, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, &mut results)?;
                results.push(Instruction::PopStorage {
                    res: *res,
                    storage: storage_op,
                });
                Ok(())
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
                let address_op =
                    self.to_operand_option_and_insns(address, vartable, &mut results)?;
                let accounts_op =
                    self.to_external_call_accounts_and_insns(accounts, vartable, &mut results)?;
                let seeds_op = self.to_operand_option_and_insns(seeds, vartable, &mut results)?;
                let payload_op = self.to_operand_and_insns(payload, vartable, &mut results)?;
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                let gas_op = self.to_operand_and_insns(gas, vartable, &mut results)?;
                let flags_op = self.to_operand_option_and_insns(flags, vartable, &mut results)?;

                results.push(Instruction::ExternalCall {
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
                Ok(())
            }
            Instr::ValueTransfer {
                success,
                address,
                value,
            } => {
                let address_op = self.to_operand_and_insns(address, vartable, &mut results)?;
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                results.push(Instruction::ValueTransfer {
                    success: *success,
                    address: address_op,
                    value: value_op,
                });
                Ok(())
            }
            Instr::SelfDestruct { recipient } => {
                let tmp = self.to_operand_and_insns(recipient, vartable, &mut results)?;
                results.push(Instruction::SelfDestruct { recipient: tmp });
                Ok(())
            }
            Instr::EmitEvent {
                event_no,
                data,
                topics,
            } => {
                let data_op = self.to_operand_and_insns(data, vartable, &mut results)?;
                let mut topic_ops = vec![];
                for topic in topics {
                    let topic_op = self.to_operand_and_insns(topic, vartable, &mut results)?;
                    topic_ops.push(topic_op);
                }
                results.push(Instruction::EmitEvent {
                    event_no: *event_no,
                    data: data_op,
                    topics: topic_ops,
                });
                Ok(())
            }
            Instr::WriteBuffer { buf, offset, value } => {
                let buf_op = self.to_operand_and_insns(buf, vartable, &mut results)?;
                let offset_op = self.to_operand_and_insns(offset, vartable, &mut results)?;
                let value_op = self.to_operand_and_insns(value, vartable, &mut results)?;
                results.push(Instruction::WriteBuffer {
                    buf: buf_op,
                    offset: offset_op,
                    value: value_op,
                });
                Ok(())
            }
            Instr::MemCopy {
                source,
                destination,
                bytes,
            } => {
                let source_op = self.to_operand_and_insns(source, vartable, &mut results)?;
                let dest_op = self.to_operand_and_insns(destination, vartable, &mut results)?;
                let bytes_op = self.to_operand_and_insns(bytes, vartable, &mut results)?;
                results.push(Instruction::MemCopy {
                    src: source_op,
                    dest: dest_op,
                    bytes: bytes_op,
                });
                Ok(())
            }
            Instr::Switch {
                cond,
                cases,
                default,
            } => {
                let cond_op = self.to_operand_and_insns(cond, vartable, &mut results)?;

                let mut case_ops = vec![];
                for (case, block_no) in cases {
                    let case_op = self.to_operand_and_insns(case, vartable, &mut results)?;
                    case_ops.push((case_op, *block_no));
                }

                results.push(Instruction::Switch {
                    cond: cond_op,
                    cases: case_ops,
                    default: *default,
                });

                Ok(())
            }
            Instr::ReturnData { data, data_len } => {
                let data_op = self.to_operand_and_insns(data, vartable, &mut results)?;
                let data_len_op = self.to_operand_and_insns(data_len, vartable, &mut results)?;
                results.push(Instruction::ReturnData {
                    data: data_op,
                    data_len: data_len_op,
                });
                Ok(())
            }
            Instr::ReturnCode { code } => {
                results.push(Instruction::ReturnCode { code: code.clone() });
                Ok(())
            }
            Instr::Unimplemented { reachable } => {
                results.push(Instruction::Unimplemented {
                    reachable: *reachable,
                });
                Ok(())
            }
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
                let args_op = self.to_operand_and_insns(encoded_args, vartable, &mut results)?;
                let value_op = self.to_operand_option_and_insns(value, vartable, &mut results)?;
                let gas_op = self.to_operand_and_insns(gas, vartable, &mut results)?;
                let salt_op = self.to_operand_option_and_insns(salt, vartable, &mut results)?;
                let address_op =
                    self.to_operand_option_and_insns(address, vartable, &mut results)?;
                let seeds_op = self.to_operand_option_and_insns(seeds, vartable, &mut results)?;
                let accounts =
                    self.to_external_call_accounts_and_insns(accounts, vartable, &mut results)?;

                results.push(Instruction::Constructor {
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
                });

                Ok(())
            }
        }
    }
}
