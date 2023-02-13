// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Builtin, Expression, Namespace, RetrieveType, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::eval::{check_term_for_constant_overflow, eval_const_rational};
use crate::sema::expression::arithmetic::{
    addition, bitwise_and, bitwise_or, bitwise_xor, divide, equal, incr_decr, modulo, multiply,
    power, shift_left, shift_right, subtract,
};
use crate::sema::expression::assign::{assign_expr, assign_single};
use crate::sema::expression::constructor::{constructor_named_args, new};
use crate::sema::expression::function_call::{call_expr, named_call_expr};
use crate::sema::expression::integers::{
    bigint_to_expression, coerce, coerce_number, get_int_length,
};
use crate::sema::expression::literals::{
    address_literal, array_literal, hex_literal, hex_number_literal, number_literal,
    rational_number_literal, string_literal,
};
use crate::sema::expression::member_access::member_access;
use crate::sema::expression::subscript::array_subscript;
use crate::sema::expression::variable::variable;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{
    check_function_call, check_var_usage_expression, used_variable,
};
use crate::sema::Recurse;
use num_bigint::BigInt;
use num_traits::{Num, One};
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

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

                bigint_to_expression(loc, &-n, ns, diagnostics, resolve_to, Some(s.len()))
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
                    bigint_to_expression(loc, &-value, ns, diagnostics, resolve_to, None)
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
