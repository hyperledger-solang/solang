// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use crate::lir::expressions::Operand;
use crate::sema::ast;
use crate::sema::ast::ArrayLength;

/// A struct type definition that is similar to the one in ast.rs,
/// extended with a Vector type, as we need a lower level representation of
/// String and DynamicBytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructType {
    UserDefined(usize),
    SolAccountInfo,
    SolAccountMeta,
    SolParameters,
    ExternalFunction,
    /// Vector is used here to represent String and DynamicBytes
    Vector(Box<Type>),
}

/// A struct that contains the AST type and the LIR type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LIRType {
    pub ast_type: ast::Type,
    pub lir_type: Type,
}

/// Types for LIR. Some types present in the AST are not present here, as they
/// are lowered to other types. See the `lower_ast_type` function in the `lir::converter::Converter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    Ptr(Box<Type>),
    StoragePtr(bool, Box<Type>),
    Function {
        params: Vec<Type>,
        returns: Vec<Type>,
    },
    Mapping {
        key_ty: Box<Type>,
        value_ty: Box<Type>,
    },
    Array(Box<Type>, Vec<ArrayLength>),
    Struct(StructType),
    Slice(Box<Type>),
}

#[derive(Clone, Debug)]
pub enum InternalCallTy {
    Static { cfg_no: usize },
    Dynamic(Operand),
    Builtin { ast_func_no: usize },
    HostFunction { name: String },
}

#[derive(Clone, Debug)]
pub struct PhiInput {
    pub operand: Operand,
    pub block_no: usize,
}

impl From<&ast::StructType> for StructType {
    fn from(ty: &ast::StructType) -> Self {
        match ty {
            ast::StructType::AccountInfo => StructType::SolAccountInfo,
            ast::StructType::AccountMeta => StructType::SolAccountMeta,
            ast::StructType::ExternalFunction => StructType::ExternalFunction,
            ast::StructType::SolParameters => StructType::SolParameters,
            ast::StructType::UserDefined(i) => StructType::UserDefined(*i),
        }
    }
}

impl fmt::Display for StructType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructType::UserDefined(i) => write!(f, "{}", i),
            StructType::SolAccountInfo => write!(f, "SolAccountInfo"),
            StructType::SolAccountMeta => write!(f, "SolAccountMeta"),
            StructType::ExternalFunction => write!(f, "ExternalFunction"),
            StructType::SolParameters => write!(f, "SolParameters"),
            StructType::Vector(elem_ty) => write!(f, "vector<{}>", elem_ty),
        }
    }
}

impl fmt::Display for LIRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lir_type)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int(width) => write!(f, "int{}", width),
            Type::Uint(width) => write!(f, "uint{}", width),
            Type::Bytes(width) => write!(f, "bytes{}", width),
            Type::Ptr(ty) => write!(f, "ptr<{}>", ty),
            Type::StoragePtr(immutable, ty) => {
                if *immutable {
                    write!(f, "const_storage_ptr<{}>", ty)
                } else {
                    write!(f, "storage_ptr<{}>", ty)
                }
            }
            Type::Array(ty, len) => {
                write!(f, "{}", ty)?;
                len.iter().for_each(|len| match len {
                    ArrayLength::Fixed(len) => write!(f, "[{}]", len).unwrap(),
                    ArrayLength::Dynamic => write!(f, "[]").unwrap(),
                    ArrayLength::AnyFixed => write!(f, "[?]").unwrap(),
                });
                Ok(())
            }
            Type::Slice(ty) => write!(f, "slice<{}>", ty),
            Type::Struct(ty) => write!(f, "struct.{}", ty),
            Type::Function { params, returns } => {
                write!(f, "function (")?;
                for (i, param) in params.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") returns (")?;
                for (i, ret) in returns.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ret)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Type::Mapping { key_ty, value_ty } => {
                write!(f, "mapping({} => {})", key_ty, value_ty)
            }
        }
    }
}

impl PhiInput {
    pub fn new(operand: Operand, block_no: usize) -> Self {
        Self { operand, block_no }
    }
}
