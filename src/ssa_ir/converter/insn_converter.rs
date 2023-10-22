// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::sema::ast::RetrieveType;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::typechecker::TypeChecker;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub(crate) fn from_instr(
        &self,
        instr: &Instr,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        match instr {
            Instr::Nop => Ok(vec![Insn::Nop]),
            Instr::Set { res, expr, .. } => {
                TypeChecker::check_assignment(&self.get_ast_type_by_id(res)?, &expr.ty())?;
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let dest_operand = vartable.get_operand(res)?;
                self.from_expression(&dest_operand, &expr, vartable)
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()

                let dest_op = vartable.new_temp(&self.from_ast_type(&dest.ty())?);
                let mut dest_insns = self.from_expression(&dest_op, dest, vartable)?;

                let data_op = vartable.new_temp(&self.from_ast_type(&data.ty())?);
                let mut data_insns = self.from_expression(&data_op, data, vartable)?;

                let mut insns = Vec::new();
                insns.append(&mut dest_insns);
                insns.append(&mut data_insns);
                insns.push(Insn::Store {
                    dest: dest_op,
                    data: data_op,
                });
                Ok(insns)
            }
            Instr::PushMemory {
                res, array, value, ..
            } => {
                let value_op = vartable.get_operand(res)?;
                let mut value_insns = self.from_expression(&value_op, value, vartable)?;

                let mut insns = Vec::new();
                insns.append(&mut value_insns);
                insns.push(Insn::PushMemory {
                    res: res.clone(),
                    array: array.clone(),
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::PopMemory {
                ..
            } => todo!("PopMemory"),
            Instr::Constructor {
                ..
            } => todo!("Constructor"),
            Instr::Branch { block } => Ok(vec![Insn::Branch {
                block: block.clone(),
            }]),
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                let op = vartable.new_temp(&self.from_ast_type(&cond.ty())?);
                let mut cond_insns = self.from_expression(&op, cond, vartable)?;
                let mut insns = Vec::new();
                insns.append(&mut cond_insns);
                insns.push(Insn::BranchCond {
                    cond: op,
                    true_block: true_block.clone(),
                    false_block: false_block.clone(),
                });
                Ok(insns)
            }
            Instr::Return { value } => {
                let mut operands = Vec::new();
                let mut insns = Vec::new();
                for v in value {
                    let tmp = vartable.new_temp(&self.from_ast_type(&v.ty())?);
                    let mut expr_insns = self.from_expression(&tmp, v, vartable)?;
                    insns.append(&mut expr_insns);
                    operands.push(tmp);
                }
                insns.push(Insn::Return { value: operands });
                Ok(insns)
            }
            Instr::AssertFailure { encoded_args } => match encoded_args {
                Some(args) => {
                    let tmp = vartable.new_temp(&self.from_ast_type(&args.ty())?);
                    let mut expr_insns = self.from_expression(&tmp, args, vartable)?;
                    let mut insns = Vec::new();
                    insns.append(&mut expr_insns);
                    insns.push(Insn::AssertFailure {
                        encoded_args: Some(tmp),
                    });
                    Ok(insns)
                }
                None => Ok(vec![Insn::AssertFailure { encoded_args: None }]),
            },
            _ => unimplemented!("{:?}", instr),
        }
    }
}
