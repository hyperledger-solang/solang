use num_bigint::BigInt;
use solang::ssa_ir::expr::{BinaryOperator, Expr, Operand};
use solang_parser::pt::Loc;
use crate::build_solidity;

#[test]
fn test_binary_expr() {
    let expr_add = Expr::BinaryExpr {
        loc: Loc::Codegen,
        op: BinaryOperator::Add { overflowing: false },
        left: Box::new(Operand::NumberLiteral {
            value: BigInt::from(3)
        }),
        right: Box::new(Operand::NumberLiteral {
            value: BigInt::from(4)
        })
    };

    println!(format!("{}", expr_add));
}