// SPDX-License-Identifier: Apache-2.0

use indexmap::IndexMap;
use num_bigint::BigInt;
use solang::ssa_ir::{
    expressions::{BinaryOperator, Expression, Operand, UnaryOperator},
    printer::Printer,
    ssa_type::Type,
    vartable::Vartable,
};
use solang_parser::pt::Loc;

pub(crate) fn binop_expr(left: Operand, op: BinaryOperator, right: Operand) -> Expression {
    Expression::BinaryExpr {
        loc: Loc::Codegen,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

pub(crate) fn unop_expr(op: UnaryOperator, right: Operand) -> Expression {
    Expression::UnaryExpr {
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
        loc: Loc::Codegen,
    }
}

#[macro_export]
macro_rules! stringfy_expr {
    ($printer:expr, $expr:expr) => {{
        let mut buffer = Vec::new();
        $printer.print_expr(&mut buffer, $expr);
        String::from_utf8(buffer).expect("Failed to convert to string")
    }};
}

#[macro_export]
macro_rules! stringfy_insn {
    ($printer:expr, $insn:expr) => {{
        let mut buf = Vec::new();
        $printer.print_insn(&mut buf, $insn);
        String::from_utf8(buf).unwrap()
    }};
}

#[macro_export]
macro_rules! stringfy_cfg {
    ($printer:expr, $cfg:expr) => {{
        let mut buf = Vec::new();
        $printer.print_cfg(&mut buf, $cfg);
        String::from_utf8(buf).unwrap()
    }};
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

#[macro_export]
macro_rules! new_printer {
    () => {
        new_printer(new_vartable())
    };
}

pub(crate) fn bool_literal(value: bool) -> Operand {
    Operand::BoolLiteral {
        value,
        loc: Loc::Codegen,
    }
}

pub(crate) fn identifier(id: usize) -> Operand {
    Operand::Id {
        id,
        loc: Loc::Codegen,
    }
}

pub fn new_printer(v: Vartable) -> Printer {
    Printer::new(Box::new(v))
}

pub fn new_vartable() -> Vartable {
    Vartable {
        vars: IndexMap::new(),
        args: IndexMap::new(),
        next_id: 0,
    }
}
