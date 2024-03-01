// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, Namespace, RetrieveType, StringLocation, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::eval::eval_const_rational;
use crate::sema::expression::integers::{coerce, coerce_number, type_bits_and_sign};
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{user_defined_operator, ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{check_var_usage_expression, used_variable};
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

pub(super) fn subtract(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Subtract,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        let expr = Expression::Subtract {
            loc: *loc,
            ty,
            unchecked: false,
            left: Box::new(left),
            right: Box::new(right),
        };

        return match eval_const_rational(&expr, ns) {
            Ok(_) => Ok(expr),
            Err(diag) => {
                diagnostics.push(diag);
                Err(())
            }
        };
    }

    Ok(Expression::Subtract {
        loc: *loc,
        ty: ty.clone(),
        unchecked: context.unchecked,
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn bitwise_or(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::BitwiseOr,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseOr {
        loc: *loc,
        ty: ty.clone(),
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn bitwise_and(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::BitwiseAnd,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseAnd {
        loc: *loc,
        ty: ty.clone(),
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn bitwise_xor(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::BitwiseXor,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseXor {
        loc: *loc,
        ty: ty.clone(),
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn shift_left(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);
    // left hand side may be bytes/int/uint
    // right hand size may be int/uint
    let _ = type_bits_and_sign(&left.ty(), &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = type_bits_and_sign(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    let left_type = left.ty().deref_any().clone();
    let right_type = right.ty().deref_any().clone();

    Ok(Expression::ShiftLeft {
        loc: *loc,
        ty: left_type.clone(),
        left: Box::new(left.cast(loc, &left_type, true, ns, diagnostics)?),
        right: Box::new(cast_shift_arg(
            loc,
            right.cast(loc, &right_type, true, ns, diagnostics)?,
            right_length,
            &left_type,
            ns,
        )),
    })
}

pub(super) fn shift_right(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let left_type = left.ty().deref_any().clone();
    let right_type = right.ty().deref_any().clone();
    // left hand side may be bytes/int/uint
    // right hand size may be int/uint
    let _ = type_bits_and_sign(&left_type, &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = type_bits_and_sign(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    Ok(Expression::ShiftRight {
        loc: *loc,
        ty: left_type.clone(),
        left: Box::new(left.cast(loc, &left_type, true, ns, diagnostics)?),
        right: Box::new(cast_shift_arg(
            loc,
            right.cast(loc, &right_type, true, ns, diagnostics)?,
            right_length,
            &left_type,
            ns,
        )),
        sign: left_type.is_signed_int(ns),
    })
}

pub(super) fn multiply(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Multiply,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        let expr = Expression::Multiply {
            loc: *loc,
            ty,
            unchecked: false,
            left: Box::new(left),
            right: Box::new(right),
        };

        return match eval_const_rational(&expr, ns) {
            Ok(_) => Ok(expr),
            Err(diag) => {
                diagnostics.push(diag);
                Err(())
            }
        };
    }

    // If we don't know what type the result is going to be, make any possible result fit.
    if resolve_to == ResolveTo::Unknown {
        let bits = std::cmp::min(256, ty.bits(ns) * 2);

        if ty.is_signed_int(ns) {
            multiply(
                loc,
                l,
                r,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Int(bits)),
            )
        } else {
            multiply(
                loc,
                l,
                r,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Uint(bits)),
            )
        }
    } else {
        Ok(Expression::Multiply {
            loc: *loc,
            ty: ty.clone(),
            unchecked: context.unchecked,
            left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
            right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
        })
    }
}

pub(super) fn divide(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Divide,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Divide {
        loc: *loc,
        ty: ty.clone(),
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn modulo(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Modulo,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Modulo {
        loc: *loc,
        ty: ty.clone(),
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

pub(super) fn power(
    loc: &pt::Loc,
    b: &pt::Expression,
    e: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut base = expression(b, context, ns, symtable, diagnostics, resolve_to)?;

    // If we don't know what type the result is going to be, assume
    // the result is 256 bits
    if resolve_to == ResolveTo::Unknown {
        if base.ty().is_signed_int(ns) {
            base = expression(
                b,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Int(256)),
            )?;
        } else {
            base = expression(
                b,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Uint(256)),
            )?;
        };
    }

    let exp = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &base, &exp, symtable);

    let base_type = base.ty();
    let exp_type = exp.ty();

    // solc-0.5.13 does not allow either base or exp to be signed
    if base_type.is_signed_int(ns) || exp_type.is_signed_int(ns) {
        diagnostics.push(Diagnostic::error(
            *loc,
            "exponation (**) is not allowed with signed types".to_string(),
        ));
        return Err(());
    }

    let ty = coerce_number(
        &base_type,
        &b.loc(),
        &exp_type,
        &e.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Power {
        loc: *loc,
        ty: ty.clone(),
        unchecked: context.unchecked,
        base: Box::new(base.cast(&b.loc(), &ty, true, ns, diagnostics)?),
        exp: Box::new(exp.cast(&e.loc(), &ty, true, ns, diagnostics)?),
    })
}

/// Test for equality; first check string equality, then integer equality
pub(super) fn equal(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Equal,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let left_type = left.ty();
    let right_type = right.ty();

    if let Some(expr) =
        is_string_equal(loc, &left, &left_type, &right, &right_type, ns, diagnostics)?
    {
        return Ok(expr);
    }

    let ty = coerce(
        &left_type,
        &left.loc(),
        &right_type,
        &right.loc(),
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot use rational numbers with '==' operator".into(),
        ));
        return Err(());
    }

    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Type(&ty))?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Type(&ty))?;

    let expr = Expression::Equal {
        loc: *loc,
        left: Box::new(left.cast(&left.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&right.loc(), &ty, true, ns, diagnostics)?),
    };

    Ok(expr)
}

pub(super) fn not_equal(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::NotEqual,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let left_type = left.ty();
    let right_type = right.ty();

    if let Some(expr) =
        is_string_equal(loc, &left, &left_type, &right, &right_type, ns, diagnostics)?
    {
        return Ok(Expression::Not {
            loc: *loc,
            expr: expr.into(),
        });
    }

    let ty = coerce(
        &left_type,
        &left.loc(),
        &right_type,
        &right.loc(),
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot use rational numbers with '!=' operator".into(),
        ));
        return Err(());
    }

    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Type(&ty))?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Type(&ty))?;

    let expr = Expression::NotEqual {
        loc: *loc,
        left: Box::new(left.cast(&left.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&right.loc(), &ty, true, ns, diagnostics)?),
    };

    Ok(expr)
}

/// If the left and right arguments are part of string comparison, return
/// a string comparision expression, else None.
fn is_string_equal(
    loc: &pt::Loc,
    left: &Expression,
    left_type: &Type,
    right: &Expression,
    right_type: &Type,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    // compare string against literal
    match (&left, &right_type.deref_any()) {
        (Expression::BytesLiteral { value: l, .. }, Type::String)
        | (Expression::BytesLiteral { value: l, .. }, Type::DynamicBytes) => {
            return Ok(Some(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(right.cast(
                    &right.loc(),
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::CompileTime(l.clone()),
            }));
        }
        _ => {}
    }

    match (&right, &left_type.deref_any()) {
        (Expression::BytesLiteral { value, .. }, Type::String)
        | (Expression::BytesLiteral { value, .. }, Type::DynamicBytes) => {
            return Ok(Some(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(left.cast(
                    &left.loc(),
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::CompileTime(value.clone()),
            }));
        }
        _ => {}
    }

    // compare string
    match (&left_type.deref_any(), &right_type.deref_any()) {
        (Type::String, Type::String) | (Type::DynamicBytes, Type::DynamicBytes) => {
            return Ok(Some(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(left.cast(
                    &left.loc(),
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::RunTime(Box::new(right.cast(
                    &right.loc(),
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
            }));
        }
        _ => {}
    }

    Ok(None)
}

/// Try string concatenation
pub(super) fn addition(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    if let Some(expr) = user_defined_operator(
        loc,
        &[&left, &right],
        pt::UserDefinedOperator::Add,
        diagnostics,
        ns,
    ) {
        return Ok(expr);
    }

    let left_type = left.ty();
    let right_type = right.ty();

    // Solang 0.3.3 and earlier supported + for concatenating strings/bytes. Give a specific error
    // saying this must be done using string.concat() and bytes.concat() builtin.
    match (&left_type, &right_type) {
        (Type::DynamicBytes | Type::Bytes(_), Type::DynamicBytes | Type::Bytes(_)) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "concatenate bytes using the builtin bytes.concat(a, b)".into(),
            ));
            return Err(());
        }
        (Type::String, Type::String) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "concatenate string using the builtin string.concat(a, b)".into(),
            ));
            return Err(());
        }
        _ => (),
    }

    let ty = coerce_number(
        &left_type,
        &l.loc(),
        &right_type,
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        let expr = Expression::Add {
            loc: *loc,
            ty,
            unchecked: false,
            left: Box::new(left),
            right: Box::new(right),
        };

        return match eval_const_rational(&expr, ns) {
            Ok(_) => Ok(expr),
            Err(diag) => {
                diagnostics.push(diag);
                Err(())
            }
        };
    }

    Ok(Expression::Add {
        loc: *loc,
        ty: ty.clone(),
        unchecked: context.unchecked,
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    })
}

/// Resolve an increment/decrement with an operator
pub(super) fn incr_decr(
    v: &pt::Expression,
    expr: &pt::Expression,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let prev_lvalue = context.lvalue;
    context.lvalue = true;

    let mut context = scopeguard::guard(context, |context| {
        context.lvalue = prev_lvalue;
    });
    let unchecked = context.unchecked;

    let op = |e: Expression, ty: Type| -> Expression {
        match expr {
            pt::Expression::PreIncrement(loc, _) => Expression::PreIncrement {
                loc: *loc,
                ty,
                unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PreDecrement(loc, _) => Expression::PreDecrement {
                loc: *loc,
                ty,
                unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PostIncrement(loc, _) => Expression::PostIncrement {
                loc: *loc,
                ty,
                unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PostDecrement(loc, _) => Expression::PostDecrement {
                loc: *loc,
                ty,
                unchecked,
                expr: Box::new(e),
            },
            _ => unreachable!(),
        }
    };

    let var = expression(
        v,
        &mut context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    used_variable(ns, &var, symtable);
    let var_ty = var.ty();

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
        Expression::Variable { ty, var_no, .. } => {
            match ty {
                Type::Int(_) | Type::Uint(_) => (),
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
            Ok(op(var.clone(), ty.clone()))
        }
        _ => match &var_ty {
            Type::Ref(r_ty) => match r_ty.as_ref() {
                Type::Int(_) | Type::Uint(_) => Ok(op(var, r_ty.as_ref().clone())),
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
                                var.loc(),
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }
                match r_ty.as_ref() {
                    Type::Int(_) | Type::Uint(_) => Ok(op(var, r_ty.as_ref().clone())),
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
                    "expression is not modifiable".to_string(),
                ));
                Err(())
            }
        },
    }
}

// When generating shifts, llvm wants both arguments to have the same width. We want the
// result of the shift to be left argument, so this function coercies the right argument
// into the right length.
pub fn cast_shift_arg(
    loc: &pt::Loc,
    expr: Expression,
    from_width: u16,
    ty: &Type,
    ns: &Namespace,
) -> Expression {
    let to_width = ty.bits(ns);

    if from_width == to_width {
        expr
    } else if from_width < to_width && ty.is_signed_int(ns) {
        Expression::SignExt {
            loc: *loc,
            to: ty.clone(),
            expr: Box::new(expr),
        }
    } else if from_width < to_width && !ty.is_signed_int(ns) {
        Expression::ZeroExt {
            loc: *loc,
            to: ty.clone(),
            expr: Box::new(expr),
        }
    } else {
        Expression::Trunc {
            loc: *loc,
            to: ty.clone(),
            expr: Box::new(expr),
        }
    }
}
