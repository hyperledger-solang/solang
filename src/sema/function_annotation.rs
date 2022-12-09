// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{ConstructorAnnotation, Diagnostic, Namespace, Type},
    diagnostics::Diagnostics,
    expression::{expression, ExprContext, ResolveTo},
    unused_variable::used_variable,
    Symtable,
};
use crate::Target;
use solang_parser::pt::{self, CodeLocation};

/// Collect the seeds, bump, payer, and space for constructors. This is a no-op on Substrate/EVM since
/// there should be no seed or bump annotations permitted on other targets.
pub fn function_annotations(
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
