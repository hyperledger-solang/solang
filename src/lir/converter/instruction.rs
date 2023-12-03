// SPDX-License-Identifier: Apache-2.0

use solang_parser::pt::Loc;

use crate::codegen::cfg::Instr;
use crate::lir::converter::Converter;
use crate::lir::expressions::Operand;
use crate::lir::instructions::Instruction;
use crate::lir::vartable::Vartable;

impl Converter<'_> {
    /// lower the `codegen::cfg::Instr` into a list of `Instruction`s.
    /// Input:
    /// - `instr`: the `codegen::cfg::Instr` to be lowered.
    /// - `vartable`: the `Vartable` that stores the variables and their types.
    /// - `results`: the list of `Instruction`s that the lowered instructions will be appended to.
    pub(crate) fn lower_instr(
        &self,
        instr: &Instr,
        vartable: &mut Vartable,
        results: &mut Vec<Instruction>,
    ) {
        match instr {
            Instr::Nop => {
                results.push(Instruction::Nop);
            }
            Instr::Set { res, expr, loc, .. } => {
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let dest_operand = vartable.get_operand(res, *loc);
                self.lower_expression(&dest_operand, expr, vartable, results);
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()
                let dest_op = self.to_operand_and_insns(dest, vartable, results);
                let data_op = self.to_operand_and_insns(data, vartable, results);
                results.push(Instruction::Store {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    dest: dest_op,
                    data: data_op,
                });
            }
            Instr::PushMemory {
                res, array, value, ..
            } => {
                let value_op = self.to_operand_and_insns(value, vartable, results);
                results.push(Instruction::PushMemory {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: *res,
                    array: *array,
                    value: value_op,
                });
            }
            Instr::PopMemory {
                res, array, loc, ..
            } => {
                results.push(Instruction::PopMemory {
                    res: *res,
                    array: *array,
                    loc: *loc,
                });
            }

            Instr::Branch { block } => {
                results.push(Instruction::Branch {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    block: *block,
                });
            }
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                let cond_op = self.to_operand_and_insns(cond, vartable, results);
                results.push(Instruction::BranchCond {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    cond: cond_op,
                    true_block: *true_block,
                    false_block: *false_block,
                });
            }
            Instr::Return { value } => {
                let operands = value
                    .iter()
                    .map(|v| self.to_operand_and_insns(v, vartable, results))
                    .collect::<Vec<Operand>>();
                results.push(Instruction::Return {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    value: operands,
                });
            }
            Instr::AssertFailure { encoded_args } => match encoded_args {
                Some(args) => {
                    let tmp = self.to_operand_and_insns(args, vartable, results);
                    results.push(Instruction::AssertFailure {
                        loc: /*missing from cfg*/ Loc::Codegen,
                        encoded_args: Some(tmp),
                    });
                }
                None => {
                    results.push(Instruction::AssertFailure {
                        loc: /*missing from cfg*/ Loc::Codegen,
                        encoded_args: None,
                    });
                }
            },
            Instr::Call {
                res, call, args, ..
            } => {
                // resolve the function
                let callty = self.to_internal_call_ty_and_insns(call, vartable, results);

                // resolve the arguments
                let arg_ops = args
                    .iter()
                    .map(|arg| self.to_operand_and_insns(arg, vartable, results))
                    .collect::<Vec<Operand>>();

                results.push(Instruction::Call {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: res.clone(),
                    call: callty,
                    args: arg_ops,
                });
            }
            Instr::Print { expr } => {
                let tmp = self.to_operand_and_insns(expr, vartable, results);
                results.push(Instruction::Print {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    operand: tmp,
                });
            }
            Instr::LoadStorage { res, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                results.push(Instruction::LoadStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: *res,
                    storage: storage_op,
                });
            }
            Instr::ClearStorage { storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                results.push(Instruction::ClearStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    storage: storage_op,
                });
            }
            Instr::SetStorage { value, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                let value_op = self.to_operand_and_insns(value, vartable, results);
                results.push(Instruction::SetStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    value: value_op,
                    storage: storage_op,
                });
            }
            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                let value_op = self.to_operand_and_insns(value, vartable, results);
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                let offset_op = self.to_operand_and_insns(offset, vartable, results);
                results.push(Instruction::SetStorageBytes {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    value: value_op,
                    storage: storage_op,
                    offset: offset_op,
                });
            }
            Instr::PushStorage {
                res,
                value,
                storage,
                ..
            } => {
                let value_op = self.to_operand_option_and_insns(value, vartable, results);
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                results.push(Instruction::PushStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: *res,
                    value: value_op,
                    storage: storage_op,
                });
            }
            Instr::PopStorage { res, storage, .. } => {
                let storage_op = self.to_operand_and_insns(storage, vartable, results);
                results.push(Instruction::PopStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: *res,
                    storage: storage_op,
                });
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
                let address_op = self.to_operand_option_and_insns(address, vartable, results);
                let accounts_op =
                    self.to_external_call_accounts_and_insns(accounts, vartable, results);
                let seeds_op = self.to_operand_option_and_insns(seeds, vartable, results);
                let payload_op = self.to_operand_and_insns(payload, vartable, results);
                let value_op = self.to_operand_and_insns(value, vartable, results);
                let gas_op = self.to_operand_and_insns(gas, vartable, results);
                let flags_op = self.to_operand_option_and_insns(flags, vartable, results);

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
            }
            Instr::ValueTransfer {
                success,
                address,
                value,
            } => {
                let address_op = self.to_operand_and_insns(address, vartable, results);
                let value_op = self.to_operand_and_insns(value, vartable, results);
                results.push(Instruction::ValueTransfer {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    success: *success,
                    address: address_op,
                    value: value_op,
                });
            }
            Instr::SelfDestruct { recipient } => {
                let tmp = self.to_operand_and_insns(recipient, vartable, results);
                results.push(Instruction::SelfDestruct {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    recipient: tmp,
                });
            }
            Instr::EmitEvent {
                event_no,
                data,
                topics,
            } => {
                let data_op = self.to_operand_and_insns(data, vartable, results);
                let topic_ops = topics
                    .iter()
                    .map(|topic| self.to_operand_and_insns(topic, vartable, results))
                    .collect::<Vec<Operand>>();
                results.push(Instruction::EmitEvent {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    event_no: *event_no,
                    data: data_op,
                    topics: topic_ops,
                });
            }
            Instr::WriteBuffer { buf, offset, value } => {
                let buf_op = self.to_operand_and_insns(buf, vartable, results);
                let offset_op = self.to_operand_and_insns(offset, vartable, results);
                let value_op = self.to_operand_and_insns(value, vartable, results);
                results.push(Instruction::WriteBuffer {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    buf: buf_op,
                    offset: offset_op,
                    value: value_op,
                });
            }
            Instr::MemCopy {
                source,
                destination,
                bytes,
            } => {
                let source_op = self.to_operand_and_insns(source, vartable, results);
                let dest_op = self.to_operand_and_insns(destination, vartable, results);
                let bytes_op = self.to_operand_and_insns(bytes, vartable, results);
                results.push(Instruction::MemCopy {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    src: source_op,
                    dest: dest_op,
                    bytes: bytes_op,
                });
            }
            Instr::Switch {
                cond,
                cases,
                default,
            } => {
                let cond_op = self.to_operand_and_insns(cond, vartable, results);

                let case_ops = cases
                    .iter()
                    .map(|(case, block_no)| {
                        let case_op = self.to_operand_and_insns(case, vartable, results);
                        (case_op, *block_no)
                    })
                    .collect::<Vec<(Operand, usize)>>();

                results.push(Instruction::Switch {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    cond: cond_op,
                    cases: case_ops,
                    default: *default,
                });
            }
            Instr::ReturnData { data, data_len } => {
                let data_op = self.to_operand_and_insns(data, vartable, results);
                let data_len_op = self.to_operand_and_insns(data_len, vartable, results);
                results.push(Instruction::ReturnData {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    data: data_op,
                    data_len: data_len_op,
                });
            }
            Instr::ReturnCode { code } => {
                results.push(Instruction::ReturnCode {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    code: code.clone(),
                });
            }
            Instr::Unimplemented { .. } => unreachable!("Unimplemented should be removed"),
            Instr::AccountAccess { .. } => {
                unreachable!("AccountAccess should be replaced by Subscript")
            }
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
                let args_op = self.to_operand_and_insns(encoded_args, vartable, results);
                let value_op = self.to_operand_option_and_insns(value, vartable, results);
                let gas_op = self.to_operand_and_insns(gas, vartable, results);
                let salt_op = self.to_operand_option_and_insns(salt, vartable, results);
                let address_op = self.to_operand_option_and_insns(address, vartable, results);
                let seeds_op = self.to_operand_option_and_insns(seeds, vartable, results);
                let accounts =
                    self.to_external_call_accounts_and_insns(accounts, vartable, results);

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
            }
        }
    }
}
