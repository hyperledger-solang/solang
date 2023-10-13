// SPDX-License-Identifier: Apache-2.0
use crate::ssa_ir::expr::{BinaryOperator, UnaryOperator};
use crate::ssa_ir::ssa_type::Type;

pub fn check_assignment(lhs: &Type, rhs: &Type) -> Result<(), &'static str> {
    todo!("Implement type checking")
}

pub fn check_binary_op(op: &BinaryOperator, lhs: &Type, rhs: &Type) -> Result<(), &'static str> {
    todo!("Implement type checking")
}

pub fn check_unary_op(op: &UnaryOperator, operand: &Type) -> Result<(), &'static str> {
    todo!("Implement type checking")
}