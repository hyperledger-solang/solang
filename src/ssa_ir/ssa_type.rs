use std::fmt;

use solang_parser::pt::Identifier;

use crate::pt::Loc;
use crate::sema::ast::{self, ParameterAnnotation};
use crate::sema::ast::{ArrayLength, StructType};
use crate::ssa_ir::expr::Operand;

#[derive(Debug, Clone)]
pub enum Type {
    Bool,
    Int(u16),
    /// Enum can be represented by Uint here.
    /// TODO: what is the width for Enum?
    Uint(u16),
    Bytes(u8),

    // String is the same as DynamicBytes
    /// Array can be represented as Ptr(Box<Array>)
    /// Struct can be represented as Ptr(Box<Struct>)
    /// Slice can be represented as Ptr(Box<Slice(Box<Type>)>)
    /// BufferPointer is a Ptr to u8 (a byte)
    /// DynamicBytes is a Ptr of Bytes
    /// String is a Ptr of Bytes
    Ptr(Box<Type>),
    /// pointer to another address space
    StoragePtr(Box<Type>),
    FunctionPtr {
        params: Vec<Type>,
        returns: Vec<Type>,
    },

    // contract == address
    // address is an array of byte
    Array(Box<Type>, Vec<ArrayLength>),
    Struct(StructType),
    // a slice is a ptr to struct that contains the ptr to data and the length
    Slice(Box<Type>),
    // a UserType will be lower into a primitive type it is representing

    // Solana a value 64bits, TODO: Polkadot value length is 16bits or 16bytes?
    // Value is a integer, but width is platform dependent.

    // FunctionSelector is an integer, 4bytes on Polkadot and 8bytes on Solana
}

#[derive(Clone, Debug)]
pub enum InternalCallTy {
    Static { cfg_no: usize },
    Dynamic(Operand),
    Builtin { ast_func_no: usize },
}

#[derive(Clone, Debug)]
pub struct PhiInput {
    pub operand: Operand,
    pub block_no: usize,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub loc: Loc,
    /// The name can empty (e.g. in an event field or unnamed parameter/return)
    pub id: Option<Identifier>,
    pub ty: Type,
    /// Yul function parameters may not have a type identifier
    pub ty_loc: Option<Loc>,
    /// Event fields may indexed, which means they are sent to the log
    pub indexed: bool,
    /// Some builtin structs have readonly fields
    pub readonly: bool,
    /// A recursive struct may contain itself which make the struct infinite size in memory.
    pub infinite_size: bool,
    /// Is this struct field recursive. Recursive does not mean infinite size in all cases:
    /// `struct S { S[] s }` is recursive but not of infinite size.
    pub recursive: bool,

    pub annotation: Option<ParameterAnnotation>,
}

impl TryFrom<&ast::Parameter> for Parameter {
    type Error = &'static str;

    fn try_from(param: &ast::Parameter) -> Result<Self, Self::Error> {
        let ty = Type::try_from(&param.ty)?;
        Ok(Self {
            loc: param.loc,
            id: param.id.clone(),
            ty,
            ty_loc: param.ty_loc,
            indexed: param.indexed,
            readonly: param.readonly,
            infinite_size: param.infinite_size,
            recursive: param.recursive,
            annotation: param.annotation.clone(),
        })
    }
}

impl TryFrom<&ast::Type> for Type {
    type Error = &'static str;

    fn try_from(ty: &ast::Type) -> Result<Self, Self::Error> {
        match ty {
            ast::Type::Address(_) => Ok(Type::Bytes(20)),
            ast::Type::Bool => Ok(Type::Bool),
            ast::Type::Int(width) => Ok(Type::Int(width.clone())),
            ast::Type::Uint(width) => Ok(Type::Uint(width.clone())),
            ast::Type::Rational => {
                // throw error
                Err("Rational is not supported")
            }
            ast::Type::Bytes(width) => Ok(Type::Bytes(width.clone())),
            /// DynamicBytes is a Ptr of Bytes
            ast::Type::DynamicBytes => Ok(Type::Ptr(Box::new(Type::Bytes(1)))),
            /// String is a Ptr of Bytes
            ast::Type::String => Ok(Type::Ptr(Box::new(Type::Bytes(1)))),
            // ast::Type::Array(ty, len) => {
            //     let ty = Self::try_from(ty)?;
            //     Ok(Type::Array(Box::new(ty), len.clone()))
            // }
            // ast::Type::Enum(width) => Ok(Type::Uint(width)),
            // ast::Type::Struct(_) => {}
            // ast::Type::Mapping(_) => {}
            // ast::Type::Contract(_) => {}
            // ast::Type::Ref(_) => {}
            // ast::Type::StorageRef(_, _) => {}
            // ast::Type::InternalFunction { .. } => {}
            // ast::Type::ExternalFunction { .. } => {}
            // ast::Type::UserType(_) => {}
            // ast::Type::Value => {}
            // ast::Type::Void => {}
            // ast::Type::Unreachable => {}
            // ast::Type::Slice(_) => {}
            // ast::Type::Unresolved => {}
            // ast::Type::BufferPointer => {}
            // ast::Type::FunctionSelector => {}
            _ => todo!("{:?}", ty),
        }
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
            Type::StoragePtr(ty) => write!(f, "storage_ptr<{}>", ty),
            Type::Array(ty, len) => {
                // example, for fixed length: ty: uint8, len: [2, 3] -> uint8[2][3]
                // for dynamic length: ty: uint8, len: dyn -> uint8[]
                // for any fixed length: ty: uint8, len: [any, any] -> uint8[?][?]
                write!(f, "{}", ty)?;
                len.iter().for_each(|len| match len {
                    ArrayLength::Fixed(len) => write!(f, "[{}]", len).unwrap(),
                    ArrayLength::Dynamic => write!(f, "[]").unwrap(),
                    ArrayLength::AnyFixed => write!(f, "[?]").unwrap(),
                });
                Ok(())
            }
            Type::Slice(ty) => write!(f, "slice<{}>", ty),
            Type::Struct(ty) => write!(f, "{:?}", ty),
            Type::FunctionPtr { params, returns } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ")")?;
                if !returns.is_empty() {
                    write!(f, " -> (")?;
                    for (i, ret) in returns.iter().enumerate() {
                        if i != 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", ret)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            _ => todo!("{:?}", self),
        }
    }
}

impl PhiInput {
    pub fn new(operand: Operand, block_no: usize) -> Self {
        Self { operand, block_no }
    }
}

impl fmt::Display for PhiInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, block#{}]", self.operand, self.block_no)
    }
}
