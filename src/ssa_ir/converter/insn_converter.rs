use crate::codegen::cfg::Instr;
use crate::sema::ast::RetrieveType;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::expr::Expr;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::ssa_type::Type;
use crate::ssa_ir::typechecker;
use crate::ssa_ir::vartable::Vartable;
use solang_parser::pt::Loc;

impl Converter {
    pub(crate) fn from_instr(
        instr: &Instr,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, &'static str> {
        match instr {
            Instr::Nop => Ok(vec![Insn::Nop]),
            Instr::Set { loc, res, expr } => {
                // [t] a = b + c * d
                // converts to:
                //   1. [t1] tmp_1 = c * d;
                //   2. [t2] tmp_2 = b + tmp_1
                //   3. [t] a = tmp_2;
                let expr_operand = vartable.get_operand(res)?;
                let mut expr_insns = Converter::from_expression(&expr_operand, &expr, vartable)?;

                // TODO type checking
                let dest_ty = vartable.get_type(res)?;
                typechecker::check_assignment(dest_ty, &Type::try_from(&expr.ty())?)?;

                let mut insns = Vec::new();
                insns.append(&mut expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: res.clone(),
                    expr: Expr::Cast {
                        ty: dest_ty.clone(),
                        loc: Loc::Codegen,
                        operand: Box::new(expr_operand),
                    },
                });

                Ok(insns)
            }
            Instr::Store { dest, data } => {
                // type checking the dest.ty() and data.ty()

                let mut dest_op = vartable.new_temp(Type::try_from(&dest.ty())?);
                let mut dest_insns = Converter::from_expression(&dest_op, dest, vartable)?;

                let mut data_op = vartable.new_temp(Type::try_from(&data.ty())?);
                let mut data_insns = Converter::from_expression(&data_op, data, vartable)?;

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
                res,
                ty,
                array,
                value,
            } => {
                let mut value_op = vartable.new_temp(Type::try_from(&value.ty())?);
                let mut value_insns = Converter::from_expression(&value_op, value, vartable)?;

                let mut insns = Vec::new();
                insns.append(&mut value_insns);
                insns.push(Insn::PushMemory {
                    res: res.clone(),
                    ty: ty.clone(),
                    array: array.clone(),
                    value: value_op,
                });
                Ok(insns)
            }
            Instr::PopMemory {
                res,
                ty,
                array,
                loc,
            } => todo!("PopMemory"),
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
            } => todo!("Constructor"),
            _ => Err("Not implemented yet"),
        }
    }
}
