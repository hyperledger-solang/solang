// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::codegen::cfg::{ASTFunction, BasicBlock, ControlFlowGraph, Instr};
use crate::codegen::subexpression_elimination::anticipated_expressions::AnticipatedExpressions;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::{AvailableExpression, AvailableExpressionSet};
use crate::codegen::Expression;
use crate::sema::ast::{StringLocation, Type};
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use solang_parser::pt::Loc;

#[test]
fn add_variable_function_arg() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(1), 1);
    let add = Expression::Add(
        Loc::Codegen,
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(arg.clone()),
    );

    let instr = Instr::Set {
        loc: Loc::Codegen,
        res: 6,
        expr: add,
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&var).is_some());
    assert!(set.find_expression(&arg).is_some());
}

#[test]
fn add_constants() {
    let var =
        Expression::NumberLiteral(Loc::Codegen, Type::Int(2), BigInt::new(Sign::Plus, vec![3]));
    let num =
        Expression::NumberLiteral(Loc::Codegen, Type::Int(1), BigInt::new(Sign::Plus, vec![2]));
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(num.clone()),
    );

    let instr = Instr::SelfDestruct { recipient: sub };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&var).is_some());
    assert!(set.find_expression(&num).is_some());
}

#[test]
fn add_commutative() {
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(32), BigInt::from(20));
    let var = Expression::Variable(Loc::Codegen, Type::Int(32), 3);
    let expr = Expression::Add(
        Loc::Codegen,
        Type::Int(32),
        true,
        Box::new(cte.clone()),
        Box::new(var.clone()),
    );

    let instr = Instr::ValueTransfer {
        success: None,
        address: var.clone(),
        value: expr.clone(),
    };

    let expr_other = Expression::Add(
        Loc::Codegen,
        Type::Int(32),
        true,
        Box::new(var),
        Box::new(cte),
    );

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&expr).is_some());
    assert!(set.find_expression(&expr_other).is_some());
}

#[test]
fn non_commutative() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(1), 1);
    let add = Expression::Add(
        Loc::Codegen,
        Type::Int(0),
        false,
        Box::new(var.clone()),
        Box::new(arg.clone()),
    );
    let num =
        Expression::NumberLiteral(Loc::Codegen, Type::Int(1), BigInt::new(Sign::Plus, vec![2]));
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(0),
        false,
        Box::new(add.clone()),
        Box::new(num.clone()),
    );

    let instr = Instr::AssertFailure {
        encoded_args: Some(sub.clone()),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&sub).is_some());
    assert!(set.find_expression(&num).is_some());
    assert!(set.find_expression(&add).is_some());
    assert!(set.find_expression(&arg).is_some());
    assert!(set.find_expression(&var).is_some());
}

#[test]
fn unary_operation() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(1), 1);
    let cast = Expression::Cast(Loc::Codegen, Type::Int(32), Box::new(var));
    let exp = Expression::ShiftLeft(
        Loc::Codegen,
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

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&cast).is_some());
    assert!(set.find_expression(&exp).is_some());
}

#[test]
fn not_tracked() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(2), 1);
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(1), 1);
    let load = Expression::Load(Loc::Codegen, Type::DynamicBytes, Box::new(var));
    let minus = Expression::UnaryMinus(Loc::Codegen, Type::Int(32), Box::new(load.clone()));
    let exp = Expression::ShiftLeft(
        Loc::Codegen,
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

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&minus).is_none());
    assert!(set.find_expression(&exp).is_none());
    assert!(set.find_expression(&load).is_none());
}

#[test]
fn invalid() {
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(1), 1);
    let exp = Expression::List(Loc::Codegen, vec![arg.clone()]);

    let instr = Instr::AbiDecode {
        res: vec![],
        selector: None,
        exception_block: None,
        tys: vec![],
        data: exp.clone(),
        data_len: None,
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&arg).is_none());
    assert!(set.find_expression(&exp).is_none());
}

#[test]
fn complex_expression() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 2);
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(8), BigInt::from(3));
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(9), 5);

    let sum = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::SignedDivide(
        Loc::Codegen,
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc::Codegen,
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::SignedModulo(
        Loc::Codegen,
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc::Codegen, Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc::Codegen, Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc::Codegen,
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc::Codegen,
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

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

    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 4);
    let sum2 = Expression::Add(
        Loc::Codegen,
        Type::Int(2),
        false,
        Box::new(var),
        Box::new(cte),
    );
    assert!(set.find_expression(&sum2).is_none());
}

#[test]
fn string() {
    let var1 = Expression::Variable(Loc::Codegen, Type::String, 3);
    let var2 = Expression::Variable(Loc::Codegen, Type::String, 4);

    let op1 = StringLocation::RunTime(Box::new(var1.clone()));
    let op2 = StringLocation::RunTime(Box::new(var2.clone()));

    let op3 = StringLocation::CompileTime(vec![0, 1]);

    let concat = Expression::StringConcat(Loc::Codegen, Type::String, op1.clone(), op2.clone());
    let compare = Expression::StringCompare(Loc::Codegen, op2.clone(), op1.clone());

    let concat2 = Expression::StringConcat(Loc::Codegen, Type::String, op2.clone(), op1);
    let compare2 = Expression::StringCompare(Loc::Codegen, op2, op3);

    let instr = Instr::Constructor {
        success: None,
        res: 0,
        contract_no: 0,
        encoded_args: concat.clone(),
        encoded_args_len: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::zero()),
        value: Some(compare.clone()),
        gas: concat2.clone(),
        salt: Some(compare2.clone()),
        address: None,
        seeds: None,
        loc: Loc::Codegen,
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));

    assert!(set.find_expression(&concat).is_some());
    assert!(set.find_expression(&compare).is_some());
    assert!(set.find_expression(&concat2).is_some());
    assert!(set.find_expression(&compare2).is_none());

    assert!(set.find_expression(&var1).is_some());
    assert!(set.find_expression(&var2).is_some());
}

#[test]
fn kill() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 2);
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(8), BigInt::from(3));
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(9), 5);

    let sum = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::SignedDivide(
        Loc::Codegen,
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc::Codegen,
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::SignedModulo(
        Loc::Codegen,
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc::Codegen, Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc::Codegen, Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc::Codegen,
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc::Codegen,
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));
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
    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 2);
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(8), BigInt::from(3));
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(9), 5);

    let sum = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::SignedDivide(
        Loc::Codegen,
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc::Codegen,
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::SignedModulo(
        Loc::Codegen,
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc::Codegen, Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc::Codegen, Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc::Codegen,
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let instr = Instr::Set {
        loc: Loc::Codegen,
        res: 0,
        expr: pot.clone(),
    };

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();

    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));
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
    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 1);
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(8), BigInt::from(3));
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(9), 5);

    let sum = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::SignedDivide(
        Loc::Codegen,
        Type::Int(8),
        Box::new(sum.clone()),
        Box::new(sub.clone()),
    );
    let mul = Expression::Multiply(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc::Codegen,
        Type::Int(2),
        Box::new(mul.clone()),
        Box::new(div.clone()),
        true,
    );
    let modu = Expression::SignedModulo(
        Loc::Codegen,
        Type::Int(8),
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );

    let zero = Expression::ZeroExt(Loc::Codegen, Type::Int(54), Box::new(shift.clone()));
    let unary = Expression::UnaryMinus(Loc::Codegen, Type::Int(44), Box::new(modu.clone()));

    let pot = Expression::Power(
        Loc::Codegen,
        Type::Int(4),
        true,
        Box::new(zero.clone()),
        Box::new(unary.clone()),
    );

    let var2 = Expression::Variable(Loc::Codegen, Type::Int(8), 2);
    let var3 = Expression::Variable(Loc::Codegen, Type::Int(8), 3);

    let instr = Instr::Set {
        loc: Loc::Codegen,
        res: 0,
        expr: pot.clone(),
    };

    let instr2 = Instr::Return {
        value: vec![var2.clone(), var3.clone()],
    };

    let sum2 = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        true,
        Box::new(var2),
        Box::new(var3),
    );
    let sub2 = Expression::Subtract(
        Loc::Codegen,
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

    let mut ave = AvailableExpression::default();
    let mut set = AvailableExpressionSet::default();
    let mut cst = CommonSubExpressionTracker::default();
    let cfg_dag = vec![vec![1, 2], vec![], vec![1]];
    let reverse_dag = vec![vec![], vec![0, 2], vec![0]];
    let traversing_order = vec![(1, false), (2, false), (0, false)];
    let mut anticipated = AnticipatedExpressions::new(&cfg_dag, reverse_dag, traversing_order);

    let instructions = vec![
        vec![instr.clone(), instr2.clone()],
        vec![instr3.clone()],
        vec![instr3.clone()],
    ];

    let mut cfg = ControlFlowGraph::new("name".to_string(), ASTFunction::None);
    cfg.blocks = vec![BasicBlock::default(); 3];
    anticipated.calculate_anticipated_expressions(&instructions, &cfg);
    cst.set_anticipated(anticipated);

    ave.set_cur_block(0);
    cst.set_cur_block(0);
    set.process_instruction(&instr, &mut ave, &mut Some(&mut cst));
    set.process_instruction(&instr2, &mut ave, &mut Some(&mut cst));
    let mut set_2 = set.clone().clone_for_parent_block(0);
    cst.set_cur_block(2);
    ave.set_cur_block(2);
    set.kill(1);

    set.process_instruction(&instr3, &mut ave, &mut Some(&mut cst));
    cst.set_cur_block(1);
    ave.set_cur_block(1);
    set_2.process_instruction(&instr3, &mut ave, &mut Some(&mut cst));

    set_2.intersect_sets(&set, &cst);

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

#[test]
fn test_flow() {
    // Case 1:
    let dag = vec![
        vec![1, 2], // 0 -> 1, 2
        vec![3, 4], // 1 -> 3, 4
        vec![3, 4], // 2 -> 3, 4
        vec![],     // 3
        vec![],     // 4
    ];
    let reverse_dag = vec![
        vec![],
        vec![0],    // 1 -> 0
        vec![0],    // 2 -> 0
        vec![1, 2], // 3 -> 1, 2
        vec![1, 2], // 4 -> 1, 2
    ];
    let mut traversing_order = vec![(0, false), (1, false), (2, false), (3, false), (4, false)];
    traversing_order.reverse();
    let anticipated = AnticipatedExpressions::new(&dag, reverse_dag, traversing_order);
    let flow = anticipated.calculate_flow(3, 4);
    assert_eq!(flow, vec![2000.0, 1000.0, 1000.0, 1000.0, 1000.0]);

    // Case 2:
    let dag = vec![
        vec![1, 2, 4], // 0 -> 1, 2, 4
        vec![2, 3],    // 1 -> 2, 3
        vec![4],       // 2 -> 4
        vec![],        // 3
        vec![],        // 4
    ];
    let reverse_dag = vec![
        vec![],     // 0
        vec![0],    // 1 -> 0
        vec![1, 0], // 2 -> 1, 0
        vec![1],    // 3 -> 1
        vec![0, 2], // 4 -> 0, 2
    ];
    let mut traversing_order = vec![(0, false), (1, false), (2, false), (3, false), (4, false)];
    traversing_order.reverse();
    let anticipated_expressions = AnticipatedExpressions::new(&dag, reverse_dag, traversing_order);
    let flow = anticipated_expressions.calculate_flow(3, 4);
    assert_eq!(flow, vec![2000.0, 1250.0, 500.0, 1000.0, 1000.0]);

    // Case 3
    let dag = vec![
        vec![1, 4], // 0 -> 1, 4
        vec![2, 3], // 1 -> 2, 3
        vec![],     // 2
        vec![5],    // 3 -> 5
        vec![5],    // 4 -> 5
        vec![],     // 5
    ];
    let reverse_dag = vec![
        vec![],     // 0
        vec![0],    // 1 -> 0
        vec![1],    // 2 -> 1
        vec![1],    // 3 -> 1
        vec![0],    // 4 -> 0
        vec![3, 4], // 5 -> 3, 4
    ];
    let mut traversing_order = vec![
        (0, false),
        (1, false),
        (4, false),
        (2, false),
        (3, false),
        (5, false),
    ];
    traversing_order.reverse();
    let anticipated_expressions = AnticipatedExpressions::new(&dag, reverse_dag, traversing_order);
    let flow = anticipated_expressions.calculate_flow(4, 5);
    assert_eq!(flow, vec![2000.0, 500.0, 0.0, 500.0, 1500.0, 1000.0]);

    // Case 4
    let dag = vec![
        vec![1, 6],    // 0 -> 1, 6
        vec![2, 4],    // 1 -> 2, 4
        vec![3, 4],    // 2 -> 3, 4
        vec![],        // 3
        vec![5],       // 4 -> 5
        vec![],        // 5
        vec![4, 7, 8], // 6 -> 4, 7, 8
        vec![5],       // 7 -> 5
        vec![],        // 8
    ];
    let reverse_dag = vec![
        vec![],        // 0
        vec![0],       // 1 -> 0
        vec![1],       // 2 -> 1
        vec![2],       // 3 -> 2
        vec![2, 1, 6], // 4 -> 2, 1, 6
        vec![4],       // 5 -> 4
        vec![0],       // 6 -> 0
        vec![6],       // 7 -> 6
        vec![6],       // 8 -> 6
    ];
    let mut traversing_order = vec![
        (0, false),
        (1, false),
        (6, false),
        (8, false),
        (7, false),
        (2, false),
        (3, false),
        (4, false),
        (5, false),
    ];
    traversing_order.reverse();
    let anticipated_expressions = AnticipatedExpressions::new(&dag, reverse_dag, traversing_order);
    let flow = anticipated_expressions.calculate_flow(5, 8);
    for (item_no, flow_mag) in flow.iter().enumerate() {
        if item_no == 0 {
            assert!((*flow_mag - 2000.0).abs() < 0.000001);
        } else {
            assert!((*flow_mag - 2000.0).abs() > 0.000001);
        }
    }

    let flow = anticipated_expressions.calculate_flow(3, 5);
    for (item_no, flow_mag) in flow.iter().enumerate() {
        if item_no == 0 {
            // TODO: Is this precision enough
            assert!((*flow_mag - 2000.0).abs() < 0.01);
        } else {
            assert!((*flow_mag - 2000.0).abs() > 0.01);
        }
    }

    // Case 5
    let dag = vec![
        vec![1, 3],    // 0 -> 1, 3
        vec![2, 4],    // 1 -> 2, 4
        vec![7, 8, 6], // 2 -> 7, 8, 6
        vec![2, 6],    // 3 -> 2, 6
        vec![7, 5],    // 4 -> 7, 5
        vec![],        // 5
        vec![],        // 6
        vec![],        // 7
        vec![],        // 8
    ];
    let reverse_dag = vec![
        vec![],     // 0
        vec![0],    // 1 -> 0
        vec![1, 3], // 2 -> 1, 3
        vec![0],    // 3 -> 0
        vec![1],    // 4 -> 1
        vec![4],    // 5 -> 4
        vec![2, 3], // 6 -> 2, 3
        vec![4, 2], // 7 -> 2, 4
        vec![2],    // 8 -> 2
    ];
    let mut traversing_order = vec![
        (0, false),
        (1, false),
        (3, false),
        (4, false),
        (2, false),
        (5, false),
        (6, false),
        (7, false),
        (8, false),
    ];
    traversing_order.reverse();
    let anticipated_expressions = AnticipatedExpressions::new(&dag, reverse_dag, traversing_order);
    let flow = anticipated_expressions.calculate_flow(8, 6);
    for (item_no, flow_mag) in flow.iter().enumerate() {
        if item_no == 0 {
            assert!((*flow_mag - 2000.0).abs() < 0.01);
        } else {
            assert!((*flow_mag - 2000.0).abs() > 0.01);
        }
    }
    let flow = anticipated_expressions.calculate_flow(8, 3);
    for (item_no, flow_mag) in flow.iter().enumerate() {
        if item_no == 0 {
            assert!((*flow_mag - 2000.0).abs() < 0.01);
        } else {
            assert!((*flow_mag - 2000.0).abs() > 0.01);
        }
    }
}

#[test]
fn unite_expressions() {
    let var = Expression::Variable(Loc::Codegen, Type::Int(8), 1);
    let cte = Expression::NumberLiteral(Loc::Codegen, Type::Int(8), BigInt::from(3));
    let arg = Expression::FunctionArg(Loc::Codegen, Type::Int(9), 5);

    let sum = Expression::Add(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var.clone()),
        Box::new(cte.clone()),
    );
    let sub = Expression::Subtract(
        Loc::Codegen,
        Type::Int(3),
        false,
        Box::new(cte.clone()),
        Box::new(arg.clone()),
    );
    let div = Expression::SignedDivide(Loc::Codegen, Type::Int(8), Box::new(sum), Box::new(sub));
    let mul = Expression::Multiply(
        Loc::Codegen,
        Type::Int(8),
        false,
        Box::new(var),
        Box::new(cte.clone()),
    );

    let shift = Expression::ShiftRight(
        Loc::Codegen,
        Type::Int(2),
        Box::new(mul),
        Box::new(div),
        true,
    );
    let modu = Expression::SignedModulo(Loc::Codegen, Type::Int(8), Box::new(cte), Box::new(arg));

    let mut set_1 = AvailableExpressionSet::default();
    let mut ave = AvailableExpression::default();
    let _ = set_1.gen_expression_aux(&shift, &mut ave, &mut None);

    let mut set_2 = AvailableExpressionSet::default();
    let _ = set_2.gen_expression_aux(&modu, &mut ave, &mut None);

    assert!(set_1.find_expression(&shift).is_some());
    assert!(set_1.find_expression(&modu).is_none());
    assert!(set_2.find_expression(&shift).is_none());
    assert!(set_2.find_expression(&modu).is_some());

    set_1.union_sets(&set_2);

    let id_1 = set_1.find_expression(&shift);
    assert!(id_1.is_some());
    let id_2 = set_1.find_expression(&modu);
    assert!(id_2.is_some());
}

// TODO: Test find ancestor
// TODO: Add more codegen cases
