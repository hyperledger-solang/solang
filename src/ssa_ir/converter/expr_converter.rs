// SPDX-License-Identifier: Apache-2.0

use core::panic;

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
            Expression::BoolLiteral { loc, value, .. } => {
                let expr = Expr::BoolLiteral {
                    loc: loc.clone(),
                    value: *value,
                };
                let res = dest.get_id()?;
                Ok(vec![Insn::Set {
                    loc: loc.clone(),
                    res,
                    expr,
                }])
            }
            Expression::Builtin {
                loc, kind, args, ..
            } => {
                let mut insns = vec![];
                let mut arg_ops = vec![];
                for arg in args {
                    let (op, insn) = self.as_operand_and_insns(arg, vartable)?;
                    insns.extend(insn);
                    arg_ops.push(op);
                }
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Builtin {
                        loc: loc.clone(),
                        kind: kind.clone(),
                        args: arg_ops,
                    },
                });
                Ok(insns)
            }
            Expression::BytesCast {
                loc,
                expr,
                from,
                ty,
                ..
            } => {
                TypeChecker::assert_ty_eq(from, &expr.ty())?;
                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::BytesCast {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                        to_ty: self.from_ast_type(ty)?,
                    },
                });
                Ok(insns)
            }
            Expression::BytesLiteral { loc, ty, value, .. } => {
                let expr = Expr::BytesLiteral {
                    loc: loc.clone(),
                    ty: self.from_ast_type(ty)?,
                    value: value.clone(),
                };
                let res = dest.get_id()?;
                Ok(vec![Insn::Set {
                    loc: loc.clone(),
                    res,
                    expr,
                }])
            }
            Expression::Cast { loc, ty, expr, .. } => {
                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Cast {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                        to_ty: self.from_ast_type(ty)?,
                    },
                });
                Ok(insns)
            }
            Expression::BitwiseNot { loc, expr, ty, .. } => {
                let operator = UnaryOperator::BitNot;
                self.unary_operation(dest, loc, ty, operator, expr, vartable)
            }
            Expression::ConstArrayLiteral {
                loc,
                ty,
                dimensions,
                values,
                ..
            } => {
                let arr_ty = self.from_ast_type(ty)?;

                // check values type
                TypeChecker::check_array_elem_tys(
                    ty,
                    &values.iter().map(|v| v.ty()).collect::<Vec<_>>(),
                )?;

                let mut insns = vec![];

                let value_ops = values
                    .iter()
                    .map(|value| {
                        let (op, insn) = self.as_operand_and_insns(value, vartable)?;
                        insns.extend(insn);
                        Ok(op)
                    })
                    .collect::<Result<Vec<Operand>, String>>()?;

                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::ConstArrayLiteral {
                        loc: loc.clone(),
                        ty: arr_ty,
                        dimensions: dimensions.clone(),
                        values: value_ops,
                    },
                });

                Ok(insns)
            }
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
            Expression::FormatString { loc, args, .. } => {
                let mut insns = vec![];
                let mut arg_ops = vec![];
                for (format, arg) in args {
                    let (op, insn) = self.as_operand_and_insns(arg, vartable)?;
                    insns.extend(insn);
                    arg_ops.push((format.clone(), op));
                }
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::FormatString {
                        loc: loc.clone(),
                        args: arg_ops,
                    },
                });
                Ok(insns)
            }
            Expression::FunctionArg {
                loc, ty, arg_no, ..
            } => {
                let arg_ty = self.from_ast_type(ty)?;
                let expr = Expr::FunctionArg {
                    loc: loc.clone(),
                    ty: arg_ty,
                    arg_no: arg_no.clone(),
                };
                let res = dest.get_id()?;
                Ok(vec![Insn::Set {
                    loc: loc.clone(),
                    res,
                    expr,
                }])
            }
            Expression::GetRef { loc, expr, .. } => {
                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::GetRef {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                    },
                });
                Ok(insns)
            }
            Expression::InternalFunctionCfg { cfg_no, .. } => {
                let expr = Expr::InternalFunctionCfg {
                    cfg_no: cfg_no.clone(),
                };
                let res = dest.get_id()?;
                Ok(vec![Insn::Set {
                    loc: Loc::Codegen,
                    res,
                    expr,
                }])
            }
            Expression::Keccak256 { loc, exprs, .. } => {
                let mut insns = vec![];
                let mut expr_ops = vec![];
                for expr in exprs {
                    let (op, insn) = self.as_operand_and_insns(expr, vartable)?;
                    insns.extend(insn);
                    expr_ops.push(op);
                }
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Keccak256 {
                        loc: loc.clone(),
                        args: expr_ops,
                    },
                });
                Ok(insns)
            }
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
            Expression::Load { loc, ty, expr, .. } => {
                TypeChecker::check_load(ty, &expr.ty())?;

                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Load {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                    },
                });
                Ok(insns)
            }
            Expression::UnsignedModulo {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::UMod;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::SignedModulo {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Mod;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
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
            Expression::Multiply {
                loc,
                left,
                right,
                overflowing,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Mul {
                    overflowing: *overflowing,
                };
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::Not { loc, expr, .. } => {
                let operator = UnaryOperator::Not;
                self.unary_operation(dest, loc, &ast::Type::Bool, operator, expr, vartable)
            }
            Expression::NotEqual {
                loc, left, right, ..
            } => {
                let operator = BinaryOperator::Neq;
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            Expression::NumberLiteral { loc, value, .. } => Ok(vec![
                // assign the constant value to the destination
                Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::NumberLiteral {
                        loc: loc.clone(),
                        value: value.clone(),
                    },
                },
            ]),
            Expression::Poison => panic!("Poison expression shouldn't be here"),
            Expression::Power {
                loc,
                base,
                exp,
                overflowing,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Pow {
                    overflowing: *overflowing,
                };
                self.binary_operation(dest, loc, ty, operator, base, exp, vartable)
            }
            Expression::RationalNumberLiteral { .. } => {
                panic!("RationalNumberLiteral shouldn't be here")
            }
            Expression::ReturnData { .. } => todo!("Expression::ReturnData"),
            Expression::SignExt { loc, ty, expr, .. } => {
                // TODO: type checking
                // TypeChecker::check_sign_ext(&ty, &expr.ty())?;
                let (tmp, expr_insns) = self.as_operand_and_insns(expr, vartable)?; // TODO: type checking
                let sext = Expr::SignExt {
                    loc: loc.clone(),
                    operand: Box::new(tmp),
                    to_ty: self.from_ast_type(ty)?,
                };
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
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
            Expression::ShiftRight {
                loc,
                left,
                right,
                signed,
                ty,
                ..
            } => {
                let operator = if *signed {
                    BinaryOperator::Shr
                } else {
                    BinaryOperator::UShr
                };
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::StorageArrayLength { loc, ty, array, .. } => {
                TypeChecker::assert_ty_eq(&array.ty(), ty)?;

                let (array_op, array_insns) = self.as_operand_and_insns(array, vartable)?;
                let mut insns = vec![];
                insns.extend(array_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::StorageArrayLength {
                        loc: loc.clone(),
                        array: Box::new(array_op),
                    },
                });
                Ok(insns)
            }
            Expression::StringCompare {
                loc, left, right, ..
            } => {
                let mut insns = vec![];

                let (left_string_loc, left_insns) =
                    self.as_string_location_and_insns(left, vartable)?;
                let (right_string_loc, right_insns) =
                    self.as_string_location_and_insns(right, vartable)?;
                insns.extend(left_insns);
                insns.extend(right_insns);

                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::StringCompare {
                        loc: loc.clone(),
                        left: left_string_loc,
                        right: right_string_loc,
                    },
                });
                Ok(insns)
            }
            Expression::StringConcat {
                loc, left, right, ..
            } => {
                let mut insns = vec![];

                let (left_string_loc, left_insns) =
                    self.as_string_location_and_insns(left, vartable)?;
                let (right_string_loc, right_insns) =
                    self.as_string_location_and_insns(right, vartable)?;
                insns.extend(left_insns);
                insns.extend(right_insns);

                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::StringConcat {
                        loc: loc.clone(),
                        left: left_string_loc,
                        right: right_string_loc,
                    },
                });
                Ok(insns)
            }
            Expression::StructLiteral {
                loc, ty, values, ..
            } => {
                let struct_ty = self.from_ast_type(ty)?;

                // check values type
                // TypeChecker::check_struct_elem_tys(
                //     ty,
                //     &values.iter().map(|v| v.ty()).collect::<Vec<_>>(),
                // )?;

                let mut insns = vec![];

                let value_ops = values
                    .iter()
                    .map(|value| {
                        let (op, insn) = self.as_operand_and_insns(value, vartable)?;
                        insns.extend(insn);
                        Ok(op)
                    })
                    .collect::<Result<Vec<Operand>, String>>()?;

                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::StructLiteral {
                        loc: loc.clone(),
                        ty: struct_ty,
                        values: value_ops,
                    },
                });

                Ok(insns)
            }
            Expression::StructMember {
                loc, expr, member, ..
            } => {
                let (struct_op, struct_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(struct_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::StructMember {
                        loc: loc.clone(),
                        operand: Box::new(struct_op),
                        member: member.clone(),
                    },
                });
                Ok(insns)
            }
            Expression::Subscript {
                loc,
                array_ty,
                ty: elem_ty,
                expr,
                index,
                ..
            } => {
                TypeChecker::check_subscript(&array_ty, &elem_ty, &index.ty())?;
                let (array_op, array_insns) = self.as_operand_and_insns(expr, vartable)?;
                let (index_op, index_insns) = self.as_operand_and_insns(index, vartable)?;
                let mut insns = vec![];
                insns.extend(array_insns);
                insns.extend(index_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Subscript {
                        loc: loc.clone(),
                        arr: Box::new(array_op),
                        index: Box::new(index_op),
                    },
                });
                Ok(insns)
            }
            Expression::Subtract {
                loc,
                left,
                right,
                overflowing,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Sub {
                    overflowing: *overflowing,
                };
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            Expression::Trunc { loc, ty, expr, .. } => {
                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::Trunc {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                        to_ty: self.from_ast_type(ty)?,
                    },
                });
                Ok(insns)
            }
            Expression::Negate {
                loc,
                ty,
                expr,
                overflowing,
                ..
            } => {
                let operator = UnaryOperator::Neg {
                    overflowing: *overflowing,
                };
                self.unary_operation(dest, loc, ty, operator, expr, vartable)
            }
            Expression::Undefined { .. } => panic!("Undefined expression shouldn't be here"),
            Expression::Variable { loc, var_no, .. } => {
                let expr = Expr::Id {
                    loc: loc.clone(),
                    id: var_no.clone(),
                };
                let res = dest.get_id()?;
                Ok(vec![Insn::Set {
                    loc: Loc::Codegen,
                    res,
                    expr,
                }])
            }
            Expression::ZeroExt { loc, ty, expr, .. } => {
                let (from_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                let mut insns = vec![];
                insns.extend(expr_insns);
                insns.push(Insn::Set {
                    loc: loc.clone(),
                    res: dest.get_id()?,
                    expr: Expr::ZeroExt {
                        loc: loc.clone(),
                        operand: Box::new(from_op),
                        to_ty: self.from_ast_type(ty)?,
                    },
                });
                Ok(insns)
            }
            Expression::AdvancePointer {
                pointer,
                bytes_offset,
                ..
            } => {
                let (pointer_op, pointer_insns) = self.as_operand_and_insns(pointer, vartable)?;
                let (bytes_offset_op, bytes_offset_insns) =
                    self.as_operand_and_insns(bytes_offset, vartable)?;
                let mut insns = vec![];
                insns.extend(pointer_insns);
                insns.extend(bytes_offset_insns);
                insns.push(Insn::Set {
                    loc: Loc::Codegen,
                    res: dest.get_id()?,
                    expr: Expr::AdvancePointer {
                        pointer: Box::new(pointer_op),
                        bytes_offset: Box::new(bytes_offset_op),
                    },
                });
                Ok(insns)
            }
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
        let (left_op, left_insns) = self.as_operand_and_insns(left, vartable)?;
        let (right_op, right_insns) = self.as_operand_and_insns(right, vartable)?;
        let mut insns = vec![];
        insns.extend(left_insns);
        insns.extend(right_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id()?,
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
        TypeChecker::check_unary_op(ty, &expr.ty())?;
        let (expr_op, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id()?,
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
        let (size_op, left_insns) = self.as_operand_and_insns(size, vartable)?;
        let mut insns = vec![];
        insns.extend(left_insns);
        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id()?,
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
                let (op, insn) = self.as_operand_and_insns(value, vartable)?;
                insns.extend(insn);
                Ok(op)
            })
            .collect::<Result<Vec<Operand>, String>>()?;

        insns.push(Insn::Set {
            loc: loc.clone(),
            res: dest.get_id()?,
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
