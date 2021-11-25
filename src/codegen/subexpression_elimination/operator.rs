use crate::sema::ast::Expression;

/// This enum defines operator types for the graph
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    Or,
    And,
    More,
    Less,
    MoreEqual,
    LessEqual,
    Equal,
    NotEqual,
    StringConcat,
    StringCompare,
    //Unary operations
    Not,
    ZeroExt,
    SignExt,
    Trunc,
    Cast,
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
            Expression::Divide(..) => Operator::Divide,
            Expression::Modulo(..) => Operator::Modulo,
            Expression::Power(..) => Operator::Power,
            Expression::BitwiseOr(..) => Operator::BitwiseOr,
            Expression::BitwiseAnd(..) => Operator::BitwiseAnd,
            Expression::BitwiseXor(..) => Operator::BitwiseXor,
            Expression::ShiftLeft(..) => Operator::ShiftLeft,
            Expression::ShiftRight(..) => Operator::ShiftRight,
            Expression::Or(..) => Operator::Or,
            Expression::And(..) => Operator::And,
            Expression::Not(..) => Operator::Not,
            Expression::ZeroExt(..) => Operator::ZeroExt,
            Expression::SignExt(..) => Operator::SignExt,
            Expression::Trunc(..) => Operator::Trunc,
            Expression::Cast(..) => Operator::Cast,
            Expression::BytesCast(..) => Operator::BytesCast,
            Expression::UnaryMinus(..) => Operator::UnaryMinus,
            Expression::More(..) => Operator::More,
            Expression::Less(..) => Operator::Less,
            Expression::MoreEqual(..) => Operator::MoreEqual,
            Expression::LessEqual(..) => Operator::LessEqual,
            Expression::Equal(..) => Operator::Equal,
            Expression::NotEqual(..) => Operator::NotEqual,
            Expression::Complement(..) => Operator::Complement,
            Expression::StringCompare(..) => Operator::StringCompare,
            Expression::StringConcat(..) => Operator::StringConcat,
            _ => {
                unreachable!("Expression does not represent an operator.")
            }
        }
    }
}
