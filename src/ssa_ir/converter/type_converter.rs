// SPDX-License-Identifier: Apache-2.0
use super::Converter;
use num_bigint::BigInt;

use crate::sema::ast::{self, ArrayLength};
use crate::ssa_ir::ssa_type::{StructType, Type};

impl Converter<'_> {
    pub fn from_ast_type(&self, ty: &ast::Type) -> Result<Type, String> {
        match ty {
            // ast::Type::Rational => {},
            ast::Type::Bool => Ok(Type::Bool),
            ast::Type::Int(width) => Ok(Type::Int(*width)),
            ast::Type::Uint(width) => Ok(Type::Uint(*width)),
            ast::Type::Value => Ok(Type::Uint(self.value_length() as u16 * 8)),
            // DynamicBytes is a Ptr of an array of Bytes with dynamic length
            // an address is an array of byte
            ast::Type::Address(_) | ast::Type::Contract(_) => Ok(Type::Array(
                Box::new(Type::Uint(8)),
                vec![ArrayLength::Fixed(BigInt::from(self.address_length()))],
            )),
            // Bytes is a Ptr of an array of Bytes with fixed length
            //
            // endians is different: in llvm level, bytes - big-endian, int is little-endian
            // so they cannot be converted here without switching the endianess
            ast::Type::Bytes(width) => Ok(Type::Bytes(*width)),
            // String is equivalent to dynamic bytes
            ast::Type::String | ast::Type::DynamicBytes => Ok(Type::Ptr(Box::new(Type::Struct(
                StructType::Vector(Box::new(Type::Uint(8))),
            )))),
            ast::Type::Array(ty, len) => {
                let ty = self.from_ast_type(ty.as_ref())?;
                let len = len
                    .iter()
                    .map(|len| match len {
                        ast::ArrayLength::Fixed(len) => ArrayLength::Fixed(len.clone()),
                        ast::ArrayLength::Dynamic => ArrayLength::Dynamic,
                        ast::ArrayLength::AnyFixed => unreachable!(),
                    })
                    .collect();
                Ok(Type::Ptr(Box::new(Type::Array(Box::new(ty), len))))
            }
            ast::Type::Enum(enum_no) => self.get_enum_type(enum_no.clone()),
            ast::Type::Struct(struct_ty) => Ok(Type::Ptr(Box::new(Type::Struct(
                StructType::from(struct_ty),
            )))),
            ast::Type::Mapping(mapping) => {
                // let value = self.from_ast_type(&mapping.value)?;
                // Ok(Type::StoragePtr(false, Box::new(value)))
                // Instead of converting mapping to storage pointer, we convert it to a struct
                // to prevent key type information loss
                let key = self.from_ast_type(&mapping.key)?;
                let value = self.from_ast_type(&mapping.value)?;
                Ok(Type::Mapping {
                    key_ty: Box::new(key),
                    value_ty: Box::new(value),
                })
            }
            ast::Type::Ref(rty) => {
                let ty = self.from_ast_type(rty.as_ref())?;
                Ok(Type::Ptr(Box::new(ty)))
            }
            ast::Type::StorageRef(immutable, ty) => {
                let ty = self.from_ast_type(ty.as_ref())?;
                Ok(Type::StoragePtr(*immutable, Box::new(ty)))
            }
            ast::Type::BufferPointer => Ok(Type::Ptr(Box::new(Type::Uint(8)))),
            ast::Type::ExternalFunction { .. } => Ok(Type::Ptr(Box::new(Type::Struct(
                StructType::ExternalFunction,
            )))),
            ast::Type::InternalFunction {
                params, returns, ..
            } => {
                let params = params
                    .iter()
                    .map(|param| self.from_ast_type(param))
                    .collect::<Result<Vec<_>, _>>()?;
                let returns = returns
                    .iter()
                    .map(|ret| self.from_ast_type(ret))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Type::Ptr(Box::new(Type::Function { params, returns })))
            }
            ast::Type::UserType(_) => self.unwrap_user_type(ty),
            // ast::Type::Void => {}
            // ast::Type::Unreachable => {}
            ast::Type::Slice(ty) => {
                let ty = self.from_ast_type(ty.as_ref())?;
                Ok(Type::Ptr(Box::new(Type::Slice(Box::new(ty)))))
            }
            // ast::Type::Unresolved => {}
            ast::Type::FunctionSelector => Ok(Type::Uint(self.fn_selector_length() as u16 * 8)),
            _ => unreachable!(),
        }
    }
}
