// SPDX-License-Identifier: Apache-2.0

mod arithmetics;
pub(crate) mod constructor;
pub mod expression_impl;
pub(crate) mod function_call;
pub(crate) mod integers;
pub(crate) mod literals;
pub mod retrieve_type;
pub(crate) mod strings;
mod tests;

use super::ast::{
    ArrayLength, Builtin, Diagnostic, Expression, Mutability, Namespace, RetrieveType, Symbol, Type,
};
use super::builtin;
use super::diagnostics::Diagnostics;
use super::eval::check_term_for_constant_overflow;
use super::eval::eval_const_rational;
use super::symtable::Symtable;
use crate::sema::expression::arithmetics::{
    addition, bitwise_and, bitwise_or, bitwise_xor, divide, equal, incr_decr, modulo, multiply,
    power, shift_left, shift_right, subtract,
};
use crate::sema::expression::constructor::{circular_reference, constructor_named_args, new};
use crate::sema::expression::function_call::{
    available_functions, call_expr, function_type, named_call_expr,
};
use crate::sema::expression::integers::{
    bigint_to_expression, coerce, coerce_number, get_int_length,
};
use crate::sema::expression::literals::{
    address_literal, array_literal, hex_literal, hex_number_literal, number_literal,
    rational_number_literal, string_literal,
};
use crate::sema::unused_variable::{
    assigned_variable, check_function_call, check_var_usage_expression, used_variable,
};
use crate::sema::Recurse;
use num_bigint::{BigInt, Sign};
use num_traits::{FromPrimitive, Num, One, Pow, Zero};
use solang_parser::pt::{self, CodeLocation};
use std::ops::{Shl, Sub};

/// Compare two mutability levels
pub fn compatible_mutability(left: &Mutability, right: &Mutability) -> bool {
    matches!(
        (left, right),
        // only payable is compatible with payable
        (Mutability::Payable(_), Mutability::Payable(_))
            // default is compatible with anything but pure and view
            | (Mutability::Nonpayable(_), Mutability::Nonpayable(_) | Mutability::Payable(_))
            // view is compatible with anything but pure
            | (Mutability::View(_), Mutability::View(_) | Mutability::Nonpayable(_) | Mutability::Payable(_))
            // pure is compatible with anything
            | (Mutability::Pure(_), _) // everything else is not compatible
    )
}

/// When resolving an expression, what type are we looking for
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ResolveTo<'a> {
    Unknown,        // We don't know what we're looking for, best effort
    Integer,        // Try to resolve to an integer type value (signed or unsigned, any bit width)
    Discard,        // We won't be using the result. For example, an expression as a statement
    Type(&'a Type), // We will be wanting this type please, e.g. `int64 x = 1;`
}

#[derive(Clone, Default)]
pub struct ExprContext {
    /// What source file are we in
    pub file_no: usize,
    // Are we resolving a contract, and if so, which one
    pub contract_no: Option<usize>,
    /// Are resolving the body of a function, and if so, which one
    pub function_no: Option<usize>,
    /// Are we currently in an unchecked block
    pub unchecked: bool,
    /// Are we evaluating a constant expression
    pub constant: bool,
    /// Are we resolving an l-value
    pub lvalue: bool,
    /// Are we resolving a yul function (it cannot have external dependencies)
    pub yul_function: bool,
}

/// Resolve a parsed expression into an AST expression. The resolve_to argument is a hint to what
/// type the result should be.
pub fn expression(
    expr: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    match expr {
        pt::Expression::Parenthesis(_, expr) => {
            expression(expr, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::ArrayLiteral(loc, exprs) => {
            let res = array_literal(loc, exprs, context, ns, symtable, diagnostics, resolve_to);

            if let Ok(exp) = &res {
                used_variable(ns, exp, symtable);
            }

            res
        }
        pt::Expression::BoolLiteral(loc, v) => Ok(Expression::BoolLiteral {
            loc: *loc,
            value: *v,
        }),
        pt::Expression::StringLiteral(v) => {
            Ok(string_literal(v, context.file_no, diagnostics, resolve_to))
        }
        pt::Expression::HexLiteral(v) => hex_literal(v, diagnostics, resolve_to),
        pt::Expression::NumberLiteral(loc, integer, exp) => number_literal(
            loc,
            integer,
            exp,
            ns,
            &BigInt::one(),
            diagnostics,
            resolve_to,
        ),
        pt::Expression::RationalNumberLiteral(loc, integer, fraction, exp) => {
            rational_number_literal(
                loc,
                integer,
                fraction,
                exp,
                &BigInt::one(),
                ns,
                diagnostics,
                resolve_to,
            )
        }
        pt::Expression::HexNumberLiteral(loc, n) => {
            hex_number_literal(loc, n, ns, diagnostics, resolve_to)
        }
        pt::Expression::AddressLiteral(loc, address) => {
            address_literal(loc, address, ns, diagnostics)
        }
        pt::Expression::Variable(id) => {
            variable(id, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Add(loc, l, r) => {
            addition(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Subtract(loc, l, r) => {
            subtract(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseOr(loc, l, r) => {
            bitwise_or(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseAnd(loc, l, r) => {
            bitwise_and(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseXor(loc, l, r) => {
            bitwise_xor(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::ShiftLeft(loc, l, r) => {
            shift_left(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::ShiftRight(loc, l, r) => {
            shift_right(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Multiply(loc, l, r) => {
            multiply(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Divide(loc, l, r) => {
            divide(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Modulo(loc, l, r) => {
            modulo(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Power(loc, b, e) => {
            power(loc, b, e, context, ns, symtable, diagnostics, resolve_to)
        }
        // compare
        pt::Expression::More(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;

            check_var_usage_expression(ns, &left, &right, symtable);
            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            let expr = Expression::More {
                loc: *loc,
                left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
                right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
            };

            if ty.is_rational() {
                if let Err(diag) = eval_const_rational(&expr, ns) {
                    diagnostics.push(diag);
                }
            }

            Ok(expr)
        }
        pt::Expression::Less(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;

            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            let expr = Expression::Less {
                loc: *loc,
                left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
                right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
            };

            if ty.is_rational() {
                if let Err(diag) = eval_const_rational(&expr, ns) {
                    diagnostics.push(diag);
                }
            }

            Ok(expr)
        }
        pt::Expression::MoreEqual(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            let expr = Expression::MoreEqual {
                loc: *loc,
                left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
                right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
            };

            if ty.is_rational() {
                if let Err(diag) = eval_const_rational(&expr, ns) {
                    diagnostics.push(diag);
                }
            }

            Ok(expr)
        }
        pt::Expression::LessEqual(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            let expr = Expression::LessEqual {
                loc: *loc,
                left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
                right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
            };

            if ty.is_rational() {
                if let Err(diag) = eval_const_rational(&expr, ns) {
                    diagnostics.push(diag);
                }
            }

            Ok(expr)
        }
        pt::Expression::Equal(loc, l, r) => equal(loc, l, r, context, ns, symtable, diagnostics),

        pt::Expression::NotEqual(loc, l, r) => Ok(Expression::Not {
            loc: *loc,
            expr: Box::new(equal(loc, l, r, context, ns, symtable, diagnostics)?),
        }),
        // unary expressions
        pt::Expression::Not(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

            used_variable(ns, &expr, symtable);
            Ok(Expression::Not {
                loc: *loc,
                expr: Box::new(expr.cast(loc, &Type::Bool, true, ns, diagnostics)?),
            })
        }
        pt::Expression::Complement(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

            used_variable(ns, &expr, symtable);
            let expr_ty = expr.ty();

            get_int_length(&expr_ty, loc, true, ns, diagnostics)?;

            Ok(Expression::Complement {
                loc: *loc,
                ty: expr_ty,
                expr: Box::new(expr),
            })
        }
        pt::Expression::UnaryMinus(loc, e) => match e.as_ref() {
            pt::Expression::NumberLiteral(_, integer, exp) => number_literal(
                loc,
                integer,
                exp,
                ns,
                &BigInt::from(-1),
                diagnostics,
                resolve_to,
            ),
            pt::Expression::HexNumberLiteral(_, v) => {
                // a hex literal with a minus before it cannot be an address literal or a bytesN value
                let s: String = v.chars().skip(2).filter(|v| *v != '_').collect();

                let n = BigInt::from_str_radix(&s, 16).unwrap();

                bigint_to_expression(loc, &-n, ns, diagnostics, resolve_to)
            }
            pt::Expression::RationalNumberLiteral(loc, integer, fraction, exp) => {
                rational_number_literal(
                    loc,
                    integer,
                    fraction,
                    exp,
                    &BigInt::from(-1),
                    ns,
                    diagnostics,
                    resolve_to,
                )
            }
            e => {
                let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

                used_variable(ns, &expr, symtable);
                let expr_type = expr.ty();

                if let Expression::NumberLiteral { value, .. } = expr {
                    bigint_to_expression(loc, &-value, ns, diagnostics, resolve_to)
                } else if let Expression::RationalNumberLiteral { ty, value: r, .. } = expr {
                    Ok(Expression::RationalNumberLiteral {
                        loc: *loc,
                        ty,
                        value: -r,
                    })
                } else {
                    get_int_length(&expr_type, loc, false, ns, diagnostics)?;

                    Ok(Expression::UnaryMinus {
                        loc: *loc,
                        ty: expr_type,
                        expr: Box::new(expr),
                    })
                }
            }
        },
        pt::Expression::UnaryPlus(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;
            used_variable(ns, &expr, symtable);
            let expr_type = expr.ty();

            get_int_length(&expr_type, loc, false, ns, diagnostics)?;

            Ok(expr)
        }

        pt::Expression::ConditionalOperator(loc, c, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
            let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;
            check_var_usage_expression(ns, &left, &right, symtable);
            let cond = expression(c, context, ns, symtable, diagnostics, resolve_to)?;
            used_variable(ns, &cond, symtable);

            let cond = cond.cast(&c.loc(), &Type::Bool, true, ns, diagnostics)?;

            let ty = coerce(&left.ty(), &l.loc(), &right.ty(), &r.loc(), ns, diagnostics)?;
            let left = left.cast(&l.loc(), &ty, true, ns, diagnostics)?;
            let right = right.cast(&r.loc(), &ty, true, ns, diagnostics)?;

            Ok(Expression::ConditionalOperator {
                loc: *loc,
                ty,
                cond: Box::new(cond),
                true_option: Box::new(left),
                false_option: Box::new(right),
            })
        }

        // pre/post decrement/increment
        pt::Expression::PostIncrement(loc, var)
        | pt::Expression::PreIncrement(loc, var)
        | pt::Expression::PostDecrement(loc, var)
        | pt::Expression::PreDecrement(loc, var) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "operator not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            incr_decr(var, expr, context, ns, symtable, diagnostics)
        }

        // assignment
        pt::Expression::Assign(loc, var, e) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            assign_single(loc, var, e, context, ns, symtable, diagnostics)
        }

        pt::Expression::AssignAdd(loc, var, e)
        | pt::Expression::AssignSubtract(loc, var, e)
        | pt::Expression::AssignMultiply(loc, var, e)
        | pt::Expression::AssignDivide(loc, var, e)
        | pt::Expression::AssignModulo(loc, var, e)
        | pt::Expression::AssignOr(loc, var, e)
        | pt::Expression::AssignAnd(loc, var, e)
        | pt::Expression::AssignXor(loc, var, e)
        | pt::Expression::AssignShiftLeft(loc, var, e)
        | pt::Expression::AssignShiftRight(loc, var, e) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };
            let expr = assign_expr(loc, var, expr, e, context, ns, symtable, diagnostics);
            if let Ok(expression) = &expr {
                expression.recurse(ns, check_term_for_constant_overflow);
            }
            expr
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => named_call_expr(
            loc,
            ty,
            args,
            false,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
        pt::Expression::New(loc, call) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "new not allowed in constant expression".to_string(),
                ));
                return Err(());
            }

            match call.remove_parenthesis() {
                pt::Expression::FunctionCall(_, ty, args) => {
                    let res = new(loc, ty, args, context, ns, symtable, diagnostics);

                    if let Ok(exp) = &res {
                        check_function_call(ns, exp, symtable);
                    }
                    res
                }
                pt::Expression::NamedFunctionCall(_, ty, args) => {
                    let res =
                        constructor_named_args(loc, ty, args, context, ns, symtable, diagnostics);

                    if let Ok(exp) = &res {
                        check_function_call(ns, exp, symtable);
                    }

                    res
                }
                pt::Expression::Variable(id) => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("missing constructor arguments to {}", id.name),
                    ));
                    Err(())
                }
                expr => {
                    diagnostics.push(Diagnostic::error(
                        expr.loc(),
                        "type with arguments expected".into(),
                    ));
                    Err(())
                }
            }
        }
        pt::Expression::Delete(loc, _) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "delete not allowed in expression".to_string(),
            ));
            Err(())
        }
        pt::Expression::FunctionCall(loc, ty, args) => call_expr(
            loc,
            ty,
            args,
            false,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
        pt::Expression::ArraySubscript(loc, _, None) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "expected expression before ']' token".to_string(),
            ));

            Err(())
        }
        pt::Expression::ArraySlice(loc, ..) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "slice not supported yet".to_string(),
            ));

            Err(())
        }
        pt::Expression::ArraySubscript(loc, array, Some(index)) => {
            array_subscript(loc, array, index, context, ns, symtable, diagnostics)
        }
        pt::Expression::MemberAccess(loc, e, id) => member_access(
            loc,
            e.remove_parenthesis(),
            id,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
        pt::Expression::Or(loc, left, right) => {
            let boolty = Type::Bool;
            let l = expression(
                left,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&boolty),
            )?
            .cast(loc, &boolty, true, ns, diagnostics)?;
            let r = expression(
                right,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&boolty),
            )?
            .cast(loc, &boolty, true, ns, diagnostics)?;

            check_var_usage_expression(ns, &l, &r, symtable);

            Ok(Expression::Or {
                loc: *loc,
                left: Box::new(l),
                right: Box::new(r),
            })
        }
        pt::Expression::And(loc, left, right) => {
            let boolty = Type::Bool;
            let l = expression(
                left,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&boolty),
            )?
            .cast(loc, &boolty, true, ns, diagnostics)?;
            let r = expression(
                right,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&boolty),
            )?
            .cast(loc, &boolty, true, ns, diagnostics)?;
            check_var_usage_expression(ns, &l, &r, symtable);

            Ok(Expression::And {
                loc: *loc,
                left: Box::new(l),
                right: Box::new(r),
            })
        }
        pt::Expression::Type(loc, _) => {
            diagnostics.push(Diagnostic::error(*loc, "type not expected".to_owned()));
            Err(())
        }
        pt::Expression::List(loc, _) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "lists only permitted in destructure statements".to_owned(),
            ));
            Err(())
        }
        pt::Expression::FunctionCallBlock(loc, ..) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "unexpect block encountered".to_owned(),
            ));
            Err(())
        }
        pt::Expression::Unit(loc, expr, unit) => {
            match unit {
                pt::Unit::Wei(loc) | pt::Unit::Gwei(loc) | pt::Unit::Ether(loc)
                    if ns.target != crate::Target::EVM =>
                {
                    diagnostics.push(Diagnostic::warning(
                        *loc,
                        "ethereum currency unit used while not targetting ethereum".to_owned(),
                    ));
                }
                _ => (),
            }

            let unit = match unit {
                pt::Unit::Seconds(_) => BigInt::from(1),
                pt::Unit::Minutes(_) => BigInt::from(60),
                pt::Unit::Hours(_) => BigInt::from(60 * 60),
                pt::Unit::Days(_) => BigInt::from(60 * 60 * 24),
                pt::Unit::Weeks(_) => BigInt::from(60 * 60 * 24 * 7),
                pt::Unit::Wei(_) => BigInt::from(1),
                pt::Unit::Gwei(_) => BigInt::from(10).pow(9u32),
                pt::Unit::Ether(_) => BigInt::from(10).pow(18u32),
            };

            match expr.as_ref() {
                pt::Expression::NumberLiteral(_, integer, exp) => {
                    number_literal(loc, integer, exp, ns, &unit, diagnostics, resolve_to)
                }
                pt::Expression::RationalNumberLiteral(_, significant, mantissa, exp) => {
                    rational_number_literal(
                        loc,
                        significant,
                        mantissa,
                        exp,
                        &unit,
                        ns,
                        diagnostics,
                        resolve_to,
                    )
                }
                pt::Expression::HexNumberLiteral(loc, _) => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "hexadecimal numbers cannot be used with unit denominations".to_owned(),
                    ));
                    Err(())
                }
                _ => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "unit denominations can only be used with number literals".to_owned(),
                    ));
                    Err(())
                }
            }
        }
        pt::Expression::This(loc) => match context.contract_no {
            Some(contract_no) => Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![Type::Contract(contract_no)],
                kind: Builtin::GetAddress,
                args: Vec::new(),
            }),
            None => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "this not allowed outside contract".to_owned(),
                ));
                Err(())
            }
        },
    }
}

fn variable(
    id: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let Some(v) = symtable.find(&id.name) {
        return if context.constant {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!("cannot read variable '{}' in constant expression", id.name),
            ));
            Err(())
        } else {
            Ok(Expression::Variable {
                loc: id.loc,
                ty: v.ty.clone(),
                var_no: v.pos,
            })
        };
    }

    if let Some((builtin, ty)) = builtin::builtin_var(&id.loc, None, &id.name, ns, diagnostics) {
        return Ok(Expression::Builtin {
            loc: id.loc,
            tys: vec![ty],
            kind: builtin,
            args: vec![],
        });
    }

    // are we trying to resolve a function type?
    let function_first = if let ResolveTo::Type(resolve_to) = resolve_to {
        matches!(
            resolve_to,
            Type::InternalFunction { .. } | Type::ExternalFunction { .. }
        )
    } else {
        false
    };

    match ns.resolve_var(context.file_no, context.contract_no, id, function_first) {
        Some(Symbol::Variable(_, Some(var_contract_no), var_no)) => {
            let var_contract_no = *var_contract_no;
            let var_no = *var_no;

            let var = &ns.contracts[var_contract_no].variables[var_no];

            if var.constant {
                Ok(Expression::ConstantVariable {
                    loc: id.loc,
                    ty: var.ty.clone(),
                    contract_no: Some(var_contract_no),
                    var_no,
                })
            } else if context.constant {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "cannot read contract variable '{}' in constant expression",
                        id.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::StorageVariable {
                    loc: id.loc,
                    ty: Type::StorageRef(var.immutable, Box::new(var.ty.clone())),
                    contract_no: var_contract_no,
                    var_no,
                })
            }
        }
        Some(Symbol::Variable(_, None, var_no)) => {
            let var_no = *var_no;

            let var = &ns.constants[var_no];

            Ok(Expression::ConstantVariable {
                loc: id.loc,
                ty: var.ty.clone(),
                contract_no: None,
                var_no,
            })
        }
        Some(Symbol::Function(_)) => {
            let mut name_matches = 0;
            let mut expr = None;

            for function_no in
                available_functions(&id.name, true, context.file_no, context.contract_no, ns)
            {
                let func = &ns.functions[function_no];

                if func.ty != pt::FunctionTy::Function {
                    continue;
                }

                let ty = Type::InternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;
                expr = Some(Expression::InternalFunction {
                    loc: id.loc,
                    ty,
                    function_no,
                    signature: if func.is_virtual || func.is_override.is_some() {
                        Some(func.signature.clone())
                    } else {
                        None
                    },
                });
            }

            if name_matches == 1 {
                Ok(expr.unwrap())
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("function '{}' is overloaded", id.name),
                ));
                Err(())
            }
        }
        None if id.name == "now"
            && matches!(
                resolve_to,
                ResolveTo::Type(Type::Uint(_)) | ResolveTo::Integer
            ) =>
        {
            diagnostics.push(
                Diagnostic::error(
                    id.loc,
                    "'now' not found. 'now' was an alias for 'block.timestamp' in older versions of the Solidity language. Please use 'block.timestamp' instead.".to_string(),
                ));
            Err(())
        }
        sym => {
            diagnostics.push(Namespace::wrong_symbol(sym, id));
            Err(())
        }
    }
}

/// Resolve type(x).foo
pub fn type_name_expr(
    loc: &pt::Loc,
    args: &[pt::Expression],
    field: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if args.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "missing argument to type()".to_string(),
        ));
        return Err(());
    }

    if args.len() > 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!("got {} arguments to type(), only one expected", args.len(),),
        ));
        return Err(());
    }

    let ty = ns.resolve_type(
        context.file_no,
        context.contract_no,
        false,
        &args[0],
        diagnostics,
    )?;

    match (&ty, field.name.as_str()) {
        (Type::Uint(_), "min") => {
            bigint_to_expression(loc, &BigInt::zero(), ns, diagnostics, resolve_to)
        }
        (Type::Uint(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "min") => {
            let min = BigInt::zero().sub(BigInt::one().shl(*bits as usize - 1));
            bigint_to_expression(loc, &min, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize - 1).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Contract(n), "name") => Ok(Expression::BytesLiteral {
            loc: *loc,
            ty: Type::String,
            value: ns.contracts[*n].name.as_bytes().to_vec(),
        }),
        (Type::Contract(n), "interfaceId") => {
            let contract = &ns.contracts[*n];

            if !contract.is_interface() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "type(…).interfaceId is permitted on interface, not {} {}",
                        contract.ty, contract.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::InterfaceId {
                    loc: *loc,
                    contract_no: *n,
                })
            }
        }
        (Type::Contract(no), "program_id") => {
            let contract = &ns.contracts[*no];

            if let Some(v) = &contract.program_id {
                Ok(Expression::NumberLiteral {
                    loc: *loc,
                    ty: Type::Address(false),
                    value: BigInt::from_bytes_be(Sign::Plus, v),
                })
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "{} '{}' has no declared program_id",
                        contract.ty, contract.name
                    ),
                ));
                Err(())
            }
        }
        (Type::Contract(no), "creationCode") | (Type::Contract(no), "runtimeCode") => {
            let contract_no = match context.contract_no {
                Some(contract_no) => contract_no,
                None => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "type().{} not permitted outside of contract code",
                            field.name
                        ),
                    ));
                    return Err(());
                }
            };

            // check for circular references
            if *no == contract_no {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "containing our own contract code for '{}' would generate infinite size contract",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if circular_reference(*no, contract_no, ns) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "circular reference creating contract code for '{}'",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if !ns.contracts[contract_no].creates.contains(no) {
                ns.contracts[contract_no].creates.push(*no);
            }

            Ok(Expression::CodeLiteral {
                loc: *loc,
                contract_no: *no,
                runtime: field.name == "runtimeCode",
            })
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "type '{}' does not have type function {}",
                    ty.to_string(ns),
                    field.name
                ),
            ));
            Err(())
        }
    }
}

/// Resolve an assignment
fn assign_single(
    loc: &pt::Loc,
    left: &pt::Expression,
    right: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let mut lcontext = context.clone();
    lcontext.lvalue = true;

    let var = expression(
        left,
        &lcontext,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    assigned_variable(ns, &var, symtable);

    let var_ty = var.ty();
    let val = expression(
        right,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(var_ty.deref_any()),
    )?;

    val.recurse(ns, check_term_for_constant_overflow);

    used_variable(ns, &val, symtable);
    match &var {
        Expression::ConstantVariable {
            loc,
            contract_no: Some(contract_no),
            var_no,
            ..
        } => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant '{}'",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::ConstantVariable {
            loc,
            contract_no: None,
            var_no,
            ..
        } => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot assign to constant '{}'", ns.constants[*var_no].name),
            ));
            Err(())
        }
        Expression::StorageVariable {
            loc,
            ty,
            contract_no: var_contract_no,
            var_no,
        } => {
            let store_var = &ns.contracts[*var_contract_no].variables[*var_no];

            if store_var.immutable {
                if let Some(function_no) = context.function_no {
                    if !ns.functions[function_no].is_constructor() {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "cannot assign to immutable '{}' outside of constructor",
                                store_var.name
                            ),
                        ));
                        return Err(());
                    }
                }
            }

            let ty = ty.deref_any();

            Ok(Expression::Assign {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(var.clone()),
                right: Box::new(val.cast(&right.loc(), ty, true, ns, diagnostics)?),
            })
        }
        Expression::Variable { ty: var_ty, .. } => Ok(Expression::Assign {
            loc: *loc,
            ty: var_ty.clone(),
            left: Box::new(var.clone()),
            right: Box::new(val.cast(&right.loc(), var_ty, true, ns, diagnostics)?),
        }),
        _ => match &var_ty {
            Type::Ref(r_ty) => Ok(Expression::Assign {
                loc: *loc,
                ty: *r_ty.clone(),
                left: Box::new(var),
                right: Box::new(val.cast(&right.loc(), r_ty, true, ns, diagnostics)?),
            }),
            Type::StorageRef(immutable, r_ty) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                Ok(Expression::Assign {
                    loc: *loc,
                    ty: *r_ty.clone(),
                    left: Box::new(var),
                    right: Box::new(val.cast(&right.loc(), r_ty, true, ns, diagnostics)?),
                })
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    var.loc(),
                    "expression is not assignable".to_string(),
                ));
                Err(())
            }
        },
    }
}

/// Resolve an assignment with an operator
fn assign_expr(
    loc: &pt::Loc,
    left: &pt::Expression,
    expr: &pt::Expression,
    right: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let mut lcontext = context.clone();
    lcontext.lvalue = true;

    let var = expression(
        left,
        &lcontext,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    assigned_variable(ns, &var, symtable);
    let var_ty = var.ty();

    let resolve_to = if matches!(
        expr,
        pt::Expression::AssignShiftLeft(..) | pt::Expression::AssignShiftRight(..)
    ) {
        ResolveTo::Unknown
    } else {
        ResolveTo::Type(var_ty.deref_any())
    };

    let set = expression(right, context, ns, symtable, diagnostics, resolve_to)?;
    used_variable(ns, &set, symtable);
    let set_type = set.ty();

    let assign_operation = |assign: Expression,
                            ty: &Type,
                            ns: &Namespace,
                            diagnostics: &mut Diagnostics|
     -> Result<Expression, ()> {
        let set = match expr {
            pt::Expression::AssignShiftLeft(..) | pt::Expression::AssignShiftRight(..) => {
                let left_length = get_int_length(ty, loc, true, ns, diagnostics)?;
                let right_length = get_int_length(&set_type, &left.loc(), false, ns, diagnostics)?;

                // TODO: does shifting by negative value need compiletime/runtime check?
                if left_length == right_length {
                    set
                } else if right_length < left_length && set_type.is_signed_int() {
                    Expression::SignExt {
                        loc: *loc,
                        to: ty.clone(),
                        expr: Box::new(set),
                    }
                } else if right_length < left_length && !set_type.is_signed_int() {
                    Expression::ZeroExt {
                        loc: *loc,
                        to: ty.clone(),
                        expr: Box::new(set),
                    }
                } else {
                    Expression::Trunc {
                        loc: *loc,
                        to: ty.clone(),
                        expr: Box::new(set),
                    }
                }
            }
            _ => set.cast(&right.loc(), ty, true, ns, diagnostics)?,
        };

        Ok(match expr {
            pt::Expression::AssignAdd(..) => Expression::Add {
                loc: *loc,
                ty: ty.clone(),
                unchecked: context.unchecked,
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignSubtract(..) => Expression::Subtract {
                loc: *loc,
                ty: ty.clone(),
                unchecked: context.unchecked,
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignMultiply(..) => Expression::Multiply {
                loc: *loc,
                ty: ty.clone(),
                unchecked: context.unchecked,
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignOr(..) => Expression::BitwiseOr {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignAnd(..) => Expression::BitwiseAnd {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignXor(..) => Expression::BitwiseXor {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignShiftLeft(..) => Expression::ShiftLeft {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignShiftRight(..) => Expression::ShiftRight {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
                sign: ty.is_signed_int(),
            },
            pt::Expression::AssignDivide(..) => Expression::Divide {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            pt::Expression::AssignModulo(..) => Expression::Modulo {
                loc: *loc,
                ty: ty.clone(),
                left: Box::new(assign),
                right: Box::new(set),
            },
            _ => unreachable!(),
        })
    };

    match &var {
        Expression::ConstantVariable {
            loc,
            contract_no: Some(contract_no),
            var_no,
            ..
        } => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant '{}'",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::ConstantVariable {
            loc,
            contract_no: None,
            var_no,
            ..
        } => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot assign to constant '{}'", ns.constants[*var_no].name),
            ));
            Err(())
        }
        Expression::Variable { var_no, .. } => {
            match var_ty {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => (),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!(
                            "variable '{}' of incorrect type {}",
                            symtable.get_name(*var_no),
                            var_ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            };
            Ok(Expression::Assign {
                loc: *loc,
                ty: var_ty.clone(),
                left: Box::new(var.clone()),
                right: Box::new(assign_operation(var, &var_ty, ns, diagnostics)?),
            })
        }
        _ => match &var_ty {
            Type::Ref(r_ty) => match r_ty.as_ref() {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => Ok(Expression::Assign {
                    loc: *loc,
                    ty: *r_ty.clone(),
                    left: Box::new(var.clone()),
                    right: Box::new(assign_operation(
                        var.cast(loc, r_ty, true, ns, diagnostics)?,
                        r_ty,
                        ns,
                        diagnostics,
                    )?),
                }),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                    ));
                    Err(())
                }
            },
            Type::StorageRef(immutable, r_ty) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                match r_ty.as_ref() {
                    Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => Ok(Expression::Assign {
                        loc: *loc,
                        ty: *r_ty.clone(),
                        left: Box::new(var.clone()),
                        right: Box::new(assign_operation(
                            var.cast(loc, r_ty, true, ns, diagnostics)?,
                            r_ty,
                            ns,
                            diagnostics,
                        )?),
                    }),
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            var.loc(),
                            format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                        ));
                        Err(())
                    }
                }
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    var.loc(),
                    "expression is not assignable".to_string(),
                ));
                Err(())
            }
        },
    }
}

/// Try to resolve expression as an enum value. An enum can be prefixed
/// with import symbols, contract namespace before the enum type
fn enum_value(
    loc: &pt::Loc,
    expr: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    let mut namespace = Vec::new();

    let mut expr = expr;

    // the first element of the path is the deepest in the parse tree,
    // so walk down and add to a list
    while let pt::Expression::MemberAccess(_, member, name) = expr {
        namespace.push(name);

        expr = member.as_ref();
    }

    if let pt::Expression::Variable(name) = expr {
        namespace.push(name);
    } else {
        return Ok(None);
    }

    // The leading part of the namespace can be import variables
    let mut file_no = file_no;

    // last element in our namespace vector is first element
    while let Some(name) = namespace.last().map(|f| f.name.clone()) {
        if let Some(Symbol::Import(_, import_file_no)) =
            ns.variable_symbols.get(&(file_no, None, name))
        {
            file_no = *import_file_no;
            namespace.pop();
        } else {
            break;
        }
    }

    if namespace.is_empty() {
        return Ok(None);
    }

    let mut contract_no = contract_no;

    if let Some(no) = ns.resolve_contract(file_no, namespace.last().unwrap()) {
        contract_no = Some(no);
        namespace.pop();
    }

    if namespace.len() != 1 {
        return Ok(None);
    }

    if let Some(e) = ns.resolve_enum(file_no, contract_no, namespace[0]) {
        match ns.enums[e].values.get_full(&id.name) {
            Some((val, _, _)) => Ok(Some(Expression::NumberLiteral {
                loc: *loc,
                ty: Type::Enum(e),
                value: BigInt::from_usize(val).unwrap(),
            })),
            None => {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("enum {} does not have value {}", ns.enums[e], id.name),
                ));
                Err(())
            }
        }
    } else {
        Ok(None)
    }
}

/// Resolve an member access expression
fn member_access(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    // is it a builtin special variable like "block.timestamp"
    if let pt::Expression::Variable(namespace) = e {
        if let Some((builtin, ty)) =
            builtin::builtin_var(loc, Some(&namespace.name), &id.name, ns, diagnostics)
        {
            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![ty],
                kind: builtin,
                args: vec![],
            });
        }

        if builtin::builtin_namespace(&namespace.name) {
            diagnostics.push(Diagnostic::error(
                e.loc(),
                format!("builtin '{}.{}' does not exist", namespace.name, id.name),
            ));

            return Err(());
        }
    }

    // is it an enum value
    if let Some(expr) = enum_value(
        loc,
        e,
        id,
        context.file_no,
        context.contract_no,
        ns,
        diagnostics,
    )? {
        return Ok(expr);
    }

    // is it a constant (unless basecontract is a local variable)
    if let Some(expr) = contract_constant(
        loc,
        e,
        id,
        context.file_no,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )? {
        return Ok(expr);
    }

    // is it a basecontract.function.selector expression (unless basecontract is a local variable)
    if let pt::Expression::Variable(namespace) = e {
        if symtable.find(&namespace.name).is_none() {
            if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
                // find function with this name
                let mut name_matches = 0;
                let mut expr = Err(());

                for function_no in ns.contracts[call_contract_no].all_functions.keys() {
                    let func = &ns.functions[*function_no];

                    if func.name != id.name || func.ty != pt::FunctionTy::Function {
                        continue;
                    }

                    name_matches += 1;

                    expr = Ok(Expression::InternalFunction {
                        loc: e.loc(),
                        ty: function_type(func, false, resolve_to),
                        function_no: *function_no,
                        signature: None,
                    })
                }

                return match name_matches {
                    0 => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "contract '{}' does not have a member called '{}'",
                                ns.contracts[call_contract_no].name, id.name,
                            ),
                        ));
                        Err(())
                    }
                    1 => expr,
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "function '{}' of contract '{}' is overloaded",
                                id.name, ns.contracts[call_contract_no].name,
                            ),
                        ));
                        Err(())
                    }
                };
            }
        }
    }

    // is of the form "type(x).field", like type(c).min
    if let pt::Expression::FunctionCall(_, name, args) = e {
        if let pt::Expression::Variable(func_name) = name.as_ref() {
            if func_name.name == "type" {
                return type_name_expr(loc, args, id, context, ns, diagnostics, resolve_to);
            }
        }
    }

    let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;
    let expr_ty = expr.ty();

    if let Type::Struct(struct_ty) = expr_ty.deref_memory() {
        if let Some((i, f)) = struct_ty
            .definition(ns)
            .fields
            .iter()
            .enumerate()
            .find(|f| id.name == f.1.name_as_str())
        {
            return if context.lvalue && f.readonly {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "struct '{}' field '{}' is readonly",
                        struct_ty.definition(ns),
                        id.name
                    ),
                ));
                Err(())
            } else if f.readonly {
                // readonly fields return the value, not a reference
                Ok(Expression::StructMember {
                    loc: id.loc,
                    ty: f.ty.clone(),
                    expr: Box::new(expr),
                    field: i,
                })
            } else {
                Ok(Expression::StructMember {
                    loc: id.loc,
                    ty: Type::Ref(Box::new(f.ty.clone())),
                    expr: Box::new(expr),
                    field: i,
                })
            };
        } else {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!(
                    "struct '{}' does not have a field called '{}'",
                    struct_ty.definition(ns),
                    id.name
                ),
            ));
            return Err(());
        }
    }

    // Dereference if need to
    let (expr, expr_ty) = if let Type::Ref(ty) = &expr_ty {
        (
            Expression::Load {
                loc: *loc,
                ty: expr_ty.clone(),
                expr: Box::new(expr),
            },
            ty.as_ref().clone(),
        )
    } else {
        (expr, expr_ty)
    };

    match expr_ty {
        Type::Bytes(n) => {
            if id.name == "length" {
                //We should not eliminate an array from the code when 'length' is called
                //So the variable is also assigned a value to be read from 'length'
                assigned_variable(ns, &expr, symtable);
                used_variable(ns, &expr, symtable);
                return Ok(Expression::NumberLiteral {
                    loc: *loc,
                    ty: Type::Uint(8),
                    value: BigInt::from_u8(n).unwrap(),
                });
            }
        }
        Type::Array(_, dim) => {
            if id.name == "length" {
                return match dim.last().unwrap() {
                    ArrayLength::Dynamic => Ok(Expression::Builtin {
                        loc: *loc,
                        tys: vec![Type::Uint(32)],
                        kind: Builtin::ArrayLength,
                        args: vec![expr],
                    }),
                    ArrayLength::Fixed(d) => {
                        //We should not eliminate an array from the code when 'length' is called
                        //So the variable is also assigned a value to be read from 'length'
                        assigned_variable(ns, &expr, symtable);
                        used_variable(ns, &expr, symtable);
                        bigint_to_expression(
                            loc,
                            d,
                            ns,
                            diagnostics,
                            ResolveTo::Type(&Type::Uint(32)),
                        )
                    }
                    ArrayLength::AnyFixed => unreachable!(),
                };
            }
        }
        Type::String | Type::DynamicBytes => {
            if id.name == "length" {
                return Ok(Expression::Builtin {
                    loc: *loc,
                    tys: vec![Type::Uint(32)],
                    kind: Builtin::ArrayLength,
                    args: vec![expr],
                });
            }
        }
        Type::StorageRef(immutable, r) => match *r {
            Type::Struct(str_ty) => {
                return if let Some((field_no, field)) = str_ty
                    .definition(ns)
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| id.name == field.name_as_str())
                {
                    Ok(Expression::StructMember {
                        loc: id.loc,
                        ty: Type::StorageRef(immutable, Box::new(field.ty.clone())),
                        expr: Box::new(expr),
                        field: field_no,
                    })
                } else {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "struct '{}' does not have a field called '{}'",
                            str_ty.definition(ns).name,
                            id.name
                        ),
                    ));
                    Err(())
                }
            }
            Type::Array(_, dim) => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    if let Some(ArrayLength::Fixed(dim)) = dim.last() {
                        // sparse array could be large than ns.storage_type() on Solana
                        if dim.bits() > ns.storage_type().bits(ns) as u64 {
                            return Ok(Expression::StorageArrayLength {
                                loc: id.loc,
                                ty: Type::Uint(256),
                                array: Box::new(expr),
                                elem_ty,
                            });
                        }
                    }

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: ns.storage_type(),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            Type::Bytes(_) | Type::DynamicBytes | Type::String => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: Type::Uint(32),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            _ => {}
        },
        Type::Address(_) => {
            if id.name == "balance" {
                if ns.target.is_substrate() {
                    let mut is_this = false;

                    if let Expression::Cast { expr: this, .. } = &expr {
                        if let Expression::Builtin {
                            kind: Builtin::GetAddress,
                            ..
                        } = this.as_ref()
                        {
                            is_this = true;
                        }
                    }

                    if !is_this {
                        diagnostics.push(Diagnostic::error(
                            expr.loc(),
                            "substrate can only retrieve balance of this, like 'address(this).balance'".to_string(),
                        ));
                        return Err(());
                    }
                }
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: *loc,
                    tys: vec![Type::Value],
                    kind: Builtin::Balance,
                    args: vec![expr],
                });
            }
        }
        Type::Contract(ref_contract_no) => {
            let mut name_matches = 0;
            let mut ext_expr = Err(());

            for function_no in ns.contracts[ref_contract_no].all_functions.keys() {
                let func = &ns.functions[*function_no];

                if func.name != id.name || func.ty != pt::FunctionTy::Function || !func.is_public()
                {
                    continue;
                }

                let ty = Type::ExternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;
                ext_expr = Ok(Expression::ExternalFunction {
                    loc: id.loc,
                    ty,
                    address: Box::new(expr.clone()),
                    function_no: *function_no,
                });
            }

            return match name_matches {
                0 => {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "{} '{}' has no public function '{}'",
                            ns.contracts[ref_contract_no].ty,
                            ns.contracts[ref_contract_no].name,
                            id.name
                        ),
                    ));
                    Err(())
                }
                1 => ext_expr,
                _ => {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "function '{}' of {} '{}' is overloaded",
                            id.name,
                            ns.contracts[ref_contract_no].ty,
                            ns.contracts[ref_contract_no].name
                        ),
                    ));
                    Err(())
                }
            };
        }
        Type::ExternalFunction { .. } => {
            if id.name == "address" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: e.loc(),
                    tys: vec![Type::Address(false)],
                    kind: Builtin::ExternalFunctionAddress,
                    args: vec![expr],
                });
            }
            if id.name == "selector" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: e.loc(),
                    tys: vec![Type::FunctionSelector],
                    kind: Builtin::FunctionSelector,
                    args: vec![expr],
                });
            }
        }
        Type::InternalFunction { .. } => {
            if let Expression::InternalFunction { .. } = expr {
                if id.name == "selector" {
                    used_variable(ns, &expr, symtable);
                    return Ok(Expression::Builtin {
                        loc: e.loc(),
                        tys: vec![Type::FunctionSelector],
                        kind: Builtin::FunctionSelector,
                        args: vec![expr],
                    });
                }
            }
        }
        _ => (),
    }

    diagnostics.push(Diagnostic::error(*loc, format!("'{}' not found", id.name)));

    Err(())
}

fn contract_constant(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    let namespace = match e {
        pt::Expression::Variable(namespace) => namespace,
        _ => return Ok(None),
    };

    if symtable.find(&namespace.name).is_some() {
        return Ok(None);
    }

    if let Some(contract_no) = ns.resolve_contract(file_no, namespace) {
        if let Some((var_no, var)) = ns.contracts[contract_no]
            .variables
            .iter_mut()
            .enumerate()
            .find(|(_, variable)| variable.name == id.name)
        {
            if !var.constant {
                let resolve_function = if let ResolveTo::Type(ty) = resolve_to {
                    matches!(
                        ty,
                        Type::InternalFunction { .. } | Type::ExternalFunction { .. }
                    )
                } else {
                    false
                };

                if resolve_function {
                    // requested function, fall through
                    return Ok(None);
                } else {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "need instance of contract '{}' to get variable value '{}'",
                            ns.contracts[contract_no].name,
                            ns.contracts[contract_no].variables[var_no].name,
                        ),
                    ));
                    return Err(());
                }
            }

            var.read = true;

            return Ok(Some(Expression::ConstantVariable {
                loc: *loc,
                ty: var.ty.clone(),
                contract_no: Some(contract_no),
                var_no,
            }));
        }
    }

    Ok(None)
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &pt::Loc,
    array: &pt::Expression,
    index: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let mut array = expression(
        array,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    let array_ty = array.ty();

    if array.ty().is_mapping() {
        return mapping_subscript(loc, array, index, context, ns, symtable, diagnostics);
    }

    let index_width_ty = if array_ty.is_contract_storage() && !array_ty.is_storage_bytes() {
        Type::Uint(256)
    } else {
        Type::Uint(32)
    };

    let mut index = expression(
        index,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(&index_width_ty),
    )?;

    let index_ty = index.ty();

    index.recurse(ns, check_term_for_constant_overflow);

    match index_ty.deref_any() {
        Type::Uint(_) => (),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "array subscript must be an unsigned integer, not '{}'",
                    index.ty().to_string(ns)
                ),
            ));
            return Err(());
        }
    };

    if array_ty.is_storage_bytes() {
        return Ok(Expression::Subscript {
            loc: *loc,
            ty: Type::StorageRef(false, Box::new(Type::Bytes(1))),
            array_ty,
            array: Box::new(array),
            index: Box::new(index.cast(&index.loc(), &Type::Uint(32), false, ns, diagnostics)?),
        });
    }

    // make sure we load the index value if needed
    index = index.cast(&index.loc(), index_ty.deref_any(), true, ns, diagnostics)?;

    let deref_ty = array_ty.deref_any();
    match deref_ty {
        Type::Bytes(_) | Type::Array(..) | Type::DynamicBytes => {
            if array_ty.is_contract_storage() {
                let elem_ty = array_ty.storage_array_elem();

                // When subscripting a bytes32 type array, we need to load it. It is not
                // assignable and the value is calculated by shifting in codegen
                if let Type::Bytes(_) = deref_ty {
                    array = array.cast(&array.loc(), deref_ty, true, ns, diagnostics)?;
                }

                Ok(Expression::Subscript {
                    loc: *loc,
                    ty: elem_ty,
                    array_ty,
                    array: Box::new(array),
                    index: Box::new(index),
                })
            } else {
                let elem_ty = array_ty.array_deref();

                array = array.cast(
                    &array.loc(),
                    if array_ty.deref_memory().is_fixed_reference_type() {
                        &array_ty
                    } else {
                        array_ty.deref_any()
                    },
                    true,
                    ns,
                    diagnostics,
                )?;

                Ok(Expression::Subscript {
                    loc: *loc,
                    ty: elem_ty,
                    array_ty,
                    array: Box::new(array),
                    index: Box::new(index),
                })
            }
        }
        Type::String => {
            diagnostics.push(Diagnostic::error(
                array.loc(),
                "array subscript is not permitted on string".to_string(),
            ));
            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                array.loc(),
                "expression is not an array".to_string(),
            ));
            Err(())
        }
    }
}

/// Traverse the literal looking for sub arrays. Ensure that all the sub
/// arrays are the same length, and returned a flattened array of elements
fn check_subarrays<'a>(
    exprs: &'a [pt::Expression],
    dims: &mut Option<&mut Vec<u32>>,
    flatten: &mut Vec<&'a pt::Expression>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    if let Some(pt::Expression::ArrayLiteral(_, first)) = exprs.get(0) {
        // ensure all elements are array literals of the same length
        check_subarrays(first, dims, flatten, diagnostics)?;

        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(_, other) = e {
                if other.len() != first.len() {
                    diagnostics.push(Diagnostic::error(
                        e.loc(),
                        format!(
                            "array elements should be identical, sub array {} has {} elements rather than {}", i + 1, other.len(), first.len()
                        ),
                    ));
                    return Err(());
                }
                check_subarrays(other, &mut None, flatten, diagnostics)?;
            } else {
                diagnostics.push(Diagnostic::error(
                    e.loc(),
                    format!("array element {} should also be an array", i + 1),
                ));
                return Err(());
            }
        }
    } else {
        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(loc, _) = e {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "array elements should be of the type, element {} is unexpected array",
                        i + 1
                    ),
                ));
                return Err(());
            }
        }
        flatten.extend(exprs);
    }

    if let Some(dims) = dims.as_deref_mut() {
        dims.push(exprs.len() as u32);
    }

    Ok(())
}

/// Is it an (new C).value(1).gas(2)(1, 2, 3) style constructor (not supported)?
fn deprecated_constructor_arguments(
    expr: &pt::Expression,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match expr.remove_parenthesis() {
        pt::Expression::FunctionCall(func_loc, ty, _) => {
            if let pt::Expression::MemberAccess(_, ty, call_arg) = ty.as_ref() {
                if deprecated_constructor_arguments(ty, diagnostics).is_err() {
                    // location should be the identifier and the arguments
                    let mut loc = call_arg.loc;
                    if let pt::Loc::File(_, _, end) = &mut loc {
                        *end = func_loc.end();
                    }
                    diagnostics.push(Diagnostic::error(
                        loc,
                        format!("deprecated call argument syntax '.{}(...)' is not supported, use '{{{}: ...}}' instead", call_arg.name, call_arg.name)
                    ));
                    return Err(());
                }
            }
        }
        pt::Expression::New(..) => {
            return Err(());
        }
        _ => (),
    }

    Ok(())
}

/// Calculate storage subscript
fn mapping_subscript(
    loc: &pt::Loc,
    mapping: Expression,
    index: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let ty = mapping.ty();
    let elem_ty = ty.storage_array_elem();

    if let Type::Mapping(key_ty, _) = ty.deref_any() {
        let index_expr = expression(
            index,
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(key_ty),
        )?
        .cast(&index.loc(), key_ty, true, ns, diagnostics)?;

        Ok(Expression::Subscript {
            loc: *loc,
            ty: elem_ty,
            array_ty: ty,
            array: Box::new(mapping),
            index: Box::new(index_expr),
        })
    } else {
        unreachable!()
    }
}
