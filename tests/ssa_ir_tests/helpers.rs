use num_bigint::BigInt;
use solang::ssa_ir::{
    expr::{BinaryOperator, Expr, Operand, UnaryOperator},
    ssa_type::Type,
};
use solang_parser::pt::Loc;

pub(crate) fn binop_expr(left: Operand, op: BinaryOperator, right: Operand) -> Expr {
    Expr::BinaryExpr {
        loc: Loc::Codegen,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

pub(crate) fn unop_expr(op: UnaryOperator, right: Operand) -> Expr {
    Expr::UnaryExpr {
        loc: Loc::Codegen,
        operator: op,
        right: Box::new(right),
    }
}

pub(crate) fn num_literal(value: i32, signed: bool, width: u16) -> Operand {
    Operand::NumberLiteral {
        value: BigInt::from(value),
        ty: if signed {
            Type::Int(width)
        } else {
            Type::Uint(width)
        },
    }
}

#[macro_export]
macro_rules! num_literal {
    ($value: expr, $width: expr) => {
        num_literal($value, false, $width)
    };
    ($value: expr) => {
        num_literal($value, false, 8)
    };
    // error
    () => {
        panic!("invalid number literal")
    };
}

pub(crate) fn bool_literal(value: bool) -> Operand {
    Operand::BoolLiteral { value }
}

pub(crate) fn identifier(id: usize) -> Operand {
    Operand::Id { id }
}
