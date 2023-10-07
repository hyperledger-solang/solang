use crate::sema::ast;
use crate::sema::ast::{ArrayLength, StructType};

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
    StorageRef(Box<Type>),

    // contract == address
    // address is an array of byte
    Array(Box<Type>, Vec<ArrayLength>),
    Struct(StructType),
    // a slice is a ptr to struct that contains the ptr to data and the length
    Slice(Box<Type>),

    FunctionPtr {
        params: Vec<Type>,
        returns: Vec<Type>,
    },

    // a UserType will be lower into a primitive type it is representing

    // Solana a value 64bits, TODO: Polkadot value length is 16bits or 16bytes?
    // Value is a integer, but width is platform dependent.




    // FunctionSelector is an integer, 4bytes on Polkadot and 8bytes on Solana
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