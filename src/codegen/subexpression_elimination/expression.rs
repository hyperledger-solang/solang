// SPDX-License-Identifier: Apache-2.0

use crate::codegen::subexpression_elimination::{ConstantType, ExpressionType};
use crate::codegen::Expression;
use crate::sema::ast::StringLocation;

impl Expression {
    /// Rebuild a binary expression given the new left and right subexpressions
    #[must_use]
    pub fn rebuild_binary_expression(&self, left: &Expression, right: &Expression) -> Expression {
        match self {
            Expression::Add(loc, expr_type, check, ..) => Expression::Add(
                *loc,
                expr_type.clone(),
                *check,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::Multiply(loc, expr_type, check, ..) => Expression::Multiply(
                *loc,
                expr_type.clone(),
                *check,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::BitwiseOr(loc, expr_type, ..) => Expression::BitwiseOr(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::BitwiseAnd(loc, expr_type, ..) => Expression::BitwiseAnd(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::BitwiseXor(loc, expr_type, ..) => Expression::BitwiseXor(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            Expression::Equal(loc, ..) => {
                Expression::Equal(*loc, Box::new(left.clone()), Box::new(right.clone()))
            }

            Expression::NotEqual(loc, ..) => {
                Expression::NotEqual(*loc, Box::new(left.clone()), Box::new(right.clone()))
            }

            Expression::Subtract(loc, expr_type, check, ..) => Expression::Subtract(
                *loc,
                expr_type.clone(),
                *check,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            Expression::UnsignedDivide(loc, expr_type, ..) => Expression::UnsignedDivide(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::SignedDivide(loc, expr_type, ..) => Expression::SignedDivide(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::SignedModulo(loc, expr_type, ..) => Expression::SignedModulo(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::UnsignedModulo(loc, expr_type, ..) => Expression::UnsignedModulo(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::Power(loc, expr_type, check, ..) => Expression::Power(
                *loc,
                expr_type.clone(),
                *check,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::ShiftLeft(loc, expr_type, ..) => Expression::ShiftLeft(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),

            Expression::ShiftRight(loc, expr_type, _, _, check) => Expression::ShiftRight(
                *loc,
                expr_type.clone(),
                Box::new(left.clone()),
                Box::new(right.clone()),
                *check,
            ),

            Expression::More { loc, signed, .. } => Expression::More {
                loc: *loc,
                signed: *signed,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::Less { loc, signed, .. } => Expression::Less {
                loc: *loc,
                signed: *signed,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::MoreEqual { loc, signed, .. } => Expression::MoreEqual {
                loc: *loc,
                signed: *signed,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::LessEqual { loc, signed, .. } => Expression::LessEqual {
                loc: *loc,
                signed: *signed,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::AdvancePointer { .. } => Expression::AdvancePointer {
                pointer: Box::new(left.clone()),
                bytes_offset: Box::new(right.clone()),
            },

            Expression::StringCompare(loc, left_exp, right_exp) => {
                if !matches!(
                    (left_exp, right_exp),
                    (StringLocation::RunTime(_), StringLocation::RunTime(_))
                ) {
                    unreachable!("String compare operation does not contain runtime arguments");
                }

                Expression::StringCompare(
                    *loc,
                    StringLocation::RunTime(Box::new(left.clone())),
                    StringLocation::RunTime(Box::new(right.clone())),
                )
            }

            Expression::StringConcat(loc, expr_type, left_exp, right_exp) => {
                if !matches!(
                    (left_exp, right_exp),
                    (StringLocation::RunTime(_), StringLocation::RunTime(_))
                ) {
                    unreachable!("String concat operation does not contain runtime argumetns")
                }

                Expression::StringConcat(
                    *loc,
                    expr_type.clone(),
                    StringLocation::RunTime(Box::new(left.clone())),
                    StringLocation::RunTime(Box::new(right.clone())),
                )
            }

            _ => unreachable!("Cannot rebuild this expression"),
        }
    }

    /// Rebuild a unary expression give the new operand expression
    #[must_use]
    pub fn rebuild_unary_expression(&self, operand: &Expression) -> Expression {
        match self {
            Expression::ZeroExt(loc, expr_type, ..) => {
                Expression::ZeroExt(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            Expression::SignExt(loc, expr_type, ..) => {
                Expression::SignExt(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            Expression::Trunc(loc, expr_type, ..) => {
                Expression::Trunc(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            Expression::Cast(loc, expr_type, ..) => {
                Expression::Cast(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            Expression::BytesCast(loc, type_1, type_2, ..) => Expression::BytesCast(
                *loc,
                type_1.clone(),
                type_2.clone(),
                Box::new(operand.clone()),
            ),

            Expression::Not(loc, ..) => Expression::Not(*loc, Box::new(operand.clone())),

            Expression::Complement(loc, expr_type, ..) => {
                Expression::Complement(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            Expression::Negate(loc, expr_type, ..) => {
                Expression::Negate(*loc, expr_type.clone(), Box::new(operand.clone()))
            }

            _ => unreachable!("Cannot rebuild this unary expression"),
        }
    }

    /// Retrieve the operands of a commutative expression
    pub fn get_commutative_operands(&self) -> Option<(&Expression, &Expression)> {
        match self {
            Expression::Add(_, _, _, left, right)
            | Expression::Multiply(_, _, _, left, right)
            | Expression::BitwiseOr(_, _, left, right)
            | Expression::BitwiseAnd(_, _, left, right)
            | Expression::BitwiseXor(_, _, left, right)
            | Expression::Equal(_, left, right)
            | Expression::NotEqual(_, left, right) => Some((left, right)),

            _ => None,
        }
    }

    /// Retrieve the operands of a non-commutative expression
    pub fn get_non_commutative_operands(&self) -> Option<(&Expression, &Expression)> {
        match self {
            Expression::Subtract(_, _, _, left, right)
            | Expression::UnsignedDivide(_, _, left, right)
            | Expression::SignedDivide(_, _, left, right)
            | Expression::SignedModulo(_, _, left, right)
            | Expression::UnsignedModulo(_, _, left, right)
            | Expression::Power(_, _, _, left, right)
            | Expression::ShiftLeft(_, _, left, right)
            | Expression::ShiftRight(_, _, left, right, _)
            | Expression::More { left, right, .. }
            | Expression::Less { left, right, .. }
            | Expression::MoreEqual { left, right, .. }
            | Expression::AdvancePointer {
                pointer: left,
                bytes_offset: right,
            }
            | Expression::LessEqual { left, right, .. } => Some((left, right)),

            _ => None,
        }
    }

    /// Retrieve the operands of a unary expression
    pub fn get_unary_operand(&self) -> Option<&Expression> {
        match self {
            Expression::ZeroExt(_, _, operand)
            | Expression::SignExt(_, _, operand)
            | Expression::Trunc(_, _, operand)
            | Expression::Cast(_, _, operand)
            | Expression::BytesCast(_, _, _, operand)
            | Expression::Not(_, operand)
            | Expression::Complement(_, _, operand)
            | Expression::Negate(_, _, operand) => Some(operand),

            _ => None,
        }
    }

    /// Get the expression type for a constant-like expression
    pub fn get_constant_expression_type(&self) -> ExpressionType {
        let cte_type = match self {
            Expression::BoolLiteral(_, value) => ConstantType::Bool(*value),
            Expression::NumberLiteral(_, _, value) => ConstantType::Number(value.clone()),
            Expression::BytesLiteral(_, _, value) => ConstantType::Bytes(value.clone()),
            _ => unreachable!("Not a constant expression"),
        };

        ExpressionType::Literal(cte_type)
    }
}
