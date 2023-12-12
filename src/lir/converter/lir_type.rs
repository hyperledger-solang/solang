// SPDX-License-Identifier: Apache-2.0
use super::Converter;
use num_bigint::BigInt;

use crate::lir::lir_type::{LIRType, StructType, Type};
use crate::sema::ast::{self, ArrayLength};

impl Converter<'_> {
    /// lower the `ast::Type` into a `lir::lir_type::LIRType`.
    pub fn lower_ast_type(&self, ty: &ast::Type) -> LIRType {
        LIRType {
            ast_type: ty.clone(),
            lir_type: self.lower_ast_type_by_depth(ty, 0),
        }
    }

    fn lower_ast_type_by_depth(&self, ty: &ast::Type, depth: u8) -> Type {
        match ty {
            ast::Type::Bool => Type::Bool,
            ast::Type::Int(width) => Type::Int(*width),
            ast::Type::Uint(width) => Type::Uint(*width),
            ast::Type::Value => Type::Uint(self.value_length() as u16 * 8),
            ast::Type::Address(_) | ast::Type::Contract(_) => Type::Array(
                Box::new(Type::Uint(8)),
                vec![ArrayLength::Fixed(BigInt::from(self.address_length()))],
            ),
            ast::Type::Bytes(width) => Type::Bytes(*width),
            // String is equivalent to dynamic bytes
            ast::Type::String | ast::Type::DynamicBytes => self.wrap_ptr_by_depth(
                Type::Struct(StructType::Vector(Box::new(Type::Uint(8)))),
                depth,
            ),
            ast::Type::Array(ty, len) => {
                let ty = self.lower_ast_type_by_depth(ty.as_ref(), depth + 1);
                let len = len
                    .iter()
                    .map(|len| match len {
                        ast::ArrayLength::Fixed(len) => ArrayLength::Fixed(len.clone()),
                        ast::ArrayLength::Dynamic => ArrayLength::Dynamic,
                        ast::ArrayLength::AnyFixed => unreachable!(),
                    })
                    .collect();
                self.wrap_ptr_by_depth(Type::Array(Box::new(ty), len), depth)
            }
            ast::Type::Enum(enum_no) => self.lower_enum_type(*enum_no),
            ast::Type::Struct(struct_ty) => {
                self.wrap_ptr_by_depth(Type::Struct(StructType::from(struct_ty)), depth)
            }
            ast::Type::Mapping(mapping) => {
                let key = self.lower_ast_type_by_depth(&mapping.key, depth + 1);
                let value = self.lower_ast_type_by_depth(&mapping.value, depth + 1);
                Type::Mapping {
                    key_ty: Box::new(key),
                    value_ty: Box::new(value),
                }
            }
            ast::Type::Ref(rty) => {
                let ty = self.lower_ast_type_by_depth(rty.as_ref(), depth + 1);
                Type::Ptr(Box::new(ty))
            }
            ast::Type::StorageRef(immutable, ty) => {
                let ty = self.lower_ast_type_by_depth(ty.as_ref(), depth + 1);
                Type::StoragePtr(*immutable, Box::new(ty))
            }
            ast::Type::BufferPointer => self.wrap_ptr_by_depth(Type::Uint(8), depth),
            ast::Type::ExternalFunction { .. } => {
                self.wrap_ptr_by_depth(Type::Struct(StructType::ExternalFunction), depth)
            }
            ast::Type::InternalFunction {
                params, returns, ..
            } => {
                let params = params
                    .iter()
                    .map(|param| self.lower_ast_type_by_depth(param, depth + 1))
                    .collect::<Vec<_>>();
                let returns = returns
                    .iter()
                    .map(|ret| self.lower_ast_type_by_depth(ret, depth + 1))
                    .collect::<Vec<_>>();
                self.wrap_ptr_by_depth(Type::Function { params, returns }, depth)
            }
            ast::Type::UserType(_) => self.lower_user_type(ty),
            ast::Type::Slice(ty) => {
                let ty = self.lower_ast_type_by_depth(ty.as_ref(), depth + 1);
                self.wrap_ptr_by_depth(Type::Slice(Box::new(ty)), depth)
            }
            ast::Type::FunctionSelector => Type::Uint(self.fn_selector_length() as u16 * 8),
            ast::Type::Rational => unreachable!(),
            ast::Type::Void => unreachable!(),
            ast::Type::Unreachable => unreachable!(),
            ast::Type::Unresolved => unreachable!(),
        }
    }

    /// This function is used when only the first level of the type should be wrapped by a pointer.
    fn wrap_ptr_by_depth(&self, ty: Type, depth: u8) -> Type {
        if depth == 0 {
            Type::Ptr(Box::new(ty))
        } else {
            ty
        }
    }

    /// retrieve the enum type by enum_no and lower it into a lir::lir_type::Type.
    fn lower_enum_type(&self, enum_no: usize) -> Type {
        let ty = &self.ns.enums[enum_no].ty;
        self.lower_ast_type_by_depth(ty, 0)
    }

    fn lower_user_type(&self, user_ty: &ast::Type) -> Type {
        // clone happens here because function unwrap_user_type takes ownership
        let real_ty = user_ty.clone().unwrap_user_type(self.ns);
        self.lower_ast_type_by_depth(&real_ty, 0)
    }
}
