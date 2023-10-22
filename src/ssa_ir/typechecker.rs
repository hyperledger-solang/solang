// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, Type};

pub struct TypeChecker {}

impl TypeChecker {
    pub fn check_assignment(lhs: &Type, rhs: &Type) -> Result<(), String> {
        if lhs != rhs {
            return Err(format!("Type mismatch: lhs: {:?}, rhs: {:?}", lhs, rhs));
        }
        Ok(())
    }

    pub fn check_binary_op(rhs_left_ty: &Type, rhs_right_ty: &Type) -> Result<(), String> {
        // the two types has to be equal
        if rhs_left_ty != rhs_right_ty {
            return Err(format!(
                "Type mismatch: rhs_left_ty: {:?}, rhs_right_ty: {:?}",
                rhs_left_ty, rhs_right_ty
            ));
        }
        Ok(())
    }

    pub fn check_unary_op(res_ty: &Type, ty: &Type) -> Result<(), String> {
        if res_ty != ty {
            return Err(format!("Type mismatch: res_ty: {:?}, ty: {:?}", res_ty, ty));
        }
        Ok(())
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

    pub fn check_subscript(arr_ty: &Type, elem_ty: &Type, index_ty: &Type) -> Result<(), String> {
        match index_ty {
            Type::Uint(_) => {}
            Type::Int(_) => {}
            _ => {
                return Err(format!(
                    "Expected index type to be uint or int, got {:?}",
                    index_ty
                ))
            }
        }

        match (arr_ty, elem_ty) {
            (Type::Array(arr_elem_ty, _), Type::Ref(elem_ty)) => {
                if arr_elem_ty.as_ref() != elem_ty.as_ref() {
                    return Err(format!(
                        "Expecting array element type {:?} to be equal to {:?}",
                        arr_elem_ty, elem_ty
                    ));
                }
                Ok(())
            }
            _ => Err(format!(
                "Type mismatch: arr_ty: {:?}, elem_ty: {:?}",
                arr_ty, elem_ty
            )),
        }
    }
}
