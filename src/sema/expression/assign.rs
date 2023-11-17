// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, Namespace, RetrieveType, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::integers::type_bits_and_sign;
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{assigned_variable, used_variable};
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

/// Resolve an assignment
pub(super) fn assign_single(
    loc: &pt::Loc,
    left: &pt::Expression,
    right: &pt::Expression,
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

    let var = expression(
        left,
        &mut context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    assigned_variable(ns, &var, symtable);

    context.lvalue = false;
    let var_ty = var.ty();
    let val = expression(
        right,
        &mut context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(var_ty.deref_any()),
    )?;

    val.check_constant_overflow(diagnostics);

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
            // If the variable is a Type::Ref(Type::Ref(..)), we must load it.
            Type::Ref(inner) if matches!(**inner, Type::Ref(_)) => Ok(Expression::Assign {
                loc: *loc,
                ty: inner.deref_memory().clone(),
                left: Box::new(var.cast(loc, inner, true, ns, diagnostics)?),
                right: Box::new(val.cast(
                    &right.loc(),
                    inner.deref_memory(),
                    true,
                    ns,
                    diagnostics,
                )?),
            }),
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
pub(super) fn assign_expr(
    loc: &pt::Loc,
    left: &pt::Expression,
    expr: &pt::Expression,
    right: &pt::Expression,
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

    let var = expression(
        left,
        &mut context,
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
        ResolveTo::Type(var_ty.deref_any().deref_any())
    };

    context.lvalue = false;
    let set = expression(right, &mut context, ns, symtable, diagnostics, resolve_to)?;
    used_variable(ns, &set, symtable);
    let set_type = set.ty();

    let assign_operation = |assign: Expression,
                            ty: &Type,
                            ns: &Namespace,
                            diagnostics: &mut Diagnostics|
     -> Result<Expression, ()> {
        let set = match expr {
            pt::Expression::AssignShiftLeft(..) | pt::Expression::AssignShiftRight(..) => {
                let left_length = type_bits_and_sign(ty, loc, true, ns, diagnostics)?;
                let right_length =
                    type_bits_and_sign(&set_type, &left.loc(), false, ns, diagnostics)?;

                // TODO: does shifting by negative value need compiletime/runtime check?
                if left_length == right_length {
                    set
                } else if right_length < left_length && set_type.is_signed_int(ns) {
                    Expression::SignExt {
                        loc: *loc,
                        to: ty.clone(),
                        expr: Box::new(set),
                    }
                } else if right_length < left_length && !set_type.is_signed_int(ns) {
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
                sign: ty.is_signed_int(ns),
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
                // If the variable is a Type::Ref(Type::Ref(..)), we must load it first.
                Type::Ref(inner)
                    if matches!(**inner, Type::Bytes(_) | Type::Int(_) | Type::Uint(_)) =>
                {
                    Ok(Expression::Assign {
                        loc: *loc,
                        ty: *inner.clone(),
                        left: Box::new(var.cast(loc, r_ty, true, ns, diagnostics)?),
                        right: Box::new(assign_operation(
                            var.cast(loc, inner, true, ns, diagnostics)?,
                            inner,
                            ns,
                            diagnostics,
                        )?),
                    })
                }
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
