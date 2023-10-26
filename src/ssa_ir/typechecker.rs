// SPDX-License-Identifier: Apache-2.0

// use crate::sema::ast::Type;

// pub struct TypeChecker;

// impl TypeChecker {
// pub fn assert_ty_eq(t1: &Type, t2: &Type) -> Result<(), String> {
//     if t1 != t2 {
//         return Err(format!("Type mismatch, expected {:?}, got {:?}", t1, t2));
//     }
//     Ok(())
// }

// pub fn check_assignment(lhs: &Type, rhs: &Type) -> Result<(), String> {
//     match lhs {
//         Type::StorageRef(_, ty) => TypeChecker::assert_ty_eq(ty, rhs),
//         _ => TypeChecker::assert_ty_eq(lhs, rhs),
//     }
// }

// pub fn check_binary_op(rhs_left_ty: &Type, rhs_right_ty: &Type) -> Result<(), String> {
//     // the two types has to be equal
//     println!(
//         "[check_binary_op] rhs_left_ty: {:?}, rhs_right_ty: {:?}",
//         rhs_left_ty, rhs_right_ty
//     );
//     TypeChecker::assert_ty_eq(rhs_left_ty, rhs_right_ty)
// }

// pub fn check_unary_op(res_ty: &Type, ty: &Type) -> Result<(), String> {
//     println!("[check_unary_op] res_ty: {:?}, ty: {:?}", res_ty, ty);
//     TypeChecker::assert_ty_eq(res_ty, ty)
// }

// pub fn check_alloc_dynamic_bytes(ty: &Type, size_ty: &Type) -> Result<(), String> {
//     println!(
//         "[check_alloc_dynamic_bytes] ty: {:?}, size_ty: {:?}",
//         ty, size_ty
//     );
//     Ok(())
// }

// pub fn check_subscript(arr_ty: &Type, elem_ty: &Type, index_ty: &Type) -> Result<(), String> {
//     println!(
//         "[check_subscript] arr_ty: {:?}, elem_ty: {:?}, index_ty: {:?}",
//         arr_ty, elem_ty, index_ty
//     );
//     match index_ty {
//         Type::Uint(_) => {}
//         Type::Int(_) => {}
//         _ => {
//             return Err(format!(
//                 "Expected index type to be uint or int, got {:?}",
//                 index_ty
//             ))
//         }
//     }
//     Ok(())
// }

// pub fn check_array_elem_tys(ty: &Type, elem_tys: &Vec<Type>) -> Result<(), String> {
//     println!(
//         "[check_array_elem_tys] ty: {:?}, elem_tys: {:?}",
//         ty, elem_tys
//     );
//     match ty {
//         Type::Array(ty, _) => {
//             elem_tys.iter().for_each(|elem_ty| {
//                 TypeChecker::assert_ty_eq(ty, elem_ty).unwrap();
//             });
//             Ok(())
//         }
//         _ => Err(format!("Expected array type, got {:?}", ty)),
//     }
// }

// pub fn check_load(ty: &Type, dest_ty: &Type) -> Result<(), String> {
//     println!("[check_load] ty: {:?}, dest_ty: {:?}", ty, dest_ty);
//     match dest_ty {
//         Type::Ref(ty) => TypeChecker::assert_ty_eq(ty, ty),
//         _ => Err(format!("Expected pointer type, got {:?}", dest_ty)),
//     }
// }
// }
