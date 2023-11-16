// SPDX-License-Identifier: Apache-2.0
use super::Converter;
use num_bigint::BigInt;

use crate::lir::lir_type::{StructType, Type};
use crate::sema::ast::{self, ArrayLength};

impl Converter<'_> {
    pub fn lowering_ast_type(&self, ty: &ast::Type) -> Type {
        self.lowering_ast_type_by_depth(ty, 0)
    }

    fn lowering_ast_type_by_depth(&self, ty: &ast::Type, depth: u8) -> Type {
        match ty {
            ast::Type::Bool => Type::Bool,
            ast::Type::Int(width) => Type::Int(*width),
            ast::Type::Uint(width) => Type::Uint(*width),
            ast::Type::Value => Type::Uint(self.value_length() as u16 * 8),
            // DynamicBytes is a Ptr of an array of Bytes with dynamic length
            // an address is an array of byte
            ast::Type::Address(_) | ast::Type::Contract(_) => Type::Array(
                Box::new(Type::Uint(8)),
                vec![ArrayLength::Fixed(BigInt::from(self.address_length()))],
            ),
            // Bytes is a Ptr of an array of Bytes with fixed length
            //
            // endians is different: in llvm level, bytes - big-endian, int is little-endian
            // so they cannot be converted here without switching the endianess
            ast::Type::Bytes(width) => Type::Bytes(*width),
            // String is equivalent to dynamic bytes
            ast::Type::String | ast::Type::DynamicBytes => self.wrap_ptr_by_depth(
                Type::Struct(StructType::Vector(Box::new(Type::Uint(8)))),
                depth,
            ),
            ast::Type::Array(ty, len) => {
                let ty = self.lowering_ast_type_by_depth(ty.as_ref(), depth + 1);
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
            ast::Type::Enum(enum_no) => self.convert_enum_type(*enum_no),
            ast::Type::Struct(struct_ty) => {
                self.wrap_ptr_by_depth(Type::Struct(StructType::from(struct_ty)), depth)
            }
            ast::Type::Mapping(mapping) => {
                let key = self.lowering_ast_type_by_depth(&mapping.key, depth + 1);
                let value = self.lowering_ast_type_by_depth(&mapping.value, depth + 1);
                Type::Mapping {
                    key_ty: Box::new(key),
                    value_ty: Box::new(value),
                }
            }
            ast::Type::Ref(rty) => {
                let ty = self.lowering_ast_type_by_depth(rty.as_ref(), depth + 1);
                Type::Ptr(Box::new(ty))
            }
            ast::Type::StorageRef(immutable, ty) => {
                let ty = self.lowering_ast_type_by_depth(ty.as_ref(), depth + 1);
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
                    .map(|param| self.lowering_ast_type_by_depth(param, depth + 1))
                    .collect::<Vec<_>>();
                let returns = returns
                    .iter()
                    .map(|ret| self.lowering_ast_type_by_depth(ret, depth + 1))
                    .collect::<Vec<_>>();
                self.wrap_ptr_by_depth(Type::Function { params, returns }, depth)
            }
            ast::Type::UserType(_) => self.convert_user_type(ty),
            ast::Type::Slice(ty) => {
                let ty = self.lowering_ast_type_by_depth(ty.as_ref(), depth + 1);
                self.wrap_ptr_by_depth(Type::Slice(Box::new(ty)), depth)
            }
            ast::Type::FunctionSelector => Type::Uint(self.fn_selector_length() as u16 * 8),
            _ => unreachable!(),
        }
    }

    fn wrap_ptr_by_depth(&self, ty: Type, depth: u8) -> Type {
        if depth == 0 {
            Type::Ptr(Box::new(ty))
        } else {
            ty
        }
    }
}
