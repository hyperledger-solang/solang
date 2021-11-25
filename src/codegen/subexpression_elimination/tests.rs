#![cfg(test)]

use crate::codegen::cfg::Instr;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::{AvailableExpression, AvailableExpressionSet};
use crate::parser::pt::Loc;
use crate::sema::ast::{Expression, StringLocation, Type};
use num_bigint::{BigInt, Sign};

#[test]
fn add_variable_function_arg() {
    let var = Expression::Variable(Loc(0, 0, 0), Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc(0, 1, 2), Type::Int(1), 1);
    let add = Expression::Add(
        Loc(0, 1, 2),
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(arg.clone()),
    );

    let instr = Instr::Set {
        loc: Loc(0, 3, 5),
        res: 6,
        expr: add,
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&var).is_some());
    assert!(set.find_expression(&arg).is_some());
}

#[test]
fn add_constants() {
    let var = Expression::ConstantVariable(Loc(0, 0, 0), Type::Int(2), None, 1);
    let num =
        Expression::NumberLiteral(Loc(0, 1, 2), Type::Int(1), BigInt::new(Sign::Plus, vec![2]));
    let sub = Expression::Subtract(
        Loc(0, 1, 2),
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(num.clone()),
    );

    let instr = Instr::SelfDestruct { recipient: sub };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&var).is_some());
    assert!(set.find_expression(&num).is_some());
}

#[test]
fn add_commutative() {
    let cte = Expression::BoolLiteral(Loc(0, 1, 3), false);
    let var = Expression::Variable(Loc(0, 1, 2), Type::Bool, 3);
    let expr = Expression::Or(Loc(1, 2, 3), Box::new(cte.clone()), Box::new(var.clone()));

    let instr = Instr::ValueTransfer {
        success: None,
        address: var.clone(),
        value: expr.clone(),
    };

    let expr_other = Expression::Or(Loc(1, 2, 3), Box::new(var), Box::new(cte));

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&expr).is_some());
    assert!(set.find_expression(&expr_other).is_some());
}

#[test]
fn non_commutative() {
    let var = Expression::Variable(Loc(0, 0, 0), Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc(0, 1, 2), Type::Int(1), 1);
    let add = Expression::Add(
        Loc(0, 1, 2),
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(arg.clone()),
    );
    let num =
        Expression::NumberLiteral(Loc(0, 1, 2), Type::Int(1), BigInt::new(Sign::Plus, vec![2]));
    let sub = Expression::Subtract(
        Loc(0, 1, 2),
        Type::Int(0),
        false,
        Box::new(add.clone()),
        Box::new(num.clone()),
    );

    let instr = Instr::AssertFailure {
        expr: Some(sub.clone()),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&sub).is_some());
    assert!(set.find_expression(&num).is_some());
    assert!(set.find_expression(&add).is_some());
    assert!(set.find_expression(&arg).is_some());
    assert!(set.find_expression(&var).is_some());
}

#[test]
fn unary_operation() {
    let var = Expression::Variable(Loc(0, 0, 0), Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc(0, 1, 2), Type::Int(1), 1);
    let cast = Expression::Cast(Loc(1, 54, 234), Type::Int(32), Box::new(var));
    let exp = Expression::ShiftLeft(
        Loc(1, 2, 3),
        Type::Int(32),
        Box::new(arg),
        Box::new(cast.clone()),
    );

    let instr = Instr::Return {
        value: vec![exp.clone()],
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&cast).is_some());
    assert!(set.find_expression(&exp).is_some());
}

#[test]
fn not_tracked() {
    let var = Expression::Variable(Loc(0, 0, 0), Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc(0, 1, 2), Type::Int(1), 1);
    let load = Expression::Load(Loc(1, 2, 3), Type::DynamicBytes, Box::new(var));
    let minus = Expression::UnaryMinus(Loc(1, 54, 234), Type::Int(32), Box::new(load.clone()));
    let exp = Expression::ShiftLeft(
        Loc(1, 2, 3),
        Type::Int(32),
        Box::new(arg),
        Box::new(minus.clone()),
    );

    let instr = Instr::PushMemory {
        res: 0,
        ty: Type::Bool,
        array: 0,
        value: Box::new(exp.clone()),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&minus).is_none());
    assert!(set.find_expression(&exp).is_none());
    assert!(set.find_expression(&load).is_none());
}

#[test]
fn invalid() {
    let arg = Expression::FunctionArg(Loc(0, 1, 2), Type::Int(1), 1);
    let exp = Expression::List(Loc(1, 2, 3), vec![arg.clone()]);

    let instr = Instr::AbiDecode {
        res: vec![],
        selector: None,
        exception_block: None,
        tys: vec![],
        data: exp.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&arg).is_none());
    assert!(set.find_expression(&exp).is_none());
}

#[test]
fn complex_expression() {
    let var = Expression::Variable(Loc(1, 2, 3), Type::Int(8), 2);
    let cte = Expression::ConstantVariable(Loc(0, 1, 2), Type::Int(8), Some(2), 3);
    let arg = Expression::FunctionArg(Loc(0, 45, 234), Type::Int(9), 5);

    let sum = Expression::Add(
        Loc(0, 23, 24),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc(0, 34, 43),
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::Divide(
        Loc(0, 76, 76),
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc(4, 6, 2),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc(2, 3, 3),
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::Modulo(
        Loc(0, 2, 3),
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc(0, 234, 244), Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc(0, 3, 44), Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc(1, 2, 33),
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc(0, 0, 0),
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&pot).is_some());
    assert!(set.find_expression(&unary).is_some());
    assert!(set.find_expression(&zero).is_some());
    assert!(set.find_expression(&modu).is_some());
    assert!(set.find_expression(&shift).is_some());
    assert!(set.find_expression(&mul).is_some());
    assert!(set.find_expression(&div).is_some());
    assert!(set.find_expression(&sub).is_some());
    assert!(set.find_expression(&sum).is_some());
    assert!(set.find_expression(&arg).is_some());
    assert!(set.find_expression(&cte).is_some());
    assert!(set.find_expression(&var).is_some());

    let var = Expression::Variable(Loc(1, 2, 3), Type::Int(8), 4);
    let sum2 = Expression::Add(
        Loc(1, 2, 3),
        Type::Int(2),
        false,
        Box::new(var),
        Box::new(cte),
    );
    assert!(set.find_expression(&sum2).is_none());
}

#[test]
fn string() {
    let var1 = Expression::Variable(Loc(0, 1, 2), Type::String, 3);
    let var2 = Expression::Variable(Loc(0, 1, 3), Type::String, 4);

    let op1 = StringLocation::RunTime(Box::new(var1.clone()));
    let op2 = StringLocation::RunTime(Box::new(var2.clone()));

    let op3 = StringLocation::CompileTime(vec![0, 1]);

    let concat = Expression::StringConcat(Loc(0, 22, 23), Type::String, op1.clone(), op2.clone());
    let compare = Expression::StringCompare(Loc(0, 23, 25), op2.clone(), op1.clone());

    let concat2 = Expression::StringConcat(Loc(0, 21, 23), Type::String, op2.clone(), op1);
    let compare2 = Expression::StringCompare(Loc(0, 5, 65), op2, op3);

    let instr = Instr::Constructor {
        success: None,
        res: 0,
        contract_no: 0,
        constructor_no: None,
        args: vec![concat.clone()],
        value: Some(compare.clone()),
        gas: concat2.clone(),
        salt: Some(compare2.clone()),
        space: None,
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);

    assert!(set.find_expression(&concat).is_some());
    assert!(set.find_expression(&compare).is_some());
    assert!(set.find_expression(&concat2).is_some());
    assert!(set.find_expression(&compare2).is_none());

    assert!(set.find_expression(&var1).is_some());
    assert!(set.find_expression(&var2).is_some());
}

#[test]
fn kill() {
    let var = Expression::Variable(Loc(1, 2, 3), Type::Int(8), 2);
    let cte = Expression::ConstantVariable(Loc(0, 1, 2), Type::Int(8), Some(2), 3);
    let arg = Expression::FunctionArg(Loc(0, 45, 234), Type::Int(9), 5);

    let sum = Expression::Add(
        Loc(0, 23, 24),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc(0, 34, 43),
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::Divide(
        Loc(0, 76, 76),
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc(4, 6, 2),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc(2, 3, 3),
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::Modulo(
        Loc(0, 2, 3),
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc(0, 234, 244), Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc(0, 3, 44), Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc(1, 2, 33),
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc(0, 0, 0),
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);
    set.kill(2);

    // Available expressions
    assert!(set.find_expression(&unary).is_some());
    assert!(set.find_expression(&modu).is_some());
    assert!(set.find_expression(&sub).is_some());
    assert!(set.find_expression(&arg).is_some());
    assert!(set.find_expression(&cte).is_some());

    // Unavailable expressions
    assert!(set.find_expression(&var).is_none());
    assert!(set.find_expression(&sum).is_none());
    assert!(set.find_expression(&shift).is_none());
    assert!(set.find_expression(&mul).is_none());
    assert!(set.find_expression(&div).is_none());
    assert!(set.find_expression(&zero).is_none());
    assert!(set.find_expression(&pot).is_none());
}

#[test]
fn clone() {
    let var = Expression::Variable(Loc(1, 2, 3), Type::Int(8), 2);
    let cte = Expression::ConstantVariable(Loc(0, 1, 2), Type::Int(8), Some(2), 3);
    let arg = Expression::FunctionArg(Loc(0, 45, 234), Type::Int(9), 5);

    let sum = Expression::Add(
        Loc(0, 23, 24),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc(0, 34, 43),
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::Divide(
        Loc(0, 76, 76),
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc(4, 6, 2),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc(2, 3, 3),
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::Modulo(
        Loc(0, 2, 3),
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc(0, 234, 244), Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc(0, 3, 44), Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc(1, 2, 33),
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc(0, 0, 0),
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);
    let set_2 = set.clone_for_parent_block(1);

    // Available expressions
    assert!(set_2.find_expression(&unary).is_some());
    assert!(set_2.find_expression(&modu).is_some());
    assert!(set_2.find_expression(&sub).is_some());
    assert!(set_2.find_expression(&arg).is_some());
    assert!(set_2.find_expression(&cte).is_some());
    assert!(set_2.find_expression(&var).is_some());
    assert!(set_2.find_expression(&sum).is_some());
    assert!(set_2.find_expression(&shift).is_some());
    assert!(set_2.find_expression(&mul).is_some());
    assert!(set_2.find_expression(&div).is_some());
    assert!(set_2.find_expression(&zero).is_some());
    assert!(set_2.find_expression(&pot).is_some());
}

#[test]
fn intersect() {
    let var = Expression::Variable(Loc(1, 2, 3), Type::Int(8), 1);
    let cte = Expression::ConstantVariable(Loc(0, 1, 2), Type::Int(8), Some(2), 3);
    let arg = Expression::FunctionArg(Loc(0, 45, 234), Type::Int(9), 5);

    let sum = Expression::Add(
        Loc(0, 23, 24),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc(0, 34, 43),
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::Divide(
        Loc(0, 76, 76),
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc(4, 6, 2),
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc(2, 3, 3),
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::Modulo(
        Loc(0, 2, 3),
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc(0, 234, 244), Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc(0, 3, 44), Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc(1, 2, 33),
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let var2 = Expression::Variable(Loc(0, 3, 4), Type::Int(8), 2);
    let var3 = Expression::Variable(Loc(0, 5, 6), Type::Int(8), 3);

    let instr = Instr::Set {
        loc: Loc(0, 0, 0),
        res: 0,
        expr: pot.clone(),
    };

    let instr2 = Instr::Return {
        value: vec![var2.clone(), var3.clone()],
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut cst);
    set.process_instruction(&instr2, &mut ave, &mut cst);
    let mut set_2 = set.clone_for_parent_block(1);
    set.kill(1);

    let sum2 = Expression::Add(
        Loc(0, 45, 55),
        Type::Int(8),
        true,
        Box::new(var2),
        Box::new(var3),
    );
    let sub2 = Expression::Subtract(
        Loc(0, 676, 635),
        Type::Int(8),
        false,
        Box::new(arg.clone()),
        Box::new(sum2.clone()),
    );

    let instr3 = Instr::PushMemory {
        res: 0,
        ty: Type::Bool,
        array: 0,
        value: Box::new(sub2.clone()),
    };

    set.process_instruction(&instr3, &mut ave, &mut cst);
    set_2.process_instruction(&instr3, &mut ave, &mut cst);

    set_2.intersect_sets(&set);

    // Available expressions
    assert!(set_2.find_expression(&unary).is_some());
    assert!(set_2.find_expression(&modu).is_some());
    assert!(set_2.find_expression(&sub).is_some());
    assert!(set_2.find_expression(&arg).is_some());
    assert!(set_2.find_expression(&cte).is_some());

    // Unavailable expressions
    assert!(set_2.find_expression(&var).is_none());
    assert!(set_2.find_expression(&sum).is_none());
    assert!(set_2.find_expression(&shift).is_none());
    assert!(set_2.find_expression(&mul).is_none());
    assert!(set_2.find_expression(&div).is_none());
    assert!(set_2.find_expression(&zero).is_none());
    assert!(set_2.find_expression(&pot).is_none());

    // Expression formed with nodes that existed before clone should be available
    assert!(set_2.find_expression(&sum2).is_some());

    // Child of expression created on both sets should not be available
    assert!(set_2.find_expression(&sub2).is_none());
}
