// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, Namespace, RetrieveType, StringLocation, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::eval::eval_const_rational;
use crate::sema::expression::integers::{coerce, coerce_number, get_int_length};
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{check_var_usage_expression, used_variable};
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

pub(super) fn subtract(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

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

    if ty.is_rational(ns) {
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

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
    context: &ExprContext,
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
    let _ = get_int_length(&left.ty(), &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    let left_type = left.ty().deref_any().clone();

    Ok(Expression::ShiftLeft {
        loc: *loc,
        ty: left_type.clone(),
        left: Box::new(left.cast(loc, &left_type, true, ns, diagnostics)?),
        right: Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
    })
}

pub(super) fn shift_right(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let left_type = left.ty().deref_any().clone();
    // left hand side may be bytes/int/uint
    // right hand size may be int/uint
    let _ = get_int_length(&left_type, &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    Ok(Expression::ShiftRight {
        loc: *loc,
        ty: left_type.clone(),
        left: Box::new(left.cast(loc, &left_type, true, ns, diagnostics)?),
        right: Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
        sign: left_type.is_signed_int(),
    })
}

pub(super) fn multiply(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

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

    if ty.is_rational(ns) {
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

        if ty.is_signed_int() {
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut base = expression(b, context, ns, symtable, diagnostics, resolve_to)?;

    // If we don't know what type the result is going to be, assume
    // the result is 256 bits
    if resolve_to == ResolveTo::Unknown {
        if base.ty().is_signed_int() {
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
    if base_type.is_signed_int() || exp_type.is_signed_int() {
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Integer)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Integer)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    // Comparing stringliteral against stringliteral
    if let (Expression::BytesLiteral { value: l, .. }, Expression::BytesLiteral { value: r, .. }) =
        (&left, &right)
    {
        return Ok(Expression::BoolLiteral {
            loc: *loc,
            value: l == r,
        });
    }

    let left_type = left.ty();
    let right_type = right.ty();

    // compare string against literal
    match (&left, &right_type.deref_any()) {
        (Expression::BytesLiteral { value: l, .. }, Type::String)
        | (Expression::BytesLiteral { value: l, .. }, Type::DynamicBytes) => {
            return Ok(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(right.cast(
                    &r.loc(),
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::CompileTime(l.clone()),
            });
        }
        _ => {}
    }

    match (&right, &left_type.deref_any()) {
        (Expression::BytesLiteral { value, .. }, Type::String)
        | (Expression::BytesLiteral { value, .. }, Type::DynamicBytes) => {
            return Ok(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(left.cast(
                    &l.loc(),
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::CompileTime(value.clone()),
            });
        }
        _ => {}
    }

    // compare string
    match (&left_type.deref_any(), &right_type.deref_any()) {
        (Type::String, Type::String) | (Type::DynamicBytes, Type::DynamicBytes) => {
            return Ok(Expression::StringCompare {
                loc: *loc,
                left: StringLocation::RunTime(Box::new(left.cast(
                    &l.loc(),
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                right: StringLocation::RunTime(Box::new(right.cast(
                    &r.loc(),
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
            });
        }
        _ => {}
    }

    let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, diagnostics)?;

    let expr = Expression::Equal {
        loc: *loc,
        left: Box::new(left.cast(&l.loc(), &ty, true, ns, diagnostics)?),
        right: Box::new(right.cast(&r.loc(), &ty, true, ns, diagnostics)?),
    };

    if ty.is_rational(ns) {
        if let Err(diag) = eval_const_rational(&expr, ns) {
            diagnostics.push(diag);
        }
    }

    Ok(expr)
}

/// Try string concatenation
pub(super) fn addition(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let mut right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;
    check_var_usage_expression(ns, &left, &right, symtable);

    // Concatenate stringliteral with stringliteral
    if let (Expression::BytesLiteral { value: l, .. }, Expression::BytesLiteral { value: r, .. }) =
        (&left, &right)
    {
        let mut c = Vec::with_capacity(l.len() + r.len());
        c.extend_from_slice(l);
        c.extend_from_slice(r);
        let length = c.len();
        return Ok(Expression::BytesLiteral {
            loc: *loc,
            ty: Type::Bytes(length as u8),
            value: c,
        });
    }

    let left_type = left.ty();
    let right_type = right.ty();

    // compare string against literal
    match (&left, &right_type) {
        (Expression::BytesLiteral { value, .. }, Type::String)
        | (Expression::BytesLiteral { value, .. }, Type::DynamicBytes) => {
            return Ok(Expression::StringConcat {
                loc: *loc,
                ty: right_type,
                left: StringLocation::CompileTime(value.clone()),
                right: StringLocation::RunTime(Box::new(right)),
            });
        }
        _ => {}
    }

    match (&right, &left_type) {
        (Expression::BytesLiteral { value, .. }, Type::String)
        | (Expression::BytesLiteral { value, .. }, Type::DynamicBytes) => {
            return Ok(Expression::StringConcat {
                loc: *loc,
                ty: left_type,
                left: StringLocation::RunTime(Box::new(left)),
                right: StringLocation::CompileTime(value.clone()),
            });
        }
        _ => {}
    }

    // compare string
    match (&left_type, &right_type) {
        (Type::String, Type::String) | (Type::DynamicBytes, Type::DynamicBytes) => {
            return Ok(Expression::StringConcat {
                loc: *loc,
                ty: right_type,
                left: StringLocation::RunTime(Box::new(left)),
                right: StringLocation::RunTime(Box::new(right)),
            });
        }
        _ => {}
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

    if ty.is_rational(ns) {
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

    // If we don't know what type the result is going to be
    if resolve_to == ResolveTo::Unknown {
        let bits = std::cmp::min(256, ty.bits(ns) * 2);
        let resolve_to = if ty.is_signed_int() {
            Type::Int(bits)
        } else {
            Type::Uint(bits)
        };

        left = expression(
            l,
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(&resolve_to),
        )?;
        right = expression(
            r,
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(&resolve_to),
        )?;
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let op = |e: Expression, ty: Type| -> Expression {
        match expr {
            pt::Expression::PreIncrement(loc, _) => Expression::PreIncrement {
                loc: *loc,
                ty,
                unchecked: context.unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PreDecrement(loc, _) => Expression::PreDecrement {
                loc: *loc,
                ty,
                unchecked: context.unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PostIncrement(loc, _) => Expression::PostIncrement {
                loc: *loc,
                ty,
                unchecked: context.unchecked,
                expr: Box::new(e),
            },
            pt::Expression::PostDecrement(loc, _) => Expression::PostDecrement {
                loc: *loc,
                ty,
                unchecked: context.unchecked,
                expr: Box::new(e),
            },
            _ => unreachable!(),
        }
    };

    let mut context = context.clone();

    context.lvalue = true;

    let var = expression(v, &context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
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
    } else if from_width < to_width && ty.is_signed_int() {
        Expression::SignExt {
            loc: *loc,
            to: ty.clone(),
            expr: Box::new(expr),
        }
    } else if from_width < to_width && !ty.is_signed_int() {
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
