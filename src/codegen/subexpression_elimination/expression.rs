// SPDX-License-Identifier: Apache-2.0

use crate::codegen::subexpression_elimination::{ConstantType, ExpressionType};
use crate::codegen::Expression;
use crate::sema::ast::StringLocation;

impl Expression {
    /// Rebuild a binary expression given the new left and right subexpressions
    #[must_use]
    pub fn rebuild_binary_expression(&self, left: &Expression, right: &Expression) -> Expression {
        match self {
            Expression::Add {
                loc,
                ty: expr_type,
                overflowing: check,
                ..
            } => Expression::Add {
                loc: *loc,
                ty: expr_type.clone(),
                overflowing: *check,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::Multiply {
                loc,
                ty: expr_type,
                overflowing: check,
                ..
            } => Expression::Multiply {
                loc: *loc,
                ty: expr_type.clone(),
                overflowing: *check,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::BitwiseOr {
                loc, ty: expr_type, ..
            } => Expression::BitwiseOr {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::BitwiseAnd {
                loc, ty: expr_type, ..
            } => Expression::BitwiseAnd {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::BitwiseXor {
                loc, ty: expr_type, ..
            } => Expression::BitwiseXor {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },
            Expression::Equal { loc, .. } => Expression::Equal {
                loc: *loc,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::NotEqual { loc, .. } => Expression::NotEqual {
                loc: *loc,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::Subtract {
                loc,
                ty: expr_type,
                overflowing: check,
                ..
            } => Expression::Subtract {
                loc: *loc,
                ty: expr_type.clone(),
                overflowing: *check,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },
            Expression::UnsignedDivide {
                loc, ty: expr_type, ..
            } => Expression::UnsignedDivide {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::SignedDivide {
                loc, ty: expr_type, ..
            } => Expression::SignedDivide {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::SignedModulo {
                loc, ty: expr_type, ..
            } => Expression::SignedModulo {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::UnsignedModulo {
                loc, ty: expr_type, ..
            } => Expression::UnsignedModulo {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::Power {
                loc,
                ty: expr_type,
                overflowing: check,
                ..
            } => Expression::Power {
                loc: *loc,
                ty: expr_type.clone(),
                overflowing: *check,
                base: Box::new(left.clone()),
                exp: Box::new(right.clone()),
            },

            Expression::ShiftLeft {
                loc, ty: expr_type, ..
            } => Expression::ShiftLeft {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },

            Expression::ShiftRight {
                loc,
                ty: expr_type,
                left: _,
                right: _,
                signed: check,
            } => Expression::ShiftRight {
                loc: *loc,
                ty: expr_type.clone(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
                signed: *check,
            },

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

            Expression::StringCompare {
                loc,
                left: left_exp,
                right: right_exp,
            } => {
                if !matches!(
                    (left_exp, right_exp),
                    (StringLocation::RunTime(_), StringLocation::RunTime(_))
                ) {
                    unreachable!("String compare operation does not contain runtime arguments");
                }

                Expression::StringCompare {
                    loc: *loc,
                    left: StringLocation::RunTime(Box::new(left.clone())),
                    right: StringLocation::RunTime(Box::new(right.clone())),
                }
            }

            _ => unreachable!("Cannot rebuild this expression"),
        }
    }

    /// Rebuild a unary expression give the new operand expression
    #[must_use]
    pub fn rebuild_unary_expression(&self, operand: &Expression) -> Expression {
        match self {
            Expression::ZeroExt {
                loc, ty: expr_type, ..
            } => Expression::ZeroExt {
                loc: *loc,
                ty: expr_type.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::SignExt {
                loc, ty: expr_type, ..
            } => Expression::SignExt {
                loc: *loc,
                ty: expr_type.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::Trunc {
                loc, ty: expr_type, ..
            } => Expression::Trunc {
                loc: *loc,
                ty: expr_type.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::Cast {
                loc, ty: expr_type, ..
            } => Expression::Cast {
                loc: *loc,
                ty: expr_type.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::BytesCast { loc, ty, from, .. } => Expression::BytesCast {
                loc: *loc,
                ty: ty.clone(),
                from: from.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::Not { loc, .. } => Expression::Not {
                loc: *loc,
                expr: Box::new(operand.clone()),
            },

            Expression::BitwiseNot {
                loc, ty: expr_type, ..
            } => Expression::BitwiseNot {
                loc: *loc,
                ty: expr_type.clone(),
                expr: Box::new(operand.clone()),
            },

            Expression::Negate {
                loc,
                ty: expr_type,
                overflowing,
                ..
            } => Expression::Negate {
                loc: *loc,
                ty: expr_type.clone(),
                overflowing: *overflowing,
                expr: Box::new(operand.clone()),
            },

            _ => unreachable!("Cannot rebuild this unary expression"),
        }
    }

    /// Retrieve the operands of a commutative expression
    pub fn get_commutative_operands(&self) -> Option<(&Expression, &Expression)> {
        match self {
            Expression::Add { left, right, .. }
            | Expression::Multiply { left, right, .. }
            | Expression::BitwiseOr { left, right, .. }
            | Expression::BitwiseAnd { left, right, .. }
            | Expression::BitwiseXor { left, right, .. }
            | Expression::Equal { left, right, .. }
            | Expression::NotEqual { left, right, .. } => Some((left, right)),

            _ => None,
        }
    }

    /// Retrieve the operands of a non-commutative expression
    pub fn get_non_commutative_operands(&self) -> Option<(&Expression, &Expression)> {
        match self {
            Expression::Subtract { left, right, .. }
            | Expression::UnsignedDivide { left, right, .. }
            | Expression::SignedDivide { left, right, .. }
            | Expression::SignedModulo { left, right, .. }
            | Expression::UnsignedModulo { left, right, .. }
            | Expression::Power {
                base: left,
                exp: right,
                ..
            }
            | Expression::ShiftLeft { left, right, .. }
            | Expression::ShiftRight { left, right, .. }
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
            Expression::ZeroExt { expr, .. }
            | Expression::SignExt { expr, .. }
            | Expression::Trunc { expr, .. }
            | Expression::Cast { expr, .. }
            | Expression::BytesCast { expr, .. }
            | Expression::Not { expr, .. }
            | Expression::BitwiseNot { expr, .. }
            | Expression::Negate { expr, .. } => Some(expr),

            _ => None,
        }
    }

    /// Get the expression type for a constant-like expression
    pub fn get_constant_expression_type(&self) -> ExpressionType {
        let cte_type = match self {
            Expression::BoolLiteral { value, .. } => ConstantType::Bool(*value),
            Expression::NumberLiteral { value, .. } => ConstantType::Number(value.clone()),
            Expression::BytesLiteral { value, .. } => ConstantType::Bytes(value.clone()),
            _ => unreachable!("Not a constant expression"),
        };

        ExpressionType::Literal(cte_type)
    }
}
