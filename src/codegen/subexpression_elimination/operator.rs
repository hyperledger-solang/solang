// SPDX-License-Identifier: Apache-2.0

use crate::codegen::Expression;
use crate::sema::ast::Type;

/// This enum defines operator types for the graph
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Operator {
    Add,
    OverflowingAdd,
    Subtract,
    OverflowingSubtract,
    Multiply,
    OverflowingMultiply,
    SignedDivide,
    UnsignedDivide,
    Modulo,
    SignedModulo,
    UnsignedModulo,
    Power,
    OverflowingPower,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    ShiftLeft,
    SignedShiftRight,
    UnsignedShiftRight,
    SignedMore,
    UnsignedMore,
    SignedLess,
    UnsignedLess,
    SignedMoreEqual,
    UnsignedMoreEqual,
    SignedLessEqual,
    UnsignedLessEqual,
    Equal,
    NotEqual,
    StringConcat,
    StringCompare,
    AdvancePointer,
    //Unary operations
    Not,
    ZeroExt(Type),
    SignExt(Type),
    Trunc(Type),
    Cast(Type),
    BytesCast,
    Negate,
    OverflowingNegate,
    BitwiseNot,
}

impl Expression {
    /// Get the respective Operator from an Expression
    pub fn get_ave_operator(&self) -> Operator {
        match self {
            Expression::Add { overflowing, .. } => {
                if *overflowing {
                    Operator::OverflowingAdd
                } else {
                    Operator::Add
                }
            }
            Expression::Subtract { overflowing, .. } => {
                if *overflowing {
                    Operator::OverflowingSubtract
                } else {
                    Operator::Subtract
                }
            }
            Expression::Multiply { overflowing, .. } => {
                if *overflowing {
                    Operator::OverflowingMultiply
                } else {
                    Operator::Multiply
                }
            }
            Expression::SignedDivide { .. } => Operator::SignedDivide,
            Expression::UnsignedDivide { .. } => Operator::UnsignedDivide,
            Expression::SignedModulo { .. } => Operator::SignedModulo,
            Expression::UnsignedModulo { .. } => Operator::UnsignedModulo,
            Expression::Power { overflowing, .. } => {
                if *overflowing {
                    Operator::OverflowingPower
                } else {
                    Operator::Power
                }
            }
            Expression::BitwiseOr { .. } => Operator::BitwiseOr,
            Expression::BitwiseAnd { .. } => Operator::BitwiseAnd,
            Expression::BitwiseXor { .. } => Operator::BitwiseXor,
            Expression::ShiftLeft { .. } => Operator::ShiftLeft,
            Expression::ShiftRight { signed: true, .. } => Operator::SignedShiftRight,
            Expression::ShiftRight { signed: false, .. } => Operator::UnsignedShiftRight,
            Expression::Not { .. } => Operator::Not,
            Expression::ZeroExt { ty, .. } => Operator::ZeroExt(ty.clone()),
            Expression::SignExt { ty, .. } => Operator::SignExt(ty.clone()),
            Expression::Trunc { ty, .. } => Operator::Trunc(ty.clone()),
            Expression::Cast { ty, .. } => Operator::Cast(ty.clone()),
            Expression::BytesCast { .. } => Operator::BytesCast,
            Expression::Negate { overflowing, .. } => {
                if *overflowing {
                    Operator::OverflowingNegate
                } else {
                    Operator::Negate
                }
            }
            Expression::More { signed: true, .. } => Operator::SignedMore,
            Expression::More { signed: false, .. } => Operator::UnsignedMore,
            Expression::Less { signed: true, .. } => Operator::SignedLess,
            Expression::Less { signed: false, .. } => Operator::UnsignedLess,
            Expression::MoreEqual { signed: true, .. } => Operator::SignedMoreEqual,
            Expression::MoreEqual { signed: false, .. } => Operator::UnsignedMoreEqual,
            Expression::LessEqual { signed: true, .. } => Operator::SignedLessEqual,
            Expression::LessEqual { signed: false, .. } => Operator::UnsignedLessEqual,
            Expression::Equal { .. } => Operator::Equal,
            Expression::NotEqual { .. } => Operator::NotEqual,
            Expression::BitwiseNot { .. } => Operator::BitwiseNot,
            Expression::StringCompare { .. } => Operator::StringCompare,
            Expression::AdvancePointer { .. } => Operator::AdvancePointer,
            _ => {
                unreachable!("Expression does not represent an operator.")
            }
        }
    }
}
