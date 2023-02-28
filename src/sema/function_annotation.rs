// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{ConstructorAnnotation, Diagnostic, Expression, Function, Namespace, Type},
    diagnostics::Diagnostics,
    eval::overflow_check,
    expression::literals::{hex_number_literal, unit_literal},
    expression::{ExprContext, ResolveTo},
    unused_variable::used_variable,
    Symtable,
};
use crate::sema::expression::literals::number_literal;
use crate::sema::expression::resolve_expression::expression;
use crate::Target;
use num_traits::ToPrimitive;
use solang_parser::pt::{self, CodeLocation};

/// Resolve the prototype annotation for functions (just the selector). These
/// annotations can be resolved for functions without a body. This means they
/// do not need to access the symbol table, like `@seed(foo)` annotations do.
pub fn function_prototype_annotations(
    func: &mut Function,
    annotations: &[&pt::Annotation],
    ns: &mut Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    for annotation in annotations {
        match annotation.id.name.as_str() {
            "selector" => function_selector(func, annotation, &mut diagnostics, ns),
            _ if !func.has_body => {
                // function_body_annotations() is called iff there is a body
                diagnostics.push(Diagnostic::error(
                    annotation.loc,
                    format!(
                        "annotation '@{}' not allowed on {} with no body",
                        annotation.id.name, func.ty
                    ),
                ));
            }
            _ => {
                // handled in function_body_annotations()
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

/// Parse the selector from an annotation and assign it to the function
fn function_selector(
    func: &mut Function,
    annotation: &pt::Annotation,
    diagnostics: &mut Diagnostics,
    ns: &mut Namespace,
) {
    if func.ty != pt::FunctionTy::Function
        && (!ns.target.is_substrate() || func.ty != pt::FunctionTy::Constructor)
    {
        diagnostics.push(Diagnostic::error(
            annotation.loc,
            format!("overriding selector not permitted on {}", func.ty),
        ));
        return;
    }

    if !func.is_public() {
        diagnostics.push(Diagnostic::error(
            annotation.loc,
            format!(
                "overriding selector only permitted on 'public' or 'external' function, not '{}'",
                func.visibility
            ),
        ));
        return;
    }

    if let Some((prev, _)) = &func.selector {
        diagnostics.push(Diagnostic::error_with_note(
            annotation.loc,
            format!("duplicate @selector annotation for {}", func.ty),
            *prev,
            "previous @selector".into(),
        ));
        return;
    }

    match &annotation.value {
        pt::Expression::ArrayLiteral(_, values) => {
            let mut selector = Vec::new();

            for expr in values {
                let uint8 = Type::Uint(8);

                let expr = match expr {
                    pt::Expression::HexNumberLiteral(loc, n, None) => {
                        hex_number_literal(loc, n, ns, diagnostics, ResolveTo::Type(&uint8))
                    }
                    pt::Expression::NumberLiteral(loc, base, exp, unit) => {
                        let unit = unit_literal(loc, unit, ns, diagnostics);

                        number_literal(
                            loc,
                            base,
                            exp,
                            ns,
                            &unit,
                            diagnostics,
                            ResolveTo::Type(&uint8),
                        )
                    }
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            expr.loc(),
                            "literal number expected".into(),
                        ));
                        continue;
                    }
                };

                if let Ok(Expression::NumberLiteral { loc, value, .. }) = &expr {
                    if let Some(diagnostic) = overflow_check(value, &uint8, loc) {
                        diagnostics.push(diagnostic);
                    } else {
                        selector.push(value.to_u8().unwrap());
                    }
                } else {
                    // Diagnostic already generated
                    assert!(expr.is_err());
                }
            }

            if !diagnostics.any_errors() {
                func.selector = Some((annotation.loc, selector));
            }
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                annotation.value.loc(),
                "expression must be an array literal".into(),
            ));
        }
    }
}

/// Collect the seeds, bump, payer, and space for constructors. This is a no-op on Substrate/EVM since
/// there should be no seed or bump annotations permitted on other targets.
///
/// These annotations need a symbol table.
pub fn function_body_annotations(
    function_no: usize,
    annotations: &[&pt::Annotation],
    symtable: &mut Symtable,
    context: &ExprContext,
    ns: &mut Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    // On Solana, the seeds and bump for a constructor can be specified using annotations, for example
    //
    // @seed(param1)
    // @seed("fizbaz")
    // @bump(param2)
    // constructor(bytes param1, uint8 param2) {}

    let mut resolved_annotations = Vec::new();
    let mut bump = None;
    let mut space = None;
    let mut payer = None;

    let is_solana_constructor =
        ns.target == Target::Solana && ns.functions[function_no].ty == pt::FunctionTy::Constructor;

    for note in annotations {
        match note.id.name.as_str() {
            "selector" => {
                // selectors already done in function_prototype_annotations
                // without using a symbol table
            }
            "seed" if is_solana_constructor => {
                let ty = Type::Slice(Box::new(Type::Bytes(1)));
                let loc = note.loc;

                if let Ok(expr) = expression(
                    &note.value,
                    context,
                    ns,
                    symtable,
                    &mut diagnostics,
                    ResolveTo::Type(&ty),
                ) {
                    if let Ok(expr) = expr.cast(&expr.loc(), &ty, true, ns, &mut diagnostics) {
                        if let Some(prev) = &bump {
                            diagnostics.push(Diagnostic::error_with_note(
                                *prev,
                                "@bump should be after the last @seed".into(),
                                loc,
                                "location of @seed annotation".into(),
                            ));
                        } else {
                            used_variable(ns, &expr, symtable);
                            resolved_annotations.push(ConstructorAnnotation::Seed(expr));
                        }
                    }
                }
            }
            "bump" if is_solana_constructor => {
                let ty = Type::Bytes(1);
                let loc = note.loc;

                if let Ok(expr) = expression(
                    &note.value,
                    context,
                    ns,
                    symtable,
                    &mut diagnostics,
                    ResolveTo::Type(&ty),
                ) {
                    if let Ok(expr) = expr.cast(&expr.loc(), &ty, true, ns, &mut diagnostics) {
                        if let Some(prev) = &bump {
                            diagnostics.push(Diagnostic::error_with_note(
                                expr.loc(),
                                "duplicate @bump annotation for constructor".into(),
                                *prev,
                                "previous @bump".into(),
                            ));
                        } else {
                            bump = Some(loc);
                            used_variable(ns, &expr, symtable);
                            resolved_annotations.push(ConstructorAnnotation::Bump(expr));
                        }
                    }
                }
            }
            "space" if is_solana_constructor => {
                let ty = Type::Uint(64);
                let loc = note.loc;

                if let Ok(expr) = expression(
                    &note.value,
                    context,
                    ns,
                    symtable,
                    &mut diagnostics,
                    ResolveTo::Type(&ty),
                ) {
                    if let Ok(expr) = expr.cast(&expr.loc(), &ty, true, ns, &mut diagnostics) {
                        if let Some(prev) = &space {
                            diagnostics.push(Diagnostic::error_with_note(
                                loc,
                                "duplicate @space annotation for constructor".into(),
                                *prev,
                                "previous @space".into(),
                            ));
                        } else {
                            space = Some(loc);
                            used_variable(ns, &expr, symtable);
                            resolved_annotations.push(ConstructorAnnotation::Space(expr));
                        }
                    }
                }
            }
            "payer" if is_solana_constructor => {
                let ty = Type::Address(false);
                let loc = note.loc;

                if let Ok(expr) = expression(
                    &note.value,
                    context,
                    ns,
                    symtable,
                    &mut diagnostics,
                    ResolveTo::Type(&ty),
                ) {
                    if let Ok(expr) = expr.cast(&expr.loc(), &ty, true, ns, &mut diagnostics) {
                        if let Some(prev) = &payer {
                            diagnostics.push(Diagnostic::error_with_note(
                                loc,
                                "duplicate @payer annotation for constructor".into(),
                                *prev,
                                "previous @payer".into(),
                            ));
                        } else {
                            payer = Some(loc);
                            used_variable(ns, &expr, symtable);
                            resolved_annotations.push(ConstructorAnnotation::Payer(expr));
                        }
                    }
                }
            }
            _ => diagnostics.push(Diagnostic::error(
                note.loc,
                format!(
                    "unknown annotation {} for {}",
                    note.id.name, ns.functions[function_no].ty
                ),
            )),
        };
    }

    if !resolved_annotations.is_empty() && diagnostics.is_empty() && payer.is_none() {
        diagnostics.push(Diagnostic::error(
            resolved_annotations[0].loc(),
            "@payer annotation required for constructor".into(),
        ));
    }

    ns.diagnostics.extend(diagnostics);

    ns.functions[function_no].annotations = resolved_annotations;
}
