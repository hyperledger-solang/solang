use crate::codegen::Expression;
use crate::sema::ast::Type;

/// This enum defines operator types for the graph
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    SignedDivide,
    UnsignedDivide,
    Modulo,
    SignedModulo,
    UnsignedModulo,
    Power,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    ShiftLeft,
    SignedShiftRight,
    UnsignedShiftRight,
    More,
    SignedMore,
    UnsignedMore,
    Less,
    SignedLess,
    UnsignedLess,
    MoreEqual,
    LessEqual,
    Equal,
    NotEqual,
    StringConcat,
    StringCompare,
    AdvancePointer,
    //Unary operations
    Not,
    ZeroExt(Type),
    SignExt(Type),
    Trunc(Type),
    Cast(Type),
    BytesCast,
    UnaryMinus,
    Complement,
}

impl Expression {
    /// Get the respective Operator from an Expression
    pub fn get_ave_operator(&self) -> Operator {
        match self {
            Expression::Add(..) => Operator::Add,
            Expression::Subtract(..) => Operator::Subtract,
            Expression::Multiply(..) => Operator::Multiply,
            Expression::SignedDivide(..) => Operator::SignedDivide,
            Expression::UnsignedDivide(..) => Operator::UnsignedDivide,
            Expression::SignedModulo(..) => Operator::SignedModulo,
            Expression::UnsignedModulo(..) => Operator::UnsignedModulo,
            Expression::Power(..) => Operator::Power,
            Expression::BitwiseOr(..) => Operator::BitwiseOr,
            Expression::BitwiseAnd(..) => Operator::BitwiseAnd,
            Expression::BitwiseXor(..) => Operator::BitwiseXor,
            Expression::ShiftLeft(..) => Operator::ShiftLeft,
            Expression::ShiftRight(_, _, _, _, true) => Operator::SignedShiftRight,
            Expression::ShiftRight(_, _, _, _, false) => Operator::UnsignedShiftRight,
            Expression::Not(..) => Operator::Not,
            Expression::ZeroExt(_, ty, ..) => Operator::ZeroExt(ty.clone()),
            Expression::SignExt(_, ty, ..) => Operator::SignExt(ty.clone()),
            Expression::Trunc(_, ty, ..) => Operator::Trunc(ty.clone()),
            Expression::Cast(_, ty, ..) => Operator::Cast(ty.clone()),
            Expression::BytesCast(..) => Operator::BytesCast,
            Expression::UnaryMinus(..) => Operator::UnaryMinus,
            Expression::SignedMore(..) => Operator::SignedMore,
            Expression::UnsignedMore(..) => Operator::UnsignedMore,
            Expression::SignedLess(..) => Operator::SignedLess,
            Expression::UnsignedLess(..) => Operator::UnsignedLess,
            Expression::MoreEqual(..) => Operator::MoreEqual,
            Expression::LessEqual(..) => Operator::LessEqual,
            Expression::Equal(..) => Operator::Equal,
            Expression::NotEqual(..) => Operator::NotEqual,
            Expression::Complement(..) => Operator::Complement,
            Expression::StringCompare(..) => Operator::StringCompare,
            Expression::StringConcat(..) => Operator::StringConcat,
            Expression::AdvancePointer { .. } => Operator::AdvancePointer,
            _ => {
                unreachable!("Expression does not represent an operator.")
            }
        }
    }
}
