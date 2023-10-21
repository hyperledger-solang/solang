// SPDX-License-Identifier: Apache-2.0

use crate::{
    sema::ast::{ArrayLength, Type},
    ssa_ir::expr::UnaryOperator,
};

pub struct TypeChecker {}

impl TypeChecker {
    pub fn check_assignment(lhs: &Type, rhs: &Type) -> Result<(), String> {
        if lhs != rhs {
            return Err(format!("Type mismatch: lhs: {:?}, rhs: {:?}", lhs, rhs));
        }
        Ok(())
    }

    pub fn check_binary_op(
        lhs_ty: &Type,
        rhs_left_ty: &Type,
        rhs_right_ty: &Type,
    ) -> Result<(), String> {
        // the three types has to be equal
        // if lhs_ty != rhs_left_ty || lhs_ty != rhs_right_ty {
        //     return Err(format!(
        //         "Type mismatch: lhs_ty: {:?}, rhs_left_ty: {:?}, rhs_right_ty: {:?}",
        //         lhs_ty, rhs_left_ty, rhs_right_ty
        //     ));
        // }
        Ok(())
    }

    pub fn check_unary_op(op: &UnaryOperator, ty: &Type) -> Result<(), String> {
        todo!("Implement type checking")
    }

    pub fn check_alloc_dynamic_bytes(ty: &Type, size_ty: &Type) -> Result<(), String> {
        match (ty, size_ty) {
            (Type::Array(_, len), Type::Uint(_)) => {
                if len.len() < 1 || len.get(0).unwrap() != &ArrayLength::Dynamic {
                    return Err(format!("Invalid array length: {:?}", len));
                }
                Ok(())
            }
            _ => Err(format!(
                "Type mismatch: ty: {:?}, size_ty: {:?}",
                ty, size_ty
            )),
        }
    }
}
