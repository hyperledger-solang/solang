// SPDX-License-Identifier: Apache-2.0

use core::panic;

use solang_parser::pt::Loc;

use crate::codegen;
use crate::sema::ast;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::expressions::{BinaryOperator, Expression, Operand, UnaryOperator};
use crate::ssa_ir::instructions::Instruction;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub(crate) fn convert_expression(
        &self,
        dest: &Operand,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        match expr {
            codegen::Expression::Add {
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
            codegen::Expression::AllocDynamicBytes {
                loc,
                ty,
                size,
                initializer,
                ..
            } => self.alloc_dynamic_bytes(dest, loc, ty, size, initializer, vartable),
            codegen::Expression::ArrayLiteral {
                loc,
                ty,
                dimensions,
                values,
                ..
            } => self.array_literal(dest, loc, ty, dimensions, values, vartable),
            codegen::Expression::BitwiseAnd {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitAnd;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::BitwiseOr {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitOr;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::BitwiseXor {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::BitXor;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::BoolLiteral { loc, value, .. } => {
                self.bool_literal(dest, loc, value)
            }
            codegen::Expression::Builtin {
                loc, kind, args, ..
            } => self.builtin(dest, loc, kind, args, vartable),
            codegen::Expression::BytesCast { loc, expr, ty, .. } => {
                self.byte_cast(dest, loc, expr, ty, vartable)
            }
            codegen::Expression::BytesLiteral { loc, ty, value, .. } => {
                self.bytes_literal(dest, loc, ty, value)
            }
            codegen::Expression::Cast { loc, ty, expr, .. } => {
                self.cast(dest, loc, ty, expr, vartable)
            }
            codegen::Expression::BitwiseNot { loc, expr, .. } => {
                let operator = UnaryOperator::BitNot;
                self.unary_operation(dest, loc, operator, expr, vartable)
            }
            codegen::Expression::ConstArrayLiteral {
                loc,
                ty,
                dimensions,
                values,
                ..
            } => self.const_array_literal(dest, loc, ty, dimensions, values, vartable),
            codegen::Expression::UnsignedDivide {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::UDiv;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::SignedDivide {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Div;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::Equal {
                loc, left, right, ..
            } => {
                let operator = BinaryOperator::Eq;
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            codegen::Expression::FormatString { loc, args, .. } => {
                self.format_string(dest, loc, args, vartable)
            }
            codegen::Expression::FunctionArg {
                loc, ty, arg_no, ..
            } => self.function_arg(dest, loc, ty, arg_no, vartable),
            codegen::Expression::GetRef { loc, expr, .. } => {
                self.get_ref(dest, loc, expr, vartable)
            }
            codegen::Expression::InternalFunctionCfg { cfg_no, .. } => {
                self.internal_function_cfg(dest, cfg_no)
            }
            codegen::Expression::Keccak256 { loc, exprs, .. } => {
                self.keccak256(dest, loc, exprs, vartable)
            }
            codegen::Expression::Less {
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
            codegen::Expression::LessEqual {
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
            codegen::Expression::Load { loc, expr, .. } => self.load(dest, loc, expr, vartable),
            codegen::Expression::UnsignedModulo {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::UMod;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::SignedModulo {
                loc,
                left,
                right,
                ty,
                ..
            } => {
                let operator = BinaryOperator::Mod;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::More {
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
            codegen::Expression::MoreEqual {
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
            codegen::Expression::Multiply {
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
            codegen::Expression::Not { loc, expr, .. } => {
                let operator = UnaryOperator::Not;
                self.unary_operation(dest, loc, operator, expr, vartable)
            }
            codegen::Expression::NotEqual {
                loc, left, right, ..
            } => {
                let operator = BinaryOperator::Neq;
                self.binary_operation(dest, loc, &ast::Type::Bool, operator, left, right, vartable)
            }
            codegen::Expression::NumberLiteral { loc, value, .. } => {
                self.number_literal(dest, loc, value)
            }
            codegen::Expression::Poison => panic!("Poison expression shouldn't be here"),
            codegen::Expression::Power {
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
            codegen::Expression::RationalNumberLiteral { .. } => {
                panic!("RationalNumberLiteral shouldn't be here")
            }
            codegen::Expression::ReturnData { loc, .. } => self.return_data(dest, loc),
            codegen::Expression::SignExt { loc, ty, expr, .. } => {
                self.sign_ext(dest, loc, ty, expr, vartable)
            }
            codegen::Expression::ShiftLeft {
                loc,
                ty,
                left,
                right,
                ..
            } => {
                let operator = BinaryOperator::Shl;
                self.binary_operation(dest, loc, ty, operator, left, right, vartable)
            }
            codegen::Expression::ShiftRight {
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
            codegen::Expression::StorageArrayLength { loc, array, .. } => {
                self.storage_array_length(dest, loc, array, vartable)
            }
            codegen::Expression::StringCompare {
                loc, left, right, ..
            } => self.string_compare(dest, loc, left, right, vartable),
            codegen::Expression::StringConcat {
                loc, left, right, ..
            } => self.string_concat(dest, loc, left, right, vartable),
            codegen::Expression::StructLiteral {
                loc, ty, values, ..
            } => self.struct_literal(dest, loc, ty, values, vartable),
            codegen::Expression::StructMember {
                loc, expr, member, ..
            } => self.struct_member(dest, loc, expr, member, vartable),
            codegen::Expression::Subscript {
                loc, expr, index, ..
            } => self.subscript(dest, loc, expr, index, vartable),
            codegen::Expression::Subtract {
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
            codegen::Expression::Trunc { loc, ty, expr, .. } => {
                self.trunc(dest, loc, ty, expr, vartable)
            }
            codegen::Expression::Negate {
                loc,
                expr,
                overflowing,
                ..
            } => {
                let operator = UnaryOperator::Neg {
                    overflowing: *overflowing,
                };
                self.unary_operation(dest, loc, operator, expr, vartable)
            }
            codegen::Expression::Undefined { .. } => {
                panic!("Undefined expression shouldn't be here")
            }
            codegen::Expression::Variable { loc, var_no, .. } => self.variable(dest, loc, var_no),
            codegen::Expression::ZeroExt { loc, ty, expr, .. } => {
                self.zero_ext(dest, loc, ty, expr, vartable)
            }
            codegen::Expression::AdvancePointer {
                pointer,
                bytes_offset,
                ..
            } => self.advance_pointer(dest, pointer, bytes_offset, vartable),
        }
    }

    fn advance_pointer(
        &self,
        dest: &Operand,
        pointer: &codegen::Expression,
        bytes_offset: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (pointer_op, pointer_insns) = self.to_operand_and_insns(pointer, vartable)?;
        let (bytes_offset_op, bytes_offset_insns) =
            self.to_operand_and_insns(bytes_offset, vartable)?;
        let mut insns = vec![];
        insns.extend(pointer_insns);
        insns.extend(bytes_offset_insns);
        insns.push(Instruction::Set {
            loc: Loc::Codegen,
            res: dest.get_id()?,
            expr: Expression::AdvancePointer {
                pointer: Box::new(pointer_op),
                bytes_offset: Box::new(bytes_offset_op),
            },
        });
        Ok(insns)
    }

    fn zero_ext(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::ZeroExt {
                loc: *loc,
                operand: Box::new(from_op),
                to_ty: self.from_ast_type(ty)?,
            },
        });
        Ok(insns)
    }

    fn trunc(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Trunc {
                loc: *loc,
                operand: Box::new(from_op),
                to_ty: self.from_ast_type(ty)?,
            },
        });
        Ok(insns)
    }

    fn subscript(
        &self,
        dest: &Operand,
        loc: &Loc,
        expr: &codegen::Expression,
        index: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (array_op, array_insns) = self.to_operand_and_insns(expr, vartable)?;
        let (index_op, index_insns) = self.to_operand_and_insns(index, vartable)?;
        let mut insns = vec![];
        insns.extend(array_insns);
        insns.extend(index_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Subscript {
                loc: *loc,
                arr: Box::new(array_op),
                index: Box::new(index_op),
            },
        });
        Ok(insns)
    }

    fn struct_member(
        &self,
        dest: &Operand,
        loc: &Loc,
        expr: &codegen::Expression,
        member: &usize,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (struct_op, struct_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(struct_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::StructMember {
                loc: *loc,
                operand: Box::new(struct_op),
                member: *member,
            },
        });
        Ok(insns)
    }

    fn struct_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        values: &[codegen::Expression],
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];
        let value_ops = values
            .iter()
            .map(|value| {
                let (op, insn) = self.to_operand_and_insns(value, vartable)?;
                insns.extend(insn);
                Ok(op)
            })
            .collect::<Result<Vec<Operand>, String>>()?;

        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::StructLiteral {
                loc: *loc,
                ty: self.from_ast_type(ty)?,
                values: value_ops,
            },
        });

        Ok(insns)
    }

    fn string_concat(
        &self,
        dest: &Operand,
        loc: &Loc,
        left: &ast::StringLocation<codegen::Expression>,
        right: &ast::StringLocation<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];

        let (left_string_loc, left_insns) = self.to_string_location_and_insns(left, vartable)?;
        let (right_string_loc, right_insns) = self.to_string_location_and_insns(right, vartable)?;
        insns.extend(left_insns);
        insns.extend(right_insns);

        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::StringConcat {
                loc: *loc,
                left: left_string_loc,
                right: right_string_loc,
            },
        });
        Ok(insns)
    }

    fn string_compare(
        &self,
        dest: &Operand,
        loc: &Loc,
        left: &ast::StringLocation<codegen::Expression>,
        right: &ast::StringLocation<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];

        let (left_string_loc, left_insns) = self.to_string_location_and_insns(left, vartable)?;
        let (right_string_loc, right_insns) = self.to_string_location_and_insns(right, vartable)?;
        insns.extend(left_insns);
        insns.extend(right_insns);

        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::StringCompare {
                loc: *loc,
                left: left_string_loc,
                right: right_string_loc,
            },
        });
        Ok(insns)
    }

    fn storage_array_length(
        &self,
        dest: &Operand,
        loc: &Loc,
        array: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (array_op, array_insns) = self.to_operand_and_insns(array, vartable)?;
        let mut insns = vec![];
        insns.extend(array_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::StorageArrayLength {
                loc: *loc,
                array: Box::new(array_op),
            },
        });
        Ok(insns)
    }

    fn sign_ext(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (tmp, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let sext = Expression::SignExt {
            loc: *loc,
            operand: Box::new(tmp),
            to_ty: self.from_ast_type(ty)?,
        };
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: sext,
        });
        Ok(insns)
    }

    fn load(
        &self,
        dest: &Operand,
        loc: &Loc,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Load {
                loc: *loc,
                operand: Box::new(from_op),
            },
        });
        Ok(insns)
    }

    fn keccak256(
        &self,
        dest: &Operand,
        loc: &Loc,
        exprs: &Vec<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];
        let mut expr_ops = vec![];
        for expr in exprs {
            let (op, insn) = self.to_operand_and_insns(expr, vartable)?;
            insns.extend(insn);
            expr_ops.push(op);
        }
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Keccak256 {
                loc: *loc,
                args: expr_ops,
            },
        });
        Ok(insns)
    }

    fn get_ref(
        &self,
        dest: &Operand,
        loc: &Loc,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::GetRef {
                loc: *loc,
                operand: Box::new(from_op),
            },
        });
        Ok(insns)
    }

    fn function_arg(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        arg_no: &usize,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let arg_ty = self.from_ast_type(ty)?;
        let expr = Expression::FunctionArg {
            loc: *loc,
            ty: arg_ty,
            arg_no: *arg_no,
        };
        let res = dest.get_id()?;
        vartable.add_function_arg(*arg_no, res);
        Ok(vec![Instruction::Set {
            loc: *loc,
            res,
            expr,
        }])
    }

    fn format_string(
        &self,
        dest: &Operand,
        loc: &Loc,
        args: &Vec<(ast::FormatArg, codegen::Expression)>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];
        let mut arg_ops = vec![];
        for (format, arg) in args {
            let (op, insn) = self.to_operand_and_insns(arg, vartable)?;
            insns.extend(insn);
            arg_ops.push((*format, op));
        }
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::FormatString {
                loc: *loc,
                args: arg_ops,
            },
        });
        Ok(insns)
    }

    fn const_array_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        dimensions: &[u32],
        values: &[codegen::Expression],
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];

        let value_ops = values
            .iter()
            .map(|value| {
                let (op, insn) = self.to_operand_and_insns(value, vartable)?;
                insns.extend(insn);
                Ok(op)
            })
            .collect::<Result<Vec<Operand>, String>>()?;

        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::ConstArrayLiteral {
                loc: *loc,
                ty: self.from_ast_type(ty)?,
                dimensions: dimensions.to_owned(),
                values: value_ops,
            },
        });

        Ok(insns)
    }

    fn cast(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Cast {
                loc: *loc,
                operand: Box::new(from_op),
                to_ty: self.from_ast_type(ty)?,
            },
        });
        Ok(insns)
    }

    fn byte_cast(
        &self,
        dest: &Operand,
        loc: &Loc,
        expr: &codegen::Expression,
        ty: &ast::Type,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (from_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::BytesCast {
                loc: *loc,
                operand: Box::new(from_op),
                to_ty: self.from_ast_type(ty)?,
            },
        });
        Ok(insns)
    }

    fn builtin(
        &self,
        dest: &Operand,
        loc: &Loc,
        kind: &crate::codegen::Builtin,
        args: &Vec<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];
        let mut arg_ops = vec![];
        for arg in args {
            let (op, insn) = self.to_operand_and_insns(arg, vartable)?;
            insns.extend(insn);
            arg_ops.push(op);
        }
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::Builtin {
                loc: *loc,
                kind: *kind,
                args: arg_ops,
            },
        });
        Ok(insns)
    }

    fn binary_operation(
        &self,
        dest: &Operand,
        loc: &Loc,
        _: &ast::Type,
        operator: BinaryOperator,
        left: &codegen::Expression,
        right: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (left_op, left_insns) = self.to_operand_and_insns(left, vartable)?;
        let (right_op, right_insns) = self.to_operand_and_insns(right, vartable)?;
        let mut insns = vec![];
        insns.extend(left_insns);
        insns.extend(right_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::BinaryExpr {
                loc: *loc,
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
        operator: UnaryOperator,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (expr_op, expr_insns) = self.to_operand_and_insns(expr, vartable)?;
        let mut insns = vec![];
        insns.extend(expr_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::UnaryExpr {
                loc: *loc,
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
        size: &codegen::Expression,
        initializer: &Option<Vec<u8>>,
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let (size_op, left_insns) = self.to_operand_and_insns(size, vartable)?;
        let mut insns = vec![];
        insns.extend(left_insns);
        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::AllocDynamicBytes {
                loc: *loc,
                ty: self.from_ast_type(ty)?,
                size: Box::new(size_op),
                initializer: initializer.clone(),
            },
        });

        Ok(insns)
    }

    fn bytes_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        value: &[u8],
    ) -> Result<Vec<Instruction>, String> {
        let expr = Expression::BytesLiteral {
            loc: *loc,
            ty: self.from_ast_type(ty)?,
            value: value.to_owned(),
        };
        Ok(vec![Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr,
        }])
    }

    fn bool_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        value: &bool,
    ) -> Result<Vec<Instruction>, String> {
        let expr = Expression::BoolLiteral {
            loc: *loc,
            value: *value,
        };
        Ok(vec![Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr,
        }])
    }

    fn number_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        value: &num_bigint::BigInt,
    ) -> Result<Vec<Instruction>, String> {
        Ok(vec![
            // assign the constant value to the destination
            Instruction::Set {
                loc: *loc,
                res: dest.get_id()?,
                expr: Expression::NumberLiteral {
                    loc: *loc,
                    value: value.clone(),
                },
            },
        ])
    }

    fn array_literal(
        &self,
        dest: &Operand,
        loc: &Loc,
        ty: &ast::Type,
        dimensions: &[u32],
        values: &[codegen::Expression],
        vartable: &mut Vartable,
    ) -> Result<Vec<Instruction>, String> {
        let mut insns = vec![];

        let value_ops = values
            .iter()
            .map(|value| {
                let (op, insn) = self.to_operand_and_insns(value, vartable)?;
                insns.extend(insn);
                Ok(op)
            })
            .collect::<Result<Vec<Operand>, String>>()?;

        insns.push(Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::ArrayLiteral {
                loc: *loc,
                ty: self.from_ast_type(ty)?,
                dimensions: dimensions.to_owned(),
                values: value_ops,
            },
        });

        Ok(insns)
    }

    fn internal_function_cfg(
        &self,
        dest: &Operand,
        cfg_no: &usize,
    ) -> Result<Vec<Instruction>, String> {
        let expr = Expression::InternalFunctionCfg { cfg_no: *cfg_no };
        Ok(vec![Instruction::Set {
            loc: Loc::Codegen,
            res: dest.get_id()?,
            expr,
        }])
    }

    fn variable(
        &self,
        dest: &Operand,
        loc: &Loc,
        var_no: &usize,
    ) -> Result<Vec<Instruction>, String> {
        let expr = Expression::Id {
            loc: *loc,
            id: *var_no,
        };
        Ok(vec![Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr,
        }])
    }

    fn return_data(&self, dest: &Operand, loc: &Loc) -> Result<Vec<Instruction>, String> {
        Ok(vec![Instruction::Set {
            loc: *loc,
            res: dest.get_id()?,
            expr: Expression::ReturnData { loc: *loc },
        }])
    }
}
