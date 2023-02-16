// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::AvailableExpression;
use crate::codegen::subexpression_elimination::{AvailableExpressionSet, AvailableVariable};
use crate::codegen::Expression;

impl AvailableExpressionSet {
    /// Check if we can add the expressions of an instruction to the graph
    pub fn process_instruction(
        &mut self,
        instr: &Instr,
        ave: &mut AvailableExpression,
        cst: &mut CommonSubExpressionTracker,
    ) {
        match instr {
            Instr::BranchCond { cond: expr, .. }
            | Instr::LoadStorage { storage: expr, .. }
            | Instr::ClearStorage { storage: expr, .. }
            | Instr::Print { expr }
            | Instr::AssertFailure {
                encoded_args: Some(expr),
            }
            | Instr::PopStorage { storage: expr, .. }
            | Instr::AbiDecode { data: expr, .. }
            | Instr::SelfDestruct { recipient: expr } => {
                let _ = self.gen_expression(expr, ave, cst);
            }

            Instr::Set { res, expr, loc } => {
                let node_id = self.gen_expression(expr, ave, cst);
                if node_id.is_some() {
                    let node = &mut *self
                        .expression_memory
                        .get(node_id.as_ref().unwrap())
                        .unwrap()
                        .borrow_mut();
                    if !node.available_variable.is_available() {
                        node.available_variable = AvailableVariable::Available(*res, *loc);
                    }
                }
                cst.invalidate_mapped_variable(res);
                self.kill(*res);
            }

            Instr::PushMemory { value: expr, .. } => {
                let _ = self.gen_expression(expr, ave, cst);
            }

            Instr::SetStorage {
                value: item_1,
                storage: item_2,
                ..
            }
            | Instr::ReturnData {
                data: item_1,
                data_len: item_2,
            }
            | Instr::Store {
                dest: item_1,
                data: item_2,
            } => {
                let _ = self.gen_expression(item_1, ave, cst);
                let _ = self.gen_expression(item_2, ave, cst);
            }
            Instr::PushStorage { value, storage, .. } => {
                if let Some(value) = value {
                    let _ = self.gen_expression(value, ave, cst);
                }
                let _ = self.gen_expression(storage, ave, cst);
            }

            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                let _ = self.gen_expression(value, ave, cst);
                let _ = self.gen_expression(storage, ave, cst);
                let _ = self.gen_expression(offset, ave, cst);
            }

            Instr::Return { value: exprs } | Instr::Call { args: exprs, .. } => {
                for expr in exprs {
                    let _ = self.gen_expression(expr, ave, cst);
                }
            }

            Instr::Constructor {
                encoded_args,
                encoded_args_len,
                value,
                gas,
                salt,
                address,
                ..
            } => {
                let _ = self.gen_expression(encoded_args, ave, cst);
                let _ = self.gen_expression(encoded_args_len, ave, cst);
                if let Some(expr) = value {
                    let _ = self.gen_expression(expr, ave, cst);
                }

                let _ = self.gen_expression(gas, ave, cst);

                if let Some(expr) = salt {
                    let _ = self.gen_expression(expr, ave, cst);
                }

                if let Some(expr) = address {
                    let _ = self.gen_expression(expr, ave, cst);
                }
            }

            Instr::ExternalCall {
                address,
                payload,
                value,
                gas,
                accounts,
                seeds,
                ..
            } => {
                if let Some(expr) = address {
                    let _ = self.gen_expression(expr, ave, cst);
                }
                if let Some(expr) = accounts {
                    let _ = self.gen_expression(expr, ave, cst);
                }
                if let Some(expr) = seeds {
                    let _ = self.gen_expression(expr, ave, cst);
                }
                let _ = self.gen_expression(payload, ave, cst);
                let _ = self.gen_expression(value, ave, cst);
                let _ = self.gen_expression(gas, ave, cst);
            }

            Instr::ValueTransfer { address, value, .. } => {
                let _ = self.gen_expression(address, ave, cst);
                let _ = self.gen_expression(value, ave, cst);
            }

            Instr::EmitEvent { data, topics, .. } => {
                for expr in data {
                    let _ = self.gen_expression(expr, ave, cst);
                }

                for expr in topics {
                    let _ = self.gen_expression(expr, ave, cst);
                }
            }

            Instr::WriteBuffer {
                buf, offset, value, ..
            } => {
                let _ = self.gen_expression(buf, ave, cst);
                let _ = self.gen_expression(offset, ave, cst);
                let _ = self.gen_expression(value, ave, cst);
            }

            Instr::MemCopy {
                source: from,
                destination: to,
                bytes,
            } => {
                let _ = self.gen_expression(from, ave, cst);
                let _ = self.gen_expression(to, ave, cst);
                let _ = self.gen_expression(bytes, ave, cst);
            }

            Instr::Switch { cond, cases, .. } => {
                let _ = self.gen_expression(cond, ave, cst);
                for (case, _) in cases {
                    let _ = self.gen_expression(case, ave, cst);
                }
            }

            Instr::AssertFailure { encoded_args: None }
            | Instr::Unreachable
            | Instr::Nop
            | Instr::ReturnCode { .. }
            | Instr::Branch { .. }
            | Instr::PopMemory { .. } => {}
        }
    }

    /// Regenerate instructions after that we exchanged common subexpressions for temporaries
    pub fn regenerate_instruction(
        &mut self,
        instr: &Instr,
        ave: &mut AvailableExpression,
        cst: &mut CommonSubExpressionTracker,
    ) -> Instr {
        match instr {
            Instr::Set { loc, res, expr } => {
                let new_instr = Instr::Set {
                    loc: *loc,
                    res: *res,
                    expr: self.regenerate_expression(expr, ave, cst).1,
                };
                self.kill(*res);
                new_instr
            }

            Instr::Call {
                res,
                return_tys,
                call,
                args,
            } => Instr::Call {
                res: res.clone(),
                return_tys: return_tys.clone(),
                call: call.clone(),
                args: args
                    .iter()
                    .map(|v| self.regenerate_expression(v, ave, cst).1)
                    .collect::<Vec<Expression>>(),
            },

            Instr::Return { value } => Instr::Return {
                value: value
                    .iter()
                    .map(|v| self.regenerate_expression(v, ave, cst).1)
                    .collect::<Vec<Expression>>(),
            },

            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => Instr::BranchCond {
                cond: self.regenerate_expression(cond, ave, cst).1,
                true_block: *true_block,
                false_block: *false_block,
            },

            Instr::Store { dest, data } => Instr::Store {
                dest: self.regenerate_expression(dest, ave, cst).1,
                data: self.regenerate_expression(data, ave, cst).1,
            },

            Instr::AssertFailure {
                encoded_args: Some(exp),
            } => Instr::AssertFailure {
                encoded_args: Some(self.regenerate_expression(exp, ave, cst).1),
            },

            Instr::Print { expr } => Instr::Print {
                expr: self.regenerate_expression(expr, ave, cst).1,
            },

            Instr::LoadStorage { res, ty, storage } => Instr::LoadStorage {
                res: *res,
                ty: ty.clone(),
                storage: self.regenerate_expression(storage, ave, cst).1,
            },

            Instr::ClearStorage { ty, storage } => Instr::ClearStorage {
                ty: ty.clone(),
                storage: self.regenerate_expression(storage, ave, cst).1,
            },

            Instr::SetStorage { ty, value, storage } => Instr::SetStorage {
                ty: ty.clone(),
                value: self.regenerate_expression(value, ave, cst).1,
                storage: self.regenerate_expression(storage, ave, cst).1,
            },

            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => Instr::SetStorageBytes {
                value: self.regenerate_expression(value, ave, cst).1,
                storage: self.regenerate_expression(storage, ave, cst).1,
                offset: self.regenerate_expression(offset, ave, cst).1,
            },

            Instr::PushStorage {
                res,
                ty,
                value,
                storage,
            } => Instr::PushStorage {
                res: *res,
                ty: ty.clone(),
                value: value
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1),
                storage: self.regenerate_expression(storage, ave, cst).1,
            },

            Instr::PopStorage { res, ty, storage } => Instr::PopStorage {
                res: *res,
                ty: ty.clone(),
                storage: self.regenerate_expression(storage, ave, cst).1,
            },

            Instr::PushMemory {
                res,
                ty,
                array,
                value,
            } => Instr::PushMemory {
                res: *res,
                ty: ty.clone(),
                array: *array,
                value: Box::new(self.regenerate_expression(value, ave, cst).1),
            },

            Instr::Constructor {
                success,
                res,
                contract_no,
                encoded_args,
                encoded_args_len,
                value,
                gas,
                salt,
                address,
                seeds,
                loc,
            } => {
                let new_value = value
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                let new_salt = salt
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                let new_address = address
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                let new_seeds = seeds
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                Instr::Constructor {
                    success: *success,
                    res: *res,
                    contract_no: *contract_no,
                    encoded_args: self.regenerate_expression(encoded_args, ave, cst).1,
                    encoded_args_len: self.regenerate_expression(encoded_args_len, ave, cst).1,
                    value: new_value,
                    gas: self.regenerate_expression(gas, ave, cst).1,
                    salt: new_salt,
                    address: new_address,
                    seeds: new_seeds,
                    loc: *loc,
                }
            }

            Instr::ExternalCall {
                success,
                address,
                accounts,
                payload,
                value,
                gas,
                callty,
                seeds,
            } => {
                let new_address = address
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                let new_accounts = accounts
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                let new_seeds = seeds
                    .as_ref()
                    .map(|expr| self.regenerate_expression(expr, ave, cst).1);

                Instr::ExternalCall {
                    success: *success,
                    address: new_address,
                    accounts: new_accounts,
                    seeds: new_seeds,
                    payload: self.regenerate_expression(payload, ave, cst).1,
                    value: self.regenerate_expression(value, ave, cst).1,
                    gas: self.regenerate_expression(gas, ave, cst).1,
                    callty: callty.clone(),
                }
            }

            Instr::ValueTransfer {
                success,
                address,
                value,
            } => Instr::ValueTransfer {
                success: *success,
                address: self.regenerate_expression(address, ave, cst).1,
                value: self.regenerate_expression(value, ave, cst).1,
            },

            Instr::AbiDecode {
                res,
                selector,
                exception_block,
                tys,
                data,
                data_len,
            } => Instr::AbiDecode {
                res: res.clone(),
                selector: *selector,
                exception_block: *exception_block,
                tys: tys.clone(),
                data: self.regenerate_expression(data, ave, cst).1,
                data_len: data_len
                    .as_ref()
                    .map(|e| self.regenerate_expression(e, ave, cst).1),
            },

            Instr::SelfDestruct { recipient } => Instr::SelfDestruct {
                recipient: self.regenerate_expression(recipient, ave, cst).1,
            },

            Instr::EmitEvent {
                event_no,
                data,
                data_tys,
                topics,
                topic_tys,
            } => Instr::EmitEvent {
                event_no: *event_no,
                data: data
                    .iter()
                    .map(|v| self.regenerate_expression(v, ave, cst).1)
                    .collect::<Vec<Expression>>(),
                data_tys: data_tys.clone(),
                topics: topics
                    .iter()
                    .map(|v| self.regenerate_expression(v, ave, cst).1)
                    .collect::<Vec<Expression>>(),
                topic_tys: topic_tys.clone(),
            },

            Instr::MemCopy {
                source: from,
                destination: to,
                bytes,
            } => Instr::MemCopy {
                source: self.regenerate_expression(from, ave, cst).1,
                destination: self.regenerate_expression(to, ave, cst).1,
                bytes: self.regenerate_expression(bytes, ave, cst).1,
            },

            Instr::Switch {
                cond,
                cases,
                default,
            } => Instr::Switch {
                cond: self.regenerate_expression(cond, ave, cst).1,
                cases: cases
                    .iter()
                    .map(|(case, goto)| (self.regenerate_expression(case, ave, cst).1, *goto))
                    .collect::<Vec<(Expression, usize)>>(),
                default: *default,
            },

            Instr::WriteBuffer { buf, offset, value } => Instr::WriteBuffer {
                buf: self.regenerate_expression(buf, ave, cst).1,
                offset: self.regenerate_expression(offset, ave, cst).1,
                value: self.regenerate_expression(value, ave, cst).1,
            },

            Instr::ReturnData { data, data_len } => Instr::ReturnData {
                data: self.regenerate_expression(data, ave, cst).1,
                data_len: self.regenerate_expression(data_len, ave, cst).1,
            },

            _ => instr.clone(),
        }
    }
}
