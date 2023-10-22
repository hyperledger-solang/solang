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
            Instr::Set { res, expr, loc, .. } => {
                TypeChecker::check_assignment(&self.get_ast_type_by_id(res)?, &expr.ty())?;
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let dest_operand = vartable.get_operand(res, loc.clone())?;
                self.from_expression(&dest_operand, &expr, vartable)
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()
                let (dest_op, dest_insns) = self.as_operand_and_insns(dest, vartable)?;
                let (data_op, data_insns) = self.as_operand_and_insns(data, vartable)?;
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
                let (value_op, value_insns) = self.as_operand_and_insns(value, vartable)?;
                let mut insns = vec![];
                insns.extend(value_insns);
                insns.push(Insn::PushMemory {
                    res: res.clone(),
                    array: array.clone(),
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::PopMemory {
                res, array, loc, ..
            } => Ok(vec![Insn::PopMemory {
                res: res.clone(),
                array: array.clone(),
                loc: loc.clone(),
            }]),
            Instr::Constructor { .. } => todo!("Constructor"),
            Instr::Branch { block } => Ok(vec![Insn::Branch {
                block: block.clone(),
            }]),
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                let (cond_op, cond_insns) = self.as_operand_and_insns(cond, vartable)?;
                let mut insns = Vec::new();
                insns.extend(cond_insns);
                insns.push(Insn::BranchCond {
                    cond: cond_op,
                    true_block: true_block.clone(),
                    false_block: false_block.clone(),
                });
                Ok(insns)
            }
            Instr::Return { value } => {
                let mut operands = vec![];
                let mut insns = vec![];
                for v in value {
                    let (tmp, expr_insns) = self.as_operand_and_insns(v, vartable)?;
                    insns.extend(expr_insns);
                    operands.push(tmp);
                }
                insns.push(Insn::Return { value: operands });
                Ok(insns)
            }
            Instr::AssertFailure { encoded_args } => match encoded_args {
                Some(args) => {
                    let (tmp, expr_insns) = self.as_operand_and_insns(args, vartable)?;
                    let mut insns = vec![];
                    insns.extend(expr_insns);
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
