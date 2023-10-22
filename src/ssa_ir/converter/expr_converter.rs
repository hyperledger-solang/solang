// SPDX-License-Identifier: Apache-2.0

use solang_parser::pt::Loc;

use crate::codegen::Expression;
use crate::sema::ast::{self, RetrieveType};
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::expr::{BinaryOperator, Expr, Operand, UnaryOperator};
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::typechecker::TypeChecker;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub(crate) fn from_expression(
        &self,
        dest: &Operand,
        expr: &Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        match expr {
            Expression::Add {
                loc,
                ty,
                overflowing,
                left,
                right,
            } => {
                let operator = BinaryOperator::Add {
                    overflowing: *overflowing,
                };
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::AllocDynamicBytes {
                loc,
                ty,
                size,
                initializer,
                ..
            } => self.alloc_dynamic_bytes(dest, loc, ty, size, initializer, vartable),
            Expression::ArrayLiteral {
                loc,
                ty,
                dimensions,
                values,
                ..
            } => self.array_literal(dest, loc, ty, dimensions, values, vartable),
            Expression::BitwiseAnd {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitAnd;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::BitwiseOr {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitOr;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::BitwiseXor {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitXor;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::BoolLiteral { .. } => todo!("Expression::BoolLiteral"),
            Expression::Builtin { .. } => todo!("Expression::Builtin"),
            Expression::BytesCast { .. } => todo!("Expression::BytesCast"),
            Expression::BytesLiteral { .. } => todo!("Expression::BytesLiteral"),
            Expression::Cast { .. } => todo!("Expression::Cast"),
            Expression::BitwiseNot { loc, expr, ty, .. } => {
                let operator = UnaryOperator::BitNot;
                self.unary_operation(dest, loc, ty, operator, expr, vartable)
            }
            Expression::ConstArrayLiteral { .. } => todo!("Expression::ConstArrayLiteral"),
            Expression::UnsignedDivide {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::UDiv;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::SignedDivide {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Div;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::Equal {
                loc, left, right, ..
            } => {
                let operator = BinaryOperator::Eq;
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            Expression::FormatString { .. } => todo!("Expression::FormatString"),
            Expression::FunctionArg {
                loc, ty, arg_no, ..
            } => {
                let arg_ty = self.from_ast_type(ty)?;
                let expr = Expr::FunctionArg {
                    loc: loc.clone(),
                    ty: arg_ty,
                    arg_no: arg_no.clone(),
                };
                let res = dest.get_id_or_err()?;
                Ok(vec![Insn::Set {
                    loc: loc.clone(),
                    res,
                    expr,
                }])
            }
            Expression::GetRef { .. } => todo!("Expression::GetRef"),
            Expression::InternalFunctionCfg { .. } => todo!("Expression::InternalFunctionCfg"),
            Expression::Keccak256 { .. } => todo!("Expression::Keccak256"),
            Expression::Less {
                loc,
                left,
                right,
                signed,
                ..
            } => {
                let operator = if *signed {
                    BinaryOperator::Lt
                } else {
                    BinaryOperator::ULt
                };
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            Expression::LessEqual {
                loc,
                left,
                right,
                signed,
                ..
            } => {
                let operator = if *signed {
                    BinaryOperator::Lt
                } else {
                    BinaryOperator::ULt
                };
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            Expression::Load { .. } => todo!("Expression::Load"),
            Expression::UnsignedModulo { .. } => todo!("Expression::UnsignedModulo"),
            Expression::SignedModulo { .. } => todo!("Expression::SignedModulo"),
            Expression::More {
                left,
                right,
                signed,
                ..
            } => {
                let operator = if *signed {
                    BinaryOperator::Gt
                } else {
                    BinaryOperator::UGt
                };
                self.binary_operation(
                    dest,
                    &Loc::Codegen,
                    &ast::Type::Bool,
                    operator,
                    left,
                    right,
                    vartable,
                )
            }
            Expression::MoreEqual {
                left,
                right,
                signed,
                ..
            } => {
                let operator = if *signed {
                    BinaryOperator::Gte
                } else {
                    BinaryOperator::UGte
                };
                self.binary_operation(
                    dest,
                    &Loc::Codegen,
                    &ast::Type::Bool,
                    operator,
                    left,
                    right,
                    vartable,
                )
            }
            Expression::Multiply { .. } => todo!("Expression::Multiply"),
            Expression::Not { .. } => todo!("Expression::Not"),
            Expression::NotEqual { .. } => todo!("Expression::NotEqual"),
            Expression::NumberLiteral { loc, value, .. } => Ok(vec![
                // assign the constant value to the destination
                Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id_or_err()?,
                    expr: Expr::NumberLiteral {
                        loc: loc.clone(),
                        value: value.clone(),
                    },
                },
            ]),
            Expression::Poison => todo!("Expression::Poison"),
            Expression::Power { .. } => todo!("Expression::Power"),
            Expression::RationalNumberLiteral { .. } => todo!("Expression::RationalNumberLiteral"),
            Expression::ReturnData { .. } => todo!("Expression::ReturnData"),
            Expression::SignExt { loc, ty, expr, .. } => {
                // TODO: type checking
                let tmp = vartable.new_temp(&self.from_ast_type(&expr.ty())?);
                let mut expr_insns = self.from_expression(&tmp, expr, vartable)?;
                let sext = Expr::SignExt {
                    loc: loc.clone(),
                    operand: Box::new(tmp),
                    to_ty: self.from_ast_type(ty)?,
                };
                let mut insns = vec![];
                insns.append(&mut expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id_or_err()?,
                    expr: sext,
                });
                Ok(insns)
            }
            Expression::ShiftLeft {
                loc,
                ty,
                left,
                right,
                ..
            } => {
                let operator = BinaryOperator::Shl;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::ShiftRight { .. } => todo!("Expression::ShiftRight"),
            Expression::StorageArrayLength { .. } => todo!("Expression::StorageArrayLength"),
            Expression::StringCompare { .. } => todo!("Expression::StringCompare"),
            Expression::StringConcat { .. } => todo!("Expression::StringConcat"),
            Expression::StructLiteral { .. } => todo!("Expression::StructLiteral"),
            Expression::StructMember { .. } => todo!("Expression::StructMember"),
            Expression::Subscript {
                loc,
                array_ty,
                ty: elem_ty,
                expr,
                index,
                ..
            } => {
                TypeChecker::check_subscript(&array_ty, &elem_ty, &index.ty())?;

                let array_op = vartable.new_temp(&self.from_ast_type(array_ty)?);
                let array_insns = self.from_expression(&array_op, expr, vartable)?;

                let index_op = vartable.new_temp(&self.from_ast_type(&index.ty())?);
                let index_insns = self.from_expression(&index_op, index, vartable)?;

                let mut insns = vec![];
                insns.extend(array_insns);
                insns.extend(index_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id_or_err()?,
                    expr: Expr::Subscript {
                        loc: loc.clone(),
                        arr: Box::new(array_op),
                        index: Box::new(index_op),
                    },
                });

                Ok(insns)
            }
            Expression::Subtract { .. } => todo!("Expression::Subtract"),
            Expression::Trunc { .. } => todo!("Expression::Trunc"),
            Expression::Negate { .. } => todo!("Expression::Negate"),
            Expression::Undefined { .. } => todo!("Expression::Undefined"),
            Expression::Variable { loc, var_no, .. } => {
                let expr = Expr::Id {
                    loc: loc.clone(),
                    id: var_no.clone(),
                };
                let res = dest.get_id_or_err()?;
                Ok(vec![Insn::Set {
                    loc: Loc::Codegen,
                    res,
                    expr,
                }])
            }
            Expression::ZeroExt { .. } => todo!("Expression::ZeroExt"),
            Expression::AdvancePointer { .. } => todo!("Expression::AdvancePointer"),
        }
    }

    fn binary_operation(
        &self,
        dest: &Operand,
        loc: &Loc,
        _: &ast::Type,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        TypeChecker::check_binary_op(&left.ty(), &right.ty())?;

        let left_ty = self.from_ast_type(&left.ty())?;
        let right_ty = self.from_ast_type(&right.ty())?;

        let left_op = vartable.new_temp(&left_ty);
        let left_insns = self.from_expression(&left_op, left, vartable)?;

        let right_op = vartable.new_temp(&right_ty);
        let right_insns = self.from_expression(&right_op, right, vartable)?;

        let mut insns = vec![];
        insns.extend(left_insns);
        insns.extend(right_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id_or_err()?,
            expr: Expr::BinaryExpr {
                loc: loc.clone(),
                operator,
                left: Box::new(left_op),
                right: Box::new(right_op),
            },
        });

        Ok(insns)
    }

    fn unary_operation(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        operator: UnaryOperator,
        expr: &Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        let res_ty = self.from_ast_type(ty)?;
        TypeChecker::check_unary_op(ty, &expr.ty())?;

        let expr_op = vartable.new_temp(&res_ty);
        let expr_insns = self.from_expression(&expr_op, expr, vartable)?;

        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id_or_err()?,
            expr: Expr::UnaryExpr {
                loc: loc.clone(),
                operator,
                right: Box::new(expr_op),
            },
        });

        Ok(insns)
    }

    fn alloc_dynamic_bytes(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        size: &Box<Expression>,
        initializer: &Option<Vec<u8>>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        TypeChecker::check_alloc_dynamic_bytes(ty, &size.ty())?;

        let size_op = vartable.new_temp(&self.from_ast_type(&size.ty())?);
        let left_insns = self.from_expression(&size_op, size, vartable)?;

        let mut insns = vec![];
        insns.extend(left_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id_or_err()?,
            expr: Expr::AllocDynamicBytes {
                loc: loc.clone(),
                ty: self.from_ast_type(ty)?,
                size: Box::new(size_op),
                initializer: initializer.clone(),
            },
        });

        Ok(insns)
    }

    fn array_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        dimensions: &Vec<u32>,
        values: &Vec<Expression>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Insn>, String> {
        let arr_ty = self.from_ast_type(ty)?;

        let mut insns = vec![];

        let value_ops = values
            .iter()
            .map(|value| {
                let op = vartable.new_temp(&arr_ty);
                let insn = self.from_expression(&op, value, vartable)?;
                insns.extend(insn);
                Ok(op)
            })
            .collect::<Result<Vec<Operand>, String>>()?;

        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id_or_err()?,
            expr: Expr::ArrayLiteral {
                loc: loc.clone(),
                ty: arr_ty,
                dimensions: dimensions.clone(),
                values: value_ops,
            },
        });

        Ok(insns)
    }
}
